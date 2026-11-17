//! 同一宿主分别链接真实生成器21/22；不伪造版本字段或改写历史产物。
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;
use xiao_artifacts::{
    ArtifactStore, Digest256, GlobalIndex, GlobalRecord, INDEX_SCHEMA_MAJOR,
    INDEX_SCHEMA_MINOR, IndexStore, ObjectKind,
};
use xiao_codegen_llvm::{
    BuildRequest, CODEGEN_VERSION, CodegenOptions, EntryObservation, NativeBuild,
    NativeMetadata, NativeRuntimeSupport, TargetDescription, Toolchain,
};
use xiao_driver::{FrontendCompiler, FrontendRequest};

fn tools() -> Toolchain {
    Toolchain::new(std::env::var_os("XIAO_CLANG").unwrap())
        .with_llvm_as(std::env::var_os("XIAO_LLVM_AS").unwrap())
        .with_llc(std::env::var_os("XIAO_LLC").unwrap())
        .with_runtime_library(std::env::var_os("XIAO_RUNTIME_LIBRARY").unwrap())
        .probe_versions().unwrap()
        .probe_native_static_libraries(std::env::var_os("RUSTC").unwrap(), &TargetDescription::host())
        .unwrap()
}

fn metadata(path: &Path) {
    assert!(!path.exists());
    let artifact = FrontendCompiler::new()
        .compile(&FrontendRequest::from_text("value = 1\n")).unwrap();
    let metadata = NativeMetadata::for_program(&artifact.ir, &TargetDescription::host()).unwrap();
    assert_eq!(metadata.generator_version, CODEGEN_VERSION);
    let bytes = metadata.encode().unwrap();
    fs::write(path, &bytes).unwrap();
    println!("{}", json!({
        "mode": "metadata", "actual_codegen": CODEGEN_VERSION,
        "metadata": metadata, "path": path,
        "sha256": Digest256::of_bytes(&bytes).as_hex(),
    }));
}

fn validate(path: &Path) {
    let bytes = fs::read(path).unwrap();
    let metadata = NativeMetadata::decode(&bytes).unwrap();
    let result = metadata.validate(&TargetDescription::host(), &NativeRuntimeSupport::default(), 0);
    let expected = metadata.generator_version <= CODEGEN_VERSION;
    assert_eq!(result.is_ok(), expected);
    println!("{}", json!({
        "mode": "validate", "actual_validator": CODEGEN_VERSION,
        "artifact_generator": metadata.generator_version, "accepted": result.is_ok(),
        "code": result.as_ref().err().map(|error| error.code),
        "reason": result.as_ref().err().map(|error| &error.message),
        "runtime_abi_min": metadata.runtime_abi_min, "runtime_abi_max": metadata.runtime_abi_max,
        "sha256": Digest256::of_bytes(&bytes).as_hex(), "entry_executed": false,
    }));
}

