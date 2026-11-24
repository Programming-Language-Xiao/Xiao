"""只读检查 B3-0B 固定 Windows 产物的 COFF 符号表和导出表可读性。"""

import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[4]
ARCHIVE = REPO / "docs/DevDocs/assets/20o-b3/entry-20261123-b3-0b/cli-import.zip"
ARCHIVE_COMMIT = "8c8c5fb19441346c607add19bf1b381160104612"
ARCHIVE_REPO_PATH = "docs/DevDocs/assets/20o-b3/entry-20261123-b3-0b/cli-import.zip"
ARCHIVE_SHA256 = "43524587e36d4d1c5d1b83fb8143b46c261cbd8c2f39f36ea55871cc4ebb19e6"
SOURCE_COMMIT = "e79fab3cdd3ea9d3a7eff31f529dec2498434114"
TARGET = "x86_64-pc-windows-msvc"
ARTIFACTS = (
    ("std", 0), ("std", 3), ("stdlib", 0), ("stdlib", 3),
)


def identity(raw):
    return {"bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


def readobj():
    executable = shutil.which("llvm-readobj")
    if executable is None:
        raise RuntimeError("找不到 llvm-readobj")
    path = Path(executable).resolve(strict=True)
    version = subprocess.run([str(path), "--version"], capture_output=True, timeout=15, check=True)
    return path, identity(path.read_bytes()), version.stdout.decode("utf-8", errors="replace").strip()


def save_log(raw):
    digest = hashlib.sha256(raw).hexdigest()
    destination = ROOT / "logs" / "sha256" / digest[:2] / f"{digest}.log"
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.exists():
        if destination.read_bytes() != raw:
            raise RuntimeError("内容摘要日志对象冲突")
    else:
        destination.write_bytes(raw)
    return {"path": destination.relative_to(ROOT).as_posix(), **identity(raw)}


def decode_output(raw, task_root):
    text = raw.decode("utf-8")
    return text.replace(str(task_root), "<task-root>").replace(str(task_root).replace("\\", "/"), "<task-root>")


def parse_exports(text):
    return re.findall(r"^\s+Name: (xiao_runtime_[A-Za-z0-9_]+)$", text, flags=re.MULTILINE)


def parse_debug_types(text):
    return re.findall(r"^\s+Type: ([A-Za-z0-9_]+) \(0x[0-9A-Fa-f]+\)$", text, flags=re.MULTILINE)


def run_one(readobj_path, archive, files, case, level, task_root):
    relative = f"probe/case-{case}/native-o{level}.exe"
    expected = files[relative]
    raw_artifact = archive.read("objects/" + expected["sha256"])
    if identity(raw_artifact) != expected:
        raise RuntimeError(f"B3-0B 固定产物摘要不符：{relative}")
    artifact_path = task_root / f"{case}-o{level}.exe"
    artifact_path.write_bytes(raw_artifact)

    headers_command = [str(readobj_path), "--file-headers", "--coff-exports", str(artifact_path)]
    headers = subprocess.run(headers_command, capture_output=True, timeout=30)
    debug_command = [str(readobj_path), "--coff-debug-directory", str(artifact_path)]
    debug = subprocess.run(debug_command, capture_output=True, timeout=30)
    if headers.returncode != 0 or debug.returncode != 0:
        raise RuntimeError(f"llvm-readobj 探测失败：{relative}")

    headers_stdout = decode_output(headers.stdout, task_root).encode("utf-8")
    headers_stderr = decode_output(headers.stderr, task_root).encode("utf-8")
    debug_stdout = decode_output(debug.stdout, task_root).encode("utf-8")
    debug_stderr = decode_output(debug.stderr, task_root).encode("utf-8")
    header_text = headers_stdout.decode("utf-8")
    debug_text = debug_stdout.decode("utf-8")
    pointer_match = re.search(r"PointerToSymbolTable: 0x([0-9A-Fa-f]+)", header_text)
    symbol_count_match = re.search(r"SymbolCount: (\d+)", header_text)
    export_rva_match = re.search(r"ExportTableRVA: 0x([0-9A-Fa-f]+)", header_text)
    export_size_match = re.search(r"ExportTableSize: 0x([0-9A-Fa-f]+)", header_text)
    if not all((pointer_match, symbol_count_match, export_rva_match, export_size_match)):
        raise RuntimeError(f"PE/COFF 表字段缺失：{relative}")
    pointer = int(pointer_match.group(1), 16)
    symbol_count = int(symbol_count_match.group(1))
    export_rva = int(export_rva_match.group(1), 16)
    export_size = int(export_size_match.group(1), 16)
    exports = parse_exports(header_text)
    debug_types = parse_debug_types(debug_text)

    return {
        "archive_path": relative,
        "artifact": expected,
        "target": TARGET,
        "prior_verification": "unverified-coff-exports",
        "commands": [
            {"argv": headers_command[:-1] + ["<artifact>"], "exit": headers.returncode,
             "stdout": save_log(headers_stdout), "stderr": save_log(headers_stderr)},
            {"argv": debug_command[:-1] + ["<artifact>"], "exit": debug.returncode,
             "stdout": save_log(debug_stdout), "stderr": save_log(debug_stderr)},
        ],
        "observed": {
            "object_format": "COFF-x86-64",
            "symbol_table_pointer": pointer,
            "symbol_count": symbol_count,
            "retained_coff_symbol_table": pointer != 0 and symbol_count > 0,
            "export_table_rva": export_rva,
            "export_table_size": export_size,
            "exported_runtime_symbols": exports,
            "debug_directory_types": debug_types,
            "codeview_present": "CodeView" in debug_types,
        },
    }


def main():
    archive_bytes = ARCHIVE.read_bytes()
    archive_identity = identity(archive_bytes)
    fixed_archive = subprocess.check_output(
        ["cmd", "/c", "git", "show", f"{ARCHIVE_COMMIT}:{ARCHIVE_REPO_PATH}"], cwd=REPO)
    if archive_identity["sha256"] != ARCHIVE_SHA256 or archive_bytes != fixed_archive:
        raise RuntimeError("B3-0B Windows 原生探测归档摘要不符")
    result_path = ROOT.parent / "entry-20261123-b3-0b" / "cli-import.zip"
    import zipfile
    with zipfile.ZipFile(result_path) as archive:
        files = json.loads(archive.read("evidence.json"))
        result_identity = files["probe/result.json"]
        result_raw = archive.read("objects/" + result_identity["sha256"])
        if identity(result_raw) != result_identity:
            raise RuntimeError("B3-0B 探测结果摘要不符")
        result = json.loads(result_raw)
        if result["source"]["commit"] != SOURCE_COMMIT or any(
            [row["level"] for row in result["cases"][case]["native"]] != [0, 3]
            or any(row["verification"] != "unverified-coff-exports" for row in result["cases"][case]["native"])
            for case in ("std", "stdlib")
        ):
            raise RuntimeError("B3-0B Windows 产品来源、优化矩阵或历史验证状态不符")
        for case, level in ARTIFACTS:
            relative = f"probe/case-{case}/native-o{level}.exe"
            native = next(item for item in result["cases"][case]["native"] if item["level"] == level)
            if files[relative] != native["exe"]:
                raise RuntimeError(f"B3-0B 原始产物清单不符：{relative}")
        if any(path.lower().endswith((".pdb", ".map")) for path in files):
            sidecars = [path for path in files if path.lower().endswith((".pdb", ".map"))]
        else:
            sidecars = []
        readobj_path, readobj_identity, readobj_version = readobj()
        task_root = Path(tempfile.mkdtemp(prefix=".coff-probe-", dir=ROOT))
        try:
            observations = [run_one(readobj_path, archive, files, case, level, task_root)
                            for case, level in ARTIFACTS]
        finally:
            shutil.rmtree(task_root)

    report = {
        "kind": "feasibility_probe_only",
        "product_acceptance": False,
        "source_commit": SOURCE_COMMIT,
        "target": TARGET,
        "archive": {"path": "../entry-20261123-b3-0b/cli-import.zip", "commit": ARCHIVE_COMMIT, **archive_identity},
        "archive_contains_pdb_or_map_sidecars": sidecars,
        "tools": {"llvm_readobj": {"path": str(readobj_path), **readobj_identity, "version": readobj_version}},
        "observations": observations,
        "conclusion": {
            "all_four_products_have_no_coff_symbol_table": all(
                item["observed"]["retained_coff_symbol_table"] is False for item in observations),
            "exports_are_readable": all(item["observed"]["export_table_size"] > 0 for item in observations),
            "product_verification_remains": "UnverifiedCoffExports",
            "windows_product_coff_status": "pending",
        },
    }
    (ROOT / "coff-probe.json").write_text(json.dumps(report, ensure_ascii=False, sort_keys=True, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(json.dumps({"artifacts": len(observations), "symbol_table_readable": False,
                      "exports_readable": report["conclusion"]["exports_are_readable"],
                      "product_status": "pending"}, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    main()
