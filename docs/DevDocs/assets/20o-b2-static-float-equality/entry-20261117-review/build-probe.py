from datetime import datetime, timezone
import gzip
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
PAST = REPOSITORY / "docs/DevDocs/assets/20o-b2-b/entry-20261116-b5-rebuild"
TIMING = REPOSITORY / "docs/DevDocs/assets/20o-b2-b/entry-20261116-b5-url-timing-20261116-01"
CACHED = Path("D:/project/Xiao/.b5/url-timing-20261116-01/B")
TASK = Path("D:/project/Xiao/.b5/float-equality-preflight-20261117-01")
BASELINE = "5864cbde627f727483ab676c6f4bc1bf0a7819bb"
sys.path.insert(0, str(PAST))
import record

def require(condition, message):
    if not condition:
        raise ValueError(message)

def sha(raw):
    return hashlib.sha256(raw).hexdigest()

def load(path):
    return json.loads(path.read_text(encoding="utf-8"))

def store(raw):
    packed = gzip.compress(raw, mtime=0) if len(raw) > 8192 else raw
    digest = sha(packed)
    path = ROOT / "objects/sha256" / digest[:2] / (digest + ".bin")
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        require(path.read_bytes() == packed, "探针内容对象损坏")
    else:
        with path.open("xb") as output:
            output.write(packed)
    return {
        "path": path.relative_to(ROOT).as_posix(), "bytes": len(packed), "sha256": digest,
        "encoding": "gzip" if len(raw) > 8192 else "identity",
        "raw_bytes": len(raw), "raw_sha256": sha(raw),
    }

def file_identity(path):
    raw = path.read_bytes()
    return {"path": str(path), "bytes": len(raw), "sha256": sha(raw)}

require(not (ROOT / "probe-build.json").exists(), "探针构建回执不可覆盖")
require(not TASK.exists() and TASK.parent.resolve() == Path("D:/project/Xiao/.b5").resolve(),
        "探针任务根必须全新且位于指定目录")
protected_status = record.git(REPOSITORY, "status", "--porcelain", "--", *record.PRODUCT_ROOTS)
require(not protected_status.strip(), "主工作区仍有产品改动；先核对回退状态")
source = load(TIMING / "sources/B.json")
build = load(TIMING / "builds/B.json")
require(source["commit"] == build["commit"] == BASELINE, "借用构建的源码提交不符")
require(record.worktree_files(CACHED / "source") == source["files"],
        "借用构建的固定诊断副本已变化")
dependencies = CACHED / "target/debug/deps"
fingerprints = CACHED / "target/debug/.fingerprint"
driver = dependencies / "libxiao_driver-60d0e559fa11f209.rlib"
require(driver.is_file(), "已登记的调试驱动库缺失")
driver_meta_path = fingerprints / "xiao-driver-60d0e559fa11f209/lib-xiao_driver.json"
driver_meta = load(driver_meta_path)
required_ids = {row[1]: row[3] for row in driver_meta["deps"]}
libraries = {"xiao_driver": driver}
library_fingerprints = {}
for name in ["xiao_codegen_llvm", "xiao_vm", "xiao_runtime", "xiao_syntax",
             "xiao_runtime_abi", "serde_json"]:
    matched = []
    for candidate in dependencies.glob("lib" + name + "-*.rlib"):
        suffix = candidate.stem.removeprefix("lib" + name + "-")
        for prefix in {name, name.replace("_", "-")}:
            fingerprint = fingerprints / (prefix + "-" + suffix) / ("lib-" + name)
            if fingerprint.is_file():
                value = int.from_bytes(bytes.fromhex(fingerprint.read_text().strip()), "little")
                if value == required_ids[name]:
                    matched.append((candidate, fingerprint))
    require(len(matched) == 1, "探针依赖指纹不能唯一匹配：" + name)
    libraries[name] = matched[0][0]
    library_fingerprints[name] = file_identity(matched[0][1])
runtime_libraries = [item for item in build["runtime_objects"]
                     if Path(item["path"]).suffix == ".lib"]
require(len(runtime_libraries) == 1, "固定构建没有唯一Runtime静态库")
runtime_identity = runtime_libraries[0]
require(file_identity(Path(runtime_identity["path"])) == runtime_identity,
        "借用Runtime静态库的字节身份变化")
