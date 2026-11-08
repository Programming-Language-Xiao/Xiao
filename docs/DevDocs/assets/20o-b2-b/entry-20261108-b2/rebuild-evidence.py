"""有意修改本批归档后，按最终实际字节生成清单；审计不会调用此命令。"""
import argparse
import hashlib
import json
from pathlib import Path

parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--root',type=Path,default=Path(__file__).resolve().parent);args=parser.parse_args();root=args.root.resolve()
rows=[{'path':p.relative_to(root).as_posix(),'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
      for p in sorted(root.rglob('*')) if p.is_file() and p!=root/'evidence.json' and '__pycache__' not in p.parts]
(root/'evidence.json').write_text(json.dumps({'scope':'B2本目录全部归档文件，清单自身除外','file_count':len(rows),'files':rows},ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
print(json.dumps({'files':len(rows)},ensure_ascii=False))
