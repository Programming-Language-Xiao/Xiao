"""复现本轮审计缺口；原版应暴露误接受，修订版必须按实际原因拒绝。"""
import argparse,copy,datetime,hashlib,json,shutil,subprocess,sys,tempfile,zipfile
from pathlib import Path
parser=argparse.ArgumentParser()
parser.add_argument('--phase',choices=['before','after'],required=True)
args=parser.parse_args()
ROOT=Path(__file__).resolve().parent
ARCHIVE=ROOT.parent/'entry-20261107'
REPOSITORY=ROOT.parents[4]
output=ROOT/args.phase;output.mkdir()
verifier=ARCHIVE/'verify-performance.py'
results=[]
def run(name,argv,expected):
    started=datetime.datetime.now(datetime.timezone.utc).isoformat()
    p=subprocess.run(argv,capture_output=True)
    (output/(name+'.stdout.log')).write_bytes(p.stdout);(output/(name+'.stderr.log')).write_bytes(p.stderr)
    actual=p.stderr.decode('utf-8').strip()
    passed=(p.returncode==0 and json.loads(p.stdout)['passed']) if args.phase=='before' else (p.returncode==1 and not p.stdout and actual==expected)
    row={'case':name,'phase':args.phase,'verification_passed':passed,'started_utc':started,'argv':argv,'exit_code':p.returncode,'expected_after_error':expected,'actual_error':actual,'stdout':name+'.stdout.log','stderr':name+'.stderr.log','stdout_sha256':hashlib.sha256(p.stdout).hexdigest(),'stderr_sha256':hashlib.sha256(p.stderr).hexdigest()}
    results.append(row);print(json.dumps({'case':name,'verification_passed':passed,'exit_code':p.returncode},ensure_ascii=False),flush=True)
    if not passed: raise SystemExit(json.dumps(row,ensure_ascii=False))
with tempfile.TemporaryDirectory(prefix='a6-review-report-') as temporary:
    directory=Path(temporary)
    with zipfile.ZipFile(ARCHIVE/'performance-archives/local-native-decimal.zip') as z:z.extractall(directory)
    report=json.loads((directory/'report.json').read_text())
    path=directory/'omit-artifact.json';r=copy.deepcopy(report);r['artifacts']=r['artifacts'][:1];path.write_text(json.dumps(r),encoding='utf-8')
    run('missing-executable-hashes',[sys.executable,'-X','utf8',str(verifier),'--report',str(path),'--artifacts',str(directory/'artifacts'),'--require-full'],'实际产物集合缺项、重复或后端不符')
    process=next((directory/'artifacts/optimization-0').glob('*.process.json'))
    r=json.loads(process.read_text());r['executable']='C:/old-cache/unrelated.exe';process.write_text(json.dumps(r),encoding='utf-8')
    run('foreign-process-executable',[sys.executable,'-X','utf8',str(verifier),'--report',str(directory/'report.json'),'--artifacts',str(directory/'artifacts'),'--require-full'],'原始进程执行文件或参数未绑定实际产物')
for name,mutate,error in [
 ('swapped-default-platforms',lambda c:c['ci_records'].update({'default-windows':c['ci_records']['default-linux'],'default-linux':c['ci_records']['default-windows']}),'CI平台角色与实际作业名称不符'),
 ('maintenance-as-workspace',lambda c:c['ci_runs'].update(workspace=c['ci_runs']['performance']),'CI工作流角色或触发方式不符：workspace')]:
    with tempfile.TemporaryDirectory(prefix='a6-review-archive-') as temporary:
        directory=Path(temporary)/'archive';shutil.copytree(ARCHIVE,directory)
        path=directory/'closeout.json';r=json.loads(path.read_text());mutate(r);path.write_text(json.dumps(r),encoding='utf-8')
        argv=[sys.executable,'-X','utf8',str(ARCHIVE/'audit.py'),'--archive-root',str(directory),'--repository-root',str(REPOSITORY)]
        p=subprocess.run([*argv,'--manifest'],capture_output=True);assert p.returncode==0,p.stderr.decode()
        run(name,argv,error)
(output/'cases.json').write_text(json.dumps({'phase':args.phase,'passed':True,'scope':'原件临时副本；before确认误接受，after核对指定实际拒绝文本','cases':results},ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
