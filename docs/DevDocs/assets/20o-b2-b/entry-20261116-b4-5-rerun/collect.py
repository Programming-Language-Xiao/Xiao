"""显式生成B4-5重开报告；校验成功后才写入正式目录。"""
import argparse
import json
from pathlib import Path
import shutil
import sys
import tempfile

sys.dont_write_bytecode = True
from audit import ROOT, REPOSITORY, check, inventory, handoff, negative_cases
from record import write_json, require, sha

OUTPUTS = ["closeout.json", "b5-handoff.json", "self-negative.json", "evidence.json"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--collect", action="store_true", required=True)
    args = parser.parse_args()
    require(not any((ROOT / name).exists() for name in OUTPUTS), "B4-5收口文件不可覆盖，需新证据目录")
    with tempfile.TemporaryDirectory(prefix="b4-5-reopen-collect-") as temporary:
        copy = Path(temporary) / "evidence"
        shutil.copytree(ROOT, copy)
        result = check(copy, REPOSITORY, sealed=False)
        write_json(copy / "closeout.json", result)
        write_json(copy / "b5-handoff.json", handoff(result))
        write_json(copy / "evidence.json", {"files": inventory(copy)})
        review = negative_cases(copy, REPOSITORY)
        write_json(copy / "self-negative.json", review)
        (copy / "evidence.json").unlink()
        write_json(copy / "evidence.json", {"files": inventory(copy)})
        check(copy, REPOSITORY)
        for name in OUTPUTS:
            with (ROOT / name).open("xb") as output:
                output.write((copy / name).read_bytes())
    check(ROOT, REPOSITORY)
    print(json.dumps({"passed": True, "files": len(inventory(ROOT)), "negative_cases": len(review["cases"]),
                      "closeout_sha256": sha((ROOT / "closeout.json").read_bytes())}))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, AssertionError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
