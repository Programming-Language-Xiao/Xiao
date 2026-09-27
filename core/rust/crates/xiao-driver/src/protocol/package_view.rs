//! REPL 包根与接口查询：只读已有环境映射和 05-B 模块接口，不运行包代码。

use std::collections::BTreeMap;
use std::path::{Component, Path};

use serde_json::json;
use xiao_modules::{
    ExportOrigin, ModuleKind, ModuleName, ModuleSymbolKind, ProjectModuleResult, analyze_project,
};
use xiao_package::{CacheLayout, PackageObjectMapping, environment_package_view};
use xiao_syntax::KeywordKind;
use xiao_types::{TypeCheckResult, check};

use super::mapping::{protocol_error_body, protocol_error_from_error};
use super::message::{ProtocolResponse, ReplExport, ReplInterface, ReplPackage};
use super::validate::validate_versions;
use crate::run::ExitCode;

/// 环境包根冲突的稳定诊断码。
pub const REPL_ROOT_CONFLICT_CODE: &str = "X11-REPL-PACKAGE-001";
/// 环境视图或模块接口不可读取的稳定诊断码。
pub const REPL_VIEW_ERROR_CODE: &str = "X11-REPL-PACKAGE-002";

/// 根据激活状态只读枚举一个环境，并按需查询已解析模块的接口。
pub(super) fn repl_packages_response(
    request_id: String,
    protocol_version: u16,
    core_version: u32,
    active_environment: Option<String>,
    module_path: Option<String>,
) -> ProtocolResponse {
    if let Err(error) = validate_versions(protocol_version, core_version) {
        return ProtocolResponse::Error {
            request_id: Some(request_id),
            error: protocol_error_from_error(&error),
            report: None,
            exit_code: ExitCode::ArtifactRejected.as_process_code(),
        };
    }
    let layout = match CacheLayout::from_environment() {
        Ok(layout) => layout,
        Err(error) => return view_error(request_id, "", None, error.to_string()),
    };
    repl_packages_with_layout(request_id, &layout, active_environment, module_path)
}

/// 可注入缓存布局的只读查询核心，供隔离环境的协议测试复用。
fn repl_packages_with_layout(
    request_id: String,
    layout: &CacheLayout,
    active_environment: Option<String>,
    module_path: Option<String>,
) -> ProtocolResponse {
    let environment = if let Some(path) = active_environment.as_deref() {
        let path = Path::new(path);
        if !path.is_absolute()
            || path
                .components()
                .any(|part| matches!(part, Component::ParentDir))
        {
            return view_error(
                request_id,
                path.to_string_lossy().as_ref(),
                None,
                "XIAO_ACTIVE_ENV 必须是规范的绝对路径".to_owned(),
            );
        }
        path.to_path_buf()
    } else {
        match layout.global_environment_path("global") {
            Ok(path) => path,
            Err(error) => return view_error(request_id, "", None, error.to_string()),
        }
    };
    let environment_name = environment.to_string_lossy().into_owned();
    let mappings = if active_environment.is_none() && !environment.exists() {
        Vec::new()
    } else {
        match environment_package_view(&environment) {
            Ok(mappings) => mappings,
            Err(error) => {
                return view_error(request_id, &environment_name, None, error.to_string());
            }
        }
    };
    let mappings = mappings
        .into_iter()
        .filter(|mapping| valid_module_segment(&mapping.package.name))
        .collect::<Vec<_>>();
    let mut roots = BTreeMap::new();
    for mapping in &mappings {
        if let Some(first) = roots.insert(mapping.package.name.as_str(), mapping) {
            return conflict_error(request_id, &environment_name, first, mapping);
        }
    }
    let interface = if let Some(path) = module_path {
        match module_interface(layout, &mappings, &path) {
            Ok(interface) => Some(interface),
            Err((package, reason)) => {
                return view_error(request_id, &environment_name, package.as_deref(), reason);
            }
        }
    } else {
        None
    };
    let mut packages = mappings
        .iter()
        .map(|mapping| ReplPackage {
            root: mapping.package.name.clone(),
            identity: mapping.package.clone(),
        })
        .collect::<Vec<_>>();
    packages.sort_by(|left, right| left.root.cmp(&right.root));
    ProtocolResponse::ReplPackagesResult {
        request_id,
        environment_path: environment_name,
        packages,
        interface,
    }
}

