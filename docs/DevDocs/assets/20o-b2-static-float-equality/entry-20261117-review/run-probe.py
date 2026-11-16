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
sys.path.insert(0, str(PAST))
import record

def require(condition, message):
    if not condition:
        raise ValueError(message)

def sha(raw):
    return hashlib.sha256(raw).hexdigest()

def file_identity(path):
    raw = path.read_bytes()
    return {"path": str(path), "bytes": len(raw), "sha256": sha(raw)}

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

def write_new(path, value):
    with path.open("xb") as output:
        output.write((json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False)
                      + chr(10)).encode("utf-8"))

require(not (ROOT / "probe-run.json").exists(), "探针执行回执不可覆盖")
build_path = ROOT / "probe-build.json"
build = json.loads(build_path.read_text(encoding="utf-8"))
require(build["exit_code"] == 0 and build["executable"] is not None,
        "探针尚未成功编译")
require(build["expected_codegen"] == 21 and build["expected_runtime_abi"] == "1.19",
        "探针不是登记的未修复基线")
task = Path(build["task_root"])
require(task.resolve() == Path("D:/project/Xiao/.b5/float-equality-preflight-20261117-01").resolve(),
        "探针任务根不符")
observations = task / "observations"
require(not observations.exists(), "观察目录已存在；禁止复用执行文件")
executable = Path(build["executable"]["path"])
require(file_identity(executable) == build["executable"], "探针可执行文件身份变化")
require(file_identity(ROOT / "probe.rs") == build["probe_source"], "探针源码身份变化")
compiler = Path(build["compiler"]["path"])
require(file_identity(compiler) == build["compiler"], "探针编译器身份变化")
runtime = Path(build["runtime_static_library"]["path"])
require(file_identity(runtime) == build["runtime_static_library"], "Runtime静态库身份变化")
protected_before = record.git(REPOSITORY, "status", "--porcelain", "--", *record.PRODUCT_ROOTS)
require(not protected_before.strip(), "产品工作区仍有未获批改动")
libraries_before = {
    name: file_identity(Path(item["path"])) for name, item in build["libraries_after"].items()
}
require(libraries_before == build["libraries_after"], "借用调试库身份变化")
tool_paths = {
    "clang": Path("D:/msys64/ucrt64/bin/clang.exe"),
    "llvm-as": Path("D:/msys64/ucrt64/bin/llvm-as.exe"),
    "llc": Path("D:/msys64/ucrt64/bin/llc.exe"),
}
tools = {}
for name, path in tool_paths.items():
    require(path.is_file(), "缺少显式工具：" + name)
    version = subprocess.run([str(path), "--version"], capture_output=True,
                             encoding="utf-8", check=True)
    tools[name] = {
        "binary": file_identity(path), "version_argv": [str(path), "--version"],
        "version_stdout": version.stdout, "version_stderr": version.stderr,
    }
argv = [
    str(executable), str(observations), str(runtime),
    str(tool_paths["clang"]), str(tool_paths["llvm-as"]), str(tool_paths["llc"]),
    str(compiler),
]
environment = {
    key: value for key, value in os.environ.items()
    if not key.upper().startswith(("XIAO_", "RUST_BIGDECIMAL_"))
}
environment.update(build["environment_overrides"])
started = datetime.now(timezone.utc).isoformat()
stdout_path = task / "probe.stdout"
stderr_path = task / "probe.stderr"
with stdout_path.open("xb") as stdout, stderr_path.open("xb") as stderr:
    child = subprocess.Popen(argv, cwd=task, env=environment, stdin=subprocess.DEVNULL,
                             stdout=stdout, stderr=stderr)
    status = child.wait()
finished = datetime.now(timezone.utc).isoformat()
raw_stdout = stdout_path.read_bytes()
raw_stderr = stderr_path.read_bytes()
parsed = None
parse_error = None
try:
    parsed = json.loads(raw_stdout)
except (json.JSONDecodeError, UnicodeDecodeError) as error:
    parse_error = str(error)
