//! 18B 归档构建：统一前端、精确闭包、全局缓存、显式资源及验证后原子写出。

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;
use xiao_artifacts::{
    ArchiveIndex, ArtifactStore, Digest256, GlobalRecord, INDEX_SCHEMA_MAJOR, INDEX_SCHEMA_MINOR,
    IndexStore, ObjectKind,
};
use xiao_bytecode::{
    XiaocMetadata, XiaocOptions, decode_xiaoc, encode_xiaoc_with_options, optimize_bytecode_checked,
};
use xiao_config::{ConfigDocument, parse_config_text};
use xiao_optimizer::{OptimizationConfig, OptimizationLevel};
use xiao_xar::{DiagnosticObjectOptions, XarBuilder, collect_config_resources, validate_xar};

use crate::archive_closure::{ClosureModule, collect_archive_closure};
use crate::archive_output::validate_archive_output;
use crate::frontend::{FrontendContext, FrontendRequest};
use crate::run::CancellationToken;

/// 一次归档构建请求；不需要 LLVM 工具链。
#[derive(Clone, Debug)]
pub struct ArchiveBuildRequest {
    /// `.xiao` 或 `config.xiao` 输入。
    pub input: PathBuf,
    /// 显式输出；缺省为输入同目录 `<stem>.xar`。
    pub output: Option<PathBuf>,
    /// 18A 归一化后的优化级别。
    pub optimization_level: OptimizationLevel,
    /// 持久激活位及独立调试符号；不自动携带源码正文。
    pub debug: bool,
    /// 已解析的有效语言。
    pub locale: String,
    /// 测试/嵌入式宿主可指定隔离缓存根；CLI 使用全局 CacheLayout。
    pub cache_root: Option<PathBuf>,
}

impl ArchiveBuildRequest {
    /// 创建保持 O0、普通发布、全局缓存的默认请求。
    pub fn new(input: impl Into<PathBuf>) -> Self {
        Self {
            input: input.into(),
            output: None,
            optimization_level: OptimizationLevel::O0,
            debug: false,
            locale: "zh-CN".to_owned(),
            cache_root: None,
        }
    }
}

/// 构建结果中的逐模块缓存证据。
#[derive(Clone, Debug)]
pub struct ArchiveModuleObservation {
    /// 逻辑模块身份。
    pub module: String,
    /// 每次都重新验证的内容摘要。
    pub digest: Digest256,
    /// 是否复用已索引对象。
    pub hit: bool,
}

/// 已经完整验证并原子写出的归档。
#[derive(Clone, Debug)]
pub struct ArchiveBuildResult {
    /// 实际输出路径。
    pub output: PathBuf,
    /// 完整归档 SHA-256。
    pub digest: Digest256,
    /// 应用模块的缓存结果。
    pub modules: Vec<ArchiveModuleObservation>,
    /// 已验证的唯一归档索引。
    pub index: ArchiveIndex,
}

/// 构建拒绝不包含任何已经执行的用户代码。
#[derive(Clone, Debug)]
pub struct ArchiveBuildError {
    /// 稳定错误码：复用构建、能力边界、资源及校验的既有诊断。
    pub code: String,
    /// 开发者原因。
    pub message: String,
}

