import datetime,hashlib,json,os,pathlib,subprocess,sys,time
TASK=pathlib.Path(__file__).resolve().parents[1]
SOURCE=TASK/'source'
EVIDENCE=SOURCE/'docs/DevDocs/assets/20o-b2-a1/entry-20261106'
name=sys.argv[1]; argv=sys.argv[2:]
if argv and argv[0]=='--': argv=argv[1:]
assert argv
if pathlib.Path(argv[0]).name.lower() in {'cargo','cargo.exe'}: assert '--locked' in argv
setup=json.loads((TASK/'environment.json').read_text(encoding='utf-8'))
removed=[key for key in os.environ if key.startswith('RUST_BIGDECIMAL_')]
env={key:value for key,value in os.environ.items() if not key.startswith('RUST_BIGDECIMAL_') and not key.startswith('XIAO_')}
env.update(setup['msvc_whitelist']); env.update(setup['overrides'])
log=EVIDENCE/(name+'.log'); assert not log.exists(),name
start=time.monotonic(); record={'name':name,'argv':argv,'cwd':str(SOURCE),'started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'environment':'environment.json','removed_environment_names':removed,'log':log.name}
with log.open('wb') as stream:
 process=subprocess.Popen(argv,cwd=SOURCE,env=env,stdout=stream,stderr=subprocess.STDOUT); record['pid']=process.pid; record['exit_code']=process.wait()
record.update(finished_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),duration_seconds=round(time.monotonic()-start,3),sha256=hashlib.sha256(log.read_bytes()).hexdigest())
(EVIDENCE/(name+'.json')).write_text(json.dumps(record,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
print(json.dumps(record,ensure_ascii=False),flush=True)
raise SystemExit(process.returncode)
