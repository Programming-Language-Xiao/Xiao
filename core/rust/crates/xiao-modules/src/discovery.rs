//! 05-B 项目文件发现、命名空间建立和源码解析。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use xiao_diagnostics::{Diagnostic, DiagnosticParam};
use xiao_source::SourceFile;
use xiao_syntax::{KeywordKind, Program, Statement, parse};

use crate::diagnostics::{INVALID_MODULE_PATH_CODE, MODULE_IO_CODE, MODULE_PATH_CONFLICT_CODE};
use crate::model::{
    ModuleKind, ModuleName, ModuleRecord, ModuleSymbol, ModuleSymbolKind, NamespaceRecord,
    ProjectModuleResult, case_fold_name,
};
use crate::resolver;

/// 分析一个项目根目录中的本地 Xiao 模块。
///
/// 根目录暂时直接作为源码根；根 `config.xiao` 和嵌套包边界不会进入模块。
/// 结果即使包含错误也会尽可能保留可解析的模块和依赖信息。
#[must_use]
pub fn analyze_project(project_root: impl AsRef<Path>) -> ProjectModuleResult {
    let requested_root = project_root.as_ref().to_path_buf();
    let root =
        fs::canonicalize(&requested_root).unwrap_or_else(|_| absolute_fallback(&requested_root));
    let mut state = DiscoveryState::new(root.clone());

    if !root.is_dir() {
        state
            .diagnostics
            .push(ModuleDiagnosticBuilder::without_path(
                MODULE_IO_CODE,
                "x05.module.project_root_not_directory",
                format!("项目根不是目录: {}", root.display()),
                root.clone(),
            ));
    } else {
        state.walk_directory(&root, &[], true);
    }

    state.finalize_namespaces();
    let mut result = state.build_result();
    resolver::resolve_project(&mut result);
    result
}

/// 发现阶段的候选文件。
#[derive(Clone, Debug)]
struct ModuleCandidate {
    name: ModuleName,
    path: PathBuf,
}

/// 文件发现状态；先收集候选，再做全局名称冲突检查。
#[derive(Debug)]
struct DiscoveryState {
    root: PathBuf,
    candidates: Vec<ModuleCandidate>,
    namespaces: BTreeMap<ModuleName, NamespaceRecord>,
    diagnostics: Vec<crate::model::ModuleDiagnostic>,
}

impl DiscoveryState {
    /// 创建空发现状态。
    fn new(root: PathBuf) -> Self {
        Self {
            root,
            candidates: Vec::new(),
            namespaces: BTreeMap::new(),
            diagnostics: Vec::new(),
        }
    }