located = subprocess.run(["rustup", "which", "--toolchain", "1.96.0", "rustc"],
                         capture_output=True, encoding="utf-8", check=True)
compiler = Path(located.stdout.strip())
version = subprocess.run([str(compiler), "-Vv"], capture_output=True,
                         encoding="utf-8", check=True)
require(version.stdout.startswith("rustc 1.96.0 "), "探针编译器不是固定Rust1.96.0")
TASK.mkdir(parents=True)
(TASK / "temp").mkdir()
(TASK / "home").mkdir()
probe_source = ROOT / "probe.rs"
executable = TASK / "baseline-probe.exe"
argv = [
    str(compiler), "--edition=2024", "--crate-name", "static_float_baseline_probe",
    str(probe_source), "-L", "dependency=" + str(dependencies), "-o", str(executable),
]
for output_path in sorted((CACHED / "target/debug/build").glob("*/output")):
    for line in output_path.read_text(encoding="utf-8").splitlines():
        if line.startswith("cargo:rustc-link-search="):
            argv.extend(["-L", line.split("=", 1)[1]])
for name, path in libraries.items():
    argv.extend(["--extern", name + "=" + str(path)])
environment = {key: value for key, value in os.environ.items()
               if not key.upper().startswith(("XIAO_", "RUST_BIGDECIMAL_"))}
overrides = {
    "RUSTUP_TOOLCHAIN": "1.96.0", "TEMP": str(TASK / "temp"), "TMP": str(TASK / "temp"),
    "XIAO_HOME": str(TASK / "home"), "XIAO_STDLIB_ROOT": str(CACHED / "source/stdlib"),
    "PYTHONUTF8": "1", "PYTHONIOENCODING": "utf-8",
}
environment.update(overrides)
library_before = {name: file_identity(path) for name, path in libraries.items()}
started = datetime.now(timezone.utc).isoformat()
log_path = TASK / "rustc.log"
with log_path.open("xb") as output:
    child = subprocess.Popen(argv, cwd=TASK, env=environment,
                             stdout=output, stderr=subprocess.STDOUT)
    status = child.wait()
finished = datetime.now(timezone.utc).isoformat()
raw_log = log_path.read_bytes()
library_after = {name: file_identity(path) for name, path in libraries.items()}
receipt = {
    "scope": "compile a standalone unchanged-generator baseline probe; no execution yet",
    "baseline_code_commit": BASELINE, "expected_runtime_abi": "1.19",
    "expected_codegen": 21, "task_root": str(TASK),
    "probe_source": file_identity(probe_source),
    "build_script": file_identity(Path(__file__).resolve()),
    "cached_source_receipt_sha256": sha((TIMING / "sources/B.json").read_bytes()),
    "cached_build_receipt_sha256": sha((TIMING / "builds/B.json").read_bytes()),
    "cached_source_identity": source["identity"], "runtime_static_library": runtime_identity,
    "compiler": file_identity(compiler), "compiler_version_stdout": version.stdout,
    "compiler_version_stderr": version.stderr, "driver_fingerprint": file_identity(driver_meta_path),
    "selected_dependency_fingerprints": library_fingerprints,
    "libraries_before": library_before, "libraries_after": library_after,
    "libraries_unchanged": library_before == library_after,
    "argv": argv, "cwd": str(TASK), "pid": child.pid, "environment_overrides": overrides,
    "started_utc": started, "finished_utc": finished, "exit_code": status,
    "log": store(raw_log),
    "executable": file_identity(executable) if status == 0 and executable.is_file() else None,
    "probe_executed": False, "repair_applied": False,
}
with (ROOT / "probe-build.json").open("xb") as output:
    output.write((json.dumps(receipt, ensure_ascii=False, indent=2) + chr(10)).encode("utf-8"))
require(library_before == library_after, "探针编译期间借用库身份变化")
print(json.dumps({
    "build_exit_code": status, "probe_executed": False,
    "executable": receipt["executable"], "receipt": str(ROOT / "probe-build.json"),
    "compiler_output": raw_log.decode("utf-8")[-8000:],
}, ensure_ascii=False))
raise SystemExit(status)