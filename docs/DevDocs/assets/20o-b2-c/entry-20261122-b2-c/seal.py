"""生成 B2-C 回执及定稿字节清单。"""

import argparse
from pathlib import Path
import sys

sys.dont_write_bytecode = True
from audit import CLOSEOUT, MANIFEST, ROOT, compute, inventory, json_bytes
from entry_audit import collect


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--preflight", action="store_true", help="生成待自反例的回执，C5 始终为 false")
    parser.add_argument("--collect-ci", type=Path, help="从已下载的原始回执生成独立内容寻址归档")
    args = parser.parse_args()
    if args.collect_ci:
        collect(ROOT / "gates.zip", [("ci", args.collect_ci)])
    (ROOT / CLOSEOUT).write_bytes(json_bytes(compute(ROOT, args.preflight)))
    (ROOT / MANIFEST).write_bytes(json_bytes({"schema": 1, "files": inventory(ROOT)}))
