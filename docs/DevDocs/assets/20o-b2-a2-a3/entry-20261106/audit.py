"""只读复核A2/A3内核归档及固定Git源码；--manifest仅重建本目录清单。"""

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]


def read(name):
    return json.loads((ROOT / name).read_text(encoding="utf-8"))


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def files():
    return {path.relative_to(ROOT).as_posix(): path for path in ROOT.rglob("*")
            if path.is_file() and path.name != "evidence.json"}


def inventory():
    rows = [{"path": name, "bytes": len(path.read_bytes()), "sha256": sha(path.read_bytes())}
            for name, path in sorted(files().items())]
    (ROOT / "evidence.json").write_text(json.dumps({
        "scope": "仅本目录全部实际文件；清单本身不计入自身摘要",
        "file_count": len(rows), "files": rows,
    }, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(json.dumps({"files": len(rows)}, ensure_ascii=False))


def canonical(raw):
    try:
        raw.decode("utf-8")
        return raw if b"\0" in raw else raw.replace(b"\r\n", b"\n")
    except UnicodeDecodeError:
        return raw


def verify_source(snapshot, commit):
    require(re.fullmatch(r"[0-9a-f]{40}", commit) is not None, "代码提交格式不符")
    scope = ['core/rust', 'stdlib', 'tests/fixtures', 'tests/benchmarks', '.github/workflows',
             '.cargo', 'tools', 'cli/ts', 'docs/UseDocs', 'docs/module-registry.json',
             'package.json', 'bun.lock', '.gitattributes']
    require(snapshot['scope'] == scope, "源码输入范围漂移")
    expected = subprocess.check_output(['cmd', '/c', 'git', 'ls-tree', '-r', '--name-only', '-z', commit, '--', *scope], cwd=REPOSITORY)
    expected_paths = {path.decode('utf-8') for path in expected.split(b'\0') if path}
    source_paths = [item['path'] for item in snapshot['files']]
    require(len(source_paths) == len(set(source_paths)) and set(source_paths) == expected_paths, "源码输入清单缺项或重复")
    require(sha(json.dumps(snapshot['files'], ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()) == snapshot['raw_tree_sha256'], "原始源码树摘要不符")
    numeric = [[item['path'], item['utf8_lf_or_binary_sha256']] for item in snapshot['files']]
    require(sha(json.dumps(numeric, ensure_ascii=False, separators=(',', ':')).encode()) == snapshot['utf8_lf_tree_sha256'], "规范源码树摘要不符")
    requests = "".join(commit + ":" + item["path"] + "\n" for item in snapshot["files"]).encode()
    blobs = subprocess.check_output(["cmd", "/c", "git", "cat-file", "--batch"], cwd=REPOSITORY, input=requests)
    offset = 0
    for item in snapshot["files"]:
        end = blobs.index(b"\n", offset)
        header = blobs[offset:end].split()
        require(len(header) == 3 and header[1] == b"blob", "提交缺少源码：" + item["path"])
        length = int(header[2])
        raw = blobs[end + 1:end + 1 + length]
        require(sha(canonical(raw)) == item["utf8_lf_or_binary_sha256"], "输入偏离固定代码提交：" + item["path"])
        offset = end + length + 2
    require(offset == len(blobs), "Git对象流长度不符")


def verify_ci(closeout):
    for workflow, response in closeout["ci_runs"].items():
        run = read(response)
        require(run["head_sha"] == closeout["code_commit"], "CI源码身份不符：" + workflow)
        require(run["status"] == "completed" and run["conclusion"] == "success", "CI尚未通过：" + workflow)
    jobs = read(closeout["ci_maintenance_jobs"])["jobs"]
    job_map = {job["id"]: job for job in jobs}
    record = read(closeout["ci_arithmetic_record"])
    require(record['job_id'] in job_map, "CI内核实际日志作业ID缺失或被替换")
    actual_job = job_map[record['job_id']]
    require(actual_job['name'] == record['job_name'] and 'workspace default tests' in actual_job['name'], "CI内核实际日志作业被其他作业替代")
    require(record['run_id'] == read(closeout['ci_runs']['maintenance'])['id'], "CI内核日志运行号不符")
    raw = (ROOT / record["log"]).read_bytes()
    require(sha(raw) == record["log_sha256"] and record["code_commit"] == closeout["code_commit"], "内核CI日志身份不符")
    require(job_map[record["job_id"]]["conclusion"] == "success", "内核实际日志作业未通过")
    lines = raw.decode("utf-8", errors="replace").splitlines()
    names = read("kernel-test-names.json")["tests"]
    for name in names:
        require(sum(("test " + name + " ... ok") in line for line in lines) == 1,
                "CI没有且仅有一次内核测试通过：" + name)
    require(len(names) == 21, "内核测试清单计数不符")


def main():
    closeout = read("closeout.json")
    require(closeout["passed"] and closeout["scope"] == "Runtime内核", "本批尚未有效收口")
    manifest = read("evidence.json")
    observed = files()
    require(len(manifest["files"]) == manifest["file_count"] == len(observed), "证据清单计数不符")
    require(set(observed) == {item["path"] for item in manifest["files"]}, "证据清单缺项或重复")
    for item in manifest["files"]:
        path = (ROOT / item["path"]).resolve()
        require(path.is_relative_to(ROOT), "证据路径越界")
        raw = path.read_bytes()
        require(len(raw) == item["bytes"] and sha(raw) == item["sha256"], "证据摘要不符：" + item["path"])
    source_ids = set()
    logs = {}
    for name in closeout["effective_checks"]:
        record = read(name + ".json")
        raw = (ROOT / record["log"]).read_bytes()
        require(record["exit_code"] == 0 and sha(raw) == record["sha256"], "命令失败或日志摘要不符：" + name)
        require(record["source_unchanged"] and record["source_before"] == record["source_after"], "执行中源码变化：" + name)
        require(record["cwd"] == read("bootstrap.json")["source_root"], "执行源码根不符")
        require(record["runner_sha256"] == sha((ROOT / "run-step.py").read_bytes()), "执行器身份不符")
        if Path(record["argv"][0]).name.lower() in {"cargo", "cargo.exe"}:
            require("--locked" in record["argv"] and "+1.96.0" in record["argv"], "Cargo未固定锁或工具链")
        source_ids.add(record["source_before"]["manifest"])
        logs[name] = raw.decode("utf-8", errors="replace")
    require(len(source_ids) == 1, "最终验收不是同一源码输入")
    snapshot = read(source_ids.pop())
    require(snapshot["raw_tree_sha256"] == closeout["raw_source_tree_sha256"], "最终源码摘要不符")
    verify_source(snapshot, closeout["code_commit"])
    pattern = r"(?m)^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;[^\r\n]*"
    for name, count in closeout["exact_test_counts"].items():
        require(re.findall(pattern, logs[name]) == [("ok", str(count), "0", "0")], "实际测试汇总不符：" + name)
        if count == 1:
            require("--exact" in read(name + ".json")["argv"], "缺少--exact：" + name)
    guards = read(closeout["guard_report"])
    require(guards["passed"] == guards["total"] == len(guards["cases"]) == 28, "B1守卫不是28/28")
    receipt = read("compound-error-order-receipt.json")
    require(receipt["answer"] == "确认：先非有限，再算子域（推荐）", "复合错误裁定不符")
    boundary = read("boundary.json")
    require(boundary["passed"] and not boundary["protected_changes"] and not boundary["unclassified_paths"], "产品边界核对失败")
    require(closeout["abi"] == "1.18" and closeout["codegen"] == 20 and closeout["active_intrinsic_max"] == 32, "版本越界")
    require(closeout["cmath"] == "planned" and closeout["numeric_tutorial_scope"] == "planned", "教程状态越界")
    require(not closeout["b1_721_executed"] and not closeout["a4_through_a7_started"], "超出本批范围")
    verify_ci(closeout)
    print(json.dumps({"passed": True, "code_commit": closeout["code_commit"], "source_files": len(snapshot["files"]),
                      "effective_checks": len(closeout["effective_checks"]), "scope": "只读归档复核，非新产品或性能执行"}, ensure_ascii=False))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", action="store_true")
    inventory() if parser.parse_args().manifest else main()
