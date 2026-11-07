"""在临时副本重建清单后，复核语义审计能否拒绝执行点或Runtime身份被删改。"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

root=Path(__file__).resolve().parent
source=root.parents[4]
original=root.parent/'entry-20261108-b2'
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--expect-rejection',action='store_true')
parser.add_argument('--output',type=Path)
args=parser.parse_args()

def snapshot(directory):
    return {p.relative_to(directory).as_posix():(hashlib.sha256(p.read_bytes()).hexdigest(),p.stat().st_size,p.stat().st_mtime_ns)
            for p in sorted(directory.rglob('*')) if p.is_file()}

def rebuild(directory):
    rows=[{'path':p.relative_to(directory).as_posix(),'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
          for p in sorted(directory.rglob('*')) if p.is_file() and p!=directory/'evidence.json']
    (directory/'evidence.json').write_text(json.dumps({'file_count':len(rows),'files':rows},ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')

records=[]
source_manifest=json.loads((original/'closeout.json').read_text(encoding='utf-8'))['source_manifest']
mutations={
 'drop_clippy':('closeout.json',lambda data:data['effective_checks'].remove('b2-final2-clippy'),'B2有效执行点集合不完整'),
 'drop_exact_checks':('closeout.json',lambda data:data.update(exact_checks=[]),'B2精确入口集合不完整'),
 'wrong_runtime_override':('environment.json',lambda data:data['overrides'].update(XIAO_RUNTIME_LIBRARY=str(Path(data['overrides']['CARGO_TARGET_DIR'])/'release/nonexistent-runtime.lib')),'B2Runtime环境路径与收口身份不符'),
 'wrong_summary_counts':('closeout.json',lambda data:data.update(runtime_tests=1),'B2收口测试计数不符'),
 'missing_source_row':(source_manifest,lambda data:data.update(files=[row for row in data['files'] if row['path']!='core/rust/crates/xiao-runtime/src/abi_lfloat_divide.rs']),'B2源码清单范围不完整'),
}
with tempfile.TemporaryDirectory(prefix='xiao-b2-review-audit-') as temporary:
    temp=Path(temporary).resolve()
    for name,(filename,mutate,reason) in mutations.items():
        directory=temp/name
        if directory.resolve().parent!=temp:raise SystemExit('临时路径越界')
        shutil.copytree(original,directory)
        target=directory/filename
        data=json.loads(target.read_text(encoding='utf-8'));mutate(data)
        target.write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
        rebuild(directory)
        before=snapshot(directory)
        argv=[sys.executable,'-X','utf8',str(original/'audit.py'),'--root',str(directory),'--source-root',str(source)]
        env=os.environ.copy();env['PYTHONUTF8']='1';env['PYTHONIOENCODING']='utf-8'
        process=subprocess.run(argv,cwd=source,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,encoding='utf-8')
        output=process.stdout.strip();read_only=before==snapshot(directory)
        expected_exit=1 if args.expect_rejection else 0
        passed=process.returncode==expected_exit and read_only
        if args.expect_rejection:passed=passed and output==reason
        records.append({'case':name,'argv':argv,'exit_code':process.returncode,'output':output,
                        'expected_reason':reason,'read_only':read_only,'passed':passed})
report={'passed':all(record['passed'] for record in records),'expect_rejection':args.expect_rejection,
        'audit_sha256':hashlib.sha256((original/'audit.py').read_bytes()).hexdigest(),'records':records}
if args.output:args.output.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
print(json.dumps(report,ensure_ascii=True))
raise SystemExit(0 if report['passed'] else 1)
