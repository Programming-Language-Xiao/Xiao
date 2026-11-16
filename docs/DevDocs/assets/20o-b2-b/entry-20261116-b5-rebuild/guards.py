"""对真实Cargo输出回放工作流守卫，派生负例另标，不冒充平台执行。"""
from pathlib import Path
import re
import subprocess
import tempfile
import sys
sys.dont_write_bytecode=True
from record import ROOT, REPOSITORY, load, sha, store, require, write_json


def workflow_step(text,name):
 lines=text.splitlines();matches=[i for i,line in enumerate(lines) if line=="      - name: "+name]
 require(len(matches)==1,"B5守卫步骤不唯一")
 start=matches[0]+1
 while lines[start]!="        run: |":start+=1
 body=[]
 for line in lines[start+1:]:
  if line.startswith("          "):body.append(line[10:])
  elif not line.strip():body.append("")
  else:break
 return "\n".join(body).rstrip()+"\n"


def main():
 workflow_path=REPOSITORY/".github/workflows/maintenance-regression.yml"
 workflow=workflow_path.read_text(encoding="utf-8")
 default=workflow_step(workflow,"Run B1 VM probe exactly once")
 windows=workflow_step(workflow,"Build runtime and run native gates")
 windows=re.search(r"(?ms)^\$b1Output = cargo test.*?^\}",windows).group()
 scripts={"default":default,"Windows":windows}
 for platform in ["Linux","macOS"]:
  body=workflow_step(workflow,"Build runtime and run "+platform+" native gates")
  helper=re.search(r"(?ms)^run_exact_native_test\(\) \{.*?^\}",body).group()
  scripts[platform]="set -euo pipefail\n"+helper+"\nrun_exact_native_test b1_capabilities_match_native_golden_outputs 20o_b1_probe\n"
 records={name:load(ROOT/"current"/(name+".json")) for name in ["b1-vm","b1-native"]}
 rows=[]
 with tempfile.TemporaryDirectory(prefix="b5-real-output-guards-") as temporary:
  for platform,script in scripts.items():
   source=records["b1-vm" if platform=="default" else "b1-native"]
   raw=(ROOT/source["log"]["path"]).read_bytes()
   text=raw.decode("utf-8").replace("\r\n","\n")
   summaries=re.findall(r"^test result:.*$",text,re.M)
   require(source["exit_code"]==0 and len(summaries)==1 and summaries[0].startswith("test result: ok. 1 passed; 0 failed; 0 ignored;"),"B5真实Cargo来源不是唯一通过")
   summary=summaries[0]
   cases=[
    ("actual",text,0,True),
    ("zero",text.replace(summary,summary.replace("1 passed","0 passed")),0,False),
    ("two",text.replace(summary,summary.replace("1 passed","2 passed")),0,False),
    ("ignored",text.replace(summary,summary.replace("1 passed","0 passed").replace("0 ignored","1 ignored")),0,False),
    ("failed",text.replace(summary,summary.replace("ok.","FAILED.").replace("1 passed","0 passed").replace("0 failed","1 failed")),101,False),
    ("failed-exit",text,101,False),
    ("duplicate",text+summary+"\n",0,False)]
   for name,output,status,accept in cases:
    log=Path(temporary)/(platform+"-"+name+".log");log.write_text(output,encoding="utf-8",newline="\n")
    if platform=="Windows":
     script_path=Path(temporary)/(platform+"-"+name+".ps1")
     escaped=str(log).replace("'","''")
     mock="function cargo { Get-Content -LiteralPath '"+escaped+"'; $global:LASTEXITCODE = "+str(status)+" }\n"
     script_path.write_text("$ErrorActionPreference='Stop'\n"+mock+script,encoding="utf-8")
     argv=["pwsh","-NoProfile","-File",str(script_path)]
    else:
     script_path=Path(temporary)/(platform+"-"+name+".sh")
     escaped=log.as_posix().replace("'","'\\''")
     mock="cargo() { cat '"+escaped+"'; return "+str(status)+"; }\n"
     script_path.write_text(mock+script,encoding="utf-8")
     argv=[r"D:\msys64\usr\bin\bash.exe",str(script_path).replace("\\","/")]
    run=subprocess.run(argv,capture_output=True,timeout=20)
    require((run.returncode==0)==accept,"B5真实输出守卫误判："+platform+"/"+name)
    rows.append({"path":platform,"case":name,"passed":True,"exit_code":run.returncode,
      "scope":"actual Windows Cargo output replay" if name=="actual" else "negative derived from actual output",
      "source_receipt_sha256":sha((ROOT/"current"/("b1-vm.json" if platform=="default" else "b1-native.json")).read_bytes()),
      "source_log_sha256":sha(raw),"input":store(output.encode("utf-8"),"text/plain"),
      "guard_sha256":sha(script.encode()),"stdout":store(run.stdout,"text/plain"),"stderr":store(run.stderr,"text/plain")})
 report={"passed":28,"total":28,"scope":"actual-output-replay plus explicitly derived negatives",
  "platforms_executed":["Windows"],"guard_shell_paths":["default","Windows","Linux","macOS"],
  "not_claimed":"Linux/macOS execution or CI evidence","workflow_sha256":sha(workflow_path.read_bytes()),
  "script_sha256":sha(Path(__file__).read_bytes()),"cases":rows}
 write_json(ROOT/"guards-report.json",report)
 print("B5-GUARDS passed=28 total=28 actual-output=4 derived-negative=24; Windows replay only")


if __name__=="__main__":main()
