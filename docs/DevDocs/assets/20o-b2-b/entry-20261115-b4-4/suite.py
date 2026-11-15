"""B4-4固定命令清单；每项由同一采集器独立记录。"""
import argparse
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
PREFIX = "docs/DevDocs/assets/20o-b2-b/entry-20261115-b4-4"
CARGO = ["cargo", "+1.96.0", "test", "--locked", "--manifest-path", "core/rust/Cargo.toml"]
DRIVER = CARGO + ["-p", "xiao-driver"]
HISTORY = {"b0": "entry-20261107", "b1": "entry-20261108-b1", "b2": "entry-20261108-b2",
           "b3": "entry-20261108-b3", "b3-cabi": "entry-20261109-b3-cabi",
           "b3-wrapper": "entry-20261110-b3-wrapper-final", "b4-0-1": "entry-20261110-b4-0-1",
           "fix1": "entry-20261112-b4-2a-fix1", "b4-2b": "entry-20261113-b4-2b",
           "b4-3": "entry-20261113-b4-3"}
REGRESSIONS = ["20o_b2_b4_cmath_library", "20o_b2_b4_cmath_llvm", "20o_b2_b4_cmath_vm",
               "20o_b2_b4_frontend", "20o_b2_b4_cmath_boundary", "20o_b2_b3_wrapper",
               "20o_b2_b2_divide", "20o_b2_a1_identity", "20o_b2_a_contract",
               "20o_b2_a_resources", "22c_vm_concurrency"]
COMMANDS = {
    "runtime-build": ["cargo", "+1.96.0", "build", "--locked", "--manifest-path", "core/rust/Cargo.toml", "-p", "xiao-runtime", "--release"],
    "native-library": DRIVER + ["--release", "--test", "20o_b2_b4_cmath_library", "--", "--ignored", "--exact", "cmath_library_matches_native_o0_o3", "--nocapture"],
    "regressions": DRIVER + sum((["--test", name] for name in REGRESSIONS), []) + ["--", "--nocapture"],
    "driver-unit": DRIVER + ["--lib", "--", "--nocapture"],
    "core-tests": CARGO + sum((["-p", name] for name in ["xiao-codegen-llvm", "xiao-runtime", "xiao-vm", "xiao-bytecode", "xiao-intrinsics", "xiao-xar"]), []) + ["--", "--nocapture"],
    "native-matrix": DRIVER + ["--release", "--test", "20o_b2_b4_cmath_llvm", "--", "--ignored", "--exact", "native_cmath_static_dynamic_o0_o3_and_shared_pow", "--nocapture"],
    "b1-vm": DRIVER + ["--release", "--test", "20o_b1_probe", "--", "--exact", "b1_capabilities_have_vm_golden_outputs", "--nocapture"],
    "b1-native": DRIVER + ["--release", "--test", "20o_b1_probe", "--", "--ignored", "--exact", "b1_capabilities_match_native_golden_outputs", "--nocapture"],
    "clippy": ["cargo", "+1.96.0", "clippy", "--manifest-path", "core/rust/Cargo.toml", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings"],
    "fmt": ["cargo", "+1.96.0", "fmt", "--manifest-path", "core/rust/Cargo.toml", "--all", "--", "--check"],
    "toolchain": ["python", "-B", "-X", "utf8", PREFIX + "/capture.py", "toolchain"],
    "artifacts": ["python", "-B", "-X", "utf8", PREFIX + "/capture.py", "artifacts"],
    "collector": ["python", "-B", "-X", "utf8", PREFIX + "/capture.py", "self-test"],
    **{"audit-" + name: ["python", "-B", "-X", "utf8", "docs/DevDocs/assets/20o-b2-b/" + directory + "/audit.py"] for name, directory in HISTORY.items()},
    "final-docs": ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("labels", nargs="+", choices=list(COMMANDS))
    args = parser.parse_args()
    for label in args.labels:
        subprocess.run([sys.executable, "-B", "-X", "utf8", str(Path(__file__).with_name("record.py")),
                        "run", "fixed-verified", label, "--", *COMMANDS[label]], check=True)


if __name__ == "__main__":
    main()
