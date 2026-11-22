"""用 B2-C 固定产品和独立 Runtime 采集 B3-0A 的宿主观察。"""

import argparse
from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
B7 = ROOT.parents[1] / "20o-b2-b/entry-20261122-b7/run.py"
spec = spec_from_file_location("b7_runner", B7)
base = module_from_spec(spec)
spec.loader.exec_module(base)
MODULES = ("time", "argv", "cli", "fs", "env", "log")
CLI_PROBE = """
import {parseArguments} from './cli/ts/src/commands/parser.ts';
const inputs = [
    ['run', 'probe.xiao'],
    ['run', 'probe.xiao', '--', '', 'two words'],
    ['run', 'probe.xiao', '--', '--help'],
    ['-xar', 'probe.xar', '--', '--flag'],
];
for (const argv of inputs) {
    try {
        const command = parseArguments(argv);
        console.log(JSON.stringify({argv, kind: command.kind, publicKeys: Object.keys(command).sort()}));
    } catch (error) {
        console.log(JSON.stringify({argv, code: error.code}));
    }
}
"""


def collect(task, llvm, archive_path=None, manifest_path=None):
    base.require(bool(archive_path) == bool(manifest_path), "源码归档与 Git 字节清单须配套")
    source, manifest, archive = base.exported(task, archive_path, manifest_path)
    product = base.source_identity(source, manifest)
    probe = ROOT / "host_probe.rs"
    identities = {"collector": base.identity(Path(__file__)), "probe": base.identity(probe), "source": product}
    runtime, environment = base.setup(task, source, identities, llvm, archive)
    host = identities["host"]
    evidence = task / "evidence"
    base.write_json(evidence / "environment.json", identities)
    tools = identities["tools"]
    cargo = tools["cargo"]["path"]
    rustc = tools["rustc"]["path"]
    executable = task / ("host-probe.exe" if host == "windows" else "host-probe")
    initial = {
        "runtime": [cargo, "build", *base.LOCKED, "--release", "-p", "xiao-runtime"],
        "probe-build": [rustc, "--edition", "2024", str(probe), "-o", str(executable)],
    }
    results = {}
    for label, command in initial.items():
        results[label] = base.capture(task, label, command, environment, runtime, manifest)
        if results[label] != 0:
            base.write_json(evidence / "results.json", {"exit": results})
            return results[label]
    binary = base.identity(executable)
    for label in MODULES:
        if label == "cli":
            command = [tools["bun"]["path"], "-e", CLI_PROBE]
        else:
            command = [executable, label, *([task / "tmp/fs/work"] if label == "fs" else [])]
        results[label] = base.capture(task, label, command, environment, runtime, manifest)
    for label, command in base.commands(cargo, "b1", host):
        results[label] = base.capture(task, label, command, environment, runtime, manifest)
    base.require(base.identity(executable) == binary, "宿主探测程序执行前后字节不符")
    base.write_json(evidence / "results.json", {"exit": results, "probe_executable": binary})
    return int(any(results.values()))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("task", type=Path)
    parser.add_argument("--llvm", type=Path, required=True)
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--manifest", type=Path)
    args = parser.parse_args()
    raise SystemExit(collect(args.task.resolve(), args.llvm.resolve(strict=True), args.archive, args.manifest))
