"""归档实测 Windows 原件，重新读取压缩包校对每件 SHA-256。"""

import hashlib
import json
from pathlib import Path
import sys
import zipfile


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main(task):
    logs = Path(__file__).resolve().parent
    output = logs / "windows-artifacts.zip"
    cases = task / "cases"
    matched = [
        *cases.glob("old27-control-o*/*"),
        *cases.glob("before28-control-o*/*"),
        *cases.glob("after28-*-o*/*"),
        *cases.glob("compatibility-verified/*.xar"),
        *cases.glob("compatibility-verified/*.index.pb"),
        *cases.glob("compatibility-verified/*/main.xiao"),
        *cases.glob("native/xiao-b3-log-*/*"),
        *cases.glob("c-probe/*"),
    ]
    files = {path.relative_to(cases).as_posix(): path for path in matched if path.is_file()}
    files.update({
        f"verifiers/{version}/xiao-core.exe": task / directory / "debug/xiao-core.exe"
        for version, directory in ((27, "target-old27"), (28, "target-tools"))
    })
    assert len([name for name in files if name.endswith(".meta")]) == 8
    manifest = []
    with zipfile.ZipFile(output, "w", zipfile.ZIP_DEFLATED, compresslevel=6) as bundle:
        for member, path in sorted(files.items()):
            data = path.read_bytes()
            if path.name.endswith("verification.json"):
                verification = json.loads(data)
                verifier = 27 if member.startswith("old27-control") else 28
                assert verification["generator"] == verifier
                for role in ("binary", "ir", "map"):
                    linked = verification[role]
                    assert digest(Path(linked["path"]).read_bytes()) == linked["sha256"]
            bundle.writestr(member, data)
            manifest.append({
                "path": str(path), "zip_member": member, "bytes": len(data), "sha256": digest(data),
            })
    with zipfile.ZipFile(output) as bundle:
        assert bundle.testzip() is None
        for item in manifest:
            assert digest(bundle.read(item["zip_member"])) == item["sha256"]
    verified = {"archive": {"path": str(output), "bytes": output.stat().st_size,
                            "sha256": digest(output.read_bytes())}, "files": manifest}
    (logs / "windows-artifact-manifest.json").write_text(
        json.dumps(verified, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"Windows ZIP {len(manifest)} 件原件通过摘要核对，SHA-256 {verified['archive']['sha256']}")


if __name__ == "__main__":
    main(Path(sys.argv[1]).resolve())
