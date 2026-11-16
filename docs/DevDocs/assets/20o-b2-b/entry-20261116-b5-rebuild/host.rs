//! B5离线证据宿主：使用各固定快照的真实编译/验证/缓存接口。
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use xiao_artifacts::{
    ArtifactStore, Digest256, GlobalIndex, GlobalRecord, INDEX_SCHEMA_MAJOR, INDEX_SCHEMA_MINOR,
    IndexStore, ObjectKind,
};
use xiao_codegen_llvm::{
    CODEGEN_VERSION, CodegenOptions, NativeBuild, NativeMetadata, NativeRuntimeSupport,
    TargetDescription, Toolchain, validate_native_metadata_file,
};
use xiao_driver::{
    ArchiveBuildRequest, CORE_VERSION, CancellationToken, FrontendCompiler, FrontendNativeDriver,
    FrontendRequest, NativeBuildRequest, PROTOCOL_VERSION, ProtocolRequest, ProtocolResponse,
    RunOptions, dispatch,
};
use xiao_runtime_abi::{ABI_ENCODED_VERSION, ABI_MINOR_VERSION};

const COMMON: &str = "from std.concurrent import current_runtime\nfrom std.math import gcd\nruntime = current_runtime()\nprint(gcd(48, 18))\nprint(\"B5-USER-ENTRY\")\n";

