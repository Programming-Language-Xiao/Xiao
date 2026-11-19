"""在临时副本调用真实只读审计，严格核对完整拒绝原因并后置归档。"""
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

sys.dont_write_bytecode = True
import audit
import common as c

REVIEW=c.REPOSITORY/audit.REVIEW_PREFIX
CASES={
    "missing_attributes":"RERUN证据缺少.gitattributes",
    "missing_file":"RERUN清单文件缺失",
    "file_count":"RERUN清单文件数不符",
    "manifest_digest":"RERUN清单与实际字节不符",
    "forged_windows_complete":"RERUN Windows导出完整性被冒进升级",
    "forged_platform_or_stage":"RERUN未验证平台或B6-2状态被冒进升级",
    "rewritten_file_and_manifest":"RERUN固定Git清单字节不符",
}


def rewrite(path,value):
    path.write_text(json.dumps(value,ensure_ascii=False,indent=2)+"\n",encoding="utf-8",newline="\n")


def rehash(root):
    value=c.read(root,"evidence.json")
    files=c.inventory(root)
    files.pop("evidence.json")
    value.update(files=files,file_count_excluding_manifest=len(files))
    rewrite(root/"evidence.json",value)


def mutate(root,case):
    if case=="missing_attributes":
        (root/".gitattributes").unlink()
    elif case=="missing_file":
        (root/"README.md").unlink()
    elif case in {"file_count","manifest_digest"}:
        value=c.read(root,"evidence.json")
        if case=="file_count":
            value["file_count_excluding_manifest"]+=1
        else:
            value["files"]["README.md"]["sha256"]="0"*64
        rewrite(root/"evidence.json",value)
    elif case in {"forged_windows_complete","forged_platform_or_stage"}:
        value=c.read(root,"result.json")
        if case=="forged_windows_complete":
            value["platforms"]["windows"]["artifact_verification"]="Complete"
            value["platforms"]["windows"]["export_integrity"]="passed"
        else:
            value.update(three_platform_passed=True,stage_passed=True,b6_2_started=True)
        rewrite(root/"result.json",value)
        rehash(root)
    elif case=="rewritten_file_and_manifest":
        p=root/"README.md"
        p.write_bytes(p.read_bytes()+b"\nRERUN-NEGATIVE-TAMPER\n")
        rehash(root)


def run(root):
    before=c.inventory(root)
    argv=[sys.executable,"-B","-X","utf8",str(c.ROOT/"audit.py"),"--root",str(root),
          "--repository-root",str(c.REPOSITORY)]
    env=os.environ.copy()
    env.update(PYTHONUTF8="1",PYTHONIOENCODING="utf-8")
    started=datetime.now(timezone.utc).isoformat()
    clock=time.monotonic()
    child=subprocess.Popen(argv,cwd=c.REPOSITORY,env=env,stdin=subprocess.DEVNULL,
                           stdout=subprocess.PIPE,stderr=subprocess.PIPE,encoding="utf-8",errors="strict")
    try:
        stdout,stderr=child.communicate(timeout=60)
    except subprocess.TimeoutExpired:
        child.kill()
        child.communicate()
        raise RuntimeError("RERUN反例超时，不能计作预期拒绝") from None
    unchanged=before==c.inventory(root)
    c.require(not stderr and unchanged,"RERUN反例审计异常或改写证据")
    result=json.loads(stdout)
    c.require(result["read_only"] is True,"RERUN反例未保持只读")
    return {"argv":argv,"pid":child.pid,"started_utc":started,
            "finished_utc":datetime.now(timezone.utc).isoformat(),"elapsed_seconds":time.monotonic()-clock,
            "exit_code":child.returncode,"stdout":stdout,"stderr":stderr,"read_only":unchanged,"outcome":result}


def collect():
    before=c.inventory(c.ROOT)
    rows=[]
    with tempfile.TemporaryDirectory(prefix="xiao-b6-1-rerun-negative-") as temporary:
        for case in ["baseline",*CASES]:
            copy=Path(temporary)/case
            shutil.copytree(c.ROOT,copy)
            if case!="baseline":
                mutate(copy,case)
            row=run(copy)
            reason=CASES.get(case)
            c.require(row["exit_code"]==(0 if case=="baseline" else 1)
                      and row["outcome"]["passed"]==(case=="baseline")
                      and row["outcome"].get("reason")==reason,"RERUN反例实际原因不符："+case+":"+row["stdout"])
            rows.append({"case":case,"expected_reason":reason,**row})
    c.require(c.inventory(c.ROOT)==before,"RERUN反例修改主归档")
    commit=audit.git(c.REPOSITORY,"log","-1","--format=%H","--",c.PREFIX+"/evidence.json").decode().strip()
    return {"passed":True,"archive_read_only":True,"case_count":len(rows),"cases":rows,
            "source_manifest_commit":commit,"source_manifest_sha256":c.sha((c.ROOT/"evidence.json").read_bytes()),
            "audit_sha256":c.sha((c.ROOT/"audit.py").read_bytes()),"script_sha256":c.sha(Path(__file__).read_bytes())}


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    record=sys.argv[1:]==["--record"]
    c.require(not sys.argv[1:] or record,"用法：self_negative.py [--record]")
    if record:
        c.require(not (REVIEW/"negative-cases.json").exists() and not (REVIEW/"evidence.json").exists(),"RERUN反例报告禁止覆盖")
    value=collect()
    if record:
        c.write_new(REVIEW/"negative-cases.json",value)
        files=c.inventory(REVIEW)
        c.write_new(REVIEW/"evidence.json",{"file_count_excluding_manifest":len(files),"files":files,
            "generated_from_actual_bytes":True,"source_manifest_commit":value["source_manifest_commit"]})
    print(json.dumps(value,ensure_ascii=False))
