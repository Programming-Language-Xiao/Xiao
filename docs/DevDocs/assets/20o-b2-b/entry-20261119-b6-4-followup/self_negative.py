"""复跑 B6-4 交接状态组合及临时归档反例。"""

from itertools import product
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
from handoff_status import derive_status


ARCHIVE = ROOT / "followup.zip"
LOCAL_CHECKS = ROOT / "local-checks.zip"
REPORT = ROOT / "negative-cases.json"
STATUS = ROOT / "status.json"
MACOS_LOG = "ci/44541850714-attempt-1/135621970069.log"
MISSING_LOG = "windows/single/01/cargo.log"


def read_archive(source):
    with ZipFile(source) as archive:
        entries = json.loads(archive.read("evidence.json"))
        objects = {name: archive.read(name) for name in archive.namelist() if name != "evidence.json"}
    return entries, objects


def make_negative_copies(source, missing, forged):
    entries, objects = read_archive(source)
    lost = "objects/" + entries[MISSING_LOG]["sha256"]
    write_archive(missing, entries, {name: raw for name, raw in objects.items() if name != lost})

    marker = b"FLOAT-EQUALITY-COMPONENTS original-pow-eq-static-o0 verification=complete"
    raw = objects["objects/" + entries[MACOS_LOG]["sha256"]]
    require(raw.count(marker) == 1, "缺少唯一的 macOS 组件观测")
    edited = raw.replace(marker, marker.replace(b"complete", b"unverified"))
    write_modified_archive(forged, entries, objects, {MACOS_LOG: edited})


def forge_export_count(source, forged):
    entries, objects = read_archive(source)
    output_name, receipt_name = "linux/exports.stdout", "linux/exports.json"
    output = objects["objects/" + entries[output_name]["sha256"]]
    marker = b"RUNTIME-COMPONENTS exports=164 "
    require(output.count(marker) == 1, "缺少唯一的 164 个导出观测")
    changed = output.replace(marker, b"RUNTIME-COMPONENTS exports=163 ")
    receipt = json.loads(objects["objects/" + entries[receipt_name]["sha256"]])
    receipt["outputs"]["stdout"] = digest(changed)
    write_modified_archive(forged, entries, objects, {
        output_name: changed,
        receipt_name: json.dumps(receipt, sort_keys=True).encode("utf-8"),
    })


def write_modified_archive(path, entries, objects, changes):
    for name, raw in changes.items():
        identity = digest(raw)
        entries[name] = identity
        objects["objects/" + identity["sha256"]] = raw
    members = {"objects/" + entry["sha256"] for entry in entries.values()}
    write_archive(path, entries, {name: objects[name] for name in sorted(members)})


def write_archive(path, entries, objects):
    with ZipFile(path, "x", ZIP_DEFLATED) as archive:
        for name, raw in objects.items():
            archive.writestr(name, raw)
        archive.writestr("evidence.json", json.dumps(entries, sort_keys=True).encode("utf-8"))


def verify(case, path, reason, *, status=STATUS, local_checks=LOCAL_CHECKS):
    inputs = (path, status, local_checks)
    before = [digest(item.read_bytes()) for item in inputs]
    argv = [sys.executable, "-B", "-X", "utf8", str(ROOT / "audit.py"), "--archive", str(path),
            "--status", str(status), "--local-checks", str(local_checks)]
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


def verify_status_combinations():
    checked = 0
    followup_pass_cases = 0
    stage_pass_cases = 0
    for original, engineering, differential_open, windows_ok, elf_complete, macho_complete, coff_complete in product(
        (False, True), repeat=7
    ):
        runners = {
            "Linux": {"conclusion": "success", "verification": "complete" if elf_complete else "unverified"},
            "macOS": {"conclusion": "success", "verification": "complete" if macho_complete else "unverified"},
            "Windows": {"conclusion": "success" if windows_ok else "failure",
                        "verification": "complete" if coff_complete else "unverified-coff-exports"},
        }
        followup = all((engineering, windows_ok, elf_complete, macho_complete, coff_complete))
        expected = {
            "b6_4_passed": original and followup and not differential_open,
            "original_passed": original,
            "followup_passed": followup,
            "windows_coff_verified": coff_complete,
            "22d_open": differential_open,
        }
        actual = derive_status(original, runners, engineering, differential_open)
        require(actual == expected, f"B6-4 状态组合不符：期望 {expected}，实际 {actual}")
        checked += 1
        followup_pass_cases += int(actual["followup_passed"])
        stage_pass_cases += int(actual["b6_4_passed"])
    return {"checked": checked, "followup_pass_cases": followup_pass_cases,
            "stage_pass_cases": stage_pass_cases}


def run_cases():
    original = digest(ARCHIVE.read_bytes())
    local_original = digest(LOCAL_CHECKS.read_bytes())
    status = json.loads(STATUS.read_text(encoding="utf-8"))
    status_matrix = verify_status_combinations()
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
        changed = root / "forged-followup-pass.json"
        changed.write_text(json.dumps({**status, "b6_4_passed": True, "followup_passed": True,
                                       "windows_coff_verified": True}), encoding="utf-8")
        cases.append(verify("forged-followup-pass", unchanged, "B6-4 交接状态不符", status=changed))
        forged_local = root / "forged-local-checks.zip"
        forge_export_count(LOCAL_CHECKS, forged_local)
        cases.append(verify("forged-export-count", unchanged, "Linux 导出对账输出不符", local_checks=forged_local))
    require(digest(ARCHIVE.read_bytes()) == original, "原归档发生变化")
    require(digest(LOCAL_CHECKS.read_bytes()) == local_original, "本机补验归档发生变化")
    return {"archive": original, "local_checks_archive": local_original, "status_matrix": status_matrix, "cases": cases}


def main():
    report = run_cases()
    require(report == json.loads(REPORT.read_text(encoding="utf-8")), "复跑结果与已封存反例回执不符")
    print(json.dumps(report, ensure_ascii=False))


if __name__ == "__main__":
    main()
