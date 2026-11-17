"""以更新后的LLVM计划指纹重跑版本/生产缓存矩阵，输出均使用新身份。"""
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
OLD_BUILD = ROOT.parent / "entry-20261117-review/probe-build.json"
PROBE = ROOT / "version-probe.rs"


def invoke(label, argv):
    env, overrides, _ = record.environment()
    result = subprocess.Popen(argv, cwd=REPOSITORY, env=env, stdin=subprocess.DEVNULL,
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    stdout, stderr = result.communicate()
    record.write_new(ROOT / (label + ".json"), {
        "label": label, "argv": argv, "pid": result.pid, "cwd": str(REPOSITORY),
        "started_utc": datetime.now(timezone.utc).isoformat(),
        "finished_utc": datetime.now(timezone.utc).isoformat(),
        "exit_code": result.returncode, "environment_overrides": overrides,
        "stdout": record.store(stdout), "stderr": record.store(stderr),
    })
    record.require(result.returncode == 0,
                   "版本探针失败：" + label + chr(10) + stderr.decode("utf-8", errors="replace"))
    return json.loads(stdout) if stdout.strip() else None


def build(label, debug, driver_path, actual_version):
    dependencies, fingerprints = debug / "deps", debug / ".fingerprint"
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
        "actual_generator": actual_version,
        "borrowed_old_build": actual_version == 21,
        "old_source_build_receipt": record.identity(OLD_BUILD) if actual_version == 21 else None,
    })
    return executable


record.require(not (ROOT / "version-matrix-r2.json").exists(), "重跑矩阵禁止覆盖")
previous = json.loads((ROOT / "version-probes.json").read_text(encoding="utf-8"))
record.require(previous["exit_code"] == 1, "须保留初轮的实际指纹失败")
old_build = json.loads(OLD_BUILD.read_text(encoding="utf-8"))
old_driver = Path(old_build["libraries_after"]["xiao_driver"]["path"])
record.require(record.identity(old_driver) == old_build["libraries_after"]["xiao_driver"],
               "已登记旧驱动库身份变化")
new_drivers = list((NEW / "deps").glob("libxiao_driver-*.rlib"))
record.require(len(new_drivers) == 1, "新驱动库不唯一")
old = build("generator21-r2", OLD, old_driver, 21)
new = build("generator22-r2", NEW, new_drivers[0], 22)
metadata = {}
for version, executable in ((21, old), (22, new)):
    label = "metadata" + str(version) + "-r2"
    path = TASK / ("generator" + str(version) + "-r2.meta")
    metadata[str(version)] = invoke(label, [str(executable), "metadata", str(path)])
    record.require(metadata[str(version)]["actual_codegen"] == version,
                   "宿主实际生成器错误")
validation = []
for reader, executable in ((21, old), (22, new)):
    for writer in (21, 22):
        row = invoke("read" + str(reader) + "-artifact" + str(writer) + "-r2",
                     [str(executable), "validate", metadata[str(writer)]["path"]])
        record.require(row["actual_validator"] == reader
                       and row["artifact_generator"] == writer
                       and row["accepted"] == (writer <= reader),
                       "真实新旧元数据矩阵错误")
        if not row["accepted"]:
            record.require(row["code"] == "X17-XAR-007" and not row["entry_executed"],
                           "旧校验器未于执行前按生成器理解上限拒绝")
        validation.append(row)
cache_rows = []
for mode in ("static", "dynamic"):
    for level in ("0", "3"):
        cache = TASK / ("cache-" + mode + "-o" + level + "-r2")
        record.require(not cache.exists(), "缓存矩阵必须使用新目录")
        rows = []
        for version, executable in ((21, old), (22, new)):
            label = "cache-" + mode + "-o" + level + "-g" + str(version) + "-r2"
            row = invoke(label, [str(executable), "cache", str(cache), mode, level])
            rows.append(row)
            record.require(not row["cache_hit"] and row["regenerated"],
                           "生成器变化未触发失配重生成")
            index = cache / "global.index.pb"
            record.write_new(ROOT / ("index-" + mode + "-o" + level + "-g"
                                     + str(version) + "-r2.json"), {
                "index": record.store(index.read_bytes()),
                "executable": record.identity(Path(row["executable"])),
            })
        first, second = rows
        for key in ("input_sha256", "target", "optimization", "optimization_plan",
                    "runtime_abi", "runtime_library", "versions", "native_libraries"):
            record.require(first[key] == second[key],
                           "隔离生成器矩阵共同条件不同：" + key)
        record.require(first["actual_codegen"] == 21 and second["actual_codegen"] == 22
                       and first["other_generator_fingerprint"] == second["fingerprint"]
                       and second["other_generator_fingerprint"] == first["fingerprint"]
                       and first["fingerprint"] != second["fingerprint"]
                       and first["request_key"] != second["request_key"]
                       and first["previous_codegen"] == []
                       and second["previous_codegen"] == [21],
                       "实际变化不止生成器版本或旧请求未失配")
        hit_label = "cache-" + mode + "-o" + level + "-g22-hit-r2"
        hit = invoke(hit_label, [str(new), "cache", str(cache), mode, level])
        record.require(hit["cache_hit"] and not hit["regenerated"]
                       and hit["sha256"] == second["sha256"],
                       "生成器22的缓存控制命中失败")
        cache_rows.extend([*rows, hit])
record.write_new(ROOT / "version-matrix-r2.json", {
    "passed": True, "initial_failure_preserved": previous,
    "metadata": metadata, "validation": validation, "cache": cache_rows,
    "metadata_objects": {version: record.store(Path(row["path"]).read_bytes())
                         for version, row in metadata.items()},
    "scope": "actual generator21/22; production NativeMetadata, ArtifactStore and GlobalIndex APIs",
})
print(json.dumps({"passed": True, "metadata_cells": len(validation),
                  "cache_miss_regeneration_rows": 8, "cache_control_hits": 4},
                 ensure_ascii=False))