impl std::fmt::Display for ArchiveBuildError {
    /// 显示稳定诊断编号和构建失败原因。
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for ArchiveBuildError {}

/// 将构建编排错误映射到既有构建诊断。
fn failure(error: impl std::fmt::Display) -> ArchiveBuildError {
    ArchiveBuildError {
        code: crate::protocol::BUILD_ERROR_CODE.to_owned(),
        message: error.to_string(),
    }
}

/// 保留归档校验器和资源收集器已经确定的错误身份。
fn xar_failure(error: xiao_xar::XarError) -> ArchiveBuildError {
    let message = error.to_string();
    let code = error
        .diagnostic()
        .map(|diagnostic| diagnostic.code().to_owned())
        .unwrap_or_else(|| xiao_xar::XarRunError::from(error).code().to_owned());
    ArchiveBuildError { code, message }
}

/// 在编译、组装和写出阶段边界采样请求取消。
fn check_cancelled(cancellation: &CancellationToken) -> Result<(), ArchiveBuildError> {
    if cancellation.is_cancelled() {
        Err(ArchiveBuildError {
            code: crate::protocol::CANCELLED_ERROR_CODE.to_owned(),
            message: "归档构建已取消".to_owned(),
        })
    } else {
        Ok(())
    }
}

/// 在任何写出之前完成所有对象及完整归档验证；不创建 VM 或执行模块初始化。
pub fn build_archive(
    request: &ArchiveBuildRequest,
    cancellation: &CancellationToken,
) -> Result<ArchiveBuildResult, ArchiveBuildError> {
    check_cancelled(cancellation)?;
    let input = request.input.canonicalize().map_err(failure)?;
    if !input
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("xiao"))
    {
        return Err(failure("归档构建只接受 `.xiao` 或 `config.xiao`"));
    }
    let output = request
        .output
        .clone()
        .unwrap_or_else(|| input.with_extension("xar"));
    if output == input || output.canonicalize().is_ok_and(|path| path == input) {
        return Err(failure("归档输出不能覆盖输入源码"));
    }
    let config_input = input
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("config.xiao"));
    let root = if config_input {
        input.parent().unwrap()
    } else {
        input
            .ancestors()
            .skip(1)
            .find(|path| path.join("config.xiao").is_file())
            .unwrap_or(input.parent().unwrap())
    };
    let config_path = if config_input {
        input.clone()
    } else {
        root.join("config.xiao")
    };
    let config = if config_path.is_file() {
        parse_config_text(&fs::read_to_string(&config_path).map_err(failure)?).map_err(
            |diagnostics| {
                let diagnostic = &diagnostics[0];
                ArchiveBuildError {
                    code: diagnostic.code().to_owned(),
                    message: diagnostic.message().to_owned(),
                }
            },
        )?
    } else {
        parse_config_text("").map_err(|_| failure("空配置无效"))?
    };
    reject_dependencies(&config)?;
    let lock_path = root.join("xiao.lock.json");
    let lock_digest = if lock_path.is_file() {
        let bytes = fs::read(&lock_path).map_err(failure)?;
        let lock = xiao_package::LockFile::from_json(std::str::from_utf8(&bytes).map_err(failure)?)
            .map_err(|error| ArchiveBuildError {
                code: error.code().to_owned(),
                message: error.to_string(),
            })?;
        // 权威锁文件的 packages 是映射，且包含根项目本身；仅根项目不是第三方依赖。
        if lock.packages.len() > 1 {
            return Err(unsupported_dependencies());
        }
        if lock.config_fingerprint != xiao_package::fingerprint_config(&config) {
            return Err(ArchiveBuildError {
                code: xiao_package::LockfileMismatch::ConfigurationChanged {
                    package: lock.root.name,
                }
                .code()
                .to_owned(),
                message: "项目配置与依赖锁指纹不一致".to_owned(),
            });
        }
        Digest256::of_bytes(&bytes).as_hex()
    } else {
        Digest256::of_bytes(b"").as_hex()
    };
    let entry = if config_input {
        project_entry(root)?
    } else {
        input.clone()
    };
    let mut context = FrontendContext::host();
    context.project_root = Some(root.to_path_buf());
    context.target = "portable".to_owned();
    context.locale = request.locale.clone();
    context.external_standard_library = true;
    let frontend = FrontendRequest::from_file(entry)
        .map_err(failure)?
        .with_context(context);
    let closure = collect_archive_closure(frontend).map_err(failure)?;
    check_cancelled(cancellation)?;
    let resources = collect_config_resources(root, &config).map_err(|error| {
        let code = error
            .diagnostic()
            .map_or(crate::protocol::BUILD_ERROR_CODE, |diagnostic| {
                diagnostic.code()
            })
            .to_owned();
        ArchiveBuildError {
            code,
            message: error.to_string(),
        }
    })?;
    let mut inputs = vec![input.clone()];
    inputs.extend(
        [config_path, lock_path]
            .into_iter()
            .filter(|path| path.is_file()),
    );
    for compiled in closure.modules.values() {
        inputs.extend(compiled.request.source_path.iter().cloned());
        inputs.extend(
            compiled
                .artifact
                .ir
                .modules
                .iter()
                .filter(|module| module.kind == "file")
                .map(|module| PathBuf::from(&module.path)),
        );
    }
    inputs.extend(
        resources
            .iter()
            .map(|resource| root.join(&resource.source_path)),
    );
    inputs.sort();
    inputs.dedup();
    let cache_root = match &request.cache_root {
        Some(root) => root.clone(),
        None => xiao_package::CacheLayout::from_environment()
            .map_err(failure)?
            .cache_root(),
    };
    validate_archive_output(&output, &inputs, &cache_root).map_err(failure)?;
    let store = ArtifactStore::open(&cache_root).map_err(failure)?;
    let indexes = IndexStore::open(&cache_root).map_err(failure)?;
    let cached = match indexes.read_global() {
        Ok(index) => index.records,
        Err(xiao_artifacts::ArtifactError::Io(error))
            if error.kind() == std::io::ErrorKind::NotFound =>
        {
            Vec::new()
        }
        Err(error) => return Err(failure(error)),
    };
    let identity = xiao_standard_library::current_standard_library_identity();
    let mut builder = XarBuilder::new(ArchiveIndex {
        schema_major: INDEX_SCHEMA_MAJOR,
        schema_minor: INDEX_SCHEMA_MINOR,
        entry: "entry.xiaoc".to_owned(),
        entries: Vec::new(),
        dependency_lock_digest: lock_digest.clone(),
        runtime_abi_min: xiao_runtime_abi::ABI_ENCODED_VERSION,
        runtime_abi_max: xiao_runtime_abi::ABI_ENCODED_VERSION,
        platform: "portable".to_owned(),
        debug_activation: request.debug,
        language_locale: request.locale.clone(),
        stdlib_interface_version: identity.stdlib_interface_version,
        language_version: identity.language_version.clone(),
        stdlib_modules: Vec::new(),
        required_capabilities: 0,
        required_features: 0,
    })
    .with_requirements(&identity, closure.stdlib_modules, closure.intrinsics)
    .map_err(xar_failure)?;
    let mut records = Vec::new();
    let mut observations = Vec::new();
    for (module, compiled) in &closure.modules {
        if module.starts_with("stdlib.") {
            continue;
        }
        check_cancelled(cancellation)?;
        let (bytes, record, hit) =
            cached_module(module, compiled, request, &lock_digest, &store, &cached)?;
        let logical_path = if module == "entry" {
            "entry.xiaoc".to_owned()
        } else {
            format!(
                "modules/{}.xiaoc",
                module
                    .strip_prefix("project:")
                    .unwrap_or(module)
                    .replace('.', "/")
            )
        };
        builder
            .add_xiaoc(
                &logical_path,
                module,
                "portable",
                &bytes,
                DiagnosticObjectOptions {
                    include_debug: request.debug,
                    include_source: false,
                },
            )
            .map_err(xar_failure)?;
        observations.push(ArchiveModuleObservation {
            module: module.clone(),
            digest: record.digest,
            hit,
        });
        records.push(record);
    }
    builder
        .add_resources(&resources, "entry", "portable")
        .map_err(xar_failure)?;
    let bytes = builder.build().map_err(xar_failure)?;
    validate_xar(&bytes).map_err(xar_failure)?;
    let archive = xiao_xar::decode_xar(&bytes).map_err(xar_failure)?;
    check_cancelled(cancellation)?;
    validate_archive_output(&output, &inputs, &cache_root).map_err(failure)?;
    indexes.upsert_global(records).map_err(failure)?;
    xiao_artifacts::write_file_atomic(&output, &bytes).map_err(failure)?;
    Ok(ArchiveBuildResult {
        output,
        digest: Digest256::of_bytes(&bytes),
        modules: observations,
        index: archive.index().clone(),
    })
}

