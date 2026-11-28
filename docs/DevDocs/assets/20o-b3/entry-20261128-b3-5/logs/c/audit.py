"""复核生成差分、静态身份和逐件产物，归档 Windows 原件。"""

import difflib
import hashlib
import json
from pathlib import Path
import re
import struct
import sys
import zipfile


LOGS = Path(__file__).resolve().parent
REPO = LOGS.parents[6]
TASK = Path(sys.argv[1])
CASES = TASK / "cases"
SYMBOL = "xiao_runtime_log_write"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def file_info(path):
    data = path.read_bytes()
    return {"path": str(path), "bytes": len(data), "sha256": sha(data)}


def write_json(name, data):
    (LOGS / name).write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n",
                             encoding="utf-8")


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def load(name):
    result = read_json(CASES / name / "verification.json")
    assert result["generator"] == 27 and result["runtime"]["abi"] == 65556
    assert result["stdlib_interface"] == 1
    assert result["target"] == "x86_64-pc-windows-msvc"
    for role in ("binary", "ir", "map", "runtime"):
        path = Path(result[role]["path"])
        assert file_info(path)["sha256"] == result[role]["sha256"], (name, role)
    return result


def only_version_28(text):
    found = re.search(r'(?m)^@xiao_native_metadata = constant \[\d+ x i8\] c"([^"]+)"', text)
    assert found
    encoded = bytearray.fromhex(found.group(1).replace("\\", ""))
    assert encoded[:8] == b"XIAOMETA" and struct.unpack_from("<I", encoded, 40) == (27,)
    struct.pack_into("<I", encoded, 40, 28)
    body = "".join(f"\\{byte:02X}" for byte in encoded)
    return text[:found.start(1)] + body + text[found.end(1):]


def check_generator():
    summary = []
    excerpts = []
    differences = []
    for level in (0, 3):
        failure = f"before-log-o{level}-product-final"
        denied = read_json(LOGS / f"{failure}.json")
        assert denied["exit"] == 1 and not (CASES / failure / "program.ll").exists()
        reason = (LOGS / f"{failure}.stderr.log").read_text(encoding="utf-8").strip()
        assert "不支持 动态 intrinsic os_log_write" in reason
        before = load(f"before-control-o{level}-product-final")
        control = load(f"after-control-o{level}-product-final")
        logged = load(f"after-log-o{level}-product-final")
        assert before["runtime"] == control["runtime"] == logged["runtime"]
        assert before["source"] == control["source"] or (
            before["source"]["sha256"] == control["source"]["sha256"]
        )
        assert before["capabilities"] == control["capabilities"] == logged["capabilities"] == 34
        assert before["exit"] == control["exit"] == logged["exit"] == 0
        before_ir = Path(before["ir"]["path"]).read_bytes()
        control_ir = Path(control["ir"]["path"]).read_bytes()
        assert before_ir == control_ir and SYMBOL.encode() not in control_ir
        assert SYMBOL not in control["runtime_symbols"]
        assert SYMBOL in logged["runtime_symbols"]
        text = Path(logged["ir"]["path"]).read_text(encoding="utf-8")
        lines = text.splitlines()
        assert f"declare i32 @{SYMBOL}(ptr, ptr, ptr, i64)" in lines
        calls = [i for i, line in enumerate(lines) if f"call i32 @{SYMBOL}(" in line]
        assert len(calls) == 1
        call = lines[calls[0]]
        assert re.search(r'\(ptr %\w+, ptr %\w+, ptr %\w+, i64 34\)$', call)
        release_and_dispatch = lines[calls[0] + 1:calls[0] + 5]
        assert all("xiao_runtime_value_release" in line for line in release_and_dispatch[:3])
        assert "i32 6" in release_and_dispatch[3] and "i32 8" in release_and_dispatch[3]
        assert "call i32 @xiao_runtime_env_run_native(ptr @xiao_application_main, i64 34, i32 0)" in text
        excerpts.extend([f"O{level} 接线前退出 1", reason, f"O{level} 接线后退出 0",
                         f"declare i32 @{SYMBOL}(ptr, ptr, ptr, i64)", call,
                         *release_and_dispatch, f"无日志 IR SHA-256 {sha(control_ir)}", ""])
        version_28 = only_version_28(text)
        differences.extend(difflib.unified_diff(text.splitlines(keepends=True),
                                                version_28.splitlines(keepends=True),
                                                fromfile=f"log-o{level}-27",
                                                tofile=f"log-o{level}-仅改变元数据字段为28"))
        summary.append({
            "optimization": level, "before_log_rejection": reason,
            "before_control": before, "after_control": control, "after_log": logged,
            "version_28_only_ir_sha256": sha(version_28.encode("utf-8")),
        })
    (LOGS / "lowering-27.log").write_text("\n".join(excerpts), encoding="utf-8")
    (LOGS / "version-28-only.diff").write_text("".join(differences), encoding="utf-8")
    write_json("generator-diff.json", summary)


