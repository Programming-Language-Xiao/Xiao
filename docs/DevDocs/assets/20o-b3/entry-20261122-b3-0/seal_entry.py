"""封存 B3-0A 原始回执并生成定稿清单。"""

import argparse
from pathlib import Path
import sys

sys.dont_write_bytecode = True
from audit_entry import ROOT, archive_tools, inspect, inventory


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--collect", type=Path, help="由本机探测、CI 下载及守卫回放任务根生成归档")
    options = parser.parse_args()
    if options.collect:
        task = options.collect
        archive_tools.collect(ROOT / "observations.zip", [
            ("ci", task / "ci"), ("windows", task / "windows/evidence"), ("guards", task / "guards"),
        ])
    (ROOT / "entry.json").write_bytes(archive_tools.json_bytes(inspect(ROOT)))
    (ROOT / "evidence.json").write_bytes(archive_tools.json_bytes({"schema": 1, "files": inventory(ROOT)}))
