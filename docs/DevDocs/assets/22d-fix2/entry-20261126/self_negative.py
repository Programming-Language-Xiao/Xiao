"""在封存副本中重算摘要，验证审计能发现缺失的轨迹及错误的产物绑定。"""

import json
from pathlib import Path
from tempfile import TemporaryDirectory
from zipfile import ZIP_DEFLATED, ZipFile

from audit import HERE, audit, digest, read_json, require


def remove_destroy(source):
    base = "ci/trace/windows"
    matrix = read_json(source, f"{base}/matrix.json")
    run = next(row for row in matrix["rounds"] if row["case"] == "xcon-bounded-string-success")
    receipt_path = f"{base}/{run['receipt']}"
    receipt = read_json(source, receipt_path)
    trace = next(item for item in receipt["traces"] if item["path"].endswith(".native-release-trace.tsv"))
    trace_path = f"{base}/{trace['path']}"
    events = [line.split("\t") for line in source.read(trace_path).decode().splitlines()]
    destroyed = next(index for index, event in enumerate(events) if event[-1] == "destroy")
    del events[destroyed]
    raw = "".join(f"{index}\t{obj}\t{action}\n" for index, (_, obj, action) in enumerate(events)).encode()
    trace.update(bytes=len(raw), sha256=digest(raw))
    trace["terminal"]["destroy"] -= 1
    return {trace_path: raw, receipt_path: json.dumps(receipt).encode()}


def repeat_fs_artifact(source):
    name = "ci/fs/windows/result.json"
    result = read_json(source, name)
    result["artifacts"] = [result["artifacts"][0]] * len(result["artifacts"])
    return {name: json.dumps(result).encode()}


def swap_fs_artifacts(source):
    name = "ci/fs/windows/result.json"
    result = read_json(source, name)
    by_case = {row["case"]: row for row in result["artifacts"]}
    o0, o3 = by_case["fs-std-o0"], by_case["fs-std-o3"]
    for kind in ("artifact", "map"):
        o0[kind], o3[kind] = o3[kind], o0[kind]
    return {name: json.dumps(result).encode()}


def empty_baseline_trace(source):
    name = "original/baseline/summary-100.json"
    rows = read_json(source, name)
    rows[0]["trace"] = {}
    return {name: json.dumps(rows).encode()}


def reuse_round_files(source):
    prefix = "ci/trace/windows/full"
    first = read_json(source, f"{prefix}/001/receipt.json")
    second = f"{prefix}/002/receipt.json"
    first["round"] = 2
    return {second: json.dumps(first).encode()}


def remove_source_unchanged(source):
    replacements = {}
    for name in ("local/f1/matrix.json", "local/matrix/matrix.json"):
        data = read_json(source, name)
        data.pop("source_unchanged", None)
        replacements[name] = json.dumps(data).encode()
    return replacements


def check_rejection(root, mutate, message, legacy_source=True):
    bundle = root / "evidence.zip"
    with ZipFile(HERE / "evidence.zip") as source, ZipFile(bundle, "w", ZIP_DEFLATED, compresslevel=1) as changed:
        replacements = mutate(source)
        for name in source.namelist():
            changed.writestr(name, replacements[name] if name in replacements else source.read(name))
    status = json.loads((HERE / "status.json").read_bytes())
    status["evidence_sha256"] = digest(bundle.read_bytes())
    (root / "status.json").write_text(json.dumps(status), encoding="utf-8")
    try:
        audit(root, legacy_source=legacy_source)
    except ValueError as error:
        require(message in str(error), f"拒绝原因不符：{error}")
        print(f"按预期拒绝：{message}")
    else:
        raise AssertionError(f"审计遗漏：{message}")


def main():
    audit(HERE, legacy_source=True)
    with TemporaryDirectory(prefix="xiao-22d-audit-") as scratch:
        check_rejection(Path(scratch), remove_source_unchanged, "缺少 source_unchanged=true", legacy_source=False)
        check_rejection(Path(scratch), remove_destroy, "释放生命周期无效")
        check_rejection(Path(scratch), repeat_fs_artifact, "fs 六个用例缺失或重复")
        check_rejection(Path(scratch), swap_fs_artifacts, "fs 产物与原生日志不符")
        check_rejection(Path(scratch), empty_baseline_trace, "F1 基线两侧 TSV 缺项")
        check_rejection(Path(scratch), reuse_round_files, "差分轮次引用了其他轮次的原始文件")


if __name__ == "__main__":
    main()
