"""A6第二轮只读审计命令记录；每次核对产品输入未变化，不复用旧结论。"""
import datetime,hashlib,json,os,subprocess,sys,time
from pathlib import Path
ROOT=Path(__file__).resolve().parent
SOURCE=ROOT.parents[4]
name=sys.argv[1];argv=sys.argv[2:]
if argv and argv[0]=='--':argv=argv[1:]
assert argv
scope=['core/rust','stdlib','tests/fixtures','tests/benchmarks','.github/workflows','.cargo','tools','cli/ts','docs/UseDocs','docs/module-registry.json','package.json','bun.lock','.gitattributes']
def inputs():
    p=subprocess.run(['cmd','/c','git','diff','--exit-code','e7bf7a49ba3ed55679b69c97ac1bd5ce7f5777b6','--',*scope],cwd=SOURCE,capture_output=True)
    assert p.returncode==0,'产品输入偏离被测快照，不能沿用旧证据'
    untracked=subprocess.check_output(['cmd','/c','git','ls-files','--others','--exclude-standard','--',*scope],cwd=SOURCE)
    assert not untracked,'产品范围有未登记文件'
    return {'code_commit':'e7bf7a49ba3ed55679b69c97ac1bd5ce7f5777b6','scope':scope,'no_product_difference':True}
record={'name':name,'argv':argv,'cwd':str(SOURCE),'started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'product_before':inputs()}
started=time.monotonic();environment=os.environ.copy();environment['PYTHONDONTWRITEBYTECODE']='1'
for key in list(environment):
    if key.startswith('RUST_BIGDECIMAL_'):environment.pop(key)
setup=SOURCE.parent/'environment.json'
if setup.exists():
    config=json.loads(setup.read_text(encoding='utf-8'))
    environment.update(config['msvc_whitelist']);environment.update(config['overrides'])
stdout=ROOT/(name+'.stdout.log');stderr=ROOT/(name+'.stderr.log')
with stdout.open('xb') as out,stderr.open('xb') as error:
    process=subprocess.Popen(argv,cwd=SOURCE,env=environment,stdout=out,stderr=error)
    record['pid']=process.pid;record['exit_code']=process.wait()
record.update(finished_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),seconds=time.monotonic()-started,product_after=inputs(),stdout=stdout.name,stderr=stderr.name,stdout_sha256=hashlib.sha256(stdout.read_bytes()).hexdigest(),stderr_sha256=hashlib.sha256(stderr.read_bytes()).hexdigest())
(ROOT/(name+'.json')).write_text(json.dumps(record,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
print(json.dumps({'name':name,'exit_code':record['exit_code'],'stdout':stdout.name,'stderr':stderr.name},ensure_ascii=False),flush=True)
raise SystemExit(record['exit_code'])
