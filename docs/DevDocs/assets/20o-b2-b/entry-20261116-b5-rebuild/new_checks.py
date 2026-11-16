"""新入口产物在三类固定验证器上的实际读取矩阵。"""
from pathlib import Path
import subprocess
import sys
sys.dont_write_bytecode=True
from record import ROOT,load,write_json,store,require,sha
def main():
 base=Path(load(ROOT/"current/environment.json")["task_root"]).parent
 directory=base/"current/artifacts/new-entries"
 rows=[]
 for variant in ["b1","b4","final3"]:
  actual="current" if variant=="final3" else variant
  for kind,name in [("xiaoc","main.xiaoc"),("xar","main.xar"),("native","main.exe"),("meta","main.meta")]:
   label="new-entries-"+kind
   argv=[str(base/actual/"b5-host.exe"),"validate",kind,str(directory/name),"accept" if variant=="final3" else "reject"]
   subprocess.run([sys.executable,"-B","-X","utf8",str(ROOT/"record.py"),"run",variant,label,"--",*argv],check=True)
   rows.append({"variant":variant,"kind":kind,"receipt":variant+"/"+label+".json",
                "input":store((directory/name).read_bytes())})
 write_json(ROOT/"new-entry-matrix.json",{"passed":True,"cases":12,"rows":rows,
   "source":store((directory/"main.xiao").read_bytes(),"text/plain"),
   "report":store((directory/"report.json").read_bytes(),"application/json")})
 print("B5新33/34入口十二格读取矩阵通过")
if __name__=="__main__":main()
