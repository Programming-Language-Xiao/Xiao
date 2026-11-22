"""对账固定旧验证器与本次生成器 22 的真实容器、原生元数据及缓存。"""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess

from measure import ROOT, REPO, base

B5 = "docs/DevDocs/assets/20o-b2-b/entry-20261116-b5-rebuild"
FIX = "docs/DevDocs/assets/20o-b2-static-float-equality/entry-20261117-fix"
ANCHOR = "01107ea4df92108014eac4f3487292d8672a445a"


def blob(path):
    return base.git("show", f"{ANCHOR}:{path}")


def load(path):
    return json.loads(blob(path))


def compile_probe(task, settings, env, output, source, variants, identity_document):
    rustc = settings["tools"]["rustc"]["path"]
    deps = task / "target/release/deps"
    driver, = deps.glob("libxiao_driver-*.rlib")
    fingerprints = task / "target/release/.fingerprint"
    driver_key = driver.stem.removeprefix("libxiao_driver-")
    meta = json.loads((fingerprints / f"xiao-driver-{driver_key}" / "lib-xiao_driver.json").read_bytes())
    links = {row[1]: row[3] for row in meta["deps"]}
    files = {"xiao_driver": driver}
    for name in variants:
        expected = links[name]
        candidates = []
        for item in deps.glob(f"lib{name}-*.rlib"):
            key = item.stem.removeprefix(f"lib{name}-")
            for package in (name.replace("_", "-"), name):
                token = fingerprints / f"{package}-{key}" / f"lib-{name}"
                if token.is_file() and int.from_bytes(bytes.fromhex(token.read_text().strip()), "little") == expected:
                    candidates.append(item)
                    break
        base.require(len(candidates) == 1, f"Cargo 依赖指纹不能唯一匹配：{name}")
        files[name] = candidates[0]
    probe = output / source.split("/")[-1]
    probe.write_bytes(blob(source))
    exe = probe.with_suffix(".exe")
    cmd = [rustc, "--edition=2024", str(probe), "-L", f"dependency={deps}", "-o", str(exe)]
    native_paths = set()
    for path in (task / "target/release/build").glob("*/output"):
        for line in path.read_text(encoding="utf-8").splitlines():
            if line.startswith(("cargo:rustc-link-search=", "cargo::rustc-link-search=")):
                native_paths.add(line.split("=", 1)[1])
    for link in sorted(native_paths):
        cmd.extend(("-L", link))
    for name, path in files.items():
        cmd.extend(("--extern", f"{name}={path}"))
    before = {name: base.identity(path) for name, path in files.items()}
    step(output, env, f"build-{exe.stem}", cmd, expect_json=False)
    base.require(before == {name: base.identity(path) for name, path in files.items()}, "验证器链接库变化")
    base.write_json(output / identity_document, {"source": base.identity(probe), "executable": base.identity(exe),
                     "libraries": {name: {"path": str(path), **before[name]} for name, path in files.items()},
                     "driver_fingerprint": base.identity(fingerprints / f"xiao-driver-{driver_key}" / "lib-xiao_driver.json")})
    return exe


def step(directory, env, name, argv, *, expect_json=True):
    argv = list(map(str, argv))
    start = base.utc()
    with subprocess.Popen(argv, cwd=directory, env=env, stdin=subprocess.DEVNULL,
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE) as child:
        stdout, stderr = child.communicate()
    for suffix, data in (("stdout", stdout), ("stderr", stderr)):
        (directory / f"{name}.{suffix}").write_bytes(data)
    base.write_json(directory / f"{name}.receipt.json", {"argv": argv, "pid": child.pid,
                    "started_utc": start, "finished_utc": base.utc(), "exit": child.returncode,
                    "stdout_sha256": base.sha(stdout), "stderr_sha256": base.sha(stderr)})
    base.require(child.returncode == 0, f"版本探针失败：{name} {stderr.decode('utf-8', 'replace')}")
    return json.loads(stdout) if expect_json else None


def old_host(path, key):
    doc = load(path)
    if key == "executable":
        info = doc[key]
        file = Path(info["path"])
        base.require(base.identity(file) == {key: info[key] for key in ("sha256", "bytes")}, "旧生成器验证器字节不符")
    else:
        file = Path(doc["executable"])
        base.require(base.identity(file)["sha256"] == doc["executable_sha256"], "旧 ABI 验证器字节不符")
    return file


