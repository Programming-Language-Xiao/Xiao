"""保留首次编译夹具缺失，补齐固定Git原字节后重跑同一Clippy命令。"""
from datetime import datetime, timezone
import io
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import time

sys.dont_write_bytecode = True
import audit
import common as c
import run


def supplement():
    base=Path(c.WINDOWS_TASK)/"source"
    expected=audit.fixed_files(c.REPOSITORY,c.CODE,["tests/spec"])
    for name,raw in expected.items():
        path=base/name
        path.parent.mkdir(parents=True,exist_ok=True)
        with path.open("xb") as output:
            output.write(raw)
    rows={name:c.digest(raw) for name,raw in expected.items()}
    c.write_new(c.ROOT/"supplemental-source.json",{
        "code_commit":c.CODE,"roots":["tests/spec"],"files":rows,
        "reason":"原导出遗漏Clippy all-targets使用的协议编译夹具；不修改测试字节",
    })
    return rows


def snapshot(expected):
    base=Path(c.WINDOWS_TASK)/"source"
    actual={name:c.digest((base/name).read_bytes()) for name in expected}
    c.require(actual==expected,"RERUN补充编译夹具偏离固定Git字节")
    return actual


def main():
    c.require(sys.platform=="win32","Clippy补验只用于Windows")
    path=c.ROOT/"windows-clippy-r2.json"
    c.require(not path.exists(),"RERUN Clippy补验禁止覆盖")
    old=c.read(c.ROOT,"windows-clippy.json")
    c.require(old["exit_code"]==101 and b"tests/spec/11x0-protocol" in c.unpack(c.ROOT,old["log"]),
              "Clippy首次失败不是固定协议夹具缺失")
    extra=supplement()
    task=Path(c.WINDOWS_TASK)
    source=task/"source"
    env,overrides,removed=run.environment(task,source,"clippy-r2",False)
    before=c.source_snapshot(source)
    extra_before=snapshot(extra)
    tools=run.tools(env)
    runtime=c.identity(env["XIAO_RUNTIME_LIBRARY"])
    argv=c.command("clippy")
    c.require(argv==old["argv"],"RERUN Clippy重试不能缩减命令范围")
    log=task/"logs/windows-clippy-r2.log"
    started=datetime.now(timezone.utc).isoformat()
    clock=time.monotonic()
    with log.open("xb") as output:
        child=subprocess.Popen(argv,cwd=source,env=env,stdin=subprocess.DEVNULL,stdout=output,stderr=subprocess.STDOUT)
        code=child.wait()
    elapsed=time.monotonic()-clock
    after=c.source_snapshot(source)
    row={k:old[k] for k in ("platform","stage","code_commit","task_root","source_root","cwd","os","libtest_threads")}
    row.update(label="windows-clippy-r2",argv=argv,pid=child.pid,exit_code=code,
        started_utc=started,finished_utc=datetime.now(timezone.utc).isoformat(),elapsed_seconds=elapsed,
        environment_overrides=overrides,removed_environment_names=removed,tools_before=tools,tools_after=run.tools(env),
        source_before=c.store(c.canonical(before)),source_after=c.store(c.canonical(after)),source_unchanged=before==after,
        supplemental_before=c.store(c.canonical(extra_before)),supplemental_after=c.store(c.canonical(snapshot(extra))),
        runtime_before=runtime,runtime_after=c.identity(env["XIAO_RUNTIME_LIBRARY"]),
        artifacts_after=c.store(c.canonical(run.artifact_snapshot(task,Path(env["TEMP"])))),
        recorder_sha256=c.sha(Path(__file__).read_bytes()),common_sha256=c.sha(Path(c.__file__).read_bytes()),log=c.store(log.read_bytes()))
    c.write_new(path,row)
    print(json.dumps({"label":row["label"],"exit_code":code,"pid":child.pid,"elapsed_seconds":elapsed}))
    c.require(before==after and row["supplemental_before"]==row["supplemental_after"]
              and row["runtime_before"]==row["runtime_after"],"RERUN Clippy重试身份漂移")
    return code


if __name__ == "__main__":
    raise SystemExit(main())
