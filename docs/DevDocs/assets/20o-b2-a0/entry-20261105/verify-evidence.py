"""复核 A0 归档摘要、实际测试汇总与计数守卫；不重新运行产品。"""

import hashlib
import json
from pathlib import Path
import re


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def main():
    root = Path(__file__).resolve().parent
    manifest = json.loads((root / "evidence.json").read_text(encoding="utf-8"))
    for item in manifest["files"]:
        path = (root / item["path"]).resolve()
        require(path.is_relative_to(root), "证据路径越界")
        raw = path.read_bytes()
        require(len(raw) == item["bytes"] and hashlib.sha256(raw).hexdigest() == item["sha256"],
                "证据摘要不符：" + item["path"])
    expected = {path.relative_to(root).as_posix() for path in root.rglob("*") if path.is_file() and path.name != "evidence.json"}
    require(expected == {item["path"] for item in manifest["files"]}, "证据清单缺失、重复或含额外文件")
    require(len(manifest["files"]) == len(expected), "证据清单含重复路径")
    commands = json.loads((root / "commands.json").read_text(encoding="utf-8"))
    require([item["name"] for item in commands if item["exit_code"] != 0] == ["repo-all"], "存在未登记的初轮失败")
    require(all(item["passed"] == (item["exit_code"] == 0) for item in commands), "初轮退出码与通过声明冲突")
    indexed = {item["name"]: item for item in commands}
    required = {
        "rustc-version", "cargo-version", "bun-version", "powershell-version", "clang-version",
        "llvm-as-version", "llc-version", "link-version", "build-runtime", "build-diagnostics",
        "build-doc-adapter", "20o_b1_imp_contract", "20o_b1_imp_resources", "vm-exact",
        "windows-native-exact", "repo-all", "workspace-clippy", "workspace-check",
        "benchmarks-check", "lock-diff", "b1-guards", "source-diff-check",
    }
    require(set(indexed) == required and len(indexed) == len(commands), "实际命令集合漂移或重复")
    counts = {"20o_b1_imp_contract": 9, "20o_b1_imp_resources": 17, "vm-exact": 1, "windows-native-exact": 1}
    logs = {}
    for name, item in indexed.items():
        raw = (root / item["log"]).read_bytes()
        require(hashlib.sha256(raw).hexdigest() == item["sha256"], "命令日志摘要不符：" + name)
        logs[name] = raw.decode("utf-8", errors="replace")
        require(isinstance(item["pid"], int) and item["pid"] > 0, "缺少实际 PID")
        if name in counts:
            found = re.findall(r"(?m)^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;[^\r\n]*", logs[name])
            require(found == [("ok", str(counts[name]), "0", "0")], "实际测试汇总不符：" + name)
            require(item["test_summaries"] == [list(found[0])], "声明的测试汇总与日志不符")
        if name in {"vm-exact", "windows-native-exact"}:
            require("--exact" in item["argv"] and "--nocapture" in item["argv"], "缺少精确执行参数")
    for module in ["math", "hex", "base64", "url", "csv", "fmt", "statistics"]:
        for level in [0, 3]:
            marker = f"20O-B1-IMP-PROBE case={module} backend=vm/native optimization={level} result=success"
            require(logs["windows-native-exact"].splitlines().count(marker) == 1, "原生七库 O0/O3 标记缺失或重复：" + marker)
    require(indexed["repo-all"]["exit_code"] == 2 and "Cannot find package 'typescript'" in logs["repo-all"], "初轮文档失败原因不符")
    for name, exit_code in [("repo-all-retry", 1), ("repo-all-local-copy", 0), ("workspace-doc-gate", 0)]:
        item = json.loads((root / (name + ".json")).read_text(encoding="utf-8"))
        raw = (root / item["log"]).read_bytes()
        require(hashlib.sha256(raw).hexdigest() == item["sha256"], "额外门禁日志摘要不符：" + name)
        require(item["exit_code"] == exit_code and item["passed"] == (exit_code == 0), "额外门禁实际结果不符：" + name)
        require(item["argv"] == indexed["repo-all"]["argv"], "额外门禁执行点漂移")
        if name == "repo-all-retry":
            errors = [line for line in raw.decode("utf-8").splitlines() if line.startswith("[error]")]
            require(len(errors) == 1 and "A0-LAYOUT-002 tools/doc-coverage/node_modules" in errors[0], "第一次重试没有按具体目录原因失败")
        elif name == "repo-all-local-copy":
            require(item["cwd"] == indexed["repo-all"]["cwd"], "源码门禁重试来源漂移")
    guards = json.loads((root / "b1-guards.json").read_text(encoding="utf-8"))
    require(guards["scope"] == "simulated-cargo-output", "守卫范围不符")
    require(guards["passed"] == guards["total"] == len(guards["cases"]) == 28, "守卫不是 28/28")
    require(all(item["Passed"] for item in guards["cases"]), "守卫存在失败")
    result = json.loads((root / "result.json").read_text(encoding="utf-8"))
    require(not result["passed"] and result["bound_files_unchanged"] and result["source_status_after"] == "", "初轮记录被改写或源码发生变化")
    require(result["linux"] == result["macos"] == "未验证", "未执行平台状态错误")
    require(not result["b1_721_executed"] and not result["a1_started"], "超出 A0 边界")
    invocation = json.loads((root / "driver-invocation.json").read_text(encoding="utf-8"))
    require(hashlib.sha256((root / "run-baseline-observed.py").read_bytes()).hexdigest() == invocation["runner_sha256"], "实际执行器快照摘要不符")
    boundary = json.loads((root / "boundary.json").read_text(encoding="utf-8"))
    closeout = json.loads((root / "closeout.json").read_text(encoding="utf-8"))
    require(boundary["passed"] and closeout["passed"] and closeout["workspace_doc_gate_passed"], "最终边界或收口未通过")
    print(json.dumps({"passed": True, "files": len(expected), "commands": len(commands), "tests": counts,
                      "guards": "28/28", "scope": "仅归档自洽性复核，非新产品运行"}, ensure_ascii=False))


if __name__ == "__main__":
    main()
