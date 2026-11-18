"""只读回放开工审计；保留真实输出、PID和历史目录前后身份。"""
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import subprocess
import sys
import time

sys.dont_write_bytecode = True
import common as c

SCRIPTS = {
    "fix2": "20o-b2-b/entry-20261118-b4-component-fix/audit.py",
    "fix2-negative": "20o-b2-b/entry-20261118-b4-component-review/self_negative.py",
    "b5": "20o-b2-b/entry-20261117-b5-closeout/audit.py",
    "fix1": "20o-b2-b/entry-20261115-b4-3-fix1/audit.py",
    "static-equality": "20o-b2-static-float-equality/entry-20261117-fix/audit.py",
    "url": "20o-b2-b/entry-20261117-url-optimization/audit.py",
}


def check_history():
    expected = c.read(c.ROOT,"historical.json")
    for name, row in expected.items():
        c.require(c.digest((c.REPOSITORY/name).read_bytes())==row,"RERUN历史文件发生变化："+name)
    return c.sha(c.canonical(expected))


def capture(item):
    label, relative = item
    script = c.REPOSITORY/"docs/DevDocs/assets"/relative
    argv = [sys.executable,"-B","-X","utf8",str(script)]
    env = os.environ.copy()
    env.update(PYTHONUTF8="1",PYTHONIOENCODING="utf-8")
    started = datetime.now(timezone.utc).isoformat()
    clock = time.monotonic()
    child = subprocess.Popen(argv,cwd=c.REPOSITORY,env=env,stdin=subprocess.DEVNULL,
                             stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    stdout,stderr = child.communicate()
    row = {"label":label,"argv":argv,"cwd":str(c.REPOSITORY),"pid":child.pid,
           "started_utc":started,"finished_utc":datetime.now(timezone.utc).isoformat(),
           "elapsed_seconds":time.monotonic()-clock,"exit_code":child.returncode,
           "stdout":c.store(stdout),"stderr":c.store(stderr),"script":c.identity(script)}
    c.require(child.returncode==0 and not stderr,"RERUN历史审计失败："+label+":"+
              stdout.decode("utf-8",errors="replace")+stderr.decode("utf-8",errors="replace"))
    value = json.loads(stdout)
    c.require(value.get("passed",label=="static-equality") is True,"RERUN审计未通过："+label)
    row["outcome"] = value
    return row


if __name__ == "__main__":
    path = c.ROOT/"entry-gates.json"
    c.require(not path.exists(),"RERUN入口回执禁止覆盖")
    before = check_history()
    with ThreadPoolExecutor(max_workers=3) as workers:
        rows = list(workers.map(capture,SCRIPTS.items()))
    after = check_history()
    c.write_new(path,{"passed":True,"historical_read_only":before==after,
                     "history_before_sha256":before,"history_after_sha256":after,"runs":rows})
    print(json.dumps({"passed":True,"audits":len(rows),"historical_read_only":before==after}))
