"""核对 B6-2 采样后的源码差异，并可用固定快照重建核心验证语言边界。"""

import argparse
from datetime import datetime, timezone
import difflib
import json
import os
from pathlib import Path
import re
import subprocess
import sys


ARCHIVE = Path(__file__).resolve().parents[1]
REPO = ARCHIVE.parents[4]
sys.dont_write_bytecode = True
sys.path.insert(0, str(REPO / "tools/gates"))

from b2_b6_macos import (
    command_output, file_identity, git, live_source, require, sha, snapshot, source_digest, write_json,
)
from b2_b6_performance import HOSTS


PRE_B6_2 = "8167154e105d924165c1e5c3a0401a37f85fc541"
MEASURED = "79ea0375d7f2d823a5bdde73cb2c9a1c33c3e4c3"
MOVE = "58176ff5299441f921f3b54fbdc300e2e46b9bb0"
REVIEWED = "1c4bb4a7b886c68f97c7df2f7a0ee37d94709322"
DRIVER = "core/rust/crates/xiao-driver"
VM = "core/rust/crates/xiao-vm/src/semantics"
DRIVER_LIB = f"{DRIVER}/src/lib.rs"
EXEC = f"{VM}/exec.rs"
MODULE_EXEC = f"{VM}/module_exec.rs"
EXAMPLES = (f"{DRIVER}/examples/b2_b6_performance.rs",
            f"{DRIVER}/examples/numeric_perf/b6_cases.rs")
CATALOG = "tests/fixtures/20o-b2-b6/catalog.json"
RUST_APIS = ("standard_library_module_loader", "with_module_loader")
LOADER = b"""    #[must_use]
    pub fn with_module_loader(mut self, loader: &'p dyn ModuleLoader) -> Self {
        self.set_module_loader(Some(loader));
        self
    }
"""
DRIVER_LOADER = b"""pub fn standard_library_module_loader() -> impl xiao_vm::ModuleLoader {
    module_loader::DriverModuleLoader::new(None, None)
}
"""


def blob(revision, path):
    return git("show", f"{revision}:{path}")


def changed(before, after):
    return sorted(path for path in before.keys() | after.keys() if before.get(path) != after.get(path))


def with_doc(source, block):
    require(source.count(block) == 1, "加载器实现与核对范围不符")
    previous_line = source.split(block, 1)[0].splitlines(keepends=True)[-1]
    require(previous_line.lstrip().startswith(b"/// "), "加载器文档位置不符")
    return previous_line + block


def docs_only(before, after):
    left, right = before.splitlines(keepends=True), after.splitlines(keepends=True)
    changes = difflib.SequenceMatcher(None, left, right, autojunk=False).get_opcodes()
    added = 0
    for tag, _, _, start, end in changes:
        if tag != "equal":
            require(tag == "insert" and all(line.lstrip().startswith(b"/// ") for line in right[start:end]),
                    "测量器差异包含 Rustdoc 新增以外的编辑")
            added += end - start
    require(added > 0, "测量器缺少已登记的 Rustdoc 新增")
    return added


