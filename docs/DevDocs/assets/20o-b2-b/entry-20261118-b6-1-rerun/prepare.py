"""从固定Git对象导出B6-1两平台的新源码，冻结命令及历史证据身份。"""
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile

sys.dont_write_bytecode = True
import common as c


def git(*args):
    return subprocess.run(["cmd", "/c", "git", "-C", str(c.REPOSITORY), *args],
                          capture_output=True, check=True).stdout


def extract(bundle, destination):
    with tarfile.open(fileobj=io.BytesIO(bundle)) as archive:
        archive.extractall(destination, filter="data")


def main():
    linux = sys.platform != "win32"
    task = Path(c.LINUX_TASK if linux else c.WINDOWS_TASK)
    c.require(not task.exists(), "RERUN任务根必须全新，拒绝复用：" + str(task))
    if linux:
        windows_bundle = Path("/mnt/d/project/Xiao/.b6/b6-1-rerun-20261118-01/windows/source.tar")
        bundle = windows_bundle.read_bytes()
        source = c.read(c.ROOT, "source.json")
        c.require(c.sha(bundle) == source["archive_sha256"], "Linux导出归档摘要不符")
    else:
        c.require(not git("diff", "--name-only", c.CODE, "HEAD", "--", *c.PRODUCT_ROOTS),
                  "当前产品偏离FIX.2固定提交")
        c.require(not git("status", "--porcelain", "--", *c.PRODUCT_ROOTS),
                  "受保护产品工作树有未提交变化")
        bundle = git("archive", "--format=tar", c.CODE, "--", *c.PRODUCT_ROOTS)
        names = git("ls-files", "-z", "--cached", "--others", "--exclude-standard",
                    "--", *c.PRODUCT_ROOTS).decode("utf-8").split("\0")
        current = {}
        for name in sorted(set(names)-{""}):
            raw = (c.REPOSITORY/name).read_bytes()
            binary = b"\0" in raw
            current[name] = {**c.digest(raw), "binary":binary,
                "lf_sha256":c.sha(raw if binary else raw.replace(b"\r\n",b"\n"))}
        with tarfile.open(fileobj=io.BytesIO(bundle)) as archive:
            fixed = {}
            for member in archive.getmembers():
                if member.isfile():
                    raw = archive.extractfile(member).read()
                    binary = b"\0" in raw
                    fixed[member.name] = {**c.digest(raw), "binary": binary,
                        "lf_sha256": c.sha(raw if binary else raw.replace(b"\r\n", b"\n"))}
        c.require({k:v["lf_sha256"] for k,v in current.items()} ==
                  {k:v["lf_sha256"] for k,v in fixed.items()}, "工作树与固定Git源码不一致")
        c.write_new(c.ROOT / "source.json", {
            "code_commit": c.CODE, "entry_commit": git("rev-parse", c.ENTRY).decode().strip(),
            "archive_command": ["cmd", "/c", "git", "archive", "--format=tar", c.CODE,
                                "--", *c.PRODUCT_ROOTS],
            "archive_sha256": c.sha(bundle), "archive_bytes": len(bundle),
            "files": fixed, "file_count": len(fixed),
        })
        c.write_new(c.ROOT / "commands.json", {
            platform: {stage: c.command(stage) for stage in c.STAGES +
                       (["clippy"] if platform == "windows" else [])}
            for platform in ("windows", "linux")
        })
        historical = {}
        for prefix in (
            "docs/DevDocs/assets/20o-b2-b/entry-20261117-b6",
            "docs/DevDocs/assets/20o-b2-b/entry-20261118-b4-component-fix",
            "docs/DevDocs/assets/20o-b2-b/entry-20261118-b4-component-review",
            "docs/DevDocs/assets/20o-b2-b/entry-20261117-b5-closeout",
            "docs/DevDocs/assets/20o-b2-b/entry-20261117-url-optimization",
            "docs/DevDocs/assets/20o-b2-static-float-equality/entry-20261117-fix",
            "docs/DevDocs/assets/20o-b2-b/entry-20261115-b4-3-fix1",
        ):
            historical.update({prefix + "/" + k:v for k,v in c.inventory(c.REPOSITORY/prefix).items()})
        c.write_new(c.ROOT/"historical.json", historical)
        source = c.read(c.ROOT, "source.json")
    task.mkdir(parents=True)
    if not linux:
        (task / "source.tar").write_bytes(bundle)
    extract(bundle, task / "source")
    actual = c.source_snapshot(task/"source")
    c.require(actual == source["files"], "RERUN新源码解包字节不符")
    for name in ("logs", "temp", "home"):
        (task/name).mkdir()
    c.write_new(c.ROOT/(("linux" if linux else "windows")+"-setup.json"), {
        "task_root": str(task), "source_root": str(task/"source"),
        "task_root_existed_before": False, "target_existed_before": False,
        "runtime_existed_before": False, "source_matches_fixed_git_bytes": True,
        "archive_sha256": c.sha(bundle), "file_count": len(actual),
        "source": c.store(c.canonical(actual)),
    })
    print(json.dumps({"prepared":str(task), "source_files":len(actual)}))


if __name__ == "__main__":
    main()
