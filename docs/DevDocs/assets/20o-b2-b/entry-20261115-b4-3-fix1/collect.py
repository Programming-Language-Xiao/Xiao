"""显式生成FIX.1收口/清单和只读反例；默认审计不写盘。"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import sys
import tempfile

sys.dont_write_bytecode = True
from audit import ROOT, REPOSITORY, check, inventory, self_negative, sha
from record import require, write_json


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--collect", action="store_true", required=True)
    parser.add_argument("--refresh", action="store_true")
    args = parser.parse_args()
    outputs = ["closeout.json", "self-negative.json", "evidence.json"]
    existing = [name for name in outputs if (ROOT / name).exists()]
    if existing and not args.refresh:
        raise ValueError("收口文件已存在，需显式--refresh")
    with tempfile.TemporaryDirectory(prefix="fix1-collect-") as temporary:
        copy = Path(temporary) / "evidence"
        shutil.copytree(ROOT, copy)
        for name in outputs:
            candidate = copy / name
            if candidate.exists():
                candidate.unlink()
        result = check(copy, REPOSITORY, final_docs=True, sealed=False)
        write_json(copy / "closeout.json", result)
        (copy / "evidence.json").write_text(json.dumps({"files": inventory(copy)}, ensure_ascii=False, indent=2) + "\n",
                                              encoding="utf-8", newline="\n")
        review = self_negative(copy, REPOSITORY)
        write_json(copy / "self-negative.json", review)
        (copy / "evidence.json").write_text(json.dumps({"files": inventory(copy)}, ensure_ascii=False, indent=2) + "\n",
                                              encoding="utf-8", newline="\n")
        check(copy, REPOSITORY, final_docs=True, sealed=True)
        if existing:
            for name in existing:
                previous = ROOT / name.replace(".json", "-previous.json")
                if previous.exists():
                    suffix = hashlib.sha256(previous.read_bytes()).hexdigest()[:12]
                    previous = ROOT / name.replace(".json", "-previous-" + suffix + ".json")
                require(not previous.exists(), "previous保留文件已存在")
                (ROOT / name).replace(previous)
        for name in outputs:
            with (ROOT / name).open("xb") as output:
                output.write((copy / name).read_bytes())
    check(ROOT, REPOSITORY, final_docs=True, sealed=True)
    print(json.dumps({"passed": True, "closeout_sha256": sha((ROOT / "closeout.json").read_bytes()),
                      "self_negative_sha256": sha((ROOT / "self-negative.json").read_bytes()),
                      "files": len(inventory(ROOT))}, ensure_ascii=False))


if __name__ == "__main__":
    try: main()
    except (ValueError, KeyError, OSError, AssertionError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
