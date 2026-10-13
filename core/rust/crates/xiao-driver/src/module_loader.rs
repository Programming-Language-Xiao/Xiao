//! 驱动器侧的只读模块索引与统一前端编译适配器。

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use xiao_bytecode::{lower_program, verify_for_execution};
use xiao_modules::{ModuleName, ProjectModuleResult, analyze_project};
use xiao_vm::{CompiledModule, ModuleLoader, STANDARD_LIBRARY_NAMESPACE};

use crate::frontend::{FrontendCompiler, FrontendContext, FrontendRequest};
use crate::packages::PackageRegistry;

#[derive(Debug)]
/// 驱动器侧项目、包和官方标准库的统一按需模块加载器。
pub(crate) struct DriverModuleLoader<'a> {
    project: Option<ProjectModuleResult>,
    packages: Option<&'a PackageRegistry>,
    standard_library_root: PathBuf,
    standard_library: OnceLock<Option<ProjectModuleResult>>,
}

impl<'a> DriverModuleLoader<'a> {
    /// 创建带项目根、包注册表和标准库源码根的加载器。
    pub(crate) fn new(root: Option<&Path>, packages: Option<&'a PackageRegistry>) -> Self {
        Self {
            project: root.map(analyze_project),
            packages,
            standard_library_root: standard_library_root(),
            standard_library: OnceLock::new(),
        }
    }

    /// 按需读取已校验的官方标准库模块索引。
    fn standard_library(&self) -> Option<&ProjectModuleResult> {
        self.standard_library
            .get_or_init(|| load_standard_library(&self.standard_library_root))
            .as_ref()
    }

    /// 将模块身份解析为源码路径、编译根和导出名称。
    fn descriptor(&self, identity: &str) -> Option<(PathBuf, PathBuf, Vec<String>)> {
        if let Some(name) = identity.strip_prefix("project:") {
            let project = self.project.as_ref()?;
            let record = project
                .modules
                .get(&ModuleName::new(name.split('.').map(str::to_owned)))?;
            return Some((
                record.path.clone(),
                project.project_root.clone(),
                record.symbols.keys().cloned().collect(),
            ));
        }
        if let Some(name) = identity.strip_prefix("stdlib.") {
            let standard_library = self.standard_library()?;
            let record = standard_library
                .modules
                .get(&ModuleName::new(name.split('.').map(str::to_owned)))?;
            return Some((
                record.path.clone(),
                standard_library.project_root.clone(),
                record.symbols.keys().cloned().collect(),
            ));
        }
        let name = identity.strip_prefix("package:")?;
        let module = self.packages?.modules.get(name)?;
        Some((
            module.source.clone(),
            module.project_root.clone(),
            module.exports.iter().cloned().collect(),
        ))
    }

    /// 判断项目、包或官方库命名空间是否存在。
    fn namespace_exists(&self, identity: &str) -> bool {
        if identity == STANDARD_LIBRARY_NAMESPACE {
            return self.standard_library().is_some();
        }
        if let Some(name) = identity.strip_prefix("stdlib.") {
            return self.standard_library().is_some_and(|project| {
                project
                    .namespaces
                    .contains_key(&ModuleName::new(name.split('.').map(str::to_owned)))
            });
        }
        if let Some(name) = identity.strip_prefix("project:") {
            return self.project.as_ref().is_some_and(|project| {
                name.is_empty()
                    || project
                        .namespaces
                        .contains_key(&ModuleName::new(name.split('.').map(str::to_owned)))
            });
        }
        identity.strip_prefix("package:").is_some_and(|name| {
            self.packages
                .is_some_and(|registry| registry.namespaces.members.contains_key(name))
        })
    }
}

/// 定位工具链随附的标准库源码根目录。
pub(crate) fn standard_library_root() -> PathBuf {
    xiao_standard_library::source_root()
}

/// 读取并校验工具链随附的标准库源码索引。
fn load_standard_library(root: &Path) -> Option<ProjectModuleResult> {
    let project = analyze_project(root);
    project.is_success().then_some(project)
}

impl ModuleLoader for DriverModuleLoader<'_> {
    /// 判断身份是否指向文件模块或目录命名空间。
    fn contains(&self, identity: &str) -> bool {
        self.descriptor(identity).is_some() || self.namespace_exists(identity)
    }

    /// 首次引用时编译并验证指定模块；命名空间本身返回空结果。
    fn compile(&self, identity: &str) -> Result<Option<CompiledModule>, String> {
        let Some((path, root, exports)) = self.descriptor(identity) else {
            return if self.namespace_exists(identity) {
                Ok(None)
            } else {
                Err(format!("模块 {identity} 不存在"))
            };
        };
        let mut context = FrontendContext::host();
        context.project_root = Some(root);
        context.package_registry = self.packages.cloned();
        context.module_exports = exports;
        let request = FrontendRequest::from_file(&path)
            .map_err(|error| error.to_string())?
            .with_context(context);
        let artifact = FrontendCompiler::new().compile(&request).map_err(|error| {
            error
                .diagnostics
                .iter()
                .filter(|item| item.is_error())
                .map(|item| format!("{}: {}", item.code(), item.message()))
                .collect::<Vec<_>>()
                .join("; ")
        })?;
        let program = lower_program(&artifact.ir);
        verify_for_execution(&artifact.ir, &program).map_err(|error| error.to_string())?;
        Ok(Some(CompiledModule {
            ir: artifact.ir,
            program,
            source_name: path.display().to_string(),
        }))
    }

    /// 返回包模块来源环境；项目模块和官方库没有包环境。
    fn environment(&self, identity: &str) -> Option<&str> {
        let name = identity.strip_prefix("package:")?;
        let root = name.split('.').next()?;
        self.packages?
            .modules
            .values()
            .find(|module| module.package == root)
            .and_then(|module| module.environment.to_str())
    }
}

#[cfg(test)]
/// 标准库固定命名空间加载及删除边界回归。
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::DriverModuleLoader;
    use xiao_vm::ModuleLoader;

    /// 避免并行用例创建相同临时标准库目录。
    static ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    /// 为标准库加载器负例创建安全隔离的临时源码根。
    struct EmptyStandardLibrary(PathBuf);

    impl EmptyStandardLibrary {
        /// 创建一个空的标准库源码根。
        fn new() -> Self {
            let sequence = ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "xiao-20c-empty-stdlib-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("创建空标准库目录");
            Self(path)
        }
    }

    impl Drop for EmptyStandardLibrary {
        /// 仅清理本测试创建、且仍位于系统临时目录下的源码根。
        fn drop(&mut self) {
            if let (Ok(path), Ok(temp)) =
                (self.0.canonicalize(), std::env::temp_dir().canonicalize())
                && path.starts_with(temp)
                && path.file_name().is_some_and(|name| {
                    name.to_string_lossy().starts_with("xiao-20c-empty-stdlib-")
                })
            {
                let _ = fs::remove_dir_all(path);
            }
        }
    }

    /// 拒绝被移除的标准库模块，锁定专用加载器的实际绑定边界。
    #[test]
    fn removed_standard_library_module_is_not_loadable() {
        let root = EmptyStandardLibrary::new();
        let loader = DriverModuleLoader {
            project: None,
            packages: None,
            standard_library_root: root.0.clone(),
            standard_library: OnceLock::new(),
        };

        assert!(!loader.contains("stdlib.text"));
        assert!(loader.compile("stdlib.text").is_err());
    }
}
