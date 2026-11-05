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

# 所有后续验收在命令前后绑定完整源码输入；日志和DevDocs滚动状态不参与产品源码树。
def snapshot():
 roots=['core/rust','stdlib','tests/fixtures','tests/benchmarks','.github/workflows','.cargo','tools','cli/ts','docs/UseDocs','docs/module-registry.json','package.json','bun.lock','.gitattributes']
 paths=set()
 for options in [[],['--others','--exclude-standard']]:
  raw=subprocess.check_output(['cmd','/c','git','ls-files','-z',*options,'--',*roots],cwd=SOURCE)
  paths.update(name.decode('utf-8') for name in raw.split(b'\0') if name)
 rows=[]
 for relative in sorted(paths):
  raw=(SOURCE/relative).read_bytes()
  try:
   raw.decode('utf-8'); normalized=raw.replace(b'\r\n',b'\n')
  except UnicodeDecodeError: normalized=raw
  rows.append({'path':relative,'bytes':len(raw),'raw_sha256':hashlib.sha256(raw).hexdigest(),'utf8_lf_or_binary_sha256':hashlib.sha256(normalized).hexdigest()})
 serialized=json.dumps(rows,ensure_ascii=False,sort_keys=True,separators=(',',':')).encode()
 raw_tree=hashlib.sha256(serialized).hexdigest()
 canonical_tree=hashlib.sha256(json.dumps([[row['path'],row['utf8_lf_or_binary_sha256']] for row in rows],ensure_ascii=False,separators=(',',':')).encode()).hexdigest()
 directory=EVIDENCE/'source-manifests'; directory.mkdir(exist_ok=True); manifest=directory/(raw_tree+'.json')
 if not manifest.exists():
  manifest.write_text(json.dumps({'scope':roots,'source_root':str(SOURCE),'raw_tree_sha256':raw_tree,'utf8_lf_tree_sha256':canonical_tree,'files':rows},ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
 return {'manifest':manifest.relative_to(EVIDENCE).as_posix(),'raw_tree_sha256':raw_tree,'utf8_lf_tree_sha256':canonical_tree,'files':len(rows)}

log=EVIDENCE/(name+'.log'); assert not log.exists(),name
before=snapshot()
start=time.monotonic(); record={'name':name,'argv':argv,'cwd':str(SOURCE),'started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'environment':'environment.json','removed_environment_names':removed,'log':log.name}
with log.open('wb') as stream:
 process=subprocess.Popen(argv,cwd=SOURCE,env=env,stdout=stream,stderr=subprocess.STDOUT); record['pid']=process.pid; record['exit_code']=process.wait()
after=snapshot()
record.update(source_before=before,source_after=after,source_unchanged=before==after,runner_sha256=hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest())
record.update(finished_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),duration_seconds=round(time.monotonic()-start,3),sha256=hashlib.sha256(log.read_bytes()).hexdigest())
(EVIDENCE/(name+'.json')).write_text(json.dumps(record,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
print(json.dumps(record,ensure_ascii=False),flush=True)
assert before==after, '执行期间源码输入变化，不能作为冻结快照证据'
raise SystemExit(process.returncode)
