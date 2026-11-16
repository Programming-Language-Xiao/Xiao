"""Collect the final documentation receipt and explicitly seal timing evidence."""
import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
from audit import (ROOT, REPOSITORY, PREFIX, COMMITS, check, docs_snapshot,
                   inventory, json_bytes, load, negatives, record, require, sha, write_new)
from collect import store


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--collect", action="store_true", required=True)
    parser.add_argument("--label", default="docs-final")
    args = parser.parse_args()
    require(args.label and all(character in "abcdefghijklmnopqrstuvwxyz0123456789-"
                               for character in args.label), "文档门禁标签不符")
    require(not any((ROOT / name).exists() for name in
                    ["seal.json", "evidence.json", "final-self-negative.json"]),
            "计时封存输出不可覆盖")
    receipt_path = ROOT / "gates" / (args.label + ".json")
    require(not receipt_path.exists(), "文档门禁回执不可覆盖")
    result = check(sealed=False)
    require(load(ROOT / "summary.json") == result, "封存前统计与原始数据不符")
    commit = record.git(REPOSITORY, "rev-parse", "HEAD").decode().strip()
    documents_before = docs_snapshot()
    require(documents_before == docs_snapshot(commit), "封存工具与文档须先提交定稿")
    products_before = record.worktree_files(REPOSITORY)
    require({name: item["canonical_sha256"] for name, item in products_before.items()}
            == record.fixed_files(REPOSITORY, COMMITS["B"]),
            "封存前主工作区产品快照不符")
    configuration_path = (
        "docs/DevDocs/assets/20o-b2-b/entry-20261116-b5-rebuild/final3/environment.json")
    configuration_raw = record.git(REPOSITORY, "show", "06eee789:" + configuration_path)
    configuration = json.loads(configuration_raw)
    argv = ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"]
    started = datetime.now(timezone.utc).isoformat()
    with tempfile.TemporaryDirectory(prefix="b5-url-final-docs-") as temporary:
        output_path = Path(temporary) / "output.log"
        with output_path.open("xb") as output:
            child = subprocess.Popen(argv, cwd=REPOSITORY,
                                     env=record.controlled(configuration),
                                     stdout=output, stderr=subprocess.STDOUT)
            status = child.wait()
        raw_log = output_path.read_bytes()
    finished = datetime.now(timezone.utc).isoformat()
    documents_after = docs_snapshot()
    products_after = record.worktree_files(REPOSITORY)
    receipt_path.parent.mkdir(parents=True, exist_ok=True)
    gate = {
        "argv": argv, "cwd": str(REPOSITORY), "pid": child.pid,
        "started_utc": started, "finished_utc": finished, "exit_code": status,
        "tools_commit": commit, "runner_sha256": sha(Path(__file__).read_bytes()),
        "environment_origin": "06eee789:" + configuration_path,
        "environment_sha256": sha(configuration_raw),
        "docs_before": record.identity(documents_before),
        "docs_after": record.identity(documents_after),
        "docs_unchanged": documents_before == documents_after,
        "source_before": record.identity(products_before),
        "source_after": record.identity(products_after),
        "source_unchanged": products_before == products_after,
        "documents": store(json_bytes(documents_before)),
        "products": store(json_bytes(products_before)),
        "log": store(raw_log),
    }
    write_new(receipt_path, gate)
    require(status == 0 and documents_before == documents_after
            and products_before == products_after,
            "最终文档门禁失败或输入变化；原始回执已保留")
    output = json.loads(raw_log)
    require(output["passed"] is True
            and not any(row["severity"] == "error" for row in output["diagnostics"]),
            "最终文档门禁未通过；原始回执已保留")
    seal = {
        "product_commit": COMMITS["B"],
        "docs_receipt": receipt_path.relative_to(ROOT).as_posix(),
        "self_negative_receipt": "final-self-negative.json",
        "stage_passed": False, "b5_closed": False, "b6_started": False,
    }
    with tempfile.TemporaryDirectory(prefix="b5-url-seal-") as temporary:
        copy = Path(temporary) / "evidence"
        shutil.copytree(ROOT, copy)
        write_new(copy / "seal.json", seal)
        write_new(copy / "evidence.json", {"files": inventory(copy)})
        proof = negatives(copy)
    write_new(ROOT / "final-self-negative.json", proof)
    write_new(ROOT / "seal.json", seal)
    write_new(ROOT / "evidence.json", {"files": inventory(ROOT)})
    check()
    print(json.dumps({
        "evidence_sealed": True, "files": len(inventory(ROOT)),
        "docs_gate_passed": True, "self_negative_cases": len(proof["cases"]),
        "causal_attribution_proven": False, "b5_closed": False, "b6_started": False,
    }, ensure_ascii=False))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, AssertionError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