def check_cache():
    old = load("cache-old-runtime-control-o0")
    new = load("after-control-o0-product-final")
    changed = load("cache-changed-mask-log-o0-final")
    log = load("after-log-o0-product-final")
    assert old["ir"]["sha256"] == new["ir"]["sha256"]
    assert old["runtime"]["sha256"] != new["runtime"]["sha256"]
    assert old["fingerprints"]["toolchain"] != new["fingerprints"]["toolchain"]
    assert old["fingerprints"]["codegen"] == new["fingerprints"]["codegen"]
    assert changed["capabilities"] == 2 and changed["exit"] == 3
    assert changed["runtime"] == log["runtime"]
    assert changed["fingerprints"]["codegen"] != log["fingerprints"]["codegen"]
    assert read_json(LOGS / "cache-old-runtime-log-o0.json")["exit"] == 1
    baseline_log = REPO / "stdlib/log.xiao"
    source_log = TASK / "before/stdlib/log.xiao"
    assert sha(baseline_log.read_bytes()) == sha(source_log.read_bytes())
    rows = {}
    for key, result in (
        ("old_runtime", old), ("rebuilt_runtime", new),
        ("log", log), ("log_missing_stdout", changed),
    ):
        identity = {
            "abi": result["runtime"]["abi"],
            "generator": result["generator"],
            "target": result["target"],
            "runtime": result["runtime"]["sha256"],
            "stdlib_interface": result["stdlib_interface"],
            "stdlib": result["stdlib"],
            "stdlib_log_source_sha256": sha(baseline_log.read_bytes()),
            "capabilities": result["capabilities"],
            "toolchain_fingerprint": result["fingerprints"]["toolchain"],
            "codegen_fingerprint": result["fingerprints"]["codegen"],
            "source": result["source"]["sha256"],
        }
        rows[key] = {
            "identity": identity, "identity_sha256": sha(json.dumps(identity, sort_keys=True).encode()),
            "binary": result["binary"], "ir": result["ir"], "exit": result["exit"],
        }
    assert rows["old_runtime"]["identity_sha256"] != rows["rebuilt_runtime"]["identity_sha256"]
    assert rows["log"]["identity_sha256"] != rows["log_missing_stdout"]["identity_sha256"]
    write_json("cache-identity.json", rows)


def pack():
    paths = sorted(CASES.glob("*-product-final/*"))
    paths += sorted(CASES.glob("cache-old-runtime-control-o0/*"))
    paths += sorted(CASES.glob("cache-changed-mask-log-o0-final/*"))
    paths += sorted(CASES.glob("native-slot-final/xiao-b3-log-*/*"))
    paths += [CASES / "c-probe-final/product.exe", CASES / "c-probe-final/product.map"]
    paths = sorted({path for path in paths if path.is_file()})
    archive = LOGS / "windows-artifacts.zip"
    results = []
    with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED, compresslevel=6) as bundle:
        for path in paths:
            member = path.relative_to(CASES).as_posix()
            data = path.read_bytes()
            identity = {"path": str(path), "zip_member": member, "bytes": len(data),
                        "sha256": sha(data)}
            if path.name.endswith("verification.json"):
                linked = read_json(path)
                for role in ("binary", "ir", "map"):
                    assert file_info(Path(linked[role]["path"]))["sha256"] == linked[role]["sha256"]
            bundle.writestr(member, data)
            results.append(identity)
    with zipfile.ZipFile(archive) as bundle:
        assert bundle.testzip() is None
        for result in results:
            assert sha(bundle.read(result["zip_member"])) == result["sha256"]
    write_json("windows-artifact-manifest.json", {"archive": file_info(archive), "files": results})
    print(f"Windows ZIP 已复核 {len(results)} 个成员，SHA-256 {file_info(archive)['sha256']}")


if __name__ == "__main__":
    check_generator()
    check_cache()
    pack()
