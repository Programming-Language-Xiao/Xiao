"""以真实A6归档为正例，逐项破坏后核对实际错误全文；不只核对退出码。"""
import argparse
import copy
import datetime
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

parser=argparse.ArgumentParser()
parser.add_argument('--archive-root',type=Path,default=Path(__file__).resolve().parent)
parser.add_argument('--repository-root',type=Path)
parser.add_argument('--output',type=Path,required=True)
args=parser.parse_args()
root=args.archive_root.resolve()
repository=(args.repository_root or root.parents[4]).resolve()
args.output.mkdir(parents=True,exist_ok=False)

def edit(directory,name,mutate):
    path=directory/name
    value=json.loads(path.read_text(encoding='utf-8'))
    mutate(value)
    path.write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')

def actual_job(directory):
    closeout=json.loads((directory/'closeout.json').read_text())
    run=json.loads((directory/closeout['ci_runs']['default']).read_text())
    edit(directory,closeout['ci_records']['default-vm'],lambda r:r.update(job_id=run['id']))

cases=[
 ('valid',None,None),
 ('false-closeout',lambda d:edit(d,'closeout.json',lambda r:r.update(passed=False)),'A6尚未有效收口'),
 ('missing-check',lambda d:edit(d,'checks.json',lambda r:r['effective'].pop()),'A6有效检查清单缺项/重复'),
 ('unlocked-cargo',lambda d:edit(d,'a6-resources-decimal.json',lambda r:r['argv'].remove('--locked')),'A6实际Cargo命令偏离执行点：a6-resources-decimal'),
 ('false-test-count',lambda d:edit(d,'checks.json',lambda r:r['test_counts']['a6-resources-decimal'][0].__setitem__(0,99)),'A6精确入口或计数契约漂移'),
 ('aggregate-job-id',actual_job,'实际日志作业被聚合作业ID替代或缺失'),
 ('cross-platform-report',lambda d:edit(d,'closeout.json',lambda r:r['performance'].__setitem__('ci-macos',copy.deepcopy(r['performance']['ci-windows']))),'性能报告平台或实际编译器宿主不符'),
 ('wrong-artifact-id',lambda d:edit(d,'closeout.json',lambda r:r['performance']['ci-vm'].update(artifact_id=0)),'性能ZIP没有唯一的实际CI产物ID'),
]
results=[]
with tempfile.TemporaryDirectory(prefix='a6-source-audit-review-') as temporary:
    for name,mutate,expected in cases:
        directory=Path(temporary)/name
        shutil.copytree(root,directory,ignore=shutil.ignore_patterns(args.output.name,'__pycache__'))
        if mutate: mutate(directory)
        prefix=[sys.executable,'-X','utf8',str(root/'audit.py'),'--archive-root',str(directory),'--repository-root',str(repository)]
        manifest=subprocess.run([*prefix,'--manifest'],capture_output=True)
        assert manifest.returncode==0,manifest.stderr.decode()
        started=datetime.datetime.now(datetime.timezone.utc).isoformat()
        result=subprocess.run(prefix,capture_output=True)
        stdout=result.stdout.decode('utf-8');stderr=result.stderr.decode('utf-8')
        (args.output/(name+'.stdout.log')).write_bytes(result.stdout)
        (args.output/(name+'.stderr.log')).write_bytes(result.stderr)
        passed=(result.returncode==0 and not stderr and json.loads(stdout)['passed']) if expected is None else (result.returncode==1 and not stdout and stderr.strip()==expected)
        row={'case':name,'passed':passed,'argv':prefix,'started_utc':started,'exit_code':result.returncode,'expected_error':expected,'actual_error':stderr,'stdout':name+'.stdout.log','stderr':name+'.stderr.log','stdout_sha256':hashlib.sha256(result.stdout).hexdigest(),'stderr_sha256':hashlib.sha256(result.stderr).hexdigest()}
        results.append(row)
        print(json.dumps({'case':name,'passed':passed,'exit_code':result.returncode},ensure_ascii=False),flush=True)
        if not passed: raise SystemExit(json.dumps(row,ensure_ascii=False))
(args.output/'negative-cases.json').write_text(json.dumps({'passed':True,'scope':'实际错误全文匹配；临时副本重新生成清单后进入语义审计','cases':results},ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
