"""实际执行两组缓存探针，并在每次操作后立即冻结索引、对象和可执行字节。"""
import json
from pathlib import Path
import subprocess
import sys
sys.dont_write_bytecode=True
from record import ROOT,load,store,write_json,require,sha

def main():
 base=Path(load(ROOT/"current/environment.json")["task_root"]).parent
 groups=[
  ("generator",[("b1","old",20,"b4"),("b4","new",21,"b4")]),
  ("abi",[("b4","old",21,"b4"),("final2","new",21,"current")])]
 rows=[]
 for group,steps in groups:
  cache=base/("sealed-cache-"+group)
  require(not cache.exists(),"缓存矩阵任务不可复用")
  for variant,label,codegen,run_variant in steps:
   actual="current" if variant=="final2" else variant
   argv=[str(base/actual/"b5-host.exe"),"cache",str(cache),str(cache/(label+".exe")),str(codegen),str(base/run_variant/"target/release/xiao_runtime.lib")]
   record_label="cache-"+group+"-sealed-"+label
   subprocess.run([sys.executable,"-B","-X","utf8",str(ROOT/"record.py"),"run",variant,record_label,"--",*argv],check=True)
   receipt=load(ROOT/variant/(record_label+".json"))
   result=json.loads((ROOT/receipt["log"]["path"]).read_bytes())
   rows.append({"group":group,"step":label,"variant":variant,"receipt":variant+"/"+record_label+".json",
       "receipt_sha256":sha((ROOT/variant/(record_label+".json")).read_bytes()),"result":result,
       "index_bytes":store(Path(result["index_path"]).read_bytes()),
       "native_object":store(Path(result["object_path"]).read_bytes()),
       "executable":store(Path(result["executable"]).read_bytes())})
 write_json(ROOT/"cache-matrices.json",{"passed":True,"scope":"production ArtifactStore/GlobalIndex/fingerprint; explicit probe orchestration",
   "no_global_cache":True,"rows":rows})
 print("B5两组缓存索引/对象快照全部冻结")

if __name__=="__main__":main()
