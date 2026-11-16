"""定稿后单次文档门禁与B5-0来源检查封存；不升级产品状态。"""
import argparse
import json
from pathlib import Path
import shutil
import sys
import tempfile

sys.dont_write_bytecode = True
from facts import ROOT, REPOSITORY, ENTRY, PRODUCT_ROOTS, docs_snapshot, git, inventory, require, write_json
from collect import command
from audit import check, negative_cases


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--collect", action="store_true", required=True)
    parser.parse_args()
    require(not (ROOT / "final-docs.json").exists() and not (ROOT / "evidence.json").exists(), "B5最终文档门禁不能覆盖")
    check(sealed=False)
    commit = git(REPOSITORY, "rev-parse", "HEAD").decode().strip()
    before = docs_snapshot(REPOSITORY)
    fixed = docs_snapshot(REPOSITORY, commit)
    require(before["canonical_files"] == fixed["canonical_files"], "B5文档须先提交")
    require(not git(REPOSITORY, "diff", "--name-only", ENTRY, "--", *PRODUCT_ROOTS).strip(), "B5阻塞期间产品已变化")
    receipt, raw = command(["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"], timeout=120)
    after = docs_snapshot(REPOSITORY)
    require(receipt["exit_code"] == 0 and before == after
            and not git(REPOSITORY, "diff", "--name-only", ENTRY, "--", *PRODUCT_ROOTS).strip(),
            "B5最终文档门禁失败或文档/产品变化")
    write_json(ROOT / "final-docs.json", {"receipt": receipt, "docs_commit": commit,
               "docs_before": before, "docs_after": after, "fixed_files": fixed["canonical_files"]})
    with tempfile.TemporaryDirectory(prefix="b5-source-seal-") as temporary:
        copy = Path(temporary) / "evidence"
        shutil.copytree(ROOT, copy)
        write_json(copy / "evidence.json", {"files": inventory(copy)})
        write_json(copy / "self-negative.json", negative_cases(copy, REPOSITORY))
        (copy / "evidence.json").unlink()
        write_json(copy / "evidence.json", {"files": inventory(copy)})
        check(copy, REPOSITORY)
        for name in ["self-negative.json", "evidence.json"]:
            with (ROOT / name).open("xb") as output:
                output.write((copy / name).read_bytes())
    print(json.dumps(check(), ensure_ascii=False))


if __name__ == "__main__":
    main()
