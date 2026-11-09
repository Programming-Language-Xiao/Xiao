"""B3受控命令：独立环境、源码/参考输入、Runtime前后摘要与只增不覆写的日志。"""
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time

ROOT=Path(__file__).resolve().parent
SOURCE=ROOT.parents[4]
PRODUCT_ROOTS=['core/rust','stdlib','tests/fixtures','tests/benchmarks','.github/workflows','.cargo',
               'tools','cli/ts','docs/UseDocs','docs/module-registry.json','package.json','bun.lock','.gitattributes']
REFERENCE_INPUTS=['docs/DevDocs/assets/20o-b2-b/entry-20261108-b3/b1/'+name for name in ['reference-vectors.tsv','reference-inputs.json','generate-reference.py','pow-diff.json']]

def write(path,data):
    path.write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')

def snapshot():
    names=set()
    for flags in [[],['--others','--exclude-standard']]:
        raw=subprocess.check_output(['cmd','/c','git','ls-files','-z',*flags,'--',*PRODUCT_ROOTS],cwd=SOURCE)
        names.update(name.decode('utf-8') for name in raw.split(b'\0') if name)
    for relative in REFERENCE_INPUTS:
        path=SOURCE/relative
        if path.is_file():names.add(path.relative_to(SOURCE).as_posix())
    rows=[]
    for name in sorted(names):
        raw=(SOURCE/name).read_bytes()
        try:raw.decode('utf-8');canonical=raw if b'\0' in raw else raw.replace(b'\r\n',b'\n')
        except UnicodeDecodeError:canonical=raw
        rows.append({'path':name,'bytes':len(raw),'raw_sha256':hashlib.sha256(raw).hexdigest(),
                     'utf8_lf_or_binary_sha256':hashlib.sha256(canonical).hexdigest()})
    raw_tree=hashlib.sha256(json.dumps(rows,ensure_ascii=False,sort_keys=True,separators=(',',':')).encode()).hexdigest()
    canonical_tree=hashlib.sha256(json.dumps([[row['path'],row['utf8_lf_or_binary_sha256']] for row in rows],ensure_ascii=False,separators=(',',':')).encode()).hexdigest()
    directory=ROOT/'source-manifests';directory.mkdir(exist_ok=True);path=directory/(raw_tree+'.json')
    if not path.exists():write(path,{'scope':PRODUCT_ROOTS,'reference_inputs':REFERENCE_INPUTS,
                                    'raw_tree_sha256':raw_tree,'utf8_lf_tree_sha256':canonical_tree,'files':rows})
    return {'manifest':path.relative_to(ROOT).as_posix(),'raw_tree_sha256':raw_tree,
            'utf8_lf_tree_sha256':canonical_tree,'files':len(rows)}

def runtime_identity(setup):
    path=Path(setup['overrides']['XIAO_RUNTIME_LIBRARY'])
    if not path.exists():return {'path':str(path),'exists':False}
    return {'path':str(path),'exists':True,'bytes':path.stat().st_size,'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),
            'mtime_ns':path.stat().st_mtime_ns}

def main():
    phase,name,*argv=sys.argv[1:]
    if phase not in ['baseline','wrapper'] or not argv:raise SystemExit('run-step.py baseline|wrapper 名称 命令 参数...')
    directory=ROOT/phase;log=directory/(name+'.log');record_path=log.with_suffix('.json')
    if log.exists() or record_path.exists():raise SystemExit('拒绝覆盖已有证据：'+name)
    setup=json.loads((ROOT/'environment.json').read_text(encoding='utf-8'))
    env={k:v for k,v in os.environ.items() if not k.startswith(('RUST_BIGDECIMAL_','XIAO_'))}
    env.update(setup['msvc_whitelist']);env.update(setup['overrides'])
    removed=sorted(k for k in os.environ if k.startswith(('RUST_BIGDECIMAL_','XIAO_')))
    before=snapshot();runtime_before=runtime_identity(setup)
    started=time.monotonic();record={'argv':argv,'cwd':str(SOURCE),'phase':phase,'name':name,
        'started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'environment':'environment.json',
        'removed_environment_names':removed,'log':log.relative_to(ROOT).as_posix()}
    with log.open('wb') as stream:
        process=subprocess.Popen(argv,cwd=SOURCE,env=env,stdout=stream,stderr=subprocess.STDOUT)
        record.update(pid=process.pid,exit_code=process.wait())
    after=snapshot()
    record.update(source_before=before,source_after=after,source_unchanged=before==after,
        runtime_before=runtime_before,runtime_after=runtime_identity(setup),
        duration_seconds=round(time.monotonic()-started,3),finished_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),
        sha256=hashlib.sha256(log.read_bytes()).hexdigest(),runner_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
    write(record_path,record)
    print(json.dumps(record,ensure_ascii=True),flush=True)
    if before!=after:raise SystemExit('执行期间输入变化；不能作为有效证据')
    raise SystemExit(record['exit_code'])

if __name__=='__main__':main()