/// 使用 05-B 的模块记录和 04 的函数签名，不另建符号表或执行初始化。
fn module_interface(
    layout: &CacheLayout,
    mappings: &[PackageObjectMapping],
    path: &str,
) -> Result<ReplInterface, (Option<String>, String)> {
    let mut segments = path.split('.');
    let root = segments.next().unwrap_or_default();
    let Some(mapping) = mappings.iter().find(|mapping| mapping.package.name == root) else {
        return Err((None, format!("未知包根 {root:?}")));
    };
    let module_segments = segments.map(str::to_owned).collect::<Vec<_>>();
    if module_segments
        .iter()
        .any(|segment| !valid_module_segment(segment))
    {
        return Err((Some(root.to_owned()), format!("非法模块路径 {path:?}")));
    }
    let object_path = layout
        .source_object_path(&mapping.object.digest)
        .map_err(|error| (Some(root.to_owned()), error.to_string()))?;
    let project = analyze_project(&object_path);
    if let Some(diagnostic) = project
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.diagnostic.is_error())
    {
        return Err((
            Some(root.to_owned()),
            diagnostic.diagnostic.message().to_owned(),
        ));
    }
    let name = ModuleName::new(module_segments);
    let mut checked_modules = BTreeMap::new();
    let exports = if let Some(record) = project
        .modules
        .get(&name)
        .filter(|record| record.kind == ModuleKind::File)
    {
        record
            .symbols
            .values()
            .map(|symbol| {
                let signature = function_signature(&project, &name, symbol, &mut checked_modules);
                ReplExport {
                    name: symbol.name.clone(),
                    kind: symbol_kind(symbol.kind).to_owned(),
                    signature,
                }
            })
            .collect()
    } else if name.is_root() || project.namespaces.contains_key(&name) {
        project
            .modules
            .iter()
            .filter(|(candidate, _)| candidate.parent().as_ref() == Some(&name))
            .map(|(candidate, record)| ReplExport {
                name: candidate.segments().last().cloned().unwrap_or_default(),
                kind: if record.kind == ModuleKind::File {
                    "module"
                } else {
                    "namespace"
                }
                .to_owned(),
                signature: None,
            })
            .collect()
    } else {
        return Err((Some(root.to_owned()), format!("模块 {path:?} 不存在")));
    };
    Ok(ReplInterface {
        module_path: path.to_owned(),
        exports,
    })
}

/// 包名还可包含 `-` 和 `.`；自动登记只接受 05-B 模块路径可解析的单段名称。
fn valid_module_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
        && KeywordKind::from_word(segment).is_none()
}

/// 将再导出来源解析回 05-B 已发现的真实文件模块，再读取其函数签名。
fn function_signature(
    project: &ProjectModuleResult,
    module: &ModuleName,
    symbol: &xiao_modules::ModuleSymbol,
    checked_modules: &mut BTreeMap<ModuleName, TypeCheckResult>,
) -> Option<String> {
    if symbol.kind != ModuleSymbolKind::Function {
        return None;
    }
    let (source_module, name) = match &symbol.origin {
        ExportOrigin::Local => (module, symbol.name.as_str()),
        ExportOrigin::Reexport { module, name } => (module, name.as_str()),
    };
    let record = project.modules.get(source_module)?;
    let (source, program) = (record.source.as_ref()?, record.program.as_ref()?);
    if !checked_modules.contains_key(source_module) {
        checked_modules.insert(source_module.clone(), check(source, program));
    }
    let types = checked_modules.get(source_module)?;
    let declared = record.symbols.get(name)?;
    let prefix = if source.slice(declared.span).starts_with('`') {
        "backtick:"
    } else {
        "ascii:"
    };
    let signature = types
        .function_signatures()
        .get(&format!("{prefix}{name}"))?;
    let parameters = signature
        .parameters
        .iter()
        .map(|parameter| {
            let name = parameter
                .name
                .strip_prefix("ascii:")
                .or_else(|| parameter.name.strip_prefix("backtick:"))
                .unwrap_or(&parameter.name);
            format!("{name}: {}", parameter.ty)
        })
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!("({parameters}) -> {}", signature.return_type))
}

/// 将 05-B 的符号类别固定为跨语言的机器名称。
fn symbol_kind(kind: ModuleSymbolKind) -> &'static str {
    match kind {
        ModuleSymbolKind::Value => "value",
        ModuleSymbolKind::Function => "function",
        ModuleSymbolKind::Table => "table",
        ModuleSymbolKind::Module => "module",
        ModuleSymbolKind::Namespace => "namespace",
    }
}

/// 构造携带两个候选身份的根名冲突错误，不选择扫描顺序的胜者。
fn conflict_error(
    request_id: String,
    environment: &str,
    first: &PackageObjectMapping,
    second: &PackageObjectMapping,
) -> ProtocolResponse {
    let root = &first.package.name;
    ProtocolResponse::Error {
        request_id: Some(request_id),
        error: protocol_error_body(
            REPL_ROOT_CONFLICT_CODE,
            "x11.repl.package.root_conflict",
            format!("包根 {root:?} 存在冲突，请使用显式 import 指明来源"),
            Some("repl_packages".to_owned()),
            Some("使用显式 import 指明包来源".to_owned()),
            BTreeMap::from([
                ("environment".to_owned(), json!(environment)),
                ("root".to_owned(), json!(root)),
                (
                    "candidates".to_owned(),
                    json!([first.package, second.package]),
                ),
            ]),
        ),
        report: None,
        exit_code: ExitCode::ArtifactRejected.as_process_code(),
    }
}

/// 将环境或静态解析失败连同包名、环境及底层原因交给调用方。
fn view_error(
    request_id: String,
    environment: &str,
    package: Option<&str>,
    reason: String,
) -> ProtocolResponse {
    ProtocolResponse::Error {
        request_id: Some(request_id),
        error: protocol_error_body(
            REPL_VIEW_ERROR_CODE,
            "x11.repl.package.view_failed",
            format!(
                "无法查询包 {}（环境 {environment}）：{reason}",
                package.unwrap_or("<未指定>")
            ),
            Some("repl_packages".to_owned()),
            Some("检查环境和包对象后重试".to_owned()),
            BTreeMap::from([
                ("environment".to_owned(), json!(environment)),
                ("package".to_owned(), json!(package)),
                ("cause".to_owned(), json!(reason)),
            ]),
        ),
        report: None,
        exit_code: ExitCode::ArtifactRejected.as_process_code(),
    }
}

#[cfg(test)]
#[path = "../protocol_package_view_tests.rs"]
/// 隔离路径下的协议与静态查询行为回归。
mod tests;
