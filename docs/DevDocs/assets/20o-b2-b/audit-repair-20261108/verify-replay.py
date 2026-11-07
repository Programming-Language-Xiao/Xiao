"""验证 B0/B1 归档复跑、只读性、拒绝原因和 Windows 编码边界。"""

import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import tarfile


ROOT = Path(__file__).resolve().parent


def snapshot(root):
    return {
        path.relative_to(root).as_posix(): {
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "bytes": path.stat().st_size,
            "mtime_ns": path.stat().st_mtime_ns,
        }
        for path in sorted(root.rglob("*"))
        if path.is_file() and "__pycache__" not in path.parts
    }


def main(source, output):
    b0 = ROOT.parent / "entry-20261107"
    b1 = ROOT.parent / "entry-20261108-b1"
    spec = json.loads((b1 / "negative-cases.json").read_text(encoding="utf-8"))
    initial = {"b0": snapshot(b0), "b1": snapshot(b1)}
    records = []

    def run(name, script, evidence, *, mode="utf8", extra=(), expected_reason=None, self_result=None):
        env = os.environ.copy()
        env.pop("PYTHONUTF8", None)
        env.pop("PYTHONIOENCODING", None)
        if mode == "cp936":
            env["PYTHONIOENCODING"] = "cp936"
        argv = [sys.executable, "-X", "utf8=0" if mode == "cp936" else "utf8", str(script),
                "--root", str(evidence), "--source-root", str(source), *extra]
        before = snapshot(evidence)
        process = subprocess.run(argv, cwd=source, env=env, stdout=subprocess.PIPE,
                                 stderr=subprocess.STDOUT, text=True, encoding="utf-8", errors="strict")
        text = process.stdout.strip()
        read_only = before == snapshot(evidence)
        if expected_reason is not None:
            passed = process.returncode == 1 and text == expected_reason
        elif self_result is not None:
            try:
                parsed = json.loads(text)
            except json.JSONDecodeError:
                parsed = {}
            if self_result is False:
                passed = process.returncode == 1 and parsed.get("passed") is False
            else:
                passed = process.returncode == 0 and parsed.get("passed") is True
            if self_result:
                passed = passed and parsed["tampered_manifest_output"] == spec["expected_reason"]
                passed = passed and parsed["missing_gitattributes_output"] == spec["missing_gitattributes_reason"]
        else:
            try:
                parsed = json.loads(text)
            except json.JSONDecodeError:
                parsed = {}
            passed = (process.returncode == 0 and parsed.get("passed") == parsed.get("total") == 18
                      and parsed.get("files") == (50 if evidence.name == b0.name else 28))
        records.append({"name": name, "argv": argv, "cwd": str(source), "python_mode": mode,
                        "exit_code": process.returncode, "output": text, "read_only": read_only,
                        "passed": passed and read_only})

    for label, evidence in [("b0", b0), ("b1", b1)]:
        for attempt in (1, 2):
            run(f"{label}-replay-{attempt}", evidence / "audit.py", evidence)
    for mode in ("cp936", "utf8"):
        run(f"b1-self-negative-{mode}", b1 / "self-negative.py", b1, mode=mode, self_result=True)

    archive_commit = subprocess.check_output(["cmd", "/c", "git", "rev-parse", "HEAD"], cwd=source, text=True).strip()
    archive_argv = ["cmd", "/c", "git", "archive", "--format=tar", archive_commit, "--",
                    "docs/DevDocs/assets/20o-b2-b/entry-20261107",
                    "docs/DevDocs/assets/20o-b2-b/entry-20261108-b1"]
    archive = subprocess.check_output(archive_argv, cwd=source)
    with tempfile.TemporaryDirectory(prefix="xiao-b0-b1-git-export-") as temporary:
        export_root = Path(temporary).resolve()
        with tarfile.open(fileobj=io.BytesIO(archive), mode="r:") as stream:
            stream.extractall(export_root, filter="data")
        for label, evidence in [("b0", b0), ("b1", b1)]:
            copied = export_root / "docs/DevDocs/assets/20o-b2-b" / evidence.name
            run(f"{label}-git-export", evidence / "audit.py", copied)

    with tempfile.TemporaryDirectory(prefix="xiao-b0-audit-repair-") as temporary:
        task_root = Path(temporary).resolve()
        copied = task_root / "evidence"
        if copied.resolve().parent != task_root:
            raise SystemExit("临时证据路径越界")
        shutil.copytree(b0, copied)
        manifest = copied / "evidence.json"
        data = json.loads(manifest.read_text(encoding="utf-8"))
        data["files"][0]["sha256"] = "0" * 64
        manifest.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
        run("b0-tampered-manifest", b0 / "audit.py", copied, expected_reason="B0证据清单或摘要不符")
        shutil.copyfile(b0 / "evidence.json", manifest)
        (copied / ".gitattributes").unlink()
        run("b0-missing-gitattributes", b0 / "audit.py", copied, expected_reason="B0证据清单或摘要不符")

        non_repository = task_root / "non-repository"
        non_repository.mkdir()
        run("b1-unrelated-git-failure", b1 / "self-negative.py", b1,
            extra=("--source-root", str(non_repository)), self_result=False)

    final = {"b0": snapshot(b0), "b1": snapshot(b1)}
    result = {
        "passed": len(records) == 11 and all(record["passed"] for record in records) and initial == final,
        "checks": len(records),
        "source_root": str(source),
        "source_head": subprocess.check_output(["cmd", "/c", "git", "rev-parse", "HEAD"], cwd=source, text=True).strip(),
        "archive_argv": archive_argv,
        "evidence_unchanged": initial == final,
        "audit_sha256": {"b0": initial["b0"]["audit.py"]["sha256"], "b1": initial["b1"]["audit.py"]["sha256"]},
        "records": records,
    }
    if output:
        output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(json.dumps(result if not result["passed"] else {key: result[key] for key in
                     ["passed", "checks", "source_root", "source_head", "evidence_unchanged"]}, ensure_ascii=True))
    if not result["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, default=ROOT.parents[4])
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    main(args.source_root.resolve(), args.output.resolve() if args.output else None)
