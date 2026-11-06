"""原样打包报告及其独占产物根，不覆盖原件；二进制只归档，不执行。"""
import argparse
import hashlib
import json
from pathlib import Path
import zipfile

parser=argparse.ArgumentParser()
parser.add_argument('--report',type=Path,required=True)
parser.add_argument('--output',type=Path,required=True)
args=parser.parse_args()
report=json.loads(args.report.read_text(encoding='utf-8'))
root=Path(report['artifacts'][0]['path']).parent
assert root.is_dir(),root
args.output.parent.mkdir(parents=True,exist_ok=True)
with zipfile.ZipFile(args.output,'x',zipfile.ZIP_DEFLATED,compresslevel=9) as archive:
 archive.write(args.report,'report.json')
 for path in sorted(root.rglob('*')):
  if path.is_file():
   assert path.resolve().is_relative_to(root.resolve()),path
   archive.write(path,'artifacts/'+path.relative_to(root).as_posix())
raw=args.output.read_bytes()
print(json.dumps({'archive':str(args.output),'sha256':hashlib.sha256(raw).hexdigest(),'bytes':len(raw),'members':len(zipfile.ZipFile(args.output).namelist()),'report_complete':report['complete'],'report_passed':report['passed']},ensure_ascii=False))
