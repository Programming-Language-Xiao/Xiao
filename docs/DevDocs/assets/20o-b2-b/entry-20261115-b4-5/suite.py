"""B4-5固定执行清单；每项独占回执，失败停止依赖链。"""
import argparse
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
PREFIX = "docs/DevDocs/assets/20o-b2-b/entry-20261115-b4-5"
CARGO = ["cargo", "+1.96.0", "test", "--manifest-path", "core/rust/Cargo.toml", "--locked"]
DRIVER = CARGO + ["-p", "xiao-driver", "--release"]
HISTORY = {"b0": "entry-20261107", "b1": "entry-20261108-b1", "b2": "entry-20261108-b2",
           "b3": "entry-20261108-b3", "b3-cabi": "entry-20261109-b3-cabi",
           "b3-wrapper": "entry-20261110-b3-wrapper-final", "b4-0-1": "entry-20261110-b4-0-1",
           "fix1": "entry-20261112-b4-2a-fix1", "b4-2b": "entry-20261113-b4-2b",
           "b4-3": "entry-20261113-b4-3", "b4-4": "entry-20261115-b4-4"}
COMMANDS = {
    "runtime-build": ["cargo", "+1.96.0", "build", "--manifest-path", "core/rust/Cargo.toml", "--locked", "-p", "xiao-runtime", "--release"],
    "workspace": ["cargo", "+1.96.0", "test", "--manifest-path", "core/rust/Cargo.toml", "--workspace", "--locked"],
    "clippy": ["cargo", "+1.96.0", "clippy", "--manifest-path", "core/rust/Cargo.toml", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings"],
    "fmt": ["cargo", "+1.96.0", "fmt", "--manifest-path", "core/rust/Cargo.toml", "--all", "--", "--check"],
    "lock": ["bun", "run", "check:lock"],
    "native-library": DRIVER + ["--test", "20o_b2_b4_cmath_library", "--", "--ignored", "--exact", "cmath_library_matches_native_o0_o3", "--nocapture"],
    "native-matrix": DRIVER + ["--test", "20o_b2_b4_cmath_llvm", "--", "--ignored", "--exact", "native_cmath_static_dynamic_o0_o3_and_shared_pow", "--nocapture"],
    "b1-vm": DRIVER + ["--test", "20o_b1_probe", "--", "--exact", "b1_capabilities_have_vm_golden_outputs", "--nocapture"],
    "b1-native": DRIVER + ["--test", "20o_b1_probe", "--", "--ignored", "--exact", "b1_capabilities_match_native_golden_outputs", "--nocapture"],
    **{name: ["python", "-B", "-X", "utf8", PREFIX + "/capture.py", name] for name in ["guards", "toolchain", "pow", "stages"]},
    **{"audit-" + name: ["python", "-B", "-X", "utf8", "docs/DevDocs/assets/20o-b2-b/" + directory + "/audit.py"] for name, directory in HISTORY.items()},
    "final-docs": ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
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
