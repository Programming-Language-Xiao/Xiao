"""按固定Cargo库及其依赖指纹编译B5宿主，不改变旧源码。"""
import argparse
from pathlib import Path
import subprocess
import sys
sys.dont_write_bytecode = True
from record import ROOT, load, sha, write_json, require


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("variant")
    parser.add_argument("--check",action="store_true")
    args=parser.parse_args()
    environment=load(ROOT/args.variant/"environment.json")
    task=Path(environment["task_root"])
    dependencies=task/"target/release/deps"
    fingerprints=task/"target/release/.fingerprint"
    driver=list(dependencies.glob("libxiao_driver-*.rlib"))
    require(len(driver)==1,"B5固定构建应仅含一个驱动器库")
    suffix=driver[0].stem.removeprefix("libxiao_driver-")
    meta=load(fingerprints/("xiao-driver-"+suffix)/"lib-xiao_driver.json")
    deps={row[1]:row[3] for row in meta["deps"]}
    libraries={"xiao_driver":driver[0]}
    for name in ["xiao_bytecode","xiao_codegen_llvm","xiao_runtime","xiao_runtime_abi","xiao_artifacts","xiao_xar","serde_json"]:
        matched=[]
        for candidate in dependencies.glob("lib"+name+"-*.rlib"):
            key=candidate.stem.removeprefix("lib"+name+"-")
            candidates=[fingerprints/(prefix+"-"+key)/("lib-"+name) for prefix in {name,name.replace("_","-")}]
            fp=next((path for path in candidates if path.is_file()),None)
            if fp is not None and int.from_bytes(bytes.fromhex(fp.read_text().strip()),"little")==deps[name]:
                matched.append(candidate)
        require(len(matched)==1,"B5宿主依赖指纹不能唯一匹配："+name)
        libraries[name]=matched[0]
    executable=task/("b5-host-check.rmeta" if args.check else "b5-host.exe")
    require(args.check or not executable.exists(),"B5禁止覆盖验证器宿主")
    argv=["rustc","+1.96.0","--edition=2024","--crate-name","b5_host",str(ROOT/"host.rs"),
          "-L","dependency="+str(dependencies),"-o",str(executable)]
    if args.check:
        argv.append("--emit=metadata")
    for output in sorted((task/"target/release/build").glob("*/output")):
        for line in output.read_text(encoding="utf-8").splitlines():
            if line.startswith("cargo:rustc-link-search="):
                argv.extend(["-L",line.split("=",1)[1]])
    for name,path in libraries.items():
        argv.extend(["--extern",name+"="+str(path)])
    subprocess.run(argv,check=True)
    if args.check:
        print("B5宿主元数据编译通过")
        return
    write_json(ROOT/args.variant/"host-identity.json",{"argv":argv,"host_source_sha256":sha((ROOT/"host.rs").read_bytes()),
       "executable":str(executable),"executable_sha256":sha(executable.read_bytes()),
       "libraries":{name:{"path":str(path),"sha256":sha(path.read_bytes())} for name,path in libraries.items()}})
    print("B5验证器宿主编译成功",executable)


if __name__=="__main__":main()
