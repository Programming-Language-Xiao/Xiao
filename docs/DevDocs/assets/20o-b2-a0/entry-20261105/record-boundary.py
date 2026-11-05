"""记录A0所有改动路径及签字防漂移；Git调用统一经过cmd。"""

import datetime as dt
import hashlib
import json
from pathlib import Path
import subprocess


def main():
    root = Path(__file__).resolve().parent
    identity = json.loads((root / "identity.json").read_text(encoding="utf-8"))
    repository = Path(identity["repository_root"])
    source = Path(identity["source_root"])
    records = []

    def git(name, cwd, *args):
        argv = ["cmd", "/c", "git", *args]
        process = subprocess.Popen(argv, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        raw, error = process.communicate()
        log = root / "logs" / (name + ".log")
        log.write_bytes(raw)
        error_log = log.with_name(name + "-stderr.log")
        error_log.write_bytes(error)
        records.append({"name": name, "argv": argv, "cwd": str(cwd), "pid": process.pid,
                        "exit_code": process.returncode, "log": "logs/" + log.name,
                        "sha256": hashlib.sha256(raw).hexdigest(),
                        "stderr_log": "logs/" + error_log.name,
                        "stderr_sha256": hashlib.sha256(error).hexdigest()})
        assert process.returncode == 0, f"{name} exit={process.returncode}; 完整输出见{log}和{error_log}"
        return raw.decode("utf-8").strip()

    assert git("boundary-head", repository, "rev-parse", "HEAD") == identity["head"]
    changed = git("boundary-all-tracked", repository, "diff", "--name-only", "HEAD").splitlines()
    untracked = git("boundary-all-untracked", repository, "ls-files", "--others", "--exclude-standard").splitlines()
    allowed = {
        "docs/DevDocs/20o-b2-a0-baseline-plan.md", "docs/DevDocs/20o-b2-a-implementation-plan.md",
        "docs/DevDocs/20o-b2-numeric-foundation-plan.md", "docs/DevDocs/20o-official-library-program.md",
        "docs/DevDocs/20o-b2-freeze-closeout-plan.md", "docs/DevDocs/README.md",
        "docs/DevDocs/20-builtins-and-standard-library.md",
    }
    assert all(path in allowed or path.startswith("docs/DevDocs/assets/20o-b2-a0/") for path in changed + untracked)
    git("boundary-diff-check", repository, "diff", "HEAD", "--check")
    assert git("boundary-source-head", source, "rev-parse", "HEAD") == identity["head"]
    assert not git("boundary-source-status", source, "status", "--porcelain=v1")
    signed = []
    for item in identity["signed_object_checks"]:
        raw = (repository / item["file"]).read_bytes()
        observed = hashlib.sha256(raw.replace(b"\r\n", b"\n")).hexdigest()
        assert observed == item["sha256"], item["file"]
        signed.append({"path": item["file"], "utf8_lf_sha256": observed, "unchanged": True})
    # A-P回执及锁/源码均由“全部路径”白名单保护，不以某几个代码根冒充完整树。
    report = {"passed": True, "checked_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
              "source_commit": identity["head"], "scope": "所有Git跟踪差异 + 所有非忽略未跟踪路径；只准七份状态文档和A0证据目录",
              "changed_tracked_paths": changed, "untracked_paths": untracked,
              "signed_objects": signed, "source_worktree_clean": True,
              "protected_includes": ["产品源码", "Cargo/锁", ".cargo", "ABI/intrinsic", "诊断", "测试/夹具/benchmark", "工作流/门禁工具", "UseDocs", "module-registry", "签字/台账/答复", "19/11/22债务"],
              "commands": records}
    (root / "boundary.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"passed": True, "changed_tracked": len(changed), "untracked": len(untracked)}, ensure_ascii=False))


if __name__ == "__main__":
    main()