def verify_source(revision):
    require(revision == "HEAD" or re.fullmatch(r"[0-9a-f]{7,40}", revision), "须使用 HEAD 或 Git 提交哈希")
    commit = git("rev-parse", "--verify", revision).decode().strip()
    require(git("cat-file", "-t", commit).strip() == b"commit", "须提供源码提交")
    measured, moved, current = (snapshot(ref) for ref in (MEASURED, MOVE, commit))
    require(current == moved, "受保护源码相对移位快照有新增差异，须重新复核")
    paths = changed(measured, current)
    require(set(paths) == {*EXAMPLES, EXEC, MODULE_EXEC}, "受测源码变更清单不符")
    files = {path: {"measured_sha256": measured[path], "reviewed_sha256": current[path]}
             for path in paths}
    for path in EXAMPLES:
        files[path]["added_rustdoc_lines"] = docs_only(blob(MEASURED, path), blob(commit, path))
    old = blob(MEASURED, EXEC)
    new = blob(commit, MODULE_EXEC)
    header = b"impl<'p, C: Carrier, S: VmEventSink> Vm<'p, C, S> {\n"
    require(old.replace(with_doc(old, LOADER) + b"\n", b"", 1) == blob(commit, EXEC)
            and new.replace(header + with_doc(new, LOADER) + b"}\n\n", b"", 1)
            == blob(MEASURED, MODULE_EXEC), "VM 加载器以外的实现发生变化")

    previous = snapshot(PRE_B6_2)
    product_delta = [path for path in changed(previous, current)
                     if not path.startswith(f"{DRIVER}/examples/") and path != CATALOG]
    require(set(product_delta) == {DRIVER_LIB, MODULE_EXEC}, "B6-2 产品边界发生变化")
    driver = blob(commit, DRIVER_LIB)
    require(driver.replace(with_doc(driver, DRIVER_LOADER) + b"\n", b"", 1)
            == blob(PRE_B6_2, DRIVER_LIB)
            and blob(PRE_B6_2, MODULE_EXEC) == blob(MEASURED, MODULE_EXEC)
            and git("diff", "--name-only", PRE_B6_2, commit, "--", "cli") == b"",
            "Rust API 之外的产品接线发生变化")
    require(measured[CATALOG] == current[CATALOG], "固定负载清单变化")
    for path in (".github/workflows/b2-b6-performance.yml", "tools/gates/b2_b6_performance.py",
                 "tools/gates/b2_b6_macos.py", "tools/gates/b2-b6-performance-baseline.json"):
        require(blob(MOVE, path) == blob(commit, path), f"采集工具发生变化：{path}")

    return {
        "passed": True,
        "before_b6_2": PRE_B6_2,
        "measured_source": MEASURED,
        "loader_move": MOVE,
        "reviewed_source": commit,
        "measured_source_sha256": source_digest(measured),
        "reviewed_source_sha256": source_digest(current),
        "catalog_sha256": current[CATALOG],
        "changed_sources": files,
        "loader_method_sha256": sha(LOADER),
        "product_delta_since_before_b6_2": product_delta,
        "rust_apis": RUST_APIS,
        "performance_semantics_preserved_at_source_level": True,
        "new_performance_samples": 0,
    }


def frame(value):
    payload = json.dumps(value).encode("utf-8")
    return len(payload).to_bytes(8, "big") + payload


def read_frames(data):
    frames = []
    while data:
        length = int.from_bytes(data[:8], "big")
        require(0 < length <= len(data) - 8, "核心响应帧不完整")
        frames.append(json.loads(data[8:8 + length]))
        data = data[8 + length:]
    return frames


def verify_language(core, source, task, triple, env):
    protocol = source / "tests/spec/11x0-protocol"
    hello = json.loads((protocol / "hello-request.json").read_bytes())
    template = json.loads((protocol / "localized-run-request.json").read_bytes())
    object_format = {"win32": "coff", "linux": "elf", "darwin": "macho"}[sys.platform]
    template["target"].update(triple=triple, object_format=object_format)
    cases = {f"{prefix}-abs": (f"from {prefix}.cmath import abs\nprint(abs(-2.0))\n", None, "2\n")
             for prefix in ("std", "stdlib")}
    for name in RUST_APIS:
        cases[f"bare-{name}"] = (f"{name}()\n", "X02-TYPE-001", name)
        for prefix in ("std", "stdlib"):
            cases[f"{prefix}-{name}"] = (f"from {prefix}.cmath import {name}\n", "X05-MODULE-005", name)
    requests = [dict(template, request_id=case, source={"module": "main",
                "path": str(task / (case + ".xiao")), "text": code})
                for case, (code, _, _) in cases.items()]
    wire = b"".join(map(frame, [hello, *requests]))
    result = subprocess.run([core], input=wire, capture_output=True, cwd=task, env=env, timeout=60, check=True)
    hello_result, *responses = read_frames(result.stdout)
    require(hello_result["accepted"] and not result.stderr
            and len(responses) == len(requests), "核心协商、错误输出或响应数量不符")
    by_request = {row["request_id"]: row for row in responses}
    require(set(by_request) == cases.keys(), "核心响应请求编号不符")
    rows = []
    for case, (code, expected, value) in cases.items():
        row = by_request[case]
        errors = [item for item in row["diagnostics"] if item["severity"] == "error"]
        output = "".join(event["data"]["text"] for event in row["events"] if event["kind"] == "intrinsic_output")
        if expected is None:
            require(row["exit_code"] == 0 and not errors and output == value, f"合法包装调用失败：{case}")
        else:
            require(row["exit_code"] == 1 and row["exit_name"] == "source_rejected" and not row["events"]
                    and len(errors) == 1 and errors[0]["code"] == expected
                    and value in errors[0]["message"], f"Rust API 语言边界不符：{case}")
        rows.append({"case": case, "xiao_source": code, "expected_diagnostic": expected,
                     "exit_code": row["exit_code"], "exit_name": row["exit_name"],
                     "errors": errors, "output": output})
    return {"cases": rows, "request_frames_sha256": sha(wire), "response_frames_sha256": sha(result.stdout)}


