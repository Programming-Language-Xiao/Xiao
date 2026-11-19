"""从回执重算受限结论；定稿后单次运行docs并按实际字节封存。"""
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

sys.dont_write_bytecode = True
import audit
import common as c


def docs_snapshot():
    paths=[c.REPOSITORY/n for n in c.DOC_FILES+audit.REVIEW_FILES]
    paths.extend(p for p in c.ROOT.rglob("*") if p.is_file()
                 and not p.relative_to(c.ROOT).as_posix().startswith("objects/")
                 and p.name not in {"docs-final.json","evidence.json"})
    result={}
    for path in paths:
        raw=path.read_bytes()
        result[path.relative_to(c.REPOSITORY).as_posix()]={
            **c.digest(raw),"lf_sha256":c.sha(raw.replace(b"\r\n",b"\n"))}
    return result


def history_unchanged():
    for name, row in c.read(c.ROOT,"historical.json").items():
        c.require(c.digest((c.REPOSITORY/name).read_bytes())==row,"RERUN历史证据变化："+name)


def product_snapshot():
    source=c.read(c.ROOT,"source.json")["files"]
    names=audit.git(c.REPOSITORY,"ls-files","-z","--cached","--others","--exclude-standard",
                    "--",*c.PRODUCT_ROOTS).decode("utf-8").split("\0")
    c.require(set(names)-{""}==set(source),"RERUN文档门禁产品路径集合改变")
    rows={}
    for name in source:
        raw=(c.REPOSITORY/name).read_bytes()
        lf=c.sha(raw if source[name]["binary"] else raw.replace(b"\r\n",b"\n"))
        c.require(lf==source[name]["lf_sha256"],"RERUN文档门禁产品偏离固定Git："+name)
        rows[name]={**c.digest(raw),"lf_sha256":lf}
    return rows


def docs():
    path=c.ROOT/"docs-final.json"
    c.require(not path.exists(),"RERUN最终docs回执禁止覆盖")
    history_unchanged()
    before=docs_snapshot()
    product_before=product_snapshot()
    env=os.environ.copy()
    removed=[]
    for key in list(env):
        if key.upper().startswith(("XIAO_","RUST_BIGDECIMAL_")):
            removed.append(key)
            env.pop(key)
    task=Path(c.WINDOWS_TASK)/"docs"
    task.mkdir()
    env.update(TEMP=str(task),TMP=str(task),XIAO_B1_F10="0",XIAO_B2_A_PERF="0")
    bun=shutil.which("bun",path=env.get("PATH"))
    c.require(bun,"RERUN文档门禁缺少Bun")
    argv=[bun,"tools/repo-check/src/cli.ts","all","--format","json"]
    started=datetime.now(timezone.utc).isoformat()
    clock=time.monotonic()
    log=task/"docs.log"
    with log.open("xb") as stream:
        child=subprocess.Popen(argv,cwd=c.REPOSITORY,env=env,stdin=subprocess.DEVNULL,
                               stdout=stream,stderr=subprocess.STDOUT)
        code=child.wait()
    elapsed=time.monotonic()-clock
    after=docs_snapshot()
    product_after=product_snapshot()
    report=json.loads(log.read_bytes())
    c.write_new(path,{
        "argv":argv,"cwd":str(c.REPOSITORY),"pid":child.pid,"exit_code":code,
        "started_utc":started,"finished_utc":datetime.now(timezone.utc).isoformat(),
        "elapsed_seconds":elapsed,"environment_overrides":{
            k:env[k] for k in ("TEMP","TMP","XIAO_B1_F10","XIAO_B2_A_PERF")},
        "removed_environment_names":sorted(removed),"bun":c.identity(bun),
        "docs_before":c.store(c.canonical(before)),"docs_after":c.store(c.canonical(after)),
        "product_before":c.store(c.canonical(product_before)),"product_after":c.store(c.canonical(product_after)),
        "source_unchanged":before==after and product_before==product_after,"report_passed":report.get("passed") is True,
        "result_sha256":c.sha((c.ROOT/"result.json").read_bytes()),"log":c.store(log.read_bytes()),
    })
    history_unchanged()
    c.require(code==0 and report.get("passed") is True and before==after
              and product_before==product_after,"RERUN最终docs门禁失败")
    print(json.dumps({"docs_passed":True,"pid":child.pid,"elapsed_seconds":elapsed}))


def seal():
    docs=c.read(c.ROOT,"docs-final.json")
    c.require(docs["exit_code"]==0 and docs["source_unchanged"]
              and c.unpack(c.ROOT,docs["docs_after"])==c.canonical(docs_snapshot()),
              "RERUN文档门禁之后仍有改动")
    history_unchanged()
    files=c.inventory(c.ROOT)
    c.require(".gitattributes" in files and "evidence.json" not in files,"RERUN清单属性缺失或已经封存")
    c.write_new(c.ROOT/"evidence.json",{
        "file_count_excluding_manifest":len(files),"files":files,
        "generated_from_actual_bytes":True,"generator":"finish.py seal",
        "code_commit":c.CODE,"docs_receipt":"docs-final.json",
    })
    print(json.dumps({"sealed_files_excluding_manifest":len(files)}))


if __name__ == "__main__":
    c.require(len(sys.argv)==2,"用法：finish.py result | docs | seal")
    if sys.argv[1]=="result":
        result=audit.recompute(c.ROOT,c.REPOSITORY)
        history_unchanged()
        c.write_new(c.ROOT/"result.json",result)
        print(json.dumps({k:result[k] for k in ("rerun_completed","b6_1_status","stage_passed","linux_original_blocker_resolved","b6_2_started")},ensure_ascii=False))
    elif sys.argv[1]=="docs":
        docs()
    elif sys.argv[1]=="seal":
        seal()
    else:
        raise ValueError("未知finish阶段")