    /// 递归扫描目录；符号链接和点目录不进入扫描。
    fn walk_directory(&mut self, directory: &Path, parent_segments: &[String], is_root: bool) {
        if !is_root && directory.join("config.xiao").is_file() {
            // 嵌套 config.xiao 标记一个尚未接入的包边界；整个子树留给
            // 后续外部包阶段，不把其中源码混入当前项目。
            return;
        }

        let entries = match fs::read_dir(directory) {
            Ok(entries) => {
                let mut entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
                entries.sort_by_key(|entry| entry.file_name());
                entries
            }
            Err(error) => {
                self.diagnostics.push(ModuleDiagnosticBuilder::without_path(
                    MODULE_IO_CODE,
                    "x05.module.read_directory",
                    format!("无法读取目录 {}: {error}", directory.display()),
                    directory.to_path_buf(),
                ));
                return;
            }
        };

        for entry in entries {
            let path = entry.path();
            let file_name = entry.file_name();
            let Some(file_name) = file_name.to_str() else {
                self.diagnostics.push(ModuleDiagnosticBuilder::without_path(
                    INVALID_MODULE_PATH_CODE,
                    "x05.module.non_utf8_path",
                    "路径名称不是合法 UTF-8".to_owned(),
                    path,
                ));
                continue;
            };
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    self.diagnostics.push(ModuleDiagnosticBuilder::without_path(
                        MODULE_IO_CODE,
                        "x05.module.read_file_type",
                        format!("无法读取路径类型 {}: {error}", path.display()),
                        path,
                    ));
                    continue;
                }
            };
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                if file_name.starts_with('.') {
                    continue;
                }
                let mut segments = parent_segments.to_vec();
                segments.push(file_name.to_owned());
                self.walk_directory(&path, &segments, false);
                continue;
            }
            if !file_type.is_file() || file_name == "config.xiao" {
                continue;
            }
            if Path::new(file_name)
                .extension()
                .and_then(|value| value.to_str())
                != Some("xiao")
            {
                continue;
            }
            let Some(stem) = Path::new(file_name)
                .file_stem()
                .and_then(|value| value.to_str())
            else {
                continue;
            };
            let mut segments = parent_segments.to_vec();
            segments.push(stem.to_owned());
            if !segments.iter().all(|segment| valid_module_segment(segment)) {
                self.diagnostics.push(ModuleDiagnosticBuilder::without_path(
                    INVALID_MODULE_PATH_CODE,
                    "x05.module.invalid_file_name",
                    format!("文件不能映射为模块名称: {file_name}"),
                    path,
                ));
                continue;
            }
            self.candidates.push(ModuleCandidate {
                name: ModuleName::new(segments),
                path,
            });
        }
    }

    /// 根据候选文件建立纯目录命名空间，并诊断全局冲突。
    fn finalize_namespaces(&mut self) {
        self.candidates
            .sort_by(|left, right| left.name.cmp(&right.name));
        let mut namespace_paths = BTreeMap::<ModuleName, PathBuf>::new();
        for candidate in &self.candidates {
            let mut prefix = ModuleName::root();
            for segment in candidate
                .name
                .segments()
                .iter()
                .take(candidate.name.segments().len().saturating_sub(1))
            {
                prefix = prefix.child(segment.clone());
                let namespace_path = prefix
                    .segments()
                    .iter()
                    .fold(self.root.clone(), |path, segment| path.join(segment));
                namespace_paths
                    .entry(prefix.clone())
                    .or_insert(namespace_path);
            }
        }

        let mut seen_folded = BTreeMap::<String, (ModuleName, PathBuf, ModuleKind)>::new();
        let mut valid_candidates = Vec::new();
        for candidate in self.candidates.drain(..) {
            let folded = case_fold_name(&candidate.name);
            let conflict = seen_folded.get(&folded).cloned();
            if let Some((other, other_path, _)) = conflict {
                self.diagnostics.push(ModuleDiagnosticBuilder::without_path(
                    MODULE_PATH_CONFLICT_CODE,
                    "x05.module.case_fold_conflict",
                    format!(
                        "模块名称大小写折叠冲突: {} ({}) 与 {} ({})",
                        candidate.name,
                        candidate.path.display(),
                        other,
                        other_path.display()
                    ),
                    candidate.path.clone(),
                ));
                continue;
            }
            if namespace_paths.contains_key(&candidate.name) {
                self.diagnostics.push(ModuleDiagnosticBuilder::without_path(
                    MODULE_PATH_CONFLICT_CODE,
                    "x05.module.file_namespace_conflict",
                    format!("文件模块与目录命名空间同名: {}", candidate.name),
                    candidate.path.clone(),
                ));
                continue;
            }
            seen_folded.insert(
                folded,
                (
                    candidate.name.clone(),
                    candidate.path.clone(),
                    ModuleKind::File,
                ),
            );
            valid_candidates.push(candidate);
        }

        for (name, path) in namespace_paths {
            let folded = case_fold_name(&name);
            if let Some((other, other_path, _)) = seen_folded.get(&folded) {
                self.diagnostics.push(ModuleDiagnosticBuilder::without_path(
                    MODULE_PATH_CONFLICT_CODE,
                    "x05.module.namespace_case_fold_conflict",
                    format!(
                        "命名空间名称大小写折叠冲突: {} ({}) 与 {} ({})",
                        name,
                        path.display(),
                        other,
                        other_path.display()
                    ),
                    path.clone(),
                ));
                continue;
            }
            if seen_folded
                .insert(folded, (name.clone(), path.clone(), ModuleKind::Namespace))
                .is_none()
            {
                self.namespaces
                    .insert(name.clone(), NamespaceRecord { name, path });
            }
        }
        self.candidates = valid_candidates;
    }

    /// 将发现结果解析为公开项目结果。
    fn build_result(self) -> ProjectModuleResult {
        let mut modules = BTreeMap::new();
        let mut diagnostics = self.diagnostics;
        for candidate in self.candidates {
            let (source, program, parse_diagnostics) = parse_module(&candidate);
            for diagnostic in parse_diagnostics {
                diagnostics.push(crate::model::ModuleDiagnostic {
                    module: Some(candidate.name.clone()),
                    path: Some(candidate.path.clone()),
                    diagnostic,
                });
            }
            let symbols = program
                .as_ref()
                .zip(source.as_ref())
                .map_or_else(BTreeMap::new, |(program, source)| {
                    collect_local_symbols(program, source)
                });
            modules.insert(
                candidate.name.clone(),
                ModuleRecord {
                    name: candidate.name,
                    kind: ModuleKind::File,
                    path: candidate.path,
                    source,
                    program,
                    symbols,
                },
            );
        }
        ProjectModuleResult {
            project_root: self.root,
            modules,
            namespaces: self.namespaces,
            graph: Default::default(),
            bindings: Vec::new(),
            diagnostics,
        }
    }
}

