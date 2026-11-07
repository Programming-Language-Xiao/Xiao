"""只读复跑及临时副本拒绝测试；不重写真实归档清单。"""
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
parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--source-root',type=Path,default=root.parents[4]);parser.add_argument('--output',type=Path);args=parser.parse_args()
def snapshot(directory):
    return {p.relative_to(directory).as_posix():(hashlib.sha256(p.read_bytes()).hexdigest(),p.stat().st_size,p.stat().st_mtime_ns)
            for p in sorted(directory.rglob('*')) if p.is_file()}
def run(directory,reason=None):
    env=os.environ.copy();env['PYTHONUTF8']='1';env['PYTHONIOENCODING']='utf-8'
    before=snapshot(directory)
    result=subprocess.run([sys.executable,'-X','utf8',str(root/'audit.py'),'--root',str(directory),'--source-root',str(args.source_root)],
                          env=env,cwd=args.source_root,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,encoding='utf-8')
    output=result.stdout.strip();readonly=before==snapshot(directory)
    passed=result.returncode==(1 if reason else 0) and (output==reason if reason else json.loads(output)['passed'] is True) and readonly
    return {'exit_code':result.returncode,'output':output,'read_only':readonly,'passed':passed}

with tempfile.TemporaryDirectory(prefix='xiao-b2-evidence-negative-') as temporary:
    directory=Path(temporary).resolve()/root.name
    shutil.copytree(root,directory)
    results={'baseline':run(directory)}
    manifest=directory/'evidence.json';data=json.loads(manifest.read_text(encoding='utf-8'));data['files'][0]['sha256']='0'*64
    manifest.write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
    results['tampered_manifest']=run(directory,'B2证据清单或摘要不符')
    shutil.copyfile(root/'evidence.json',manifest);(directory/'.gitattributes').unlink()
    results['missing_attributes']=run(directory,'B2证据清单或摘要不符')
    shutil.copyfile(root/'.gitattributes',directory/'.gitattributes')
    data=json.loads((directory/'closeout.json').read_text(encoding='utf-8'));data['abi']='1.19'
    (directory/'closeout.json').write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
    data=json.loads((directory/'evidence.json').read_text(encoding='utf-8'))
    for row in data['files']:
        if row['path']=='closeout.json':
            row['bytes']=(directory/'closeout.json').stat().st_size
            row['sha256']=hashlib.sha256((directory/'closeout.json').read_bytes()).hexdigest()
    manifest.write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
    results['semantic_tamper_after_temp_manifest_rebuild']=run(directory,'B2版本或文档状态越界')

report={'passed':all(r['passed'] for r in results.values()),'cases':results}
if args.output: args.output.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
print(json.dumps(report,ensure_ascii=True))
raise SystemExit(0 if report['passed'] else 1)
