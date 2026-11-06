"""从固定快照、实际作业和原始ZIP生成A6退出记录；缺证据时明确拒绝。"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import zipfile
sys.dont_write_bytecode=True
ROOT=Path(__file__).resolve().parent
parser=argparse.ArgumentParser()
parser.add_argument('--code-commit',required=True)
parser.add_argument('--workspace',default='ci/decimal-workspace')
parser.add_argument('--default',default='ci/decimal-default')
parser.add_argument('--performance',default='ci/decimal-performance')
args=parser.parse_args()
commit=subprocess.check_output(['cmd','/c','git','rev-parse',args.code_commit],cwd=ROOT.parents[4],text=True).strip()
spec=importlib.util.spec_from_file_location('a6_audit',ROOT/'audit.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
common=m.helper()
def read(name): return json.loads((ROOT/name).read_text(encoding='utf-8'))
def write(name,value):
 path=ROOT/name;path.parent.mkdir(parents=True,exist_ok=True)
 path.write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
ci_runs={key:value+'/run.json' for key,value in {'workspace':args.workspace,'default':args.default,'performance':args.performance}.items()}
for role,name in ci_runs.items():
 run=read(name)
 assert run['head_sha']==commit and run['conclusion']=='success' and run['status']=='completed',(role,run['conclusion'])
roles={'vm':'Rust workspace default tests (Linux)','windows':'Native differential regression (Windows)','linux':'Native differential regression (Linux)','macos':'Native differential regression (macOS)'}
ci_records={}; selected={}
for group,directory in [('default',args.default),('perf',args.performance)]:
 jobs=read(directory+'/jobs.json')['jobs']
 for platform,job_name in roles.items():
  candidates=[job for job in jobs if job['name']==job_name]
  aggregate=max(candidates,key=lambda job:(job['run_attempt'],job['id']))
  execution=[job for job in candidates if common.execution_identity(job)==common.execution_identity(aggregate)]
  original=min(execution,key=lambda job:(job['run_attempt'],job['id']))
  assert original['conclusion']=='success'
  log=directory+f'/job-{original["id"]}.log'
  record={'run_id':original['run_id'],'run_attempt':original['run_attempt'],'job_id':original['id'],'job_name':job_name,'code_commit':commit,'conclusion':original['conclusion'],'aggregate_job_id':aggregate['id'],'aggregate_run_attempt':aggregate['run_attempt'],'execution_retained':original['id']!=aggregate['id'],'log':log,'log_sha256':sha(ROOT/log)}
  name=f'ci/records/{group}-{platform}.json';write(name,record);ci_records[group+'-'+platform]=name;selected[group+'-'+platform]=record
performance={}
for role,backend in [('local-vm','vm'),('local-native','native')]:
 archive=ROOT/'performance-archives'/(role+'-decimal.zip')
 performance[role]={'archive':archive.relative_to(ROOT).as_posix(),'sha256':sha(archive),'report':'report.json','artifacts':'artifacts','backend':backend}
metadata=read(args.performance+'/artifacts.json')['artifacts']
for platform,name in [('vm','Linux'),('windows','Windows'),('linux','Linux'),('macos','macOS')]:
 record=selected['perf-'+platform]
 prefix='b2-a6-vm-' if platform=='vm' else 'b2-a6-native-'
 expected=f"{prefix}{name}-{record['run_id']}-{record['run_attempt']}"
 artifact=next(item for item in metadata if item['name']==expected)
 archive=ROOT/args.performance/f'artifact-{artifact["id"]}.zip'
 backend='vm' if platform=='vm' else 'native'
 performance['ci-'+platform]={'archive':archive.relative_to(ROOT).as_posix(),'sha256':sha(archive),'artifact_id':artifact['id'],'report':'b2-a6-'+backend+'.json','artifacts':'b2-a6-'+backend+'-artifacts','backend':backend}
snapshot=read('a6-build-runtime-decimal.json')['source_after']['manifest']
closeout={'passed':True,'scope':'A6资源与受控性能','code_commit':commit,'functional_code_commit':commit,'source_manifest':snapshot,'functional_source_manifest':snapshot,'abi':'1.18','codegen':20,'active_intrinsic_max':32,'cmath':'planned','numeric_tutorial_scope':'planned','a7_started':False,'b1_721_executed':False,'independent_debts_closed':False,'independent_debt_record':'independent-22d-record.json','independent_debt_records':['independent-22d-record.json','independent-22c-record.json'],'ci_runs':ci_runs,'ci_jobs':{'default':args.default+'/jobs.json','performance':args.performance+'/jobs.json'},'ci_records':ci_records,'ci_artifacts':args.performance+'/artifacts.json','performance':performance}
write('closeout.json',closeout)
print(json.dumps({'created':'closeout.json','code_commit':commit,'ci_roles':list(ci_records),'performance_roles':list(performance)},ensure_ascii=False))
