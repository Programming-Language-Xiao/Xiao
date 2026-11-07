"""只读核对B2当前归档和Git导出副本；只有--output才写验证回执。"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile

root=Path(__file__).resolve().parent
source=root.parents[4]
parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path);args=parser.parse_args()
evidence=root.parent/'entry-20261108-b2'
def snapshot(path):
    return {p.relative_to(path).as_posix():(hashlib.sha256(p.read_bytes()).hexdigest(),p.stat().st_size,p.stat().st_mtime_ns)
            for p in sorted(path.rglob('*')) if p.is_file()}
def audit(path):
    argv=[sys.executable,'-X','utf8',str(evidence/'audit.py'),'--root',str(path),'--source-root',str(source)]
    before=snapshot(path)
    process=subprocess.run(argv,cwd=source,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,encoding='utf-8')
    readonly=before==snapshot(path)
    parsed=json.loads(process.stdout) if process.returncode==0 else {}
    passed=process.returncode==0 and parsed.get('passed') is True and parsed.get('files')==99 and parsed.get('checks')==14 and parsed.get('exact_checks')==4 and readonly
    return {'argv':argv,'cwd':str(source),'exit_code':process.returncode,'output':process.stdout.strip(),'read_only':readonly,'passed':passed}
local=audit(evidence)
commit=subprocess.check_output(['cmd','/c','git','rev-parse','HEAD'],cwd=source,text=True).strip()
argv=['cmd','/c','git','archive','--format=tar',commit,'--','docs/DevDocs/assets/20o-b2-b/entry-20261108-b2']
archive=subprocess.check_output(argv,cwd=source)
with tempfile.TemporaryDirectory(prefix='xiao-b2-git-replay-') as temporary:
    export=Path(temporary).resolve()
    with tarfile.open(fileobj=io.BytesIO(archive),mode='r:') as stream:stream.extractall(export,filter='data')
    copied=export/'docs/DevDocs/assets/20o-b2-b/entry-20261108-b2'
    replay=audit(copied)
report={'passed':local['passed'] and replay['passed'],'archive_commit':commit,'archive_argv':argv,
        'archive_sha256':hashlib.sha256(archive).hexdigest(),'local':local,'git_export':replay}
if args.output:args.output.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
print(json.dumps({'passed':report['passed'],'archive_commit':commit,'local_read_only':local['read_only'],'git_export_read_only':replay['read_only']},ensure_ascii=True))
raise SystemExit(0 if report['passed'] else 1)
