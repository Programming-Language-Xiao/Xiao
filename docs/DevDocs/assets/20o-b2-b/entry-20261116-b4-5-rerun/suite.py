"""B4-5重开命令清单；回执只追加，失败即停止后续执行。"""
import argparse
from pathlib import Path
import subprocess
import sys

sys.dont_write_bytecode = True
PREFIX = "docs/DevDocs/assets/20o-b2-b/entry-20261116-b4-5-rerun"
SOURCE_COMMIT = "6e172c1ecf8b3c4f58cf0c8c18b0aa3ab8b91012"
PRODUCT_COMMIT = "cfa79381bbae027cec11136ca380e82f66720c34"
CARGO = ["cargo", "+1.96.0", "test", "--locked", "--manifest-path", "core/rust/Cargo.toml"]
DRIVER = CARGO + ["-p", "xiao-driver", "--release"]
HISTORY = {
    "b0": "entry-20261107", "b1": "entry-20261108-b1", "b2": "entry-20261108-b2",
    "b3": "entry-20261108-b3", "b3-cabi": "entry-20261109-b3-cabi",
    "b3-wrapper": "entry-20261110-b3-wrapper-final", "b4-0-1": "entry-20261110-b4-0-1",
    "b4-2a-fix1": "entry-20261112-b4-2a-fix1", "b4-2b": "entry-20261113-b4-2b",
    "b4-3": "entry-20261113-b4-3", "b4-4": "entry-20261115-b4-4",
    "b4-3-fix1": "entry-20261115-b4-3-fix1", "b4-5-blocked": "entry-20261115-b4-5",
}
TARGETS = [
    "20o_b2_b4_numeric_range_routing", "n0_b_dynamic_native", "20o_b2_b4_frontend",
    "20o_b2_b4_cmath_vm", "20o_b2_b4_cmath_llvm",
    "20o_b2_b4_cmath_library", "20o_b2_b4_cmath_boundary", "20o_b2_b3_wrapper", "22c_vm_concurrency",
]
COMMANDS = {
    "runtime-build": ["cargo", "+1.96.0", "build", "--locked", "--manifest-path", "core/rust/Cargo.toml", "-p", "xiao-runtime", "--release"],
    "toolchain": ["python", "-B", "-X", "utf8", PREFIX + "/capture.py", "toolchain"],
    "stages": ["python", "-B", "-X", "utf8", PREFIX + "/capture.py", "stages"],
    "guards": ["python", "-B", "-X", "utf8", PREFIX + "/capture.py", "guards"],
    "regressions": DRIVER + sum((["--test", target] for target in TARGETS), []) + ["--", "--nocapture"],
    "tac": CARGO + ["-p", "xiao-bytecode", "--release", "--test", "b4_cmath_tac", "--", "--nocapture"],
    "workspace": CARGO + ["--workspace"],
    "clippy": ["cargo", "+1.96.0", "clippy", "--locked", "--manifest-path", "core/rust/Cargo.toml", "--workspace", "--all-targets", "--", "-D", "warnings"],
    "fmt": ["cargo", "+1.96.0", "fmt", "--manifest-path", "core/rust/Cargo.toml", "--all", "--", "--check"],
    "lock": ["bun", "run", "check:lock"],
    "native-matrix": DRIVER + ["--test", "20o_b2_b4_cmath_llvm", "--", "--ignored", "--exact", "native_cmath_static_dynamic_o0_o3_and_shared_pow", "--nocapture"],
    "native-library": DRIVER + ["--test", "20o_b2_b4_cmath_library", "--", "--ignored", "--exact", "cmath_library_matches_native_o0_o3", "--nocapture"],
    "b1-vm": DRIVER + ["--test", "20o_b1_probe", "--", "--exact", "b1_capabilities_have_vm_golden_outputs", "--nocapture"],
    "b1-native": DRIVER + ["--test", "20o_b1_probe", "--", "--ignored", "--exact", "b1_capabilities_match_native_golden_outputs", "--nocapture"],
    "pow": ["python", "-B", "-X", "utf8", PREFIX + "/capture.py", "pow"],
    "pow-verified": ["python", "-B", "-X", "utf8", PREFIX + "/capture.py", "pow"],
    **{"audit-" + name: ["python", "-B", "-X", "utf8", "docs/DevDocs/assets/20o-b2-b/" + directory + "/audit.py"]
       for name, directory in HISTORY.items()},
    "docs-final": ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("labels", nargs="+", choices=list(COMMANDS))
    args = parser.parse_args()
    for label in args.labels:
        subprocess.run([sys.executable, "-B", "-X", "utf8", str(Path(__file__).with_name("record.py")),
                        "run", "final", label, "--", *COMMANDS[label]], check=True)


if __name__ == "__main__":
    main()
