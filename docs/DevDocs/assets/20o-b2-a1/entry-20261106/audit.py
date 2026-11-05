"""只读复核A1最终证据；--manifest只更新本证据目录的逐文件清单。"""

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess


ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]


def read(name):
    return json.loads((ROOT / name).read_text(encoding="utf-8"))


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def inventory():
    files = []
    for path in sorted(ROOT.rglob("*")):
        if path.is_file() and path.name != "evidence.json":
            raw = path.read_bytes()
            files.append({"path": path.relative_to(ROOT).as_posix(), "bytes": len(raw), "sha256": sha(raw)})
    (ROOT / "evidence.json").write_text(json.dumps({"scope": "仅A1本目录全部文件；清单自身不计入自身摘要",
        "file_count": len(files), "files": files}, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(json.dumps({"files": len(files)}, ensure_ascii=False))


def main():
    closeout = read("closeout.json")
    require(closeout["passed"], "A1尚未有效收口")
    manifest = read("evidence.json")
    observed = {path.relative_to(ROOT).as_posix() for path in ROOT.rglob("*") if path.is_file() and path.name != "evidence.json"}
    require(len(manifest["files"]) == manifest["file_count"] == len(observed), "清单计数不符")
    require(observed == {item["path"] for item in manifest["files"]}, "清单缺项或重复")
    for item in manifest["files"]:
        path = (ROOT / item["path"]).resolve()
        require(path.is_relative_to(ROOT), "证据路径越界")
        raw = path.read_bytes()
        require(len(raw) == item["bytes"] and sha(raw) == item["sha256"], "证据摘要不符：" + item["path"])
    source_ids = set()
    logs = {}
    for name in closeout["effective_checks"]:
        record = read(name + ".json")
        raw = (ROOT / record["log"]).read_bytes()
        require(record["exit_code"] == 0 and sha(raw) == record["sha256"], "验收失败或日志摘要不符：" + name)
        require(record["source_unchanged"] and record["source_before"] == record["source_after"], "命令执行期间源码输入变化：" + name)
        require(record["cwd"] == read("bootstrap.json")["source_root"], "验收源码根不符")
        require(record["runner_sha256"] == sha((ROOT / "run-step.py").read_bytes()), "验收执行器不符")
        source_ids.add(record["source_before"]["manifest"])
        if Path(record["argv"][0]).name.lower() in {"cargo", "cargo.exe"}:
            require("--locked" in record["argv"] and "+1.96.0" in record["argv"], "Cargo缺少锁定或工具链参数")
        logical = next((key for key, actual in closeout["check_roles"].items() if actual == name), name)
        logs[logical] = raw.decode("utf-8", errors="replace")
    require(len(source_ids) == 1, "有效验收不是同一份源码输入")
    snapshot = read(source_ids.pop())
    require(snapshot["raw_tree_sha256"] == closeout["raw_source_tree_sha256"], "最终源码身份不符")
    requests = "".join(closeout["code_commit"] + ":" + item["path"] + "\n" for item in snapshot["files"]).encode()
    blobs = subprocess.check_output(["cmd", "/c", "git", "cat-file", "--batch"], cwd=REPOSITORY, input=requests)
    offset = 0
    for item in snapshot["files"]:
        end = blobs.index(b"\n", offset)
        header = blobs[offset:end].split()
        require(len(header) == 3 and header[1] == b"blob", "代码提交缺少源码对象：" + item["path"])
        length = int(header[2])
        blob = blobs[end + 1:end + 1 + length]
        try:
            blob.decode("utf-8")
            canonical = blob if b"\0" in blob else blob.replace(b"\r\n", b"\n")
        except UnicodeDecodeError:
            canonical = blob
        require(sha(canonical) == item["utf8_lf_or_binary_sha256"], "测试输入偏离代码提交：" + item["path"])
        offset = end + length + 2
    require(offset == len(blobs), "Git源码对象流长度不符")
    pattern = r"(?m)^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;[^\r\n]*"
    for name, count in {"b1-contract-final": 9, "b1-resources-final": 17,
                        "b1-vm-exact-final": 1, "b1-native-exact-final": 1,
                        "a1-vm-exact-final": 1, "a1-native-exact-final": 1,
                        "identity-allocations-exact-final": 1}.items():
        require(re.findall(pattern, logs[name]) == [("ok", str(count), "0", "0")], "实际测试汇总不符：" + name)
        if count == 1:
            require("--exact" in read(closeout["check_roles"].get(name, name) + ".json")["argv"], "缺少精确目标：" + name)
    for name in ["a1_identity", "a1_admission_error", "a1_pending_finally"]:
        require(logs["a1-vm-exact-final"].splitlines().count(f"B2-A1-IDENTITY case={name} backend=vm result=success") == 1, "A1 VM示例标记不符")
        for level in [0, 3]:
            prefix = f"B2-A1-IDENTITY case={name} backend=native optimization={level} artifact="
            require(sum(line.startswith(prefix) and line.endswith(" result=success") for line in logs["a1-native-exact-final"].splitlines()) == 1, "A1原生示例标记不符")
    guards = read(closeout["guard_report"])
    require(guards["passed"] == guards["total"] == len(guards["cases"]) == 28, "B1守卫不是28/28")
    features = read("resolved-features-final.json")["packages"]
    expected = {"autocfg": ("1.5.1", []), "bigdecimal": ("0.4.11", ["std"]),
                "libm": ("0.2.16", ["arch", "default"]), "num-bigint": ("0.4.8", ["std"]),
                "num-integer": ("0.1.47", ["i128", "std"]), "num-traits": ("0.2.19", ["i128", "std"])}
    require({item["name"]: (item["version"], item["enabled_features"]) for item in features} == expected, "F-7版本或特性漂移")
    require(read("licenses-rechecked.json")["verified_count"] == 11, "许可核对不完整")
    require(read("registry-path-registration.json")["all_statuses_unchanged"], "registry状态漂移")
    receipt = read("decision-receipts.json")["receipts"][0]
    require(receipt["id"] == "A1-DICT" and receipt["answer"] == "确认：保留 str 键，区分两类证据（推荐）", "字典证据裁定不符")
    require(closeout["abi"] == "1.18" and closeout["codegen"] == 20 and closeout["active_intrinsic_max"] == 32, "版本边界不符")
    require(closeout["cmath"] == "planned" and closeout["numeric_tutorial_scope"] == "planned", "文档状态越界")
    require(not closeout["b1_721_executed"] and not closeout["a2_through_a7_started"], "超出A1范围")
    run = read(closeout["ci_run_response"])
    require(run["head_sha"] == closeout["code_commit"] and run["status"] == "completed" and run["conclusion"] == "success", "CI运行身份或结论不符")
    jobs = {item["id"]: item for item in read(closeout["ci_jobs_response"])["jobs"]}
    for platform in ["windows", "linux", "macos"]:
        record = read(closeout["ci_platform_records"][platform])
        raw = (ROOT / record["log"]).read_bytes()
        require(record["exit_code"] == 0 and record["code_commit"] == closeout["code_commit"] and sha(raw) == record["log_sha256"], "平台日志身份不符：" + platform)
        require(jobs[record["job_id"]]["conclusion"] == "success", "平台作业未成功：" + platform)
        text = raw.decode("utf-8", errors="replace")
        for case in ["a1_identity", "a1_admission_error", "a1_pending_finally"]:
            for level in [0, 3]:
                marker = f"B2-A1-IDENTITY case={case} backend=native optimization={level} artifact="
                require(sum(marker in line and line.endswith(" result=success") for line in text.splitlines()) == 1, "平台缺少真实A1执行标记：" + platform)
    print(json.dumps({"passed": True, "code_commit": closeout["code_commit"], "source_files": len(snapshot["files"]),
                      "effective_checks": len(closeout["effective_checks"]), "scope": "只读证据复核，非新产品执行"}, ensure_ascii=False))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", action="store_true")
    inventory() if parser.parse_args().manifest else main()
