"""只读复核本次可达性修复的源码、日志和结果；不代表B3-4收口。"""
import argparse
import hashlib
import json
from pathlib import Path
import runpy

ROOT = Path(__file__).resolve().parent
RUNS = ["codegen-accepted", "driver-accepted", "clippy-accepted", "fmt-accepted", "final-docs"]


def require(condition, reason):
    if not condition:
        raise SystemExit(reason)


def inventory(root):
    rows = []
    for path in sorted(path for path in root.rglob("*") if path.is_file()):
        if path.name == "evidence.json" or "__pycache__" in path.parts:
            continue
        data = path.read_bytes()
        rows.append({"path": path.relative_to(root).as_posix(), "bytes": len(data),
                     "sha256": hashlib.sha256(data).hexdigest()})
    return rows


def read_run(root, label):
    record = json.loads((root / (label + ".json")).read_text(encoding="utf-8"))
    require(record["log"] == label + ".log", "执行记录日志路径不符：" + label)
    data = (root / record["log"]).read_bytes()
    require(record["log_bytes"] == len(data) and record["log_sha256"] == hashlib.sha256(data).hexdigest(),
            "执行日志摘要不符：" + label)
    require(record["source_unchanged"] is True and record["source_before"] == record["source_after"],
            "执行期间源码变化：" + label)
    return record, data.decode("utf-8")


def normalized_source(source):
    return [(row["path"], row["utf8_lf_sha256"]) for row in source["changed_product_files"]]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--repository-root", type=Path, default=ROOT.parents[4])
    args = parser.parse_args()
    root, repo = args.root.resolve(), args.repository_root.resolve()
    manifest = json.loads((root / "evidence.json").read_text(encoding="utf-8"))
    require(manifest["files"] == inventory(root), "证据文件清单与实际字节不一致")
    require(manifest["final_runs"] == RUNS, "最终执行点集合不符")
    # run_path只定义记录器的函数；__main__入口不执行，不产生字节码缓存或写出记录。
    recorder = runpy.run_path(str(root / "record-check.py"))
    require(manifest["source_scope"] == recorder["PRODUCT_ROOTS"], "受保护源码范围不符")
    identity = recorder["source_identity"]
    identity.__globals__["REPOSITORY"] = repo
    current = identity()
    require(current["baseline_commit"] == manifest["source"]["baseline_commit"]
            and current["baseline_tree"] == manifest["source"]["baseline_tree"]
            and normalized_source(current) == normalized_source(manifest["source"]),
            "当前受保护源码与最终执行身份不一致")
    logs = {}
    for label in RUNS:
        record, logs[label] = read_run(root, label)
        require(record["exit_code"] == 0, "最终执行失败：" + label)
        require(record["source_before"] == manifest["source"], "最终执行源码身份不一致：" + label)
        require(record["recorder_sha256"] == hashlib.sha256((root / "record-check.py").read_bytes()).hexdigest(),
                "最终执行记录器身份不符：" + label)
    before, log = read_run(root, "before-reachability")
    require(before["exit_code"] == 101 and "0 passed; 1 failed" in log and "cmath_abs" in log
            and "unreachable_module_cannot_pull_in_planned_cmath ... FAILED" in log,
            "旧实现反例没有按声明原因失败")
    codegen = logs["codegen-accepted"]
    require("56 passed; 0 failed; 0 ignored" in codegen and "4 passed; 0 failed; 0 ignored" in codegen,
            "LLVM单元或标准库绑定回归计数不符")
    for name in ["unreachable_module_cannot_pull_in_planned_cmath",
                 "transitive_imports_bind_dependencies_and_terminate_cycles",
                 "namespace_selection_does_not_expand_unselected_siblings"]:
        require(name + " ... ok" in codegen, "可达性场景缺失：" + name)
    driver = logs["driver-accepted"]
    require("8 passed; 0 failed; 0 ignored" in driver, "包装回归计数不符")
    require("12 passed; 0 failed; 3 ignored" in driver, "既有模块回归计数不符")
    require(driver.count("executable=false result=planned-rejected") == 30,
            "30条planned拒绝路径不完整")
    require("unimported_module_cannot_pull_cmath_into_hex ... ok" in driver,
            "真实hex不可达依赖回归缺失")
    registry = json.loads((repo / "docs/module-registry.json").read_text(encoding="utf-8"))
    require(next(row for row in registry["modules"] if row["id"] == "stdlib.cmath")["status"] == "planned",
            "cmath登记不再是planned")
    require(manifest["b3_4_closed"] is False and manifest["cmath_status"] == "planned",
            "本轮修复被错误标为B3-4收口")
    print(json.dumps({"passed": True, "files": len(manifest["files"]), "final_runs": len(RUNS),
                      "planned_rejections": 30, "b3_4_closed": False}, ensure_ascii=False))


if __name__ == "__main__":
    main()
