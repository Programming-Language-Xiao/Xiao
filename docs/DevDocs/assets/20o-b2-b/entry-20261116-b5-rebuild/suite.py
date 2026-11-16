"""B5新1.19快照验收顺序；失败停止，不覆盖回执。"""
import argparse
from pathlib import Path
import subprocess
import sys
sys.dont_write_bytecode=True
PREFIX="docs/DevDocs/assets/20o-b2-b/entry-20261116-b5-rebuild"
CARGO=["cargo","+1.96.0","test","--locked","--manifest-path","core/rust/Cargo.toml"]
DRIVER=CARGO+["-p","xiao-driver","--release"]
COMMANDS={
 "regressions":DRIVER+sum((["--test",name] for name in [
   "20o_b2_b5_versions","compatibility_matrix","20o_b1_imp_contract","20o_b1_imp_resources",
   "20o_b2_b4_frontend","20o_b2_b4_cmath_vm","20o_b2_b4_cmath_llvm","20o_b2_b4_cmath_library",
   "20o_b2_b4_cmath_boundary","20o_b2_b3_wrapper","20o_b2_b4_numeric_range_routing",
   "n0_b_dynamic_native","22c_vm_concurrency"]),[])+["--","--nocapture"],
 "native-matrix":DRIVER+["--test","20o_b2_b4_cmath_llvm","--","--ignored","--exact","native_cmath_static_dynamic_o0_o3_and_shared_pow","--nocapture"],
 "native-library":DRIVER+["--test","20o_b2_b4_cmath_library","--","--ignored","--exact","cmath_library_matches_native_o0_o3","--nocapture"],
 "b1-vm":DRIVER+["--test","20o_b1_probe","--","--exact","b1_capabilities_have_vm_golden_outputs","--nocapture"],
 "b1-native":DRIVER+["--test","20o_b1_probe","--","--ignored","--exact","b1_capabilities_match_native_golden_outputs","--nocapture"],
 "guards":["python","-B","-X","utf8",PREFIX+"/guards.py"],
 "guards-verified":["python","-B","-X","utf8",PREFIX+"/guards.py"],
 "workspace":CARGO+["--workspace"],
 "workspace-complete":CARGO+["--workspace","--no-fail-fast"],
 "size":CARGO+["-p","xiao-driver","--test","artifact_size_regression","--","--nocapture"],
 "clippy":["cargo","+1.96.0","clippy","--locked","--manifest-path","core/rust/Cargo.toml","--workspace","--all-targets","--","-D","warnings"],
 "fmt":["cargo","+1.96.0","fmt","--manifest-path","core/rust/Cargo.toml","--all","--","--check"],
 "lock":["bun","run","check:lock"],
 "docs-final":["bun","tools/repo-check/src/cli.ts","all","--format","json"],
}


def main():
 parser=argparse.ArgumentParser(description=__doc__)
 parser.add_argument("--variant",default="current",choices=["current","final","final2","final3"])
 parser.add_argument("labels",nargs="+",choices=list(COMMANDS))
 args=parser.parse_args()
 for name in args.labels:
  subprocess.run([sys.executable,"-B","-X","utf8",str(Path(__file__).with_name("record.py")),
                  "run",args.variant,name,"--",*COMMANDS[name]],check=True)


if __name__=="__main__":main()