def verify(task, output):
    source = task / "source"
    manifest = (task / "product.tree").read_bytes()
    settings = json.loads((task / "evidence/environment.json").read_bytes())
    runtime = Path(settings["overrides"]["XIAO_RUNTIME_LIBRARY"])
    before_source = base.source_identity(source, manifest)
    before_runtime = base.identity(runtime)
    base.require(before_source == settings["source"], "当前验证器源码不符")
    env = {name: value for name, value in os.environ.items() if not name.startswith(("XIAO_", "RUST_BIGDECIMAL_"))}
    env.update(settings["overrides"])
    env["PATH"] = os.pathsep.join((str(Path(settings["tools"]["linker"]["path"]).parent),
                                  str(Path(settings["tools"]["rustc"]["path"]).parent), env["PATH"]))
    output.mkdir()
    env.update({key: str(output) for key in ("TMP", "TEMP", "TMPDIR", "XIAO_HOME")})
    probe = compile_probe(task, settings, env, output, f"{B5}/host.rs",
                          ("xiao_bytecode", "xiao_codegen_llvm", "xiao_runtime", "xiao_runtime_abi",
                           "xiao_artifacts", "xiao_xar", "serde_json", "xiao_vm"), "current-host.json")
    gen22 = compile_probe(task, settings, env, output, f"{FIX}/version-probe.rs",
                          ("xiao_codegen_llvm", "xiao_artifacts", "xiao_runtime_abi", "serde_json"), "generator22.json")
    gen21 = old_host(f"{FIX}/generator21-r2-identity.json", "executable")
    current = output / "generated"
    step(output, env, "generate", (probe, "generate", current))
    kinds = {"xiaoc": "xiaoc", "xar": "xar", "meta": "meta"}
    verdicts = {}
    for side in ("b1", "b4"):
        old = old_host(f"{B5}/{side}/host-identity.json", "executable_sha256")
        for kind, suffix in kinds.items():
            baseline = source / f"tests/fixtures/compatibility/b5/{side}-main.{suffix}"
            candidate = current / f"main.{suffix}"
            for label, verifier, artifact in ((f"{side}-to-current", probe, baseline),
                                              (f"current-to-{side}", old, candidate)):
                result = step(output, env, f"{label}-{kind}", (verifier, "validate", kind, artifact, "reject"))
                base.require(result["accepted"] is False and result["executed"] is False
                             and result["input_unchanged"] is True, f"版本拒绝不符：{label}/{kind}")
                verdicts[f"{label}/{kind}"] = result
    for kind, suffix in kinds.items():
        result = step(output, env, f"current-{kind}", (probe, "validate", kind, current / f"main.{suffix}", "accept"))
        verdicts[f"current/{kind}"] = result
    m21 = output / "generator21.meta"
    step(output, env, "generate21", (gen21, "metadata", m21))
    checks = [
        ("21-validates-22", gen21, current / "main.meta", False),
        ("22-validates-21", gen22, m21, True),
    ]
    for label, host, path, accepted in checks:
        result = step(output, env, label, (host, "validate", path))
        base.require(result["accepted"] is accepted and result["entry_executed"] is False, "21→22 验证器矩阵不符")
        verdicts[label] = result
    cached = {}
    for mode in ("static", "dynamic"):
        for level in ("0", "3"):
            store = output / f"cache-{mode}-{level}"
            store.mkdir()
            rows = [step(output, env, f"{mode}-o{level}-{phase}", (host, "cache", store, mode, level))
                    for phase, host in (("old", gen21), ("new", gen22), ("hit", gen22))]
            old, new, hit = rows
            base.require([row["actual_codegen"] for row in rows] == [21, 22, 22]
                         and [row["cache_hit"] for row in rows] == [False, False, True]
                         and new["sha256"] == hit["sha256"] and old["request_key"] != new["request_key"],
                         "生成器 21→22 缓存重生成不符")
            cached[f"{mode}-o{level}"] = rows
    base.require(before_source == base.source_identity(source, manifest) and before_runtime == base.identity(runtime),
                 "矩阵期间源码或 Runtime 变化")
    base.write_json(output / "result.json", {"source": before_source, "runtime": before_runtime,
                    "validators": verdicts, "cache": cached})


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("task", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    verify(args.task.resolve(strict=True), args.output.resolve())