fn cache(root: &Path, mode: &str, level: u8) {
    assert!(matches!(mode, "static" | "dynamic"));
    let source = if mode == "static" {
        "def compare(int left, int right) -> bool\n    return left == right\ncompare(1, 1)\n"
    } else {
        "def compare(int left, int right) -> bool\n    return left == right\nprint(compare(1, 1))\n"
    };
    let artifact = FrontendCompiler::new().compile(&FrontendRequest::from_text(source)).unwrap();
    let target = TargetDescription::host();
    let toolchain = tools();
    let mut options = CodegenOptions::for_target(target.clone())
        .with_entry_observation(if mode == "static" { EntryObservation::ExitCode }
                                else { EntryObservation::Ignore });
    options.optimization_level = level;
    let module = NativeBuild::new().lower(&artifact.ir, &options).unwrap();
    let report = module
        .native_optimization_report
        .clone()
        .with_llvm_version(&toolchain.versions.clang);
    let plan = &report.plan.fingerprint;
    let fingerprint = toolchain
        .fingerprint_with_optimization(&target, CODEGEN_VERSION, level, plan).to_string();
    let input_sha = Digest256::of_bytes(source.as_bytes()).as_hex();
    let key = format!("static-float-fix:{input_sha}:{fingerprint}");
    let store = ArtifactStore::open(root).unwrap();
    let indexes = IndexStore::open(root).unwrap();
    let before = if root.join("global.index.pb").exists() {
        indexes.read_global().unwrap()
    } else {
        GlobalIndex {
            schema_major: INDEX_SCHEMA_MAJOR, schema_minor: INDEX_SCHEMA_MINOR,
            records: Vec::new(),
        }
    };
    let previous = &before.records;
    let hit = before.find(&key);
    let same_version = before.find_matching(&target.triple, u32::from(level), CODEGEN_VERSION)
        .filter(|row| row.object_kind == ObjectKind::Native).count();
    let executable = root.join(format!("{mode}-o{level}-g{CODEGEN_VERSION}{}", std::env::consts::EXE_SUFFIX));
    let regenerated;
    if let Some(hit) = hit {
        let raw = store.read_checked(hit.object_kind, hit.digest, Some(hit.length)).unwrap();
        fs::write(&executable, raw).unwrap();
        regenerated = false;
    } else {
        assert!(!executable.exists(), "禁止复用旧执行文件");
        let built = NativeBuild::new().build(&BuildRequest::new(
            artifact.ir, target.clone(), toolchain.clone(), &executable
        ).with_options(options)).unwrap();
        assert_eq!(built.toolchain_fingerprint.to_string(), fingerprint);
        let bytes = fs::read(&executable).unwrap();
        let object = store.put(ObjectKind::Native, &bytes).unwrap();
        indexes.upsert_global(vec![GlobalRecord {
            request_key: key.clone(), object_kind: ObjectKind::Native,
            digest: object.digest, target: target.triple.clone(),
            optimization_level: u32::from(level), codegen_version: CODEGEN_VERSION,
            length: bytes.len() as u64,
        }]).unwrap();
        regenerated = true;
    }
    let execution = Command::new(&executable).output().unwrap();
    assert_eq!(execution.status.code(), Some(if mode == "static" { 1 } else { 0 }));
    assert_eq!(String::from_utf8_lossy(&execution.stdout),
               if mode == "dynamic" { "true\n" } else { "" });
    assert!(execution.stderr.is_empty());
    let bytes = fs::read(&executable).unwrap();
    let expected_fp = toolchain.fingerprint_with_optimization(
        &target, if CODEGEN_VERSION == 21 { 22 } else { 21 }, level, plan
    ).to_string();
    assert_ne!(expected_fp, fingerprint);
    println!("{}", json!({
        "mode": "cache", "scope": "production ArtifactStore/GlobalIndex; explicit probe orchestration, not CLI cache",
        "actual_codegen": CODEGEN_VERSION, "lowering": mode, "optimization": level,
        "input_sha256": input_sha, "target": target.triple, "optimization_plan": plan,
        "runtime_abi": xiao_runtime_abi::ABI_ENCODED_VERSION,
        "runtime_library": toolchain.runtime_library, "versions": format!("{:?}", toolchain.versions),
        "native_libraries": toolchain.native_static_libraries,
        "fingerprint": fingerprint, "other_generator_fingerprint": expected_fp,
        "codegen_fingerprint": module.codegen_fingerprint,
        "previous_codegen": previous.iter().map(|row| row.codegen_version).collect::<Vec<_>>(),
        "same_version_candidates_before": same_version, "cache_hit": hit.is_some(),
        "regenerated": regenerated, "request_key": key, "executable": executable,
        "sha256": Digest256::of_bytes(&bytes).as_hex(),
        "exit_code": execution.status.code(), "stdout": String::from_utf8_lossy(&execution.stdout),
    }));
}

fn main() {
    assert_eq!(xiao_runtime_abi::ABI_ENCODED_VERSION, 65555);
    assert!(matches!(CODEGEN_VERSION, 21 | 22));
    let args: Vec<_> = std::env::args().collect();
    match args[1].as_str() {
        "metadata" => metadata(Path::new(&args[2])),
        "validate" => validate(Path::new(&args[2])),
        "cache" => cache(Path::new(&args[2]), &args[3], args[4].parse().unwrap()),
        _ => panic!("未知精确探针模式"),
    }
}