archived = {}
native_executables = {}
if observations.is_dir():
    for path in sorted(observations.iterdir()):
        if path.is_file() and path.suffix in {".ll", ".bc"}:
            archived[path.name] = store(path.read_bytes())
        elif path.is_file() and path.suffix == ".exe":
            native_executables[path.name] = file_identity(path)
libraries_after = {
    name: file_identity(Path(item["path"])) for name, item in build["libraries_after"].items()
}
tools_after = {name: file_identity(path) for name, path in tool_paths.items()}
protected_after = record.git(REPOSITORY, "status", "--porcelain", "--", *record.PRODUCT_ROOTS)
shape_valid = bool(
    isinstance(parsed, dict) and parsed.get("runtime_abi") == 65555
    and parsed.get("codegen_version") == 21 and len(parsed.get("cases", [])) == 16
)
receipt = {
    "scope": "unchanged generator-21 semantic and assembly baseline; not repair acceptance",
    "build_receipt_sha256": sha(build_path.read_bytes()),
    "run_script": file_identity(Path(__file__).resolve()),
    "probe_source": file_identity(ROOT / "probe.rs"),
    "executable_before": build["executable"], "executable_after": file_identity(executable),
    "runtime_static_library": file_identity(runtime), "compiler": file_identity(compiler),
    "libraries_before": libraries_before, "libraries_after": libraries_after,
    "tools_before": tools, "tools_after": tools_after,
    "libraries_unchanged": libraries_before == libraries_after,
    "product_worktree_clean_before": not protected_before.strip(),
    "product_worktree_clean_after": not protected_after.strip(),
    "argv": argv, "cwd": str(task), "pid": child.pid,
    "environment_overrides": build["environment_overrides"],
    "started_utc": started, "finished_utc": finished, "exit_code": status,
    "stdout": store(raw_stdout), "stderr": store(raw_stderr),
    "json_parse_error": parse_error, "matrix_shape_valid": shape_valid,
    "archived_llvm_files": archived, "native_executables": native_executables,
    "repair_applied": False,
}
write_new(ROOT / "probe-run.json", receipt)
if parsed is not None:
    write_new(ROOT / "observations.json", parsed)
interesting = []
if isinstance(parsed, dict):
    for row in parsed.get("cases", []):
        if row["case"] not in {"float-zero-eq", "float-zero-ne", "sfloat-zero-eq", "sfloat-zero-ne"}:
            continue
        entry = {"case": row["case"], "vm": row["vm"]}
        for mode in ["static", "dynamic"]:
            observed = row[mode]
            entry[mode] = {
                "frontend_or_lowering_error": observed.get("error"),
                "observations": [{
                    "optimization": item.get("optimization"),
                    "actual_uses_runtime": item.get("actual_uses_runtime"),
                    "assembly_exit_code": item.get("assembly", {}).get("exit_code"),
                    "native_executed": item.get("native_executed", False),
                    "execution": item.get("execution"),
                    "build_error": item.get("build_error"),
                } for item in observed.get("observations", [])],
            }
        interesting.append(entry)
print(json.dumps({
    "probe_exit_code": status, "matrix_shape_valid": shape_valid,
    "reference": parsed.get("reference") if isinstance(parsed, dict) else None,
    "interesting_cases": interesting, "json_parse_error": parse_error,
    "libraries_unchanged": receipt["libraries_unchanged"],
    "product_worktree_clean_after": receipt["product_worktree_clean_after"],
    "stderr": raw_stderr.decode("utf-8", errors="replace")[-6000:],
    "receipt": str(ROOT / "probe-run.json"),
}, ensure_ascii=False, allow_nan=False))
require(libraries_before == libraries_after, "执行期间借用调试库发生变化")
require(not protected_after.strip(), "执行期间产品工作区发生变化")
require(all(tools_after[name] == tools[name]["binary"] for name in tools),
        "执行期间外部工具身份变化")
require(status == 0 and shape_valid, "基线探针未完成；原始输出已保留")