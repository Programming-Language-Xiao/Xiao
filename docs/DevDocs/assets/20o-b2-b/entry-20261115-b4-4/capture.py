"""B4-4工具身份、真实产物和内容寻址采集器自验。"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import zipfile

sys.dont_write_bytecode = True
from record import ROOT, BASE, load, sha, require, write_json


def toolchain():
    tools = {"rustc": ["rustc", "+1.96.0", "-Vv"], "cargo": ["cargo", "+1.96.0", "-V"],
             "targets": ["rustup", "target", "list", "--installed", "--toolchain", "1.96.0"],
             **{name: [os.environ[key], "--version"] for name, key in [
                 ("clang", "XIAO_CLANG"), ("llvm-as", "XIAO_LLVM_AS"), ("llc", "XIAO_LLC")]}}
    records = {}
    for name, argv in tools.items():
        result = subprocess.run(argv, check=True, capture_output=True, encoding="utf-8")
        path = Path(shutil.which(argv[0]) or argv[0])
        records[name] = {"argv": argv, "stdout": result.stdout, "stderr": result.stderr,
                         "path": str(path), "sha256": sha(path.read_bytes())}
    print(json.dumps(records, ensure_ascii=False))


def artifacts():
    task = Path(load(ROOT / "fixed-final/environment.json")["task_root"])
    destination = ROOT / "artifacts/sha256"
    rows = []
    for directory in sorted((task / "temp").glob("b4-4-library-*")):
        require(not (directory / "main.xiao").exists(), "应用源码仍存在")
        for path in sorted(directory.iterdir()):
            if path.suffix not in {".xiaoc", ".xar", ".ll", ".exe"}:
                continue
            raw = path.read_bytes()
            row = {"path": str(path), "bytes": len(raw), "sha256": sha(raw), "kind": path.suffix,
                   "application_source_removed": True}
            if path.suffix != ".exe":
                saved = destination / (row["sha256"] + path.suffix)
                saved.parent.mkdir(parents=True, exist_ok=True)
                if saved.exists():
                    require(saved.read_bytes() == raw, "产物对象冲突")
                else:
                    with saved.open("xb") as output:
                        output.write(raw)
                row["object"] = saved.relative_to(ROOT).as_posix()
            if path.suffix == ".xar":
                with zipfile.ZipFile(path) as archive:
                    row["members"] = archive.namelist()
            rows.append(row)
    require({kind: sum(r["kind"] == kind for r in rows) for kind in [".xiaoc", ".xar", ".ll", ".exe"]}
            == {".xiaoc": 4, ".xar": 4, ".ll": 4, ".exe": 4}, "真实产物数量不符")
    manifest = ROOT / "fixed-final/artifact-manifest.json"
    if manifest.exists():
        require(load(manifest) == {"files": rows}, "产物清单与实际文件不符")
    else:
        write_json(manifest, {"files": rows})
    print(json.dumps({"passed": True, "files": len(rows), "source_removed": True,
                      "manifest_sha256": sha((ROOT / "fixed-final/artifact-manifest.json").read_bytes())}))


def self_test():
    with tempfile.TemporaryDirectory(prefix="b4-4-cas-", dir=os.environ["TEMP"]) as temporary:
        root = Path(temporary)
        first = BASE.store_content_log(root, b"same\n")
        second = BASE.store_content_log(root, b"same\n")
        different = BASE.store_content_log(root, b"other\n")
        require(first == second and first != different, "日志未去重")
        require(len(list(root.rglob("*.log"))) == 2, "重复日志对象")
        (root / first[0]).write_bytes(b"tampered\n")
        try:
            BASE.store_content_log(root, b"same\n")
        except ValueError:
            pass
        else:
            raise ValueError("损坏日志未拒绝")
        try:
            BASE.require_new_task_root(root)
        except ValueError:
            pass
        else:
            raise ValueError("已有任务根未拒绝")
    print(json.dumps({"passed": True, "identical_outputs_reused": True, "distinct_objects": 2,
                      "tampered_object_rejected": True, "existing_task_root_rejected": True}))


if __name__ == "__main__":
    {"toolchain": toolchain, "artifacts": artifacts, "self-test": self_test}[sys.argv[1]]()