/// 解析一个候选源码文件。
fn parse_module(
    candidate: &ModuleCandidate,
) -> (Option<SourceFile>, Option<Program>, Vec<Diagnostic>) {
    let bytes = match fs::read(&candidate.path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return (
                None,
                None,
                vec![Diagnostic::new(
                    MODULE_IO_CODE,
                    "x05.module.read_file",
                    xiao_diagnostics::Severity::Error,
                    None,
                    format!("无法读取模块 {}: {error}", candidate.path.display()),
                )],
            );
        }
    };
    let source = match SourceFile::from_bytes(bytes) {
        Ok(source) => source,
        Err(error) => {
            return (
                None,
                None,
                vec![Diagnostic::new(
                    MODULE_IO_CODE,
                    "x05.module.invalid_utf8",
                    xiao_diagnostics::Severity::Error,
                    None,
                    format!("模块源码不是合法 UTF-8: {error}"),
                )],
            );
        }
    };
    let parsed = parse(&source);
    (Some(source), parsed.program, parsed.diagnostics)
}

/// 收集文件模块当前阶段可见的本地顶层符号。
fn collect_local_symbols(program: &Program, source: &SourceFile) -> BTreeMap<String, ModuleSymbol> {
    let mut symbols = BTreeMap::new();
    for statement in &program.statements {
        let (name, kind, span) = match statement {
            Statement::Assignment { target, .. } => (target, ModuleSymbolKind::Value, target.span),
            Statement::Declaration { target, .. } => (target, ModuleSymbolKind::Value, target.span),
            Statement::ConstDeclaration { target, .. } => {
                (target, ModuleSymbolKind::Value, target.span)
            }
            Statement::Function { name, .. } => (name, ModuleSymbolKind::Function, name.span),
            _ => continue,
        };
        let key = name.unquoted_text(source).to_owned();
        symbols.entry(key.clone()).or_insert(ModuleSymbol {
            name: key,
            kind,
            origin: crate::model::ExportOrigin::Local,
            span,
        });
    }
    symbols
}

/// 判断一个文件/目录段是否符合跨平台模块名称规则。
fn valid_module_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    if !chars.all(|character| character == '_' || character.is_ascii_alphanumeric()) {
        return false;
    }
    KeywordKind::from_word(segment).is_none()
}

/// 在路径不是绝对路径时提供不依赖当前目录变化的兜底路径。
fn absolute_fallback(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|current| current.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

/// 统一构造无源码区间的模块诊断。
struct ModuleDiagnosticBuilder;

impl ModuleDiagnosticBuilder {
    /// 构造一条带路径参数的系统诊断。
    fn without_path(
        code: &'static str,
        message_id: &'static str,
        message: String,
        path: PathBuf,
    ) -> crate::model::ModuleDiagnostic {
        let diagnostic = Diagnostic::new(
            code,
            message_id,
            xiao_diagnostics::Severity::Error,
            None,
            message,
        )
        .with_params([(
            String::from("path"),
            DiagnosticParam::Text(path.display().to_string()),
        )]);
        crate::model::ModuleDiagnostic {
            module: None,
            path: Some(path),
            diagnostic,
        }
    }
}
