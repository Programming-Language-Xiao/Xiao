"""分别编译真实21/22宿主，再测原始元数据与生产缓存API；不改历史头。"""
from datetime import datetime, timezone
import json
from pathlib import Path
import subprocess
import sys

sys.dont_write_bytecode = True
import record

ROOT, TASK, REPOSITORY = record.ROOT, record.TASK, record.REPOSITORY
OLD = REPOSITORY.parent / ".b5/url-timing-20261116-01/B/target/debug"
NEW = TASK / "target/debug"
BUILD = ROOT.parent / "entry-20261117-review/probe-build.json"
PROBE = ROOT / "version-probe.rs"


def invoke(label, argv):
    env, overrides, removed = record.environment()
    started = datetime.now(timezone.utc).isoformat()
    result = subprocess.Popen(argv, cwd=REPOSITORY, env=env, stdin=subprocess.DEVNULL,
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    stdout, stderr = result.communicate()
    receipt = {
        "label": label, "argv": argv, "pid": result.pid, "cwd": str(REPOSITORY),
        "started_utc": started, "finished_utc": datetime.now(timezone.utc).isoformat(),
        "exit_code": result.returncode, "environment_overrides": overrides,
        "stdout": record.store(stdout), "stderr": record.store(stderr),
    }
    record.write_new(ROOT / (label + ".json"), receipt)
    record.require(result.returncode == 0,
                   "版本探针失败：" + label + chr(10) + stderr.decode("utf-8", errors="replace"))
    return json.loads(stdout) if stdout.strip() else None


def build(label, debug, driver_path):
    dependencies = debug / "deps"
    fingerprints = debug / ".fingerprint"
    suffix = driver_path.stem.removeprefix("libxiao_driver-")
    meta_path = fingerprints / ("xiao-driver-" + suffix) / "lib-xiao_driver.json"
    meta = json.loads(meta_path.read_text(encoding="utf-8"))
    required = {row[1]: row[3] for row in meta["deps"]}
    libraries = {"xiao_driver": driver_path}
    matched_fingerprints = {}
    for name in ("xiao_codegen_llvm", "xiao_runtime_abi", "xiao_artifacts", "serde_json"):
        matched = []
        for candidate in dependencies.glob("lib" + name + "-*.rlib"):
            key = candidate.stem.removeprefix("lib" + name + "-")
            for prefix in {name, name.replace("_", "-")}:
                fingerprint = fingerprints / (prefix + "-" + key) / ("lib-" + name)
                if fingerprint.is_file():
                    observed = int.from_bytes(bytes.fromhex(fingerprint.read_text().strip()), "little")
                    if observed == required[name]:
                        matched.append((candidate, fingerprint))
        record.require(len(matched) == 1, "版本宿主依赖不能唯一匹配：" + name)
        libraries[name] = matched[0][0]
        matched_fingerprints[name] = record.identity(matched[0][1])
    _, overrides, _ = record.environment()
    compiler = Path(overrides["RUSTC"])
    executable = TASK / (label + ".exe")
    record.require(not executable.exists(), "版本宿主执行文件已存在")
    argv = [str(compiler), "--edition=2024", "--crate-name", label.replace("-", "_"),
            str(PROBE), "-L", "dependency=" + str(dependencies), "-o", str(executable)]
    for output in sorted((debug / "build").glob("*/output")):
        for line in output.read_text(encoding="utf-8").splitlines():
            if line.startswith("cargo:rustc-link-search="):
                argv.extend(["-L", line.split("=", 1)[1]])
    for name, path in libraries.items():
        argv.extend(["--extern", name + "=" + str(path)])
    before = {name: record.identity(path) for name, path in libraries.items()}
    invoke("build-" + label, argv)
    after = {name: record.identity(path) for name, path in libraries.items()}
    record.require(before == after, "版本宿主编译期间借用库变化")
    record.write_new(ROOT / (label + "-identity.json"), {
        "executable": record.identity(executable), "probe": record.identity(PROBE),
        "compiler": record.identity(compiler), "driver_fingerprint": record.identity(meta_path),
        "selected_dependency_fingerprints": matched_fingerprints,
        "libraries_before": before, "libraries_after": after,
        "borrowed_old_build": label == "generator21",
        "old_source_build_receipt": record.identity(BUILD) if label == "generator21" else None,
    })
    return executable


record.require(not (ROOT / "version-matrix.json").exists(), "版本矩阵禁止覆盖")
old_build = json.loads(BUILD.read_text(encoding="utf-8"))
old_driver = Path(old_build["libraries_after"]["xiao_driver"]["path"])
record.require(record.identity(old_driver) == old_build["libraries_after"]["xiao_driver"],
               "已登记旧驱动库身份变化")
drivers = list((NEW / "deps").glob("libxiao_driver-*.rlib"))
record.require(len(drivers) == 1, "新驱动库不唯一")
old = build("generator21", OLD, old_driver)
new = build("generator22", NEW, drivers[0])
metadata = {}
for label, executable in (("21", old), ("22", new)):
    path = TASK / ("generator" + label + ".meta")
    metadata[label] = invoke("metadata" + label, [str(executable), "metadata", str(path)])
    record.require(metadata[label]["actual_codegen"] == int(label), "宿主实际生成器错误")
validation = []
for reader, executable in (("21", old), ("22", new)):
    for writer in ("21", "22"):
        row = invoke("read" + reader + "-artifact" + writer,
                     [str(executable), "validate", metadata[writer]["path"]])
        record.require(row["actual_validator"] == int(reader)
                       and row["artifact_generator"] == int(writer)
                       and row["accepted"] == (int(writer) <= int(reader)),
                       "真实新旧元数据矩阵错误")
        if not row["accepted"]:
            record.require(row["code"] == "X17-XAR-007" and not row["entry_executed"],
                           "旧校验器没有按版本要求拒绝")
        validation.append(row)
cache_rows = []
for mode in ("static", "dynamic"):
    for level in ("0", "3"):
        cache = TASK / ("cache-" + mode + "-o" + level)
        record.require(not cache.exists(), "缓存矩阵必须使用新目录")
        rows = []
        for label, executable in (("21", old), ("22", new)):
            row = invoke("cache-" + mode + "-o" + level + "-g" + label,
                         [str(executable), "cache", str(cache), mode, level])
            rows.append(row)
            record.require(not row["cache_hit"] and row["regenerated"], "旧缓存未失配重生成")
            index = cache / "global.index.pb"
            record.write_new(ROOT / ("index-" + mode + "-o" + level + "-g" + label + ".json"), {
                "index": record.store(index.read_bytes()),
                "executable": record.identity(Path(row["executable"])),
            })
        first, second = rows
        for key in ("input_sha256", "target", "optimization", "optimization_plan",
                    "runtime_abi", "runtime_library", "versions", "native_libraries"):
            record.require(first[key] == second[key], "隔离生成器矩阵的共同条件不同：" + key)
        record.require(first["other_generator_fingerprint"] == second["fingerprint"]
                       and second["other_generator_fingerprint"] == first["fingerprint"],
                       "变化不止生成器指纹")
        record.require(first["actual_codegen"] == 21 and second["actual_codegen"] == 22
                       and first["fingerprint"] != second["fingerprint"]
                       and first["request_key"] != second["request_key"]
                       and first["previous_codegen"] == []
                       and second["previous_codegen"] == [21],
                       "没有读取真实21缓存后以22失配")
        # 再读22，证明生产索引确实能命中，不把永远miss的探针当成有效矩阵。
        hit = invoke("cache-" + mode + "-o" + level + "-g22-hit",
                     [str(new), "cache", str(cache), mode, level])
        record.require(hit["cache_hit"] and not hit["regenerated"]
                       and hit["sha256"] == second["sha256"], "22缓存控制命中失败")
        cache_rows.extend([*rows, hit])
record.write_new(ROOT / "version-matrix.json", {
    "passed": True, "metadata": metadata, "validation": validation, "cache": cache_rows,
    "metadata_objects": {label: record.store(Path(row["path"]).read_bytes())
                         for label, row in metadata.items()},
    "scope": "real generator21/22 and production cache APIs; explicit probe, not CLI cache",
})
print(json.dumps({"passed": True, "metadata_cells": len(validation),
                  "cache_miss_regeneration_rows": 8, "cache_control_hits": 4}, ensure_ascii=False))