/// 第三方归档依赖尚未接通时给出稳定构建限制。
fn unsupported_dependencies() -> ArchiveBuildError {
    ArchiveBuildError {
        code: crate::protocol::UNSUPPORTED_OPERATION_CODE.to_owned(),
        message: "build -xar 首版不支持第三方包依赖；请移除包依赖后构建".to_owned(),
    }
}

/// 检查静态配置中的运行与开发期依赖，不读取或安装包。
fn reject_dependencies(config: &ConfigDocument) -> Result<(), ArchiveBuildError> {
    if ["dependencies", "devdependencies"].iter().any(|name| {
        config
            .table(name)
            .is_some_and(|table| !table.entries.is_empty())
    }) {
        return Err(unsupported_dependencies());
    }
    Ok(())
}

/// 复用静态项目发现及解析的入口模式；不执行配置或源码猜测入口。
fn project_entry(root: &Path) -> Result<PathBuf, ArchiveBuildError> {
    let project = xiao_modules::analyze_project(root);
    let candidates = project
        .modules
        .values()
        .filter(|module| {
            module.program.as_ref().is_some_and(|program| {
                matches!(program.entry_mode, xiao_syntax::EntryMode::Project { .. })
            })
        })
        .map(|module| module.path.clone())
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [entry] => Ok(entry.clone()),
        [] => Err(failure("config.xiao 项目中缺少带 [main] 的入口源码")),
        _ => Err(failure("config.xiao 项目中存在多个带 [main] 的入口源码")),
    }
}

