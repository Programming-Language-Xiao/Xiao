//! 驱动器侧的只读模块索引与统一前端编译适配器。

use std::path::{Path, PathBuf};

use xiao_bytecode::{lower_program, verify_for_execution};
use xiao_modules::{ModuleName, ProjectModuleResult, analyze_project};
use xiao_vm::{CompiledModule, ModuleLoader};

use crate::frontend::{FrontendCompiler, FrontendContext, FrontendRequest};
use crate::packages::PackageRegistry;

#[derive(Debug)]
pub(crate) struct DriverModuleLoader<'a> {
    project: Option<ProjectModuleResult>,
    packages: Option<&'a PackageRegistry>,
}

impl<'a> DriverModuleLoader<'a> {
    pub(crate) fn new(root: Option<&Path>, packages: Option<&'a PackageRegistry>) -> Self {
        Self {
            project: root.map(analyze_project),
            packages,
        }
    }

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
        let name = identity.strip_prefix("package:")?;
        let module = self.packages?.modules.get(name)?;
        Some((
            module.source.clone(),
            module.project_root.clone(),
            module.exports.iter().cloned().collect(),
        ))
    }

    fn namespace_exists(&self, identity: &str) -> bool {
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

impl ModuleLoader for DriverModuleLoader<'_> {
    fn contains(&self, identity: &str) -> bool {
        self.descriptor(identity).is_some() || self.namespace_exists(identity)
    }

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
