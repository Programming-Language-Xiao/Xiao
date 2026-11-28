"""逐条保存命令、退出码及两个未经转码的标准流。"""

import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys


def file_identity(path):
    data = path.read_bytes()
    return {"path": path.name, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def capture(root, name, command, *, broken_stdout=False, timeout=1800):
    """单独运行一个可复核命令，复现时沿当前进程的隔离工具链环境。"""
    stdout = root / f"{name}.stdout.log"
    stderr = root / f"{name}.stderr.log"
    start = datetime.datetime.now(datetime.timezone.utc)
    with stdout.open("wb") as out, stderr.open("wb") as err:
        if broken_stdout:
            reader, writer = os.pipe()
            os.close(reader)
            with os.fdopen(writer, "wb", closefd=True) as writer:
                result = subprocess.run(command, stdout=writer, stderr=err, timeout=timeout)
        else:
            result = subprocess.run(command, stdout=out, stderr=err, timeout=timeout)
    end = datetime.datetime.now(datetime.timezone.utc)
    keys = (
        "CARGO_TARGET_DIR", "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER",
        "CARGO_PROFILE_DEV_DEBUG", "CARGO_PROFILE_TEST_DEBUG", "CARGO_PROFILE_TEST_OPT_LEVEL",
        "XIAO_TARGET_TRIPLE", "XIAO_CLANG", "XIAO_LLVM_AS", "XIAO_LLC",
        "XIAO_RUNTIME_LIBRARY", "XIAO_B3_LOG_TEST_ROOT", "XIAO_RUST_DOC_ADAPTER",
        "CC_PRINT_OPTIONS_FILE", "XIAO_RUNTIME_RELEASE_TRACE_PATH",
    )
    receipt = {
        "argv": command,
        "cwd": str(Path.cwd()),
        "environment": {key: os.environ[key] for key in keys if key in os.environ},
        "started_utc": start.isoformat(),
        "elapsed_seconds": (end - start).total_seconds(),
        "exit": result.returncode,
        "stdout": file_identity(stdout),
        "stderr": file_identity(stderr),
    }
    (root / f"{name}.json").write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + "\n",
                                       encoding="utf-8")
    print(f"{name}: {result.returncode}", flush=True)
    return result.returncode


if __name__ == "__main__":
    name, *command = sys.argv[1:]
    broken = command[0] == "--broken-stdout"
    sys.exit(capture(Path(__file__).resolve().parent, name, command[broken:],
                     broken_stdout=broken))
