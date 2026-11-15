"""FIX.1固定命令与顺序：归因、路由、功能、完整门禁分开采集。"""
import argparse
from pathlib import Path
import subprocess
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
PREFIX = "docs/DevDocs/assets/20o-b2-b/entry-20261115-b4-3-fix1"
CARGO = ["cargo", "+1.96.0", "test", "--locked", "--manifest-path", "core/rust/Cargo.toml"]
EXACT = CARGO + ["-p", "xiao-driver", "--test", "n0_b_dynamic_native", "--", "--exact",
                 "generic_value_copy_registers_weak_dependency_only_when_called", "--nocapture"]
COMMANDS = {
    "runtime-build": ["cargo", "+1.96.0", "build", "--locked", "--manifest-path", "core/rust/Cargo.toml", "-p", "xiao-runtime", "--release"],
    "routing": CARGO + ["-p", "xiao-driver", "--test", "20o_b2_b4_numeric_range_routing", "--", "--nocapture"],
    "codegen-tests": CARGO + ["-p", "xiao-codegen-llvm"],
    "exact": EXACT,
    "regressions": CARGO + ["-p", "xiao-driver"] + sum((["--test", n] for n in [
        "n0_b_dynamic_native", "20o_b2_b4_numeric_range_routing", "20o_b2_b4_cmath_llvm", "20o_b2_b4_cmath_library",
        "20o_b2_b4_cmath_boundary", "20o_b2_b3_wrapper", "22c_vm_concurrency"]), []) + ["--", "--nocapture"],
    "native-matrix": CARGO + ["-p", "xiao-driver", "--release", "--test", "20o_b2_b4_cmath_llvm", "--", "--ignored", "--exact", "native_cmath_static_dynamic_o0_o3_and_shared_pow", "--nocapture"],
    "native-library": CARGO + ["-p", "xiao-driver", "--release", "--test", "20o_b2_b4_cmath_library", "--", "--ignored", "--exact", "cmath_library_matches_native_o0_o3", "--nocapture"],
    "b1-vm": CARGO + ["-p", "xiao-driver", "--release", "--test", "20o_b1_probe", "--", "--exact", "b1_capabilities_have_vm_golden_outputs", "--nocapture"],
    "b1-native": CARGO + ["-p", "xiao-driver", "--release", "--test", "20o_b1_probe", "--", "--ignored", "--exact", "b1_capabilities_match_native_golden_outputs", "--nocapture"],
    "workspace": CARGO + ["--workspace"],
    "clippy": ["cargo", "+1.96.0", "clippy", "--locked", "--manifest-path", "core/rust/Cargo.toml", "--workspace", "--all-targets", "--", "-D", "warnings"],
    "fmt": ["cargo", "+1.96.0", "fmt", "--manifest-path", "core/rust/Cargo.toml", "--all", "--", "--check"],
    "lock": ["bun", "run", "check:lock"],
    "docs": ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    **{"capture-" + label: ["python", "-B", "-X", "utf8", "docs/DevDocs/assets/20o-b2-b/entry-20261115-b4-3-fix1/capture.py", label]
       for label in ["toolchain", "stages", "guards", "pow"]},
    **{"audit-" + label: ["python", "-B", "-X", "utf8", "docs/DevDocs/assets/20o-b2-b/" + directory + "/audit.py"]
       for label, directory in {"b4-3": "entry-20261113-b4-3", "b4-4": "entry-20261115-b4-4", "b4-5": "entry-20261115-b4-5"}.items()},
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("variant")
    parser.add_argument("labels", nargs="+", choices=list(COMMANDS))
    args = parser.parse_args()
    for label in args.labels:
        result = subprocess.run([sys.executable, "-B", "-X", "utf8", str(ROOT / "record.py"), "run", args.variant, label, "--", *COMMANDS[label]])
        expected = 101 if args.variant == "introduced" and label == "exact" else 0
        if result.returncode != expected:
            raise SystemExit(result.returncode or 1)


if __name__ == "__main__":
    main()
