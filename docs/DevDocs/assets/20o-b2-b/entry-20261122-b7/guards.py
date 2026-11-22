"""使用当期四条 B1 实际输出，复跑固定工作流计数守卫。"""

import argparse
from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
import re
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT.with_name("entry-20261119-b6-4")))
from evidence import MAINTENANCE, audit_guards, guard_bodies, run_guards, step

spec = spec_from_file_location("b7_run", ROOT / "run.py")
b7_run = module_from_spec(spec)
spec.loader.exec_module(b7_run)
PRODUCT = b7_run.PRODUCT
git, require, sha, write_json = b7_run.git, b7_run.require, b7_run.sha, b7_run.write_json


def fixed_bodies():
    # 补验工作流附加了观测输出，四个计数判断必须与 B7 产品的原始工作流相同。
    workflow = git("show", f"{PRODUCT}:{MAINTENANCE}").decode("utf-8")
    bodies = guard_bodies()
    require(bodies["default"] == step(workflow, "Run B1 VM probe exactly once"), "B7 VM 计数判断有差异")
    windows = step(workflow, "Build runtime and run native gates")
    require(bodies["windows"] == "$ErrorActionPreference = 'Stop'\n"
            + re.search(r"(?ms)^\$b1Output = cargo test.*?^\}", windows).group() + "\n", "B7 Windows 计数判断有差异")
    for platform in ("Linux", "macOS"):
        native = step(workflow, f"Build runtime and run {platform} native gates")
        helper = re.search(r"(?ms)^run_exact_native_test\(\) \{.*?^\}", native).group()
        call = "run_exact_native_test b1_capabilities_match_native_golden_outputs 20o_b1_probe"
        require(call in native.splitlines() and bodies[platform.lower()] == f"set -euo pipefail\n{helper}\n{call}\n",
                f"B7 {platform} 计数判断有差异")
    return bodies


SOURCES = {
    "default": "linux/b1-vm.log",
    "windows": "windows/b1-native.log",
    "linux": "linux/b1-native.log",
    "macos": "macos-b1/b1-native.log",
}


def verify(bundle):
    sources = {name: (bundle.raw(path), {"path": path, "cargo_output": {"sha256": sha(bundle.raw(path))}})
               for name, path in SOURCES.items()}
    summary = audit_guards(bundle, sources, fixed_bodies())
    origin = bundle.read("guards/source.json")
    require(origin == {"product": PRODUCT, "workflow_sha256": sha(git("show", f"{PRODUCT}:{MAINTENANCE}")),
                       "script_sha256": sha(git("show", f"{PRODUCT}:tools/gates/b1-probe-guards.ps1")),
                       "sources": SOURCES}, "B7 守卫固定来源不符")
    return {key: summary[key] for key in ("passed", "actual", "derived")}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bash", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    for name in SOURCES:
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    run_guards(args.output.resolve(), args.bash.resolve(strict=True),
               {name: (getattr(args, name).read_bytes(), path) for name, path in SOURCES.items()}, fixed_bodies())
    write_json(args.output / "source.json", {
        "product": PRODUCT, "workflow_sha256": sha(git("show", f"{PRODUCT}:{MAINTENANCE}")),
        "script_sha256": sha(git("show", f"{PRODUCT}:tools/gates/b1-probe-guards.ps1")), "sources": SOURCES,
    })
