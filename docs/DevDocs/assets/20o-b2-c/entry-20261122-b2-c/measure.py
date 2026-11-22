"""从固定 B7 产品独立构建并采集 B2-C 的 F10 与 F-3 样本。"""

import argparse
from importlib.util import module_from_spec, spec_from_file_location
import json
import os
from pathlib import Path
import re
import subprocess
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[4]
B7 = ROOT.parent.parent / "20o-b2-b/entry-20261122-b7/run.py"
CATALOG = "tests/fixtures/20o-b2-b6/catalog.json"
B1_EXAMPLE = "b1_library_performance"
F3_EXAMPLE = "b2_b6_performance"
F10_LOG = re.compile(r"^B1-PERF-ARTIFACTS (.+)$", re.M)

spec = spec_from_file_location("b7_run", B7)
base = module_from_spec(spec)
spec.loader.exec_module(base)


def capture(task, label, argv, env, runtime, manifest):
    source = task / "source"
    logs = task / "evidence"
    temp = task / "tmp" / label
    temp.mkdir()
    home = task / "home" / label
    home.mkdir()
    effective = env | {"TMPDIR": str(temp), "TEMP": str(temp), "TMP": str(temp), "XIAO_HOME": str(home)}
    before = base.source_identity(source, manifest)
    prior = base.identity(runtime) if runtime.is_file() else None
    started = base.utc()
    log = logs / f"{label}.log"
    with log.open("xb") as out, subprocess.Popen(list(map(str, argv)), cwd=source, env=effective,
                                                 stdin=subprocess.DEVNULL, stdout=out, stderr=subprocess.STDOUT) as child:
        exit_code = child.wait()
    after = base.source_identity(source, manifest)
    runtime_after = base.identity(runtime) if runtime.is_file() else None
    base.write_json(logs / f"{label}.json", {
        "argv": list(map(str, argv)), "pid": child.pid, "exit": exit_code, "cwd": str(source),
        "temp": str(temp), "home": str(home), "started_utc": started, "finished_utc": base.utc(),
        "source_before": before, "source_after": after, "runtime_before": prior,
        "runtime_after": runtime_after, "log": base.identity(log),
    })
    base.require(before == after and (label == "runtime" or prior == runtime_after),
                 f"测量期间源码或 Runtime 变化：{label}")
    print(f"{label}: PID={child.pid} exit={exit_code}", flush=True)
    return exit_code


def f10_artifacts(log, task, backend):
    line = F10_LOG.findall(log.read_text(encoding="utf-8"))
    base.require(len(line) <= 1, "F10 产物根重复")
    if not line:
        return {}
    folder = Path(line[0]).resolve(strict=True)
    base.require(folder.parent == (task / "tmp" / f"f10-{backend}").resolve()
                 and folder.name.startswith("xiao-b1-performance-"), "F10 产物根不属于本轮")
    return {"directory": str(folder), "files": {p.name: base.identity(p) for p in sorted(folder.iterdir()) if p.is_file()}}


def measure(task, llvm, kind, backend, archive=None, manifest_file=None):
    base.require(bool(archive) == bool(manifest_file), "源码归档和清单必须配对")
    base.require(kind == "f3" and backend in ("vm", "o0", "o3")
                 or kind == "f10" and (backend == "native" or backend == "vm" and sys.platform == "linux"),
                 "测量后端不属于本批矩阵")
    source, manifest, archive_id = base.exported(task, archive, manifest_file)
    product = base.source_identity(source, manifest)
    settings = {"source": product, "kind": kind, "backend": backend}
    runtime, env = base.setup(task, source, settings, llvm, archive_id)
    example = B1_EXAMPLE if kind == "f10" else F3_EXAMPLE
    logs = task / "evidence"
    settings["measurement_catalog"] = base.identity(source / CATALOG)
    settings["stdlib"] = {path.name: base.identity(path) for path in sorted((source / "stdlib").glob("*.xiao"))}
    settings["recipe"] = base.identity(Path(__file__))
    base.write_json(logs / "environment.json", settings)
    cargo = settings["tools"]["cargo"]["path"]
    build = (cargo, "build", *base.LOCKED, "--release")
    for label, args in (("runtime", ("-p", "xiao-runtime")),
                        ("worker", ("-p", "xiao-driver", "--example", example))):
        status = capture(task, label, (*build, *args), env, runtime, manifest)
        if status:
            return status
    _, _, _, suffix = base.HOSTS[sys.platform]
    worker = task / "target/release/examples" / (example + suffix)
    worker_id = base.identity(worker)
    label = f"{kind}-{backend}"
    output = logs / ("report.json" if kind == "f10" else "samples")
    selection = ("--case", "all", "--phase", "all") if kind == "f10" else ("--suite", "all")
    status = capture(task, label, (worker, "--backend", backend, *selection, "--output", output),
                     env, runtime, manifest)
    base.require(base.identity(worker) == worker_id, "测量程序字节发生变化")
    observed = {"worker": {"path": str(worker), **worker_id}, "exit": status}
    if kind == "f10":
        observed["artifacts"] = f10_artifacts(logs / f"{label}.log", task, backend)
        if output.is_file():
            report = json.loads(output.read_bytes())
            observed["record_count"] = len(report["records"])
            observed["complete"] = report["complete"] is True
    else:
        report = output / "result.json"
        if report.is_file():
            summary = json.loads(report.read_bytes())
            observed["record_count"] = len(summary["records"])
            observed["complete"] = summary["complete"] is True
    base.write_json(logs / "measurement.json", observed)
    return status


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--task", type=Path, required=True)
    parser.add_argument("--llvm", type=Path, required=True)
    parser.add_argument("--kind", choices=("f10", "f3"), required=True)
    parser.add_argument("--backend", choices=("vm", "native", "o0", "o3"), required=True)
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--manifest", type=Path)
    args = parser.parse_args()
    return measure(args.task.resolve(), args.llvm, args.kind, args.backend, args.archive, args.manifest)


if __name__ == "__main__":
    raise SystemExit(main())
