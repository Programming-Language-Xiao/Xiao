"""在临时副本中复跑 B3-2 原样证据与拒绝用例。"""

import json
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory
from zipfile import ZipFile

sys.dont_write_bytecode = True
from audit import HERE, NEGATIVES, audit, derive, encoded, identity, inventory, require


def write_manifest(root):
    (root / "evidence.json").write_bytes(encoded({"schema": 1, "files": inventory(root)}))


def snapshot(root):
    files = inventory(root)
    files["evidence.json"] = identity((root / "evidence.json").read_bytes())
    return identity(encoded(files))["sha256"]


def edit_platform(root, case):
    original = root / "runs.zip"
    replacement = root / "replacement.zip"
    report_name = "platforms/windows.json"
    with ZipFile(original) as source, ZipFile(replacement, "w") as target:
        report = json.loads(source.read(report_name))
        if case == "forged-activation":
            report["source_commit"] = "8d1c09f084be172e8b3ce078f131592ac6db94b3"
        elif case == "forged-coff":
            report["artifacts"][0]["verification"] = "unverified-coff-exports"
        else:
            command, before, after = {
                "missing-budget": ("vm", b"test cli_limits_count_and_utf8_bytes_before_grouping ... ok", b""),
                "unicode-admitted": ("native", b"case=invalid exit=2", b"case=invalid exit=0"),
                "wrong-cli-group": ("exact-cli-std-o0", b"group\n[go]", b"group\n[wrong]"),
                "forged-tutorial": ("vm", b"module=args alias=std exact=true", b"module=args alias=std exact=false"),
            }[case]
            old = report["commands"][command]["stdout"]
            raw = source.read(f"objects/{old['sha256']}")
            require(before in raw, f"缺少用于复跑的原始标记：{case}")
            modified = raw.replace(before, after, 1)
            metadata = identity(modified)
            report["commands"][command]["stdout"] = metadata
            target.writestr(f"objects/{metadata['sha256']}", modified)
        for name in source.namelist():
            if name != report_name:
                target.writestr(name, source.read(name))
        target.writestr(report_name, encoded(report))
    replacement.replace(original)


def mutate(root, case):
    if case == "rewritten-manifest":
        manifest = root / "evidence.json"
        manifest.write_bytes(manifest.read_bytes() + b"\n")
        return
    if case == "missing-attributes":
        (root / ".gitattributes").unlink()
    elif case == "forged-status":
        path = root / "closeout.json"
        status = json.loads(path.read_bytes())
        status["b3_2_passed"] = True
        path.write_bytes(encoded(status))
    else:
        edit_platform(root, case)
    write_manifest(root)


def run_cases(root):
    document = json.loads((root / "doc-gates.json").read_bytes())["source"]["commit"]
    results = []
    for case, reason in NEGATIVES.items():
        with TemporaryDirectory(prefix="xiao-b3-2-negative-") as directory:
            work = Path(directory) / "evidence"
            shutil.copytree(root, work)
            if reason is not None:
                mutate(work, case)
            before = snapshot(work)
            result = subprocess.run([sys.executable, "-B", "-X", "utf8", str(HERE / "audit.py"),
                                     "--evidence-root", str(work), "--prepare"],
                                    capture_output=True, encoding="utf-8", timeout=120)
            after = snapshot(work)
            require(before == after, f"审计改动了副本字节：{case}")
            want = f"B3-2 拒绝：{reason}" if reason is not None else ""
            require(result.returncode == int(reason is not None) and result.stderr.strip() == want,
                    f"{case} 退出或拒绝原因不符：{result.returncode} {result.stderr}")
            results.append({"case": case, "exit": result.returncode, "reason": reason, "before": before, "after": after})
    return {"document_commit": document, "cases": results}


def main():
    recorded = json.loads((HERE / "self-negative-result.json").read_bytes())
    status = audit(HERE)
    before = snapshot(HERE)
    with TemporaryDirectory(prefix="xiao-b3-2-replay-") as directory:
        staged = Path(directory) / "evidence"
        shutil.copytree(HERE, staged)
        (staged / "self-negative-result.json").unlink()
        (staged / "closeout.json").write_bytes(encoded(derive(staged, prepare=True)))
        write_manifest(staged)
        replay = run_cases(staged)
    fields = ("case", "exit", "reason")
    require(replay["document_commit"] == recorded["document_commit"] and
            [[row[key] for key in fields] for row in replay["cases"]] ==
            [[row[key] for key in fields] for row in recorded["cases"]], "复跑的拒绝结果与封存回执不符")
    require(before == snapshot(HERE), "复跑改动了封存字节")
    print(encoded({"read_only": True, "baseline_passed": True, "rejected_cases": status["negatives"]}).decode(), end="")


if __name__ == "__main__":
    main()