/// 命中只免重新物化；每次读取都重算完整摘要并解码验证。
fn cached_module(
    module: &str,
    compiled: &ClosureModule,
    request: &ArchiveBuildRequest,
    lock_digest: &str,
    store: &ArtifactStore,
    cached: &[GlobalRecord],
) -> Result<(Vec<u8>, GlobalRecord, bool), ArchiveBuildError> {
    let mut optimization =
        OptimizationConfig::baseline("portable").with_level(request.optimization_level);
    optimization.debug_info = request.debug;
    let fingerprint = optimization
        .fingerprint()
        .map_err(failure)?
        .as_str()
        .to_owned();
    let source_digest = Digest256::of_bytes(compiled.request.source.text().as_bytes()).as_hex();
    let request_key = Digest256::of_bytes(
        &serde_json::to_vec(&json!({
            "archive_cache_version": 1, "ir": compiled.artifact.ir, "module": module,
            "source": source_digest, "optimization": fingerprint, "lock": lock_digest,
            "debug": request.debug, "toolchain": xiao_xar::toolchain_fingerprint(),
        }))
        .map_err(failure)?,
    )
    .as_hex();
    if let Some(record) = cached
        .iter()
        .find(|record| record.request_key == request_key && record.object_kind == ObjectKind::Xiaoc)
    {
        let bytes = store
            .read_checked(ObjectKind::Xiaoc, record.digest, Some(record.length))
            .map_err(failure)?;
        let file = decode_xiaoc(&bytes).map_err(failure)?;
        if bytes.len() as u64 != record.length
            || file.metadata.module_id != module
            || file.metadata.source_digest != source_digest
            || file.metadata.optimization_fingerprint != fingerprint
            || file.metadata.dependency_lock_digest != lock_digest
            || file.metadata.debug_active != request.debug
        {
            return Err(failure("归档缓存记录与已验证对象的构建身份不一致"));
        }
        return Ok((bytes, record.clone(), true));
    }
    let optimized =
        optimize_bytecode_checked(&compiled.artifact.ir, &compiled.program, optimization)
            .map_err(failure)?;
    let mut metadata = XiaocMetadata::new(module)
        .with_source_digest(source_digest)
        .with_optimization_fingerprint(fingerprint)
        .with_dependency_lock_digest(lock_digest);
    if request.debug {
        metadata = metadata.with_debug("0.1.0");
    }
    let debug_symbols = if request.debug {
        Some(serde_json::to_vec(&optimized.program.functions.iter().map(|function| json!({ "name": function.name, "span": function.span, "locals": function.locals.len() })).collect::<Vec<_>>()).map_err(failure)?)
    } else {
        None
    };
    let bytes = encode_xiaoc_with_options(
        &optimized.program,
        metadata,
        XiaocOptions {
            debug_symbols,
            ..Default::default()
        },
    )
    .map_err(failure)?;
    let reference = store.put_xiaoc(&bytes).map_err(failure)?;
    let bytes = store
        .read_checked(ObjectKind::Xiaoc, reference.digest, Some(reference.length))
        .map_err(failure)?;
    decode_xiaoc(&bytes).map_err(failure)?;
    Ok((
        bytes,
        GlobalRecord {
            request_key,
            object_kind: ObjectKind::Xiaoc,
            digest: reference.digest,
            target: "portable".to_owned(),
            optimization_level: request.optimization_level.as_u8() as u32,
            codegen_version: 1,
            length: reference.length,
        },
        false,
    ))
}
