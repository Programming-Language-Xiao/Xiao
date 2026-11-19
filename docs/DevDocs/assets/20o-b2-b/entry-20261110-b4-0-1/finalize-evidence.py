"""Generate immutable B4 closeout and byte inventory after all recorded runs."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
BASELINE = "7d8b85e4d8c588b40ec76d641edc865481295032"
PRODUCT = "c7a45e10b76b26915499717563761ed20fda6e6c"
RUNS = [
    "runtime_cmath", "runtime_all", "types_ir", "b4_frontend", "b3_wrapper",
    "b1_history", "b2_history", "b3_cabi_history", "b3_wrapper_history", "clippy", "fmt", "repo_all",
    "repo_all_closeout", "repo_all_final", "repo_after_audit_fixes", "repo_final_status",
]
SCOPE = "B4-0/B4-1 internal vector/type/IR closeout; cmath language unavailable"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def inventory():
    rows = []
    for path in sorted(item for item in ROOT.rglob("*") if item.is_file()):
        if path.name == "evidence.json" or path.name.startswith("self-negative-result") or "__pycache__" in path.parts:
            continue
        data = path.read_bytes()
        rows.append({"path": path.relative_to(ROOT).as_posix(), "bytes": len(data), "sha256": sha(data)})
    return rows


def write_json(path, value, rebuild):
    with path.open("w" if rebuild else "x", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rebuild", action="store_true",
                        help="基于新增执行记录重建本脚本生成的 closeout/evidence")
    parser.add_argument("--refresh-evidence", action="store_true",
                        help="仅按实际字节刷新文件清单，不改写历史收口和执行记录")
    args = parser.parse_args()
    closeout_path = ROOT / "closeout.json"
    evidence_path = ROOT / "evidence.json"
    if args.refresh_evidence:
        if args.rebuild or not (closeout_path.exists() and evidence_path.exists()):
            raise SystemExit("--refresh-evidence 只适用于已有证据且不能与 --rebuild 合用")
        evidence = json.loads(evidence_path.read_text(encoding="utf-8"))
        evidence["files"] = inventory()
        write_json(evidence_path, evidence, True)
        print(json.dumps({"refreshed_files": len(evidence["files"]), "closeout_unchanged": True}))
        return
    if args.rebuild and not (closeout_path.exists() and evidence_path.exists()):
        raise SystemExit("--rebuild要求closeout.json和evidence.json均已存在")
    if not args.rebuild and (closeout_path.exists() or evidence_path.exists()):
        raise SystemExit("拒绝覆盖既有 B4 closeout/evidence")

    bootstrap = json.loads((ROOT / "bootstrap.json").read_text(encoding="utf-8"))
    manifest = json.loads((ROOT / "source-manifest.json").read_text(encoding="utf-8"))
    if bootstrap["baseline_commit"] != BASELINE or bootstrap["product_commit"] != PRODUCT:
        raise SystemExit("B4入口提交身份不符")
    if manifest["baseline_commit"] != BASELINE or manifest["product_commit"] != PRODUCT:
        raise SystemExit("B4源清单提交身份不符")

    records = {}
    for key in RUNS:
        record_path = ROOT / (key + ".json")
        record = json.loads(record_path.read_text(encoding="utf-8"))
        log = (ROOT / record["log"]).read_bytes()
        if (record["label"] != key or record["exit_code"] != 0 or not record["source_unchanged"]
                or record["source_before"] != record["source_after"]
                or record["log_sha256"] != sha(log) or record["log_bytes"] != len(log)):
            raise SystemExit("B4运行记录无效：" + key)
        records[key] = record
    snapshots = {json.dumps(record["source_before"], sort_keys=True) for record in records.values()}
    if len(snapshots) != 1:
        raise SystemExit("B4运行没有绑定同一源码/锁快照")

    closeout = {
        "schema_version": 1,
        "passed": True,
        "entry_commit": BASELINE,
        "product_commit": PRODUCT,
        "runtime_abi": "1.18",
        "codegen_version": 20,
        "b4_0_closed": True,
        "b4_1_closed": True,
        "b4_closed": False,
        "language_available": False,
        "cmath_status": "planned",
        "vm_llvm_cmath_binding": False,
        "old_721_executed": False,
        "runs": {
            key: {"pid": records[key]["pid"], "log": records[key]["log"],
                  "log_sha256": records[key]["log_sha256"]}
            for key in RUNS
        },
    }
    write_json(closeout_path, closeout, args.rebuild)

    evidence = {
        "schema_version": 1,
        "scope": SCOPE,
        "baseline_commit": BASELINE,
        "product_commit": PRODUCT,
        "product_files": manifest["files"],
        "raw_tree_sha256": manifest["raw_tree_sha256"],
        "utf8_lf_tree_sha256": manifest["utf8_lf_tree_sha256"],
        "runtime_abi": "1.18",
        "codegen_version": 20,
        "runs": RUNS,
        "files": inventory(),
    }
    write_json(evidence_path, evidence, args.rebuild)
    print(json.dumps({"passed": True, "product_commit": PRODUCT,
                      "files": len(evidence["files"]), "runs": len(RUNS)},
                     ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    main()
