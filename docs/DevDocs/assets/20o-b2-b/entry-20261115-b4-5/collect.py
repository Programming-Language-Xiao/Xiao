"""显式生成B4-5 closeout/evidence；默认审计器保持只读。"""
import argparse
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True
from audit import ROOT, check, inventory, negative_cases, sha, write_json


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--collect", action="store_true", required=True)
    parser.add_argument("--refresh", action="store_true")
    args = parser.parse_args()
    outputs = ["closeout.json", "self-negative.json", "evidence.json"]
    if any((ROOT / name).exists() for name in outputs) and not args.refresh:
        raise ValueError("禁止覆盖已有收口文件；需要明确--refresh")
    if args.refresh:
        for name in outputs:
            path = ROOT / name
            if path.exists():
                path.replace(ROOT / name.replace(".json", "-previous.json"))
    result = check()
    write_json(ROOT / "closeout.json", result)
    (ROOT / "evidence.json").write_text(json.dumps({"files": inventory(ROOT)}, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    write_json(ROOT / "self-negative.json", negative_cases())
    (ROOT / "evidence.json").write_text(json.dumps({"files": inventory(ROOT)}, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(json.dumps({"passed": True, "closeout_sha256": sha((ROOT / "closeout.json").read_bytes()),
                      "negative_sha256": sha((ROOT / "self-negative.json").read_bytes()),
                      "files": len(inventory(ROOT))}, ensure_ascii=False))


if __name__ == "__main__":
    main()
