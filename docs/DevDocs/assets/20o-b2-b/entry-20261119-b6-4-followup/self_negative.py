"""在临时副本上复跑 B6-4 后续证据与交接状态的反例。"""

import json
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory
from zipfile import ZIP_DEFLATED, ZipFile

ROOT = Path(__file__).resolve().parent
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT.with_name("entry-20261119-b6-4")))

from evidence import digest, require


ARCHIVE = ROOT / "followup.zip"
REPORT = ROOT / "negative-cases.json"
STATUS = ROOT / "status.json"
MACOS_LOG = "ci/44541850714-attempt-1/135621970069.log"
MISSING_LOG = "windows/single/01/cargo.log"


def make_negative_copies(source, missing, forged):
    with ZipFile(source) as archive:
        entries = json.loads(archive.read("evidence.json"))
        objects = {name: archive.read(name) for name in archive.namelist() if name != "evidence.json"}
    lost = "objects/" + entries[MISSING_LOG]["sha256"]
    write_archive(missing, entries, {name: raw for name, raw in objects.items() if name != lost})

    marker = b"FLOAT-EQUALITY-COMPONENTS original-pow-eq-static-o0 verification=complete"
    raw = objects["objects/" + entries[MACOS_LOG]["sha256"]]
    require(raw.count(marker) == 1, "缺少唯一的 macOS 组件观测")
    edited = raw.replace(marker, marker.replace(b"complete", b"unverified"))
    identity = digest(edited)
    modified = {**entries, MACOS_LOG: identity}
    objects["objects/" + identity["sha256"]] = edited
    members = {"objects/" + entry["sha256"] for entry in modified.values()}
    write_archive(forged, modified, {name: objects[name] for name in sorted(members)})


def write_archive(path, entries, objects):
    with ZipFile(path, "x", ZIP_DEFLATED) as archive:
        for name, raw in objects.items():
            archive.writestr(name, raw)
        archive.writestr("evidence.json", json.dumps(entries, sort_keys=True).encode("utf-8"))


def verify(case, path, reason, status=STATUS):
    inputs = (path, status)
    before = [digest(item.read_bytes()) for item in inputs]
    argv = [sys.executable, "-B", "-X", "utf8", str(ROOT / "audit.py"), "--archive", str(path), "--status", str(status)]
    result = subprocess.run(argv, cwd=ROOT.parents[4], capture_output=True, encoding="utf-8", timeout=60)
    require([digest(item.read_bytes()) for item in inputs] == before, f"审计改写了输入：{case}")
    if reason is None:
        require(result.returncode == 0 and not result.stderr and json.loads(result.stdout)["verified"] is True,
                f"原样副本未通过：{result.stderr}")
        expected = observed = "verified=true"
    else:
        expected = reason
        observed = next(iter(reversed(result.stderr.splitlines())), "")
        require(result.returncode == 1 and observed == f"ValueError: {reason}" and not result.stdout,
                f"拒绝原因不符：{case}，实际退出 {result.returncode}，{observed}")
    return {"case": case, "exit": result.returncode, "expected": expected, "observed": observed, "read_only": True}


def run_cases():
    original = digest(ARCHIVE.read_bytes())
    status = json.loads(STATUS.read_text(encoding="utf-8"))
    with TemporaryDirectory(prefix="xiao-b6-4-followup-negative-") as directory:
        root = Path(directory)
        unchanged, missing, forged = (root / name for name in ("unchanged.zip", "missing.zip", "forged.zip"))
        shutil.copyfile(ARCHIVE, unchanged)
        make_negative_copies(unchanged, missing, forged)
        cases = [verify("unchanged", unchanged, None),
                 verify("missing-object", missing, "证据包清单不符"),
                 verify("forged-macos-observation", forged, "Runner 浮点矩阵不符：macOS")]
        for case, field in (("forged-stage-pass", "b6_4_passed"), ("forged-windows-coff", "windows_coff_verified")):
            require(status[field] is False, f"反例的原始状态不符：{field}")
            changed = root / f"{case}.json"
            changed.write_text(json.dumps({**status, field: True}), encoding="utf-8")
            cases.append(verify(case, unchanged, "B6-4 交接状态不符", status=changed))
    require(digest(ARCHIVE.read_bytes()) == original, "原归档发生变化")
    return {"archive": original, "cases": cases}


def main():
    report = run_cases()
    require(report == json.loads(REPORT.read_text(encoding="utf-8")), "复跑结果与已封存反例回执不符")
    print(json.dumps(report, ensure_ascii=False))


if __name__ == "__main__":
    main()
