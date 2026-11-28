"""实跑固定 27 与 28 生成器、旧核心预检和归档缓存过渡。"""

import difflib
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys
import zipfile

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from capture import capture

LOGS = Path(__file__).resolve().parent
HISTORY = LOGS.parent
TASK = Path(sys.argv[1]).resolve()
CASES = TASK / "cases"
COMPAT_SUFFIX = sys.argv[3] if len(sys.argv) > 3 else ""


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def identity(path):
    data = path.read_bytes()
    return {"path": str(path), "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def run_recorded(name, command, expected=0):
    assert capture(LOGS, name, [str(item) for item in command]) == expected, name


def pe_metadata(binary, output):
    data = binary.read_bytes()
    pe = struct.unpack_from("<I", data, 0x3c)[0]
    assert data[:2] == b"MZ" and data[pe:pe+4] == b"PE\x00\x00"
    sections = struct.unpack_from("<H", data, pe + 6)[0]
    offset = pe + 24 + struct.unpack_from("<H", data, pe + 20)[0]
    for section in range(sections):
        start = offset + section * 40
        if data[start:start+8] != b".xiaomta":
            continue
        raw_size, raw_offset = struct.unpack_from("<II", data, start + 16)
        assert data[raw_offset:raw_offset+8] == b"XIAOMETA"
        length = struct.unpack_from("<I", data, raw_offset + 12)[0]
        assert 60 <= length <= raw_size
        metadata = data[raw_offset:raw_offset+length]
        assert len(metadata) == length
        version = struct.unpack_from("<I", metadata, 40)[0]
        output.write_bytes(metadata)
        return {"generator": version, **identity(output)}
    raise AssertionError(f"缺少 .xiaomta 节 {binary}")


def generated(stage, fixture, optimization, version, expected=0):
    name = f"{stage}-{fixture}-o{optimization}"
    case = CASES / name
    tool_dirs = {"old27": "target-old27", "before28": "target-before28", "after28": "target-tools"}
    probe = TASK / tool_dirs[stage] / "debug/examples/generator_probe.exe"
    source = HISTORY / f"generator-{fixture}.xiao"
    os.environ["CC_PRINT_OPTIONS"] = "1"
    os.environ["CC_PRINT_OPTIONS_FILE"] = str(LOGS / f"{name}-link.log")
    run_recorded(name, [probe, source, case, optimization, 34], expected)
    receipt = LOGS / f"{name}.json"
    if expected:
        assert not (case / "program.ll").exists() and not (case / "program.exe").exists()
        error = (LOGS / f"{name}.stderr.log").read_text(encoding="utf-8").strip()
        assert "不支持 动态 intrinsic os_log_write" in error
        return {"exit": expected, "diagnostic": error, "source": identity(source),
                "probe": identity(probe), "receipt": identity(receipt)}
    verification = case / "verification.json"
    result = read_json(verification)
    assert result["generator"] == version and result["exit"] == 0
    assert result["runtime"]["abi"] == 65556 and result["stdlib_interface"] == 1
    assert result["source"]["sha256"] == identity(source)["sha256"]
    assert result["verification"] == "complete"
    assert result["declared_components"] == result["observed_components"]
    assert not result["unclassified_runtime_symbols"]
    for entry in ("ir", "binary", "map", "runtime"):
        assert identity(Path(result[entry]["path"]))["sha256"] == result[entry]["sha256"]
    binary = Path(result["binary"]["path"])
    metadata = pe_metadata(binary, case / "program.meta")
    assert metadata["generator"] == version
    result.pop("version_parameter_28_only")
    result.update(metadata=metadata, verification_receipt=identity(verification),
                  probe=identity(probe), receipt=identity(receipt),
                  link=identity(LOGS / f"{name}-link.log"))
    return result


def generator_matrix():
    old = TASK / "old27"
    pre = TASK / "before28"
    ignored = {"examples/generator_probe.rs", "examples/verify_archive_build.rs"}
    diffs = []
    for folder in ("core/rust", "stdlib"):
        left = {path.relative_to(old / folder): path for path in (old / folder).rglob("*")
                if path.is_file() and not path.as_posix().endswith(tuple(ignored))}
        right = {path.relative_to(pre / folder): path for path in (pre / folder).rglob("*")
                 if path.is_file() and not path.as_posix().endswith(tuple(ignored))}
        assert left.keys() == right.keys()
        for path in sorted(left):
            if identity(left[path])["sha256"] != identity(right[path])["sha256"]:
                diffs.append((left[path], right[path]))
    assert len(diffs) == 1 and diffs[0][0].as_posix().endswith("xiao-codegen-llvm/src/lib.rs")
    old_lib, new_lib = diffs[0]
    assert old_lib.read_bytes().replace(b"CODEGEN_VERSION: u32 = 27", b"CODEGEN_VERSION: u32 = 28") == new_lib.read_bytes()
    write_json(LOGS / "before28-source-delta.json", {
        "commit": "b794e2f289c263e611eb92198652fdb1d1847c37",
        "before27": identity(old_lib), "before28": identity(new_lib),
        "changed_line": "CODEGEN_VERSION 27 → 28",
    })

    matrix = []
    version_diff = []
    excerpts = []
    for level in (0, 3):
        old_control = generated("old27", "control", level, 27)
        denied = generated("before28", "log", level, 28, expected=1)
        before = generated("before28", "control", level, 28)
        control = generated("after28", "control", level, 28)
        log = generated("after28", "log", level, 28)
        texts = [Path(item["ir"]["path"]).read_bytes() for item in (old_control, before, control, log)]
        assert texts[1] == texts[2] and texts[0] != texts[1]
        only_body = lambda ir: re.sub(rb"(?m)^@xiao_native_metadata = .*\n", b"", ir)
        assert only_body(texts[0]) == only_body(texts[1])
        assert old_control["runtime"] == before["runtime"] == control["runtime"] == log["runtime"]
        assert old_control["source"] == before["source"] == control["source"]
        assert denied["source"]["sha256"] == log["source"]["sha256"]
        assert old_control["fingerprints"] != before["fingerprints"] == control["fingerprints"]
        assert "xiao_runtime_log_write" not in control["runtime_symbols"]
        assert "xiao_runtime_log_write" in log["runtime_symbols"]
        log_ir = texts[3].decode("utf-8")
        declaration = "declare i32 @xiao_runtime_log_write(ptr, ptr, ptr, i64)"
        call = re.search(r"(?m)^.+call i32 @xiao_runtime_log_write\(ptr %\w+, ptr %\w+, ptr %\w+, i64 34\)$", log_ir)
        assert declaration in log_ir and call
        assert "call i32 @xiao_runtime_env_run_native(ptr @xiao_application_main, i64 34, i32 0)" in log_ir
        excerpts.extend([f"O{level} 接线前 28", denied["diagnostic"],
                         f"O{level} 接线后 28", declaration, call[0],
                         f"无日志对照 SHA-256 {control['ir']['sha256']}", ""])
        version_diff.extend(difflib.unified_diff(texts[0].decode().splitlines(keepends=True),
                                                  texts[1].decode().splitlines(keepends=True),
                                                  fromfile=f"无日志 O{level} 真实 27",
                                                  tofile=f"无日志 O{level} 真实 28"))
        matrix.append({"optimization": level, "old27_control": old_control, "before28_log": denied,
                       "before28_control": before, "after28_control": control, "after28_log": log,
                       "control_ir_equal": True, "version_only_changed_native_metadata": True})
    (LOGS / "lowering-28.log").write_text("\n".join(excerpts), encoding="utf-8")
    (LOGS / "actual-version-only.diff").write_text("".join(version_diff), encoding="utf-8")
    write_json(LOGS / "generator-diff.json", matrix)


def frame(message):
    payload = json.dumps(message, ensure_ascii=False).encode("utf-8")
    return len(payload).to_bytes(8, "big") + payload


def replies(wire):
    messages = []
    while wire:
        length = int.from_bytes(wire[:8], "big")
        assert 0 < length <= len(wire) - 8
        messages.append(json.loads(wire[8:8+length]))
        wire = wire[8+length:]
    return messages


def protocol_case(core, version, name, verb, path, expected=0, mask=0x22, code=None):
    name += COMPAT_SUFFIX
    common = {"protocol_version": 1, "core_version": 1}
    hello = {"type": "hello", "request_id": "identity", **common}
    request = {"type": verb, "request_id": name, "path": str(path), **common}
    if verb == "verify":
        request["detail"] = True
    else:
        request["options"] = {"capabilities": mask, "max_call_depth": 1024, "event_capacity": 8192}
    data = frame(hello) + frame(request)
    request_path, response_path, stderr_path = (LOGS / f"{name}.{suffix}"
                                                for suffix in ("request.bin", "response.bin", "stderr.log"))
    request_path.write_bytes(data)
    result = subprocess.run([str(core)], input=data, capture_output=True, timeout=30)
    response_path.write_bytes(result.stdout)
    stderr_path.write_bytes(result.stderr)
    hello_reply, response = replies(result.stdout)
    assert hello_reply["versions"]["codegen"] == version
    assert hello_reply["versions"]["runtime_abi"] == 65556
    assert result.returncode == 0, result.stderr
    assert response["request_id"] == name
    actual = response["exit_code"]
    assert actual == expected, response
    if code:
        assert response["type"] == "error" and response["error"]["code"] == code, response
        assert not response.get("events")
    row = {"command": [str(core)], "cwd": str(Path.cwd()), "core": identity(core), "artifact": identity(path),
           "process_exit": result.returncode, "validation_exit": actual,
           "request_frame": identity(request_path), "response_frame": identity(response_path),
           "stderr": identity(stderr_path), "hello": hello_reply, "response": response}
    write_json(LOGS / f"{name}.json", row)
    print(f"{name}: {actual}", flush=True)
    return response


def archive_build(tool, version, name, source, archive, cache, hit):
    name += COMPAT_SUFFIX
    run_recorded(name, [tool, source, archive, cache])
    receipt = read_json(LOGS / f"{name}.stdout.log")
    assert receipt["producer_generator"] == version and receipt["runtime_abi"] == [65556, 65556]
    assert receipt["stdlib_interface"] == 1 and receipt["schema"] == [1, version - 27]
    assert bool(receipt["modules"]) and all(module["hit"] is hit for module in receipt["modules"])
    assert identity(archive)["sha256"] == receipt["archive_sha256"]
    with zipfile.ZipFile(archive) as bundle:
        index = archive.with_suffix(".index.pb")
        index.write_bytes(bundle.read("META-INF/xiao/index.pb"))
    return {**receipt, "index": identity(index)}


def compatibility():
    directory = CASES / f"compatibility{COMPAT_SUFFIX}"
    directory.mkdir(exist_ok=True)
    dirs = {27: TASK / "target-old27/debug", 28: TASK / "target-tools/debug"}
    cores = {version: path / "xiao-core.exe" for version, path in dirs.items()}
    builders = {version: path / "examples/verify_archive_build.exe" for version, path in dirs.items()}
    inputs = {}
    for fixture in ("control", "log"):
        path = directory / fixture / "main.xiao"
        path.parent.mkdir(exist_ok=True)
        shutil.copyfile(HISTORY / f"generator-{fixture}.xiao", path)
        inputs[fixture] = path
    source = inputs["control"]
    archives = {n: directory / f"control-{n}.xar" for n in (27, 28)}
    cache = directory / "cache"
    os.environ.pop("CC_PRINT_OPTIONS_FILE", None)
    cached = {}
    for version in (27, 28):
        result = archive_build(builders[version], version, f"compat-archive-{version}-miss", source,
                               archives[version], cache, False)
        index = directory / f"cache-{version}.index.pb"
        shutil.copyfile(cache / "global.index.pb", index)
        cached[version] = {"artifact": result, "cache_index": identity(index)}
    assert cached[27]["cache_index"]["sha256"] != cached[28]["cache_index"]["sha256"]
    assert cached[27]["artifact"]["archive_sha256"] != cached[28]["artifact"]["archive_sha256"]
    replay = archive_build(builders[28], 28, "archive-28-hit", source,
                           directory / "control-28-repeat.xar", cache, True)
    assert replay["archive_sha256"] == cached[28]["artifact"]["archive_sha256"]
    log_archive = directory / "log-28.xar"
    log = archive_build(builders[28], 28, "archive-28-log", inputs["log"],
                        log_archive, directory / "log-cache", False)
    assert log["capabilities"] == 0x22

    matrix = read_json(LOGS / "generator-diff.json")
    for row in matrix:
        level = row["optimization"]
        for consumer, producer, artifact in [(27, 27, row["old27_control"]),
                                             (28, 27, row["old27_control"]),
                                             (28, 28, row["after28_log"]),
                                             (27, 28, row["after28_log"])]:
            failure = consumer < producer
            result = protocol_case(cores[consumer], consumer, f"native-{consumer}-on-{producer}-o{level}",
                                   "verify", Path(artifact["binary"]["path"]),
                                   expected=2 if failure else 0,
                                   code="X17-XAR-007" if failure else None)
            if not failure:
                summary = json.loads(result["value"]["value"])
                assert summary["native_metadata"]["generator_version"] == producer
    for consumer, producer in [(27, 27), (28, 27), (28, 28), (27, 28)]:
        failure = consumer < producer
        result = protocol_case(cores[consumer], consumer, f"archive-run-{consumer}-on-{producer}", "run_archive",
                               archives[producer], expected=2 if failure else 0,
                               code="X17-XAR-007" if failure else None)
        if failure:
            assert "归档索引版本" in result["error"]["message"]
        else:
            assert any(event["kind"] == "intrinsic_output" for event in result["events"])
    protocol_case(cores[27], 27, "archive-run-27-on-log-28", "run_archive", log_archive,
                  expected=2, code="X17-XAR-007")
    for mask, missing in ((2, "stdout"), (32, "clock")):
        result = protocol_case(cores[28], 28, f"archive-log-28-mask-{mask}", "run_archive",
                               log_archive, expected=2, mask=mask, code="X06-RUNTIME-026")
        assert missing in result["error"]["message"]
    protocol_case(cores[28], 28, "archive-log-28-allowed", "run_archive", log_archive)
    write_json(LOGS / "archive-cache.json", {"source": identity(source), "versions": cached,
                                             "verified_hit": replay, "log_archive": log})


if __name__ == "__main__":
    stage = sys.argv[2]
    if stage == "generator":
        generator_matrix()
    elif stage == "compatibility":
        compatibility()
    else:
        raise ValueError(stage)