fn digest(path: &Path) -> String {
    Digest256::of_bytes(&fs::read(path).unwrap()).as_hex()
}
fn tools(runtime: Option<&str>) -> Toolchain {
    let target = TargetDescription::host();
    Toolchain::new(std::env::var_os("XIAO_CLANG").unwrap())
        .with_llvm_as(std::env::var_os("XIAO_LLVM_AS").unwrap())
        .with_llc(std::env::var_os("XIAO_LLC").unwrap())
        .with_runtime_library(
            runtime.map(PathBuf::from).unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("XIAO_RUNTIME_LIBRARY").unwrap())
            }),
        )
        .probe_versions()
        .unwrap()
        .probe_native_static_libraries("rustc", &target)
        .unwrap()
}
fn artifact(source: &str) -> xiao_driver::FrontendArtifact {
    FrontendCompiler::new()
        .compile(&FrontendRequest::from_text(source))
        .unwrap()
}
fn generate(out: &Path, supplied_source: Option<&Path>) {
    fs::create_dir(out).unwrap();
    let source = out.join("main.xiao");
    let text = supplied_source
        .map(|path| fs::read_to_string(path).unwrap())
        .unwrap_or_else(|| COMMON.to_owned());
    fs::write(&source, &text).unwrap();
    let frontend = FrontendRequest::from_file(&source).unwrap();
    let product = FrontendCompiler::new().compile(&frontend).unwrap();
    let tac = xiao_bytecode::lower_program(&product.ir);
    let xiaoc = xiao_bytecode::encode_xiaoc(&tac, Default::default()).unwrap();
    fs::write(out.join("main.xiaoc"), &xiaoc).unwrap();
    let mut archive = ArchiveBuildRequest::new(&source);
    archive.output = Some(out.join("main.xar"));
    archive.cache_root = Some(out.join("archive-cache"));
    let archive = xiao_driver::build_archive(&archive, &CancellationToken::new()).unwrap();
    let target = TargetDescription::host();
    let toolchain = tools(None);
    let request = NativeBuildRequest::new(
        frontend,
        target.clone(),
        toolchain.clone(),
        out.join("main.exe"),
    );
    let built = FrontendNativeDriver::new()
        .build_artifact(&product, &request)
        .unwrap();
    let metadata = built
        .native
        .native_metadata
        .as_ref()
        .expect("current_runtime要求真实元数据");
    fs::write(out.join("main.meta"), metadata.encode().unwrap()).unwrap();
    let header = xiao_bytecode::decode_xiaoc(&xiaoc).unwrap();
    let out_native = Command::new(out.join("main.exe")).output().unwrap();
    assert!(out_native.status.success(), "{out_native:?}");
    assert_eq!(
        String::from_utf8_lossy(&out_native.stdout),
        if supplied_source.is_some() {
            "2\n0.333\nB5-USER-ENTRY\n"
        } else {
            "6\nB5-USER-ENTRY\n"
        }
    );
    let artifacts = ["main.xiaoc", "main.xar", "main.exe", "main.meta"]
        .iter()
        .map(|name| (*name, digest(&out.join(name))))
        .collect::<std::collections::BTreeMap<_, _>>();
    let report = json!({"abi":ABI_ENCODED_VERSION,"minor":ABI_MINOR_VERSION,"codegen":CODEGEN_VERSION,
      "xiaoc":{"format_major":header.header.format_major,"format_minor":header.header.format_minor,
        "runtime_abi_min":header.header.runtime_abi_min,"runtime_abi_max":header.header.runtime_abi_max,
        "instruction_abi":header.program.abi.runtime_abi_version},
      "xar":{"schema_major":archive.index.schema_major,"schema_minor":archive.index.schema_minor,
        "runtime_abi_min":archive.index.runtime_abi_min,"runtime_abi_max":archive.index.runtime_abi_max,
        "stdlib_interface":archive.index.stdlib_interface_version},
      "native":metadata,"runtime_path":toolchain.runtime_library.as_ref().unwrap().display().to_string(),
      "runtime_sha256":digest(toolchain.runtime_library.as_ref().unwrap()),
      "native_static_libraries":toolchain.native_static_libraries,"toolchain_fingerprint":built.native.toolchain_fingerprint.to_string(),
      "llvm_uses_runtime":built.native.module.uses_runtime,
      "layouts":{"XiaoValue":std::mem::size_of::<xiao_runtime_abi::XiaoValue>(),
                 "XiaoValue_align":std::mem::align_of::<xiao_runtime_abi::XiaoValue>(),
                 "XiaoAbiBytes":std::mem::size_of::<xiao_runtime_abi::XiaoAbiBytes>(),
                 "XiaoHandle":std::mem::size_of::<xiao_runtime_abi::XiaoHandle>()},
      "artifacts": artifacts});
    fs::write(
        out.join("report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{report}");
}
fn validate(kind: &str, path: &Path, expect: &str) {
    let before = digest(path);
    let mut executed = false;
    let response: Value = match kind {
        "xiaoc" | "xar" => {
            let options = RunOptions {
                timeout_ms: Some(10_000),
                ..Default::default()
            };
            let path = path.display().to_string();
            let request = if kind == "xiaoc" {
                ProtocolRequest::RunXiaoc {
                    request_id: "b5".into(),
                    protocol_version: PROTOCOL_VERSION,
                    core_version: CORE_VERSION,
                    locale: None,
                    path,
                    options,
                    diagnostics: None,
                    debug: false,
                }
            } else {
                ProtocolRequest::RunArchive {
                    request_id: "b5".into(),
                    protocol_version: PROTOCOL_VERSION,
                    core_version: CORE_VERSION,
                    locale: None,
                    path,
                    options,
                    diagnostics: None,
                    debug: false,
                }
            };
            let result = dispatch(request);
            let value = serde_json::to_value(&result).unwrap();
            if let ProtocolResponse::Result { events, .. } = &result {
                executed = events.iter().any(|event| event.kind == "intrinsic_output");
            }
            value
        }
        "native" => {
            match validate_native_metadata_file(
                path,
                &TargetDescription::host(),
                &NativeRuntimeSupport::default(),
                0,
                true,
            ) {
                Ok(metadata) => {
                    let output = Command::new(path).output().unwrap();
                    executed = true;
                    json!({"accepted":true,"metadata":metadata,"exit_code":output.status.code(),
                   "stdout":String::from_utf8_lossy(&output.stdout),"stderr":String::from_utf8_lossy(&output.stderr)})
                }
                Err(error) => {
                    json!({"accepted":false,"code":error.code,"message":error.message,"phase":"native_metadata_before_execution"})
                }
            }
        }
        "meta" => {
            let metadata = NativeMetadata::decode(&fs::read(path).unwrap()).unwrap();
            match metadata.validate(
                &TargetDescription::host(),
                &NativeRuntimeSupport::default(),
                0,
            ) {
                Ok(()) => json!({"accepted":true,"metadata":metadata}),
                Err(error) => {
                    json!({"accepted":false,"code":error.code,"message":error.message,"phase":"native_metadata_before_execution"})
                }
            }
        }
        _ => panic!("未知验证格式"),
    };
    let accepted = if kind == "xiaoc" || kind == "xar" {
        response.get("exit_code").and_then(Value::as_i64) == Some(0)
    } else {
        response["accepted"] == true
    };
    assert_eq!(accepted, expect == "accept", "{kind}: {response}");
    if !accepted {
        assert!(!executed, "拒绝后不得执行");
    }
    assert_eq!(digest(path), before);
    println!(
        "{}",
        json!({"kind":kind,"input":path.display().to_string(),"input_sha256":before,
      "validator_abi":ABI_ENCODED_VERSION,"validator_codegen":CODEGEN_VERSION,"accepted":accepted,
      "executed":executed,"input_unchanged":true,"response":response})
    );
}
fn cache(cache: &Path, out: &Path, requested_codegen: u32, runtime: &str) {
    let source = "from std.concurrent import current_runtime\nruntime = current_runtime()\nprint(\"B5-CACHE-ENTRY\")\n";
    let program = artifact(source);
    let target = TargetDescription::host();
    let toolchain = tools(Some(runtime));
    let options = CodegenOptions::for_target(target.clone());
    let module = NativeBuild::new().lower(&program.ir, &options).unwrap();
    let plan = &module.native_optimization_report.plan.fingerprint;
    let fingerprint = toolchain
        .fingerprint_with_optimization(&target, requested_codegen, 0, plan)
        .to_string();
    let key = format!(
        "b5:{}:{fingerprint}",
        Digest256::of_bytes(source.as_bytes())
    );
    let store = ArtifactStore::open(cache).unwrap();
    let index_store = IndexStore::open(cache).unwrap();
    let mut index = index_store.read_global().unwrap_or(GlobalIndex {
        schema_major: INDEX_SCHEMA_MAJOR,
        schema_minor: INDEX_SCHEMA_MINOR,
        records: vec![],
    });
    let previous: Vec<_> = index
        .records
        .iter()
        .map(
            |r| json!({"key":r.request_key,"codegen":r.codegen_version,"sha256":r.digest.as_hex()}),
        )
        .collect();
    let hit = index.find(&key).is_some();
    assert!(!hit, "每次探针必须验证失配与实际重生成");
    let request = NativeBuildRequest::new(
        FrontendRequest::from_text(source),
        target.clone(),
        toolchain.clone(),
        out,
    )
    .with_codegen_options(options);
    let built = FrontendNativeDriver::new().build(&request).unwrap();
    let bytes = fs::read(out).unwrap();
    let object = store.put(ObjectKind::Native, &bytes).unwrap();
    index.upsert(GlobalRecord {
        request_key: key.clone(),
        object_kind: ObjectKind::Native,
        digest: object.digest,
        target: target.triple,
        optimization_level: 0,
        codegen_version: requested_codegen,
        length: bytes.len() as u64,
    });
    let index_path = index_store.write_global(&index).unwrap();
    assert_eq!(
        index_store
            .read_global()
            .unwrap()
            .find(&key)
            .unwrap()
            .digest,
        object.digest
    );
    let executed = Command::new(out).output().unwrap();
    assert!(executed.status.success());
    assert_eq!(
        String::from_utf8_lossy(&executed.stdout),
        "B5-CACHE-ENTRY\n"
    );
    println!(
        "{}",
        json!({"cache_scope":"production ArtifactStore/GlobalIndex with production Toolchain fingerprint; explicit probe orchestration",
      "input_sha256":Digest256::of_bytes(source.as_bytes()).as_hex(),"abi":ABI_ENCODED_VERSION,
      "actual_codegen":CODEGEN_VERSION,"requested_codegen":requested_codegen,"runtime_sha256":digest(Path::new(runtime)),
      "optimization":0,"optimization_plan":plan,"fingerprint":fingerprint,"request_key":key,"previous":previous,
      "cache_hit":false,"regenerated":true,"executed":true,"executable":out.display().to_string(),"sha256":object.digest.as_hex(),
      "object_path":object.path.display().to_string(),"index_path":index_path.display().to_string(),
      "index_sha256":digest(&index_path),"library":runtime,"llvm_sha256":Digest256::of_bytes(built.native.module.text.as_bytes()).as_hex()})
    );
}
fn numeric(out: &Path) {
    fs::create_dir(out).unwrap();
    let definitions = [
        (
            "integer-ok",
            "def probe(int a) -> int\n    return a + 1\nprobe(41) == 42\n",
            false,
        ),
        (
            "integer-overflow",
            "def probe(int a) -> int\n    return a + 1\nprobe(9223372036854775807)\n",
            true,
        ),
        (
            "narrow-overflow",
            "def probe(int a) -> sint\n    return sint(a)\nprobe(2147483648)\n",
            true,
        ),
        (
            "float-ok",
            "def probe(float a, float b) -> float\n    return a ** b\nprobe(2.0, 3.0) == 8.0\n",
            false,
        ),
        (
            "float-nonfinite",
            "def probe(float a, float b) -> float\n    return a ** b\nprobe(1e308, 2.0)\n",
            true,
        ),
    ];
    let target = TargetDescription::host();
    let tools = tools(None);
    let mut rows = Vec::new();
    for (name, text, fails) in definitions {
        for forced in [false, true] {
            let source = if forced {
                format!("noise = \"independent\"\n{text}")
            } else {
                text.to_owned()
            };
            let product = artifact(&source);
            assert!(!product.ir.runtime_checks.is_empty(), "{name}");
            assert!(
                product
                    .ir
                    .runtime_checks
                    .iter()
                    .all(|check| check.kind == "numeric_range"),
                "{name}: {:?}",
                product.ir.runtime_checks
            );
            let mut tac = xiao_bytecode::lower_program(&product.ir);
            let index = tac
                .functions
                .iter()
                .position(|function| function.name == "probe")
                .unwrap();
            tac.functions.swap(0, index);
            tac.categories = tac.functions[0].categories.clone();
            let values = match name {
                "integer-ok" => vec![xiao_runtime::RuntimeValue::Int(41)],
                "integer-overflow" => vec![xiao_runtime::RuntimeValue::Int(i64::MAX)],
                "narrow-overflow" => vec![xiao_runtime::RuntimeValue::Int(2_147_483_648)],
                "float-ok" => vec![
                    xiao_runtime::RuntimeValue::Float(2.0),
                    xiao_runtime::RuntimeValue::Float(3.0),
                ],
                _ => vec![
                    xiao_runtime::RuntimeValue::Float(1e308),
                    xiao_runtime::RuntimeValue::Float(2.0),
                ],
            };
            let vm = xiao_vm::run_with_values::<xiao_vm::StackCarrier>(
                &tac,
                Default::default(),
                &values,
            );
            let span = if let xiao_vm::RunResult::Error(error) = &vm.result {
                let location = error.report().location.unwrap();
                assert_eq!(error.code(), "X06-RUNTIME-009");
                Some((location.start(), location.end()))
            } else {
                assert!(!fails, "{name}: {:?}", vm.result);
                None
            };
            if !fails {
                assert!(vm.result.is_success());
                assert!(
                    matches!(
                        vm.value,
                        Some(xiao_runtime::RuntimeValue::Int(42))
                            | Some(xiao_runtime::RuntimeValue::Float(8.0))
                    ),
                    "{:?}",
                    vm.value
                );
            }
            for level in [0, 3] {
                let options = CodegenOptions::for_target(target.clone())
                    .with_optimization_level(level)
                    .unwrap()
                    .with_entry_observation(xiao_codegen_llvm::EntryObservation::ExitCode);
                let executable = out.join(format!("{name}-{forced}-{level}.exe"));
                let request = NativeBuildRequest::new(
                    FrontendRequest::from_text(&source),
                    target.clone(),
                    tools.clone(),
                    &executable,
                )
                .with_codegen_options(options);
                let built = FrontendNativeDriver::new()
                    .build_artifact(&product, &request)
                    .unwrap();
                let dynamic = built.native.module.text.contains("define void @xiao.fn.");
                assert_eq!(
                    dynamic,
                    forced || name.starts_with("integer") || name.starts_with("narrow")
                );
                let result = Command::new(&executable).output().unwrap();
                let stderr = String::from_utf8_lossy(&result.stderr);
                if let Some((start, end)) = span {
                    assert_ne!(result.status.code(), Some(0));
                    assert!(
                        stderr.contains("code=X06-RUNTIME-009")
                            && stderr.contains(&format!("span={start}..{end}")),
                        "{name} {forced} O{level}: {stderr}"
                    );
                } else {
                    assert_eq!(result.status.code(), Some(1), "{name}: {stderr}");
                }
                rows.push(json!({"case":name,"forced_dynamic":forced,"lowering_dynamic":dynamic,"optimization":level,
                    "source":source,"source_sha256":Digest256::of_bytes(source.as_bytes()).as_hex(),"span":span,
                    "expected_error":if fails{Some("X06-RUNTIME-009")}else{None},"vm_success":vm.result.is_success(),
                    "native_exit":result.status.code(),"native_stderr":stderr,"executable_sha256":digest(&executable)}));
            }
        }
    }
    println!("{}", json!({"passed":true,"cases":rows.len(),"rows":rows}));
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args[1].as_str() {
        "generate" => generate(Path::new(&args[2]), args.get(3).map(Path::new)),
        "numeric" => numeric(Path::new(&args[2])),
        "validate" => validate(&args[2], Path::new(&args[3]), &args[4]),
        "cache" => cache(
            Path::new(&args[2]),
            Path::new(&args[3]),
            args[4].parse().unwrap(),
            &args[5],
        ),
        _ => panic!("未知B5宿主动作"),
    }
}
