"""只读记录任务内 Runtime 静态库的路径、长度和 SHA-256。"""
import argparse
import hashlib
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("path", type=Path)
    arguments = parser.parse_args()
    path = arguments.path
    if not path.is_file() or path.is_symlink():
        raise SystemExit("Runtime artifact must be an existing regular file")
    digest = hashlib.sha256()
    size = 0
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            digest.update(chunk)
            size += len(chunk)
    print(json.dumps({"path": str(path), "bytes": size, "sha256": digest.hexdigest()}, sort_keys=True))


if __name__ == "__main__":
    main()
