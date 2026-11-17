"""封存显式LLVM矩阵的实际文本、bitcode与本机执行文件身份。"""
import gzip
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
TASK = Path(json.loads((ROOT / "environment.json").read_text(encoding="utf-8"))["task_root"])


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def identity(path):
    raw = Path(path).read_bytes()
    return {"path": str(path), "bytes": len(raw), "sha256": sha(raw)}


def store(raw):
    packed = gzip.compress(raw, mtime=0) if len(raw) > 8192 else raw
    digest = sha(packed)
    path = ROOT / "objects/sha256" / digest[:2] / (digest + ".bin")
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        require(path.read_bytes() == packed, "LLVM内容对象摘要冲突")
    else:
        with path.open("xb") as output:
            output.write(packed)
    return {
        "path": path.relative_to(ROOT).as_posix(), "bytes": len(packed), "sha256": digest,
        "encoding": "gzip" if len(raw) > 8192 else "identity",
        "raw_bytes": len(raw), "raw_sha256": sha(raw),
    }


report_path = TASK / "native/report.json"
report = json.loads(report_path.read_text(encoding="utf-8"))
require(report["generator"] == 22 and report["runtime_abi"] == 65555
        and report["cases"] == 62 and report["native_observations"] == 248
        and report["passed"] is True and len(report["rows"]) == 248,
        "原生比较报告的版本或矩阵尺寸不符")
rows = []
llvm_objects = set()
bitcode_objects = set()
executables = []
for row in report["rows"]:
    require(row["assembly_exit_code"] == 0 and row["exit_code"] == (
        0 if row["mode"] == "dynamic" else int(row["expected"])
    ), "原生观察失败：" + row["case"] + "/" + row["mode"])
    expected_stdout = (str(row["expected"]).lower() + chr(10)
                       if row["mode"] == "dynamic" else "")
    require(row["stdout"] == expected_stdout and row["stderr"] == "",
            "原生观察输出不符：" + row["case"] + "/" + row["mode"])
    llvm = identity(row["llvm"])
    bitcode = identity(row["bitcode"])
    executable = identity(row["executable"])
    llvm_object = store(Path(row["llvm"]).read_bytes())
    bitcode_object = store(Path(row["bitcode"]).read_bytes())
    llvm_objects.add(llvm_object["sha256"])
    bitcode_objects.add(bitcode_object["sha256"])
    rows.append({
        "case": row["case"], "mode": row["mode"], "optimization": row["optimization"],
        "expected": row["expected"], "actual_uses_runtime": row["actual_uses_runtime"],
        "codegen_fingerprint": row["codegen_fingerprint"],
        "toolchain_fingerprint": row["toolchain_fingerprint"],
        "llvm": {**llvm, "object": llvm_object},
        "bitcode": {**bitcode, "object": bitcode_object},
        "executable": executable, "exit_code": row["exit_code"],
        "stdout": row["stdout"], "stderr": row["stderr"],
    })
    executables.append(executable)
require(len({row["case"] for row in rows}) == 62 and len(rows) == 248,
        "原生矩阵重复或缺项")
result = {
    "scope": "Windows真实llvm-as、NativeBuild、O0/O3执行；非跨平台产物",
    "runtime_abi": 65555, "codegen_version": 22, "case_count": 62,
    "native_count": 248, "passed": True, "report": identity(report_path),
    "llvm_unique_content_objects": len(llvm_objects),
    "bitcode_unique_content_objects": len(bitcode_objects),
    "executables_recorded_not_archived": executables, "rows": rows,
}
output = ROOT / "native-results.json"
require(not output.exists(), "原生结果摘要禁止覆盖")
output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + chr(10), encoding="utf-8")
print(json.dumps({"passed": True, "rows": len(rows),
                  "llvm_unique": len(llvm_objects), "bitcode_unique": len(bitcode_objects),
                  "executables_recorded": len(executables)}, ensure_ascii=False))
