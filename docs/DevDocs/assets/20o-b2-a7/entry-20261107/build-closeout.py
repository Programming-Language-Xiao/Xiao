"""A7结束后对实际作业与工具/产物身份封存；缺成功证据直接拒绝。"""
from pathlib import Path
import argparse,hashlib,importlib.util,json,re,subprocess,sys
sys.dont_write_bytecode=True
ROOT=Path(__file__).resolve().parent;REPO=ROOT.parents[4]
parser=argparse.ArgumentParser();parser.add_argument('--maintenance',default='ci/maintenance');parser.add_argument('--workspace',default='ci/workspace');args=parser.parse_args()
CODE='5749d6e0f277ba9aebf2bf8468e0c79a082e42a6';PRODUCT='e7bf7a49ba3ed55679b69c97ac1bd5ce7f5777b6'
def read(name):return json.loads((ROOT/name).read_text(encoding='utf-8'))
def write(name,value):
 path=ROOT/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
def file(p):
 raw=p.read_bytes();return {'path':str(p),'bytes':len(raw),'sha256':hashlib.sha256(raw).hexdigest()}
helper=REPO/'docs/DevDocs/assets/20o-b2-a5/entry-20261106/audit.py';spec=importlib.util.spec_from_file_location('a7_common',helper);common=importlib.util.module_from_spec(spec);spec.loader.exec_module(common)
runs={role:read(directory+'/run.json') for role,directory in [('workspace',args.workspace),('maintenance',args.maintenance)]}
for role,r in runs.items():assert r['head_sha']==CODE and r['status']=='completed' and r['conclusion']=='success',(role,r['conclusion'])
jobs=read(args.maintenance+'/jobs.json')['jobs'];records={}
for role,name in {'vm':'Rust workspace default tests (Linux)','windows':'Native differential regression (Windows)','linux':'Native differential regression (Linux)','macos':'Native differential regression (macOS)'}.items():
 options=[j for j in jobs if j['name']==name];aggregate=max(options,key=lambda j:(j['run_attempt'],j['id']));same=[j for j in options if common.execution_identity(j)==common.execution_identity(aggregate)];origin=min(same,key=lambda j:(j['run_attempt'],j['id']));assert origin['conclusion']=='success'
 log=args.maintenance+f'/job-{origin["id"]}.log';raw=(ROOT/log).read_bytes()
 record={'code_commit':CODE,'run_id':origin['run_id'],'run_attempt':origin['run_attempt'],'job_id':origin['id'],'job_name':name,'conclusion':'success','aggregate_job_id':aggregate['id'],'aggregate_run_attempt':aggregate['run_attempt'],'execution_retained':origin['id']!=aggregate['id'],'log':log,'log_sha256':hashlib.sha256(raw).hexdigest()}
 path=f'ci/records/{role}.json';write(path,record);records[role]=path
snapshot=read('b1-contract.json')['source_after']['manifest'];environment=read('environment.json');settings=environment['overrides']
tools={}
for key in ['RUSTC','XIAO_CLANG','XIAO_LLVM_AS','XIAO_LLC']:
 p=Path(settings[key]);result=subprocess.run([str(p),'-Vv' if key=='RUSTC' else '--version'],capture_output=True);assert result.returncode==0;tools[key]={'file':file(p),'version':result.stdout.decode('utf-8')}
root=Path(settings['TEMP']).resolve();observed={}
for log in ['b1-native-exact.log','a1-native-exact.log']:
 text=(ROOT/log).read_text(encoding='utf-8')
 for name in re.findall(r'artifact=(.*?) result=success',text):
  path=Path(name.removeprefix(chr(92)+chr(92)+"?"+chr(92))).resolve()
  assert path.is_relative_to(root),'产物不属于本任务独占临时根'
  assert path.is_file(),'标记的本轮产物不存在：'+str(path)
  observed[str(path)]=file(path)
write('local-artifacts.json',{'source_manifest':snapshot,'runtime_build_record':'build-runtime.json','runtime':file(Path(settings['XIAO_RUNTIME_LIBRARY'])),'tools':tools,'fresh_target':settings['CARGO_TARGET_DIR'],'source_root':str(REPO),'stdlib_root':settings['XIAO_STDLIB_ROOT'],'artifacts':list(observed.values()),'note':'实际输出中标记的本轮产物；只保存身份不作为性能测量。'})
closeout={'passed':True,'scope':'B2-A内部1.18/20收口','code_commit':CODE,'product_commit':PRODUCT,'source_manifest':snapshot,'abi':'1.18','codegen':20,'active_intrinsic_max':32,'standard_library_interface':1,'cmath':'planned','final_numeric_tutorial_status':'planned','internal_numeric_scope':'已通过','b1_721_executed':False,'b2_b_c_started':False,'public_release':False,'independent_debts_closed':False,'a0_original_result':'保留passed=false；有效结论仅closeout.json','a7_registry_changed':False,'ci_runs':{'workspace':args.workspace+'/run.json','maintenance':args.maintenance+'/run.json'},'ci_jobs':args.maintenance+'/jobs.json','ci_records':records,'next_entry':'B2-B实施规划；本回执不授权新产品或B2-C最终验收'}
write('closeout.json',closeout);print(json.dumps({'created':'closeout.json','current_code':CODE,'product_code':PRODUCT,'native_artifacts':len(observed),'ci_run':runs['maintenance']['id']},ensure_ascii=False))
