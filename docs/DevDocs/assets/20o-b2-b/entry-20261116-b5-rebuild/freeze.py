"""冻结旧/新基线产物和工具、C接口声明；不称为历史原件。"""
import argparse
from pathlib import Path
import re
import subprocess
import sys
sys.dont_write_bytecode=True
from record import ROOT, load, write_json, store, sha, canonical, git, REPOSITORY


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("variant")
    args=parser.parse_args()
    directory=ROOT/args.variant
    source=load(directory/"source.json")
    config=load(directory/"environment.json")
    task=Path(config["task_root"])
    output=task/"artifacts/baseline"
    artifacts={name:store((output/name).read_bytes()) for name in ["main.xiao","main.xiaoc","main.xar","main.meta","main.exe","report.json"]}
    commands={"rustc":["rustc","+1.96.0","-Vv"],"cargo":["cargo","+1.96.0","-V"],
      **{name:[config["overrides"][key],"--version"] for name,key in [("clang","XIAO_CLANG"),("llvm-as","XIAO_LLVM_AS"),("llc","XIAO_LLC")]}}
    versions={}
    for name,argv in commands.items():
        run=subprocess.run(argv,check=True,capture_output=True,encoding="utf-8")
        binary=Path(subprocess.check_output(["cmd","/c","where",argv[0]],text=True).splitlines()[0]) if not Path(argv[0]).is_absolute() else Path(argv[0])
        if name=="rustc":
            binary=Path(subprocess.check_output(["rustup","which","--toolchain","1.96.0","rustc"],text=True).strip())
        versions[name]={"argv":argv,"stdout":run.stdout,"stderr":run.stderr,"path":str(binary),"sha256":sha(binary.read_bytes())}
    abi=git(REPOSITORY,"show",source["revision"]+":core/rust/crates/xiao-runtime-abi/src/lib.rs").decode()
    declarations=re.findall(r"pub fn (xiao_runtime_\w+)\s*(\([^;]*?\)(?:\s*->\s*[^;]+)?);",abi,re.S)
    signatures={name:" ".join(signature.split()) for name,signature in declarations}
    nm=subprocess.run([r"D:\msys64\ucrt64\bin\llvm-nm.exe","--defined-only","--extern-only",config["overrides"]["XIAO_RUNTIME_LIBRARY"]],capture_output=True,check=True)
    symbols=sorted(set(re.findall(r"\b(xiao_runtime_\w+)\s*$",nm.stdout.decode("utf-8",errors="replace"),re.M)))
    report={"baseline_kind":"reconstructed_from_B1_source" if args.variant=="b1" else "internal_source_build",
      "historical_originals_available":False,"source_commit":source["revision"],"source_identity":source["sha256"],
      "locks":{name:source["files"][name] for name in ["core/rust/Cargo.lock","core/rust/research/Cargo.lock"] if name in source["files"]},
      "artifacts":artifacts,"tools":versions,"runtime":{"path":config["overrides"]["XIAO_RUNTIME_LIBRARY"],
        "sha256":sha(Path(config["overrides"]["XIAO_RUNTIME_LIBRARY"]).read_bytes())},
      "c_abi_signatures":signatures,"exported_c_symbols":symbols,"symbols_raw":store(nm.stdout,"text/plain"),
      "report":load(output/"report.json"),"host_identity":load(directory/"host-identity.json")}
    write_json(directory/"baseline.json",report)
    print("B5基线冻结",args.variant,len(artifacts),len(signatures),len(symbols))


if __name__=="__main__":main()
