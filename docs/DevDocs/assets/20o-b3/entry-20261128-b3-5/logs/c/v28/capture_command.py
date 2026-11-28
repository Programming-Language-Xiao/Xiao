"""沿用 C 批的命令收集器，在 28 的目录保存原始记录。"""

import json
import os
from pathlib import Path
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from capture import capture

if __name__ == "__main__":
    name, *command = sys.argv[1:]
    broken = command[0] == "--broken-stdout"
    root = Path(__file__).resolve().parent
    status = capture(root, name, command[broken:], broken_stdout=broken)
    if core := os.environ.get("XIAO_CORE_PATH"):
        path = root / f"{name}.json"
        receipt = json.loads(path.read_text(encoding="utf-8"))
        receipt["environment"]["XIAO_CORE_PATH"] = core
        path.write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    sys.exit(status)
