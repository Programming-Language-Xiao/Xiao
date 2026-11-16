"""显式生成B5封存报告；默认审计永不写盘。"""
import argparse
from pathlib import Path
import shutil
import sys
import tempfile
sys.dont_write_bytecode=True
from record import ROOT,REPOSITORY,write_json,require
from audit import check,inventory,negatives,handoff

OUTPUTS=["closeout.json","b6-handoff.json","self-negative.json","evidence.json"]
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument("--collect",action="store_true",required=True);p.parse_args()
 require(not any((ROOT/name).exists() for name in OUTPUTS),"B5封存文件不可覆盖")
 with tempfile.TemporaryDirectory(prefix="b5-final-collect-") as temp:
  copy=Path(temp)/"evidence";shutil.copytree(ROOT,copy)
  result=check(copy,REPOSITORY,sealed=False)
  write_json(copy/"closeout.json",result);write_json(copy/"b6-handoff.json",handoff(result))
  write_json(copy/"evidence.json",{"files":inventory(copy)})
  review=negatives(copy,REPOSITORY);write_json(copy/"self-negative.json",review)
  (copy/"evidence.json").unlink();write_json(copy/"evidence.json",{"files":inventory(copy)})
  check(copy,REPOSITORY)
  for name in OUTPUTS:
   with (ROOT/name).open("xb") as out:out.write((copy/name).read_bytes())
 check()
 print("B5封存通过",len(inventory(ROOT)),"文件",len(review["cases"]),"项反例")
if __name__=="__main__":
 try:main()
 except (ValueError,KeyError,OSError,AssertionError) as e:print(str(e),file=sys.stderr);raise SystemExit(1)
