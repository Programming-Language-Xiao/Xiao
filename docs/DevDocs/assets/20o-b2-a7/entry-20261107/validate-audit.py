"""A7在临时副本验证拒绝原因；输出必须位于有效归档之外。"""
import argparse,copy,hashlib,json,shutil,subprocess,sys,tempfile
from pathlib import Path
parser=argparse.ArgumentParser();parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
ROOT=Path(__file__).resolve().parent;REPO=ROOT.parents[4]
if args.output.resolve().is_relative_to(ROOT):raise SystemExit('A7反例输出必须位于有效归档之外')
args.output.mkdir(parents=True,exist_ok=False)
def edit(d,name,change):
 p=d/name;value=json.loads(p.read_text());change(value);p.write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
def job(d):
 c=json.loads((d/'closeout.json').read_text());r=json.loads((d/c['ci_runs']['maintenance']).read_text());edit(d,c['ci_records']['vm'],lambda v:v.update(job_id=r['id']))
def swap(c):c['ci_records']['windows'],c['ci_records']['linux']=c['ci_records']['linux'],c['ci_records']['windows']
cases=[('valid',None,None),('final-release',lambda d:edit(d,'closeout.json',lambda v:v.update(public_release=True)),'A7后续范围或债务状态越界'),('old-f10',lambda d:edit(d,'closeout.json',lambda v:v.update(b1_721_executed=True)),'A7后续范围或债务状态越界'),('missing-check',lambda d:edit(d,'checks.json',lambda v:v['effective'].pop()),'A7有效检查清单缺项或重复'),('unlocked',lambda d:edit(d,'b1-contract.json',lambda v:v['argv'].remove('--locked')),'A7实际Cargo参数偏离冻结入口：b1-contract'),('aggregate-job-id',job,'实际日志作业被聚合作业ID替代或缺失'),('swapped-platforms',lambda d:edit(d,'closeout.json',swap),'A7平台角色与实际作业不符'),('maintenance-as-workspace',lambda d:edit(d,'closeout.json',lambda v:v['ci_runs'].update(workspace=v['ci_runs']['maintenance'])),'A7实际CI角色、触发方式或源码不符：workspace')]
rows=[]
with tempfile.TemporaryDirectory(prefix='a7-audit-review-') as temporary:
 for name,change,error in cases:
  directory=Path(temporary)/name;shutil.copytree(ROOT,directory,ignore=shutil.ignore_patterns('__pycache__'))
  if change:change(directory)
  argv=[sys.executable,'-X','utf8',str(ROOT/'audit.py'),'--archive-root',str(directory),'--repository-root',str(REPO)]
  p=subprocess.run([*argv,'--manifest'],capture_output=True);assert p.returncode==0,p.stderr.decode()
  p=subprocess.run(argv,capture_output=True);out=p.stdout.decode('utf-8');err=p.stderr.decode('utf-8')
  passed=p.returncode==0 and not err and json.loads(out)['passed'] if error is None else p.returncode==1 and not out and err.strip()==error
  (args.output/(name+'.stdout.log')).write_bytes(p.stdout);(args.output/(name+'.stderr.log')).write_bytes(p.stderr)
  row={'case':name,'passed':passed,'argv':argv,'exit_code':p.returncode,'expected_error':error,'actual_error':err,'stdout':name+'.stdout.log','stderr':name+'.stderr.log','stdout_sha256':hashlib.sha256(p.stdout).hexdigest(),'stderr_sha256':hashlib.sha256(p.stderr).hexdigest()};rows.append(row)
  print(json.dumps({'case':name,'passed':passed},ensure_ascii=False),flush=True)
  if not passed:raise SystemExit(json.dumps(row,ensure_ascii=False))
(args.output/'cases.json').write_text(json.dumps({'passed':True,'audit_sha256':hashlib.sha256((ROOT/'audit.py').read_bytes()).hexdigest(),'validator_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':rows,'scope':'1正例/7反例；临时归档重建清单后核对实际错误全文'},ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
