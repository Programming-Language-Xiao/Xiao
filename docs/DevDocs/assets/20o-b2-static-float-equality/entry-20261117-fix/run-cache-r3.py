"""复核21/22缓存矩阵，逐条记录命令实际运行时间且使用新缓存根。"""
from datetime import datetime, timezone
import json
from pathlib import Path
import subprocess
import sys

sys.dont_write_bytecode = True
import record

ROOT, TASK = record.ROOT, record.TASK
HOSTS = {21: TASK / "generator21-r2.exe", 22: TASK / "generator22-r2.exe"}


def invoke(label, argv):
    record.require(not (ROOT / (label + ".json")).exists(), "禁止覆盖命令回执")
    env, overrides, _ = record.environment()
    started = datetime.now(timezone.utc).isoformat()
    child = subprocess.Popen(argv, cwd=record.REPOSITORY, env=env, stdin=subprocess.DEVNULL,
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    stdout, stderr = child.communicate()
    finished = datetime.now(timezone.utc).isoformat()
    record.write_new(ROOT / (label + ".json"), {
        "label": label, "argv": argv, "cwd": str(record.REPOSITORY), "pid": child.pid,
        "started_utc": started, "finished_utc": finished, "exit_code": child.returncode,
        "environment_overrides": overrides,
        "stdout": record.store(stdout), "stderr": record.store(stderr),
    })
    record.require(child.returncode == 0,
                   "版本矩阵命令失败：" + label + chr(10)
                   + stderr.decode("utf-8", errors="replace"))
    return json.loads(stdout) if stdout.strip() else None


for version, host in HOSTS.items():
    receipt = json.loads((ROOT / ("generator" + str(version) + "-r2-identity.json"))
                         .read_text(encoding="utf-8"))
    record.require(record.identity(host) == receipt["executable"],
                   "真实版本宿主摘要漂移：" + str(version))

metadata = {}
for version in (21, 22):
    path = TASK / ("generator" + str(version) + "-r3.meta")
    metadata[version] = invoke(
        "metadata" + str(version) + "-r3",
        [str(HOSTS[version]), "metadata", str(path)],
    )
    record.require(metadata[version]["actual_codegen"] == version,
                   "元数据实际生成器身份不符")

validation = []
for reader in (21, 22):
    for writer in (21, 22):
        row = invoke(
            "read" + str(reader) + "-artifact" + str(writer) + "-r3",
            [str(HOSTS[reader]), "validate", metadata[writer]["path"]],
        )
        record.require(row["actual_validator"] == reader
                       and row["artifact_generator"] == writer
                       and row["accepted"] is (writer <= reader),
                       "真实21/22元数据矩阵不符")
        if writer > reader:
            record.require(row["code"] == "X17-XAR-007" and not row["entry_executed"],
                           "旧生成器未在执行前拒绝")
        validation.append(row)

cache_rows = []
for mode in ("static", "dynamic"):
    for level in (0, 3):
        cache = TASK / ("cache-" + mode + "-o" + str(level) + "-r3")
        record.require(not cache.exists(), "缓存任务根不可复用")
        pair = []
        for version in (21, 22):
            row = invoke(
                "cache-" + mode + "-o" + str(level) + "-g" + str(version) + "-r3",
                [str(HOSTS[version]), "cache", str(cache), mode, str(level)],
            )
            record.require(not row["cache_hit"] and row["regenerated"],
                           "生成器切换未导致真实miss与重建")
            pair.append(row)
            index = cache / "global.index.pb"
            record.write_new(ROOT / ("index-" + mode + "-o" + str(level)
                                     + "-g" + str(version) + "-r3.json"), {
                "index": record.store(index.read_bytes()),
                "executable": record.identity(Path(row["executable"])),
            })
        old, new = pair
        for key in ("input_sha256", "target", "optimization", "optimization_plan",
                    "runtime_abi", "runtime_library", "versions", "native_libraries"):
            record.require(old[key] == new[key], "矩阵共同条件变化：" + key)
        record.require(old["actual_codegen"] == 21 and new["actual_codegen"] == 22
                       and old["fingerprint"] != new["fingerprint"]
                       and old["request_key"] != new["request_key"]
                       and old["previous_codegen"] == []
                       and new["previous_codegen"] == [21]
                       and old["other_generator_fingerprint"] == new["fingerprint"]
                       and new["other_generator_fingerprint"] == old["fingerprint"],
                       "变化未被隔离到生成器版本")
        hit = invoke(
            "cache-" + mode + "-o" + str(level) + "-g22-hit-r3",
            [str(HOSTS[22]), "cache", str(cache), mode, str(level)],
        )
        record.require(hit["cache_hit"] and not hit["regenerated"]
                       and hit["sha256"] == new["sha256"],
                       "新生成器缓存控制命中失败")
        cache_rows.extend([old, new, hit])

output = {
    "passed": True, "scope": "实际21/22宿主与生产NativeMetadata/ArtifactStore/GlobalIndex API",
    "metadata": metadata, "validation": validation, "cache": cache_rows,
    "hosts": {version: record.identity(host) for version, host in HOSTS.items()},
}
record.write_new(ROOT / "version-matrix-r3.json", output)
print(json.dumps({"passed": True, "metadata_cells": len(validation),
                  "cache_misses_and_regenerations": 8, "new_version_cache_hits": 4},
                 ensure_ascii=False))
