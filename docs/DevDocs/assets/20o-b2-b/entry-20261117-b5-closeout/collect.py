"""显式采集历史审计、生成联合收口与最终清单；审计入口本身只读。"""
from datetime import datetime, timezone
import gzip
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

sys.dont_write_bytecode = True
import audit

ROOT, REPOSITORY = audit.ROOT, audit.REPOSITORY
TASK = REPOSITORY.parent / ".b5/b5-closeout-20261117-01"
AUXILIARY = [
    "docs/DevDocs/assets/20o-b2-b/entry-20261113-b4-3/record.py",
    "docs/DevDocs/20o-b2-static-float-equality-fix-plan.md",
]


def write(path, value):
    with path.open("xb") as output:
        output.write((json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode("utf-8"))


def store(raw):
    packed = gzip.compress(raw, mtime=0) if len(raw) > 8192 else raw
    digest = audit.sha(packed)
    path = ROOT / "objects/sha256" / digest[:2] / (digest + ".bin")
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        audit.require(path.read_bytes() == packed, "收口内容寻址对象损坏")
    else:
        with path.open("xb") as output:
            output.write(packed)
    return {"path": path.relative_to(ROOT).as_posix(), **audit.identity(packed),
            "encoding": "gzip" if len(raw) > 8192 else "identity",
            "raw_bytes": len(raw), "raw_sha256": audit.sha(raw)}


def product():
    names = audit.git(REPOSITORY, "ls-files", "-z", "--cached", "--others",
                      "--exclude-standard", "--", *audit.PRODUCT_ROOTS).decode().split("\0")
    paths = sorted(set(names) - {""})
    actual = {name: audit.sha((REPOSITORY / name).read_bytes().replace(b"\r\n", b"\n"))
              for name in paths}
    fixed = {name: audit.sha(raw.replace(b"\r\n", b"\n"))
             for name, raw in audit.files(REPOSITORY, audit.CURRENT,
                                         tuple(audit.PRODUCT_ROOTS)).items()}
    audit.require(actual == fixed, "收口批不得修改受保护产品树")
    return audit.sha(audit.canonical(actual))


def dependencies():
    fixed = audit.files(REPOSITORY, audit.ENTRY, tuple(AUXILIARY))
    identities = {}
    for name, content in fixed.items():
        actual = (REPOSITORY / name).read_bytes().replace(b"\r\n", b"\n")
        audit.require(actual == content.replace(b"\r\n", b"\n"), "历史审计依赖被改写：" + name)
        identities[name] = audit.identity(actual)
    return identities


def run(label, argv, inputs):
    destination = ROOT / (label + ".json")
    audit.require(not destination.exists(), "收口运行回执禁止覆盖")
    TASK.mkdir(parents=True, exist_ok=True)
    before, product_before = inputs(), product()
    dependencies_before = dependencies()
    env = dict(os.environ, PYTHONUTF8="1", PYTHONIOENCODING="utf-8",
               PYTHONDONTWRITEBYTECODE="1")
    started = datetime.now(timezone.utc).isoformat()
    clock = time.monotonic()
    child = subprocess.Popen(argv, cwd=REPOSITORY, env=env,
                             stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                             stderr=subprocess.PIPE)
    stdout, stderr = child.communicate()
    elapsed = time.monotonic() - clock
    finished = datetime.now(timezone.utc).isoformat()
    after, product_after = inputs(), product()
    read_only = before == after and product_before == product_after
    audit.require(dependencies() == dependencies_before, "历史审计依赖运行中变化")
    result = {
        "argv": argv, "cwd": str(REPOSITORY), "pid": child.pid,
        "started_utc": started, "finished_utc": finished,
        "elapsed_seconds": elapsed, "exit_code": child.returncode, "read_only": read_only,
        "product_before": product_before, "product_after": product_after,
        "dependency_inputs": dependencies_before,
        "stdout": store(stdout), "stderr": store(stderr),
        "python": {"version": sys.version, "binary": audit.identity(Path(sys.executable).read_bytes())},
        "collector_sha256": audit.sha(Path(__file__).read_bytes()),
    }
    return result, before, after


def replay(key):
    descriptor, _ = audit.source(REPOSITORY, key)
    directory = REPOSITORY / descriptor["directory"]
    source = audit.files(REPOSITORY, descriptor["commit"], (descriptor["directory"],))
    expected = {name[len(descriptor["directory"]) + 1:]: audit.identity(raw)
                for name, raw in source.items()}

    def inputs():
        actual = audit.inventory(directory)
        audit.require(actual == expected, "历史证据工作区偏离固定Git对象：" + key)
        return audit.sha(audit.canonical(actual))

    argv = [sys.executable, "-B", "-X", "utf8", descriptor["directory"] + "/audit.py"]
    result, before, after = run(key + "-audit", argv, inputs)
    result.update(source=descriptor, source_before=before, source_after=after)
    write(ROOT / (key + "-audit.json"), result)
    audit.require(result["exit_code"] == 0 and result["read_only"], "历史审计复跑未通过：" + key)
    print(json.dumps({"audit": key, "exit_code": result["exit_code"],
                      "pid": result["pid"], "read_only": result["read_only"]}, ensure_ascii=False))


def docs_inputs():
    paths = [REPOSITORY / name for name in audit.DOC_FILES]
    paths += [p for p in ROOT.iterdir() if p.is_file()
              and p.name not in ("docs-final.json", "evidence.json")]
    return {p.relative_to(REPOSITORY).as_posix(): {
                **audit.identity(raw := p.read_bytes()),
                "lf_sha256": audit.sha(raw.replace(b"\r\n", b"\n")),
            } for p in sorted(paths)}


def docs():
    bun = shutil.which("bun")
    audit.require(bun is not None, "找不到Bun")
    argv = [bun, "tools/repo-check/src/cli.ts", "all", "--format", "json"]
    result, before, after = run("docs-final", argv, docs_inputs)
    result.update(inputs=store(audit.canonical(before)),
                  inputs_after=store(audit.canonical(after)),
                  bun={"binary": audit.identity(Path(bun).read_bytes()),
                       "version": subprocess.run([bun, "--version"], check=True, capture_output=True,
                                                 encoding="utf-8").stdout.strip()})
    write(ROOT / "docs-final.json", result)
    report = json.loads(audit.stored(ROOT, result["stdout"]))
    audit.require(result["exit_code"] == 0 and result["read_only"] and report["passed"],
                  "最终文档门禁未通过")
    print(json.dumps({"docs_passed": True, "pid": result["pid"],
                      "elapsed_seconds": result["elapsed_seconds"]}))


def seal():
    docs = audit.load(ROOT / "docs-final.json")
    audit.require(docs["exit_code"] == 0 and docs["read_only"]
                  and json.loads(audit.stored(ROOT, docs["inputs"])) == docs_inputs()
                  == json.loads(audit.stored(ROOT, docs["inputs_after"])),
                  "最终文档门禁之后仍有改动")
    product()
    write(ROOT / "evidence.json", {"files": audit.inventory(ROOT)})
    print(json.dumps({"sealed_files_excluding_manifest": len(audit.inventory(ROOT)) - 1}))


if __name__ == "__main__":
    audit.require(len(sys.argv) in (2, 3), "用法：collect.py replay matrix|fix|url 或 closeout|docs|seal")
    if sys.argv[1] == "replay":
        audit.require(len(sys.argv) == 3 and sys.argv[2] in audit.SOURCES, "审计来源非法")
        replay(sys.argv[2])
    elif sys.argv[1] == "closeout":
        product()
        write(ROOT / "closeout.json", audit.expected(REPOSITORY, ROOT))
        print(json.dumps({"b5_closed": True, "b6_entry_open": True, "b6_started": False}))
    elif sys.argv[1] in ("docs", "seal"):
        {"docs": docs, "seal": seal}[sys.argv[1]]()
    else:
        raise ValueError("未知采集步骤")
