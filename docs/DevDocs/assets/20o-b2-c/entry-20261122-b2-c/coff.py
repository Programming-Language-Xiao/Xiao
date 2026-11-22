"""核对同一次 MSVC 链接产物的 PE 导出表、映射和组件。"""

import argparse
from collections import defaultdict
import json
from pathlib import Path
import re
import struct
import subprocess

from measure import base
from performance_audit import identity

RUNTIME = re.compile(r"(?m)^\s*([A-Fa-f0-9]{4}):([A-Fa-f0-9]{8})\s+(xiao_runtime_\w+)\s+"
                     r"([A-Fa-f0-9]+)\s+f\s+(xiao_runtime[^\r\n]+)$")
NAMESPACE = ("value_", "string_", "error_", "cmath_", "abi_", "language_context_", "fatal_", "print_values",
             "input", "resource_", "process_")


def components(symbol):
    name = symbol.removeprefix("xiao_runtime_")
    result = set()
    if name.startswith(NAMESPACE) or name in {"lfloat_divide", "dynamic_check", "random_seed", "write_i64"}:
        result.add("value")
    if name.startswith("concurrency_"):
        result.add("concurrency")
    if name.startswith("release_trace_") or name in {"release", "retain", "value_copy", "value_release",
                                                     "value_release_strong", "value_release_any", "value_release_weak"}:
        result.add("rc")
    if name.startswith("weak_") or name in {"weak", "value_weak", "value_release_weak"}:
        result.add("weak")
    containers = ("array_", "tuple_", "dict_", "set_")
    if name.startswith(containers + tuple("value_" + prefix for prefix in containers)) or name == "value_select":
        result.add("containers")
    if name.startswith(("table_", "value_table_")) or name == "dynamic_member_get":
        result.add("tables")
    return result


def pe_code_sections(data):
    pe, = struct.unpack_from("<I", data, 0x3C)
    base.require(data[:2] == b"MZ" and data[pe:pe + 4] == b"PE\0\0", "候选不是 PE")
    machine, count, stamp, _, _, length, _ = struct.unpack_from("<HHIIIHH", data, pe + 4)
    start = pe + 24
    base.require(machine == 0x8664 and struct.unpack_from("<H", data, start)[0] == 0x20B, "候选不是 AMD64 PE32+")
    image, = struct.unpack_from("<Q", data, start + 24)
    sections = {}
    for i in range(count):
        at = start + length + i * 40
        size, address, raw_size, raw_at = struct.unpack_from("<IIII", data, at + 8)
        flags, = struct.unpack_from("<I", data, at + 36)
        if flags & 0x20000000:
            sections[i + 1] = (address, min(size, raw_size), raw_at)
    return image, stamp, sections


def verify(executable, map_file, receipt, raw, wire):
    base.require(receipt["map"] == identity(map_file) and receipt["exe"] == identity(executable),
                 "候选或同链接映射字节不符")
    exports = dict(re.findall(r"(?ms)^  Name: (xiao_runtime_\w+)\s*\n  RVA: (0x[0-9A-Fa-f]+)", raw.decode("utf-8")))
    link_map = map_file.decode("utf-8").replace("\r\n", "\n")
    image, stamp, sections = pe_code_sections(executable)
    base.require(int(re.search(r"Preferred load address is ([0-9a-fA-F]+)", link_map)[1], 16) == image
                 and int(re.search(r"Timestamp is ([0-9a-fA-F]+)", link_map)[1], 16) == stamp, "链接映射 PE 身份不符")
    rows = {}
    for section, offset, symbol, address, member in RUNTIME.findall(link_map):
        number, offset, address = int(section, 16), int(offset, 16), int(address, 16)
        rva, size, _ = sections[number]
        base.require(offset < size and address == image + rva + offset, f"链接函数不位于候选可执行节：{symbol}")
        base.require(symbol not in rows, f"重复 Runtime 符号：{symbol}")
        rows[symbol] = {"rva": address - image, "archive_member": member.strip()}
    base.require(rows.keys() == set(receipt["actual_runtime_symbols"])
                 and set(receipt["direct"]) <= rows.keys(), "同链接 Runtime 调用/符号集合缺项")
    base.require(exports and all(rows[name]["rva"] == int(rva, 16) for name, rva in exports.items()),
                 "PE 导出表与实际已链接函数的 RVA 不符")
    observed = defaultdict(list)
    for symbol in rows:
        for component in components(symbol):
            observed[component].append(symbol)
    from candidate import frames
    _, reply = frames(wire)
    product = reply["artifact"]["artifact_runtime"]
    declared = set(product["declared_components"])
    base.require(set(observed) == declared and all(components(symbol) for symbol in rows),
                 f"Windows Runtime 组件对账不符：声明 {sorted(declared)}，实际 {sorted(observed)}")
    return {"product_verification": product["verification"], "same_link_verified": True,
            "exports": exports, "symbols": rows, "components": dict(observed),
            "executable": receipt["exe"], "link_map": receipt["map"]}


def inspect(folder, level, llvm, out):
    stem = f"native-o{level}"
    executable = folder / f"{stem}.exe"
    raw = subprocess.check_output([llvm, "--file-header", "--sections", "--coff-exports", executable])
    (out / f"{folder.name}-o{level}.readobj.log").write_bytes(raw)
    return verify(executable.read_bytes(), (folder / f"{stem}.exe.map").read_bytes(),
                  json.loads((folder / f"coff-o{level}.json").read_bytes()), raw,
                  (folder / f"build-o{level}.stdout").read_bytes())


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("evidence", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--readobj", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir()
    folders = {path.parent.name: path.parent for path in args.evidence.glob("candidates-*/**/coff-o0.json")}
    base.require(folders.keys() == {"cmath-library", "cmath-tutorial", "b2_explicit_divide",
                                    "a5_decimal_arithmetic", "a5_conversions"}, "Windows COFF 应测集合不符")
    rows = {}
    for name, folder in sorted(folders.items()):
        for level in (0, 3):
            key = f"{name}-o{level}"
            rows[key] = inspect(folder, level, args.readobj, args.output)
            print(f"{key}: {len(rows[key]['symbols'])} 个链接 Runtime 符号", flush=True)
    base.write_json(args.output / "result.json", {"candidate_rows": rows, "tool": base.identity(args.readobj)})