def rebuild_and_probe(report, task):
    _, triple, _, suffix = HOSTS[sys.platform]
    task = task.resolve()
    task.mkdir()
    source, target = task / "source", task / "target"
    source.mkdir()
    files = snapshot(report["reviewed_source"], source)
    require(live_source(source) == files and source_digest(files) == report["reviewed_source_sha256"],
            "重建任务的实际源码不符")
    env = {key: value for key, value in os.environ.items()
           if not key.startswith(("XIAO_", "RUST_BIGDECIMAL_", "CARGO_PROFILE_"))
           and key not in {"RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER",
                           "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_TARGET"}}
    rustc = Path(command_output(["rustup", "which", "--toolchain", "1.96.0", "rustc"], env))
    cargo = rustc.with_name("cargo" + suffix)
    rust_version = command_output([rustc, "-Vv"], env)
    require(rust_version.startswith("rustc 1.96.0") and f"host: {triple}" in rust_version, "Rust 或目标不符")
    for name in ("home", "temp"):
        (task / name).mkdir()
    env.update(RUSTC=str(rustc), RUSTUP_TOOLCHAIN="1.96.0", CARGO_TERM_COLOR="never",
               CARGO_TARGET_DIR=str(target), XIAO_STDLIB_ROOT=str(source / "stdlib"),
               XIAO_HOME=str(task / "home"), TMP=str(task / "temp"), TEMP=str(task / "temp"),
               TMPDIR=str(task / "temp"))
    argv = [str(cargo), "build", "--locked", "--release", "--manifest-path", "core/rust/Cargo.toml",
            "-p", "xiao-driver", "--bin", "xiao-core", "--example", "b2_b6_performance"]
    started = datetime.now(timezone.utc).isoformat()
    build_log = task / "build.log"
    with build_log.open("xb") as log:
        child = subprocess.Popen(argv, cwd=source, env=env, stdout=log, stderr=subprocess.STDOUT)
        code = child.wait()
    require(code == 0, f"固定快照构建失败，日志见 {build_log}")
    finished = datetime.now(timezone.utc).isoformat()
    core = target / "release" / ("xiao-core" + suffix)
    controller = target / "release/examples" / ("b2_b6_performance" + suffix)
    catalog = subprocess.check_output([controller, "--catalog"], cwd=task, env=env, timeout=60)
    require(json.loads(catalog) == json.loads((source / CATALOG).read_bytes()), "重建后生成的负载变化")
    language = verify_language(core, source, task, triple, env)
    require(live_source(source) == files, "重建或语言验证改写了固定源码")
    return {
        "target": triple, "task_root": str(task), "source_root": str(source),
        "stdlib": env["XIAO_STDLIB_ROOT"], "rust_version": rust_version,
        "rustc": file_identity(rustc), "cargo": file_identity(cargo),
        "build": {"argv": argv, "pid": child.pid, "exit_code": code, "started_utc": started,
                  "finished_utc": finished, "log": {**file_identity(build_log),
                  "text": build_log.read_bytes().decode("utf-8")}},
        "core": file_identity(core), "controller": file_identity(controller),
        "generated_catalog_sha256": sha(catalog), "generated_loads": len(json.loads(catalog)["loads"]),
        "language": language,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--revision", default=REVIEWED)
    parser.add_argument("--task", type=Path, help="为指定提交在新目录重建并执行语言验证")
    parser.add_argument("--report", type=Path, help="独占创建复核回执")
    options = parser.parse_args()
    report = verify_source(options.revision)
    if options.task:
        report["live"] = rebuild_and_probe(report, options.task)
    if options.report:
        write_json(options.report, report)
    print(json.dumps(report, ensure_ascii=False))


if __name__ == "__main__":
    main()
