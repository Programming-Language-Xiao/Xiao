"""旧C调用方对象一次编译，分别链接三类Runtime并核对声明/符号集合。"""
import json
from pathlib import Path
import re
import subprocess
import sys
sys.dont_write_bytecode=True
from record import ROOT, load, require, sha, store, write_json


def main():
    baselines={name:load(ROOT/name/"baseline.json") for name in ["b1","b4","current"]}
    old=baselines["b1"]["c_abi_signatures"]
    previous=baselines["b4"]["c_abi_signatures"]
    current=baselines["current"]["c_abi_signatures"]
    require(all(current.get(name)==signature for name,signature in old.items()),"B1 C签名改变/删除")
    require(previous==current,"B5相对B4改变了C入口")
    require(set(baselines["b1"]["exported_c_symbols"])<=set(baselines["current"]["exported_c_symbols"]),"B1 C符号缺失")
    added=sorted(set(current)-set(old))
    require(added==sorted(["xiao_runtime_lfloat_divide"]+["xiao_runtime_cmath_"+name for name in [
      "sqrt","floor","ceil","round","trunc","abs","min","max","pow","sin","cos","tan","exp","ln","log10"]]),"C加法扩展集合不符")
    layouts=[value["report"]["layouts"] for value in baselines.values()]
    require(layouts[0]==layouts[1]==layouts[2],"旧C布局改变")
    source=ROOT/"old-caller.c"
    used=set(re.findall(r"extern\s+\w+\s+(xiao_runtime_\w+)\(",source.read_text()))
    require(used<=set(old),"C调用方使用了非B1符号")
    task=Path(load(ROOT/"current/environment.json")["task_root"])
    obj=task/"old-caller.obj"
    require(not obj.exists(),"不能重编旧调用方对象")
    compile_argv=[r"D:\msys64\ucrt64\bin\clang.exe","-target","x86_64-pc-windows-msvc","-c",str(source),"-o",str(obj)]
    subprocess.run(compile_argv,check=True)
    obj_sha=sha(obj.read_bytes())
    rows=[]
    for name,baseline in baselines.items():
        executable=task/("old-caller-"+name+".exe")
        report=baseline["report"]
        argv=[r"D:\msys64\ucrt64\bin\clang.exe","-target","x86_64-pc-windows-msvc",str(obj),
              baseline["runtime"]["path"],"-o",str(executable),*report["native_static_libraries"]]
        subprocess.run(argv,check=True)
        run=subprocess.run([str(executable)],check=True,capture_output=True,encoding="utf-8")
        minor=19 if name=="current" else 18
        require(run.stdout==f"B5-CALLER result=42 runtime_minor={minor} layout=16/8/16\n","旧C调用方输出不符")
        require(sha(obj.read_bytes())==obj_sha,"旧调用方对象被改写")
        rows.append({"variant":name,"link_argv":argv,"caller_object_sha256":obj_sha,
          "runtime_sha256":baseline["runtime"]["sha256"],"executable_sha256":sha(executable.read_bytes()),
          "exit_code":run.returncode,"stdout":run.stdout,"stderr":run.stderr})
    report={"passed":True,"caller_source_sha256":sha(source.read_bytes()),"compile_argv":compile_argv,
      "caller_object":store(obj.read_bytes()),"baseline_kind":"reconstructed_from_B1_source",
      "historical_originals_available":False,"unchanged_b4_signatures":True,
      "preserved_historical_signatures":len(old),"added_signatures":added,"layouts":layouts[0],"runs":rows}
    write_json(ROOT/"c-abi.json",report)
    print(json.dumps(report,ensure_ascii=False))


if __name__=="__main__":main()
