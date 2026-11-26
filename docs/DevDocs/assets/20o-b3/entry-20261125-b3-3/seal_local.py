"""按实际字节合并本机采集、派生状态和更新清单。"""

import argparse
import json
from pathlib import Path
import zipfile

from audit import HERE, REPO, derive, inventory


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
                    encoding="utf-8", newline="\n")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--windows", type=Path, required=True)
    parser.add_argument("--linux", type=Path, required=True)
    parser.add_argument("--first-linux", type=Path, required=True)
    parser.add_argument("--missing-temp-linux", type=Path, required=True)
    parser.add_argument("--export-option-linux", type=Path, required=True)
    args = parser.parse_args()
    entries = {}
    for name, file in (
        ("windows", args.windows), ("linux-wsl", args.linux), ("linux-wsl-first", args.first_linux),
        ("linux-wsl-missing-temp", args.missing_temp_linux),
        ("linux-wsl-export-option", args.export_option_linux),
    ):
        with zipfile.ZipFile(file) as archive:
            entries[f"runs/{name}.json"] = archive.read("receipt.json")
            for key in archive.namelist():
                if key.startswith("objects/"):
                    data = archive.read(key)
                    if entries.setdefault(key, data) != data:
                        raise SystemExit("相同摘要的日志字节不一致")
    with zipfile.ZipFile(HERE / "runs.zip", "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for key, data in sorted(entries.items()):
            archive.writestr(key, data)
    write_json(HERE / "status.json", derive(HERE, REPO))
    write_json(HERE / "evidence.json", {"files": inventory(HERE)})


if __name__ == "__main__":
    main()
