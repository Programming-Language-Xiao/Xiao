"""按 B3-3 的既有口径回验当前产品，并检查生成器 25 的封存产物。"""

import argparse
from importlib.util import module_from_spec, spec_from_file_location
import json
from pathlib import Path
import platform
import zipfile


PREVIOUS = "2460e75fbdac448b51b7941e937fb3df24e0cb70"
B3 = Path(__file__).resolve().parents[2] / "20o-b3"
spec = spec_from_file_location("b3_fs", B3 / "entry-20261125-b3-3" / "run_native.py")
fs = module_from_spec(spec)
spec.loader.exec_module(fs)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--llvm-bin", required=True, type=Path)
    args = parser.parse_args()
    output = args.output.resolve()
    fs.require(not output.is_relative_to(fs.ROOT) and not output.exists(), "指定仓库外的全新目录")
    task = fs.time.Task(output, args.llvm_bin.resolve())
    try:
        system = {"Darwin": "macos", "Linux": "linux", "Windows": "windows"}[platform.system()]
        with zipfile.ZipFile(B3 / "entry-20261126-b3-3-rename-fix" / "runs.zip") as archive:
            previous = json.loads(archive.read(f"platforms/{system}.json"))
            row = next(row for row in previous["artifacts"] if row["case"] == "fs-std-o0")
            saved = row["artifact"]
            raw = archive.read(f"objects/{saved['sha256']}")
        identity = fs.identity(raw)
        fs.require(previous["source_commit"] == PREVIOUS and row["generator"] == 25
                   and identity == {key: saved[key] for key in identity}, "B3-3 生成器 25 的封存产物身份不符")
        old_artifact = output / "temp" / "baseline-generator25"
        old_artifact.write_bytes(raw)
        task.env["XIAO_B3_3_NATIVE_ARTIFACT"] = str(old_artifact)
        task.report["previous_b3_3_artifact"] = {"commit": PREVIOUS, "member": saved["path"], **identity}

        fs.verify(task, task.report["source_commit"])
        task.report["passed"] = False
        stdout = task.report["commands"]["native"]["stdout"]["sha256"]
        versions = [json.loads(line.removeprefix("B3-3-VERSION "))
                    for line in (task.logs / f"{stdout}.log").read_text(encoding="utf-8").splitlines()
                    if line.startswith("B3-3-VERSION ")]
        fs.require([row["baseline"] for row in versions] == [24, 25]
                   and all(row["current"] == 26 and row["older_metadata"] == "accepted" for row in versions),
                   "缺少生成器 24 和 25 的兼容证据")
        task.report["versions"] = versions
        task.report["passed"] = True
    finally:
        (output / "result.json").write_text(json.dumps(task.report, indent=2, ensure_ascii=False) + "\n",
                                           encoding="utf-8")


if __name__ == "__main__":
    main()
