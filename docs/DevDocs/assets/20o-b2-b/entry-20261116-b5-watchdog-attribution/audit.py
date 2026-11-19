"""只读核验 B5 前后默认 workspace 对照。"""
import argparse
from datetime import datetime
from functools import lru_cache
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
PAST = "docs/DevDocs/assets/20o-b2-b/entry-20261116-b5-rebuild"
COMMAND = ["cargo", "+1.96.0", "test", "--locked", "--manifest-path",
           "core/rust/Cargo.toml", "--workspace", "--no-fail-fast"]
RUNS = [
    ("b4", "pre-b5-cfa79381", "cfa79381bbae027cec11136ca380e82f66720c34", 0),
    ("final3", "b5-idle-replay", "5864cbde627f727483ab676c6f4bc1bf0a7819bb", 101),
]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def record_module():
    sys.path.insert(0, str(REPOSITORY / PAST))
    import record
    return record


def inventory(root):
    result = {}
    for path in sorted(root.rglob("*")):
        require(not path.is_symlink() and not getattr(path, "is_junction", lambda: False)(),
                "归因证据含链接")
        require("__pycache__" not in path.parts, "归因证据含 Python 缓存")
        if path.is_file() and path.relative_to(root).as_posix() != "evidence.json":
            raw = path.read_bytes()
            result[path.relative_to(root).as_posix()] = {"bytes": len(raw), "sha256": sha(raw)}
    return result


def tree(root):
    inventory(root)
    files = {path.relative_to(root).as_posix(): sha(path.read_bytes())
             for path in sorted(root.rglob("*")) if path.is_file()}
    return sha(json.dumps(files, sort_keys=True).encode("utf-8"))


def object_bytes(root, item):
    require(re.fullmatch(r"objects/sha256/[0-9a-f]{2}/[0-9a-f]{64}[.]bin", item["path"]),
            "对象路径不符")
    raw = (root / item["path"]).read_bytes()
    digest = sha(raw)
    require(len(raw) == item["bytes"] and digest == item["sha256"]
            and item["path"] == f"objects/sha256/{digest[:2]}/{digest}.bin",
            "对象摘要不符")
    return raw


@lru_cache(maxsize=None)
def fixed_bytes(relative):
    return record_module().git(REPOSITORY, "show", "06eee789:" + PAST + "/" + relative)


def fixed_json(record, relative):
    return json.loads(fixed_bytes(relative))


def workspace_counts(log):
    rows, key, summaries = {}, None, []
    def finish():
        if key:
            require(summaries and key not in rows, "workspace目标缺失或重复")
            rows[key] = summaries[-1]
    for line in log.splitlines():
        match = re.search(r"Running .*[/]([^/]+)-([0-9a-f]+)[.]exe[)]", line)
        next_key = match[1] + "@" + match[2] if match else None
        if line.lstrip().startswith("Doc-tests "):
            next_key = line.split("Doc-tests ", 1)[1].strip() + ":doc"
        if next_key:
            finish()
            key, summaries = next_key, []
        match = re.match(r"test result: (?:ok|FAILED)[.] (\d+) passed; (\d+) failed; (\d+) ignored;", line)
        if match:
            require(key is not None, "workspace汇总缺少目标")
            summaries.append(list(map(int, match.groups())))
    finish()
    return {"targets": len(rows), "passed": sum(row[0] for row in rows.values()),
            "failed": sum(row[1] for row in rows.values()),
            "ignored": sum(row[2] for row in rows.values())}


def check(root=ROOT, sealed=True):
    before = tree(root)
    record = record_module()
    require((root / ".gitattributes").read_bytes() == b"* -text -whitespace\n*.bin -diff\n",
            "属性文件不符")
    if (root / "evidence.json").exists():
        manifest = load(root / "evidence.json")["files"]
        require(manifest == inventory(root), "归因证据清单不符")
    elif sealed:
        raise ValueError("归因证据尚未封存")
    rows = []
    for variant, label, commit, expected_exit in RUNS:
        row = load(root / (label + ".json"))
        source = fixed_json(record, variant + "/source.json")
        environment = fixed_json(record, variant + "/environment.json")
        runtime_variant = "current" if variant == "final3" else "b4"
        libraries = fixed_json(record, runtime_variant + "/libraries.json")
        require(row["expected_product_commit"] == source["revision"] == commit
                and row["source_identity_sha256"] == source["sha256"] == record.identity(source["files"]),
                "产品来源身份不符")
        require({path: item["canonical_sha256"] for path, item in source["files"].items()}
                == record.fixed_files(REPOSITORY, commit), "固定 Git 产品树不符")
        require(row["argv"] == COMMAND and row["variant"] == variant and row["label"] == label,
                "命令或标签不符")
        require(row["source_root"] == environment["source_root"]
                and row["task_root"] == environment["task_root"]
                and row["environment_sha256"] == sha(fixed_bytes(variant + "/environment.json"))
                and row["cargo_lock_sha256"] == source["files"]["core/rust/Cargo.lock"]["raw_sha256"],
                "环境或锁身份不符")
        require(row["runner_sha256"] == sha((root / "run.py").read_bytes())
                and row["source_unchanged"] is True and row["active_build_processes_before"] == []
                and row["pid"] > 0 and row["elapsed_seconds"] > 0
                and datetime.fromisoformat(row["finished_utc"]) > datetime.fromisoformat(row["started_utc"]),
                "运行回执不完整")
        require(row["runtime_before"] == row["runtime_after"] == libraries["runtime_after"],
                "Runtime 身份不符")
        log = object_bytes(root, row["log"]).decode("utf-8").replace(chr(92), "/").replace("\r\n", "\n")
        require(row["exit_code"] == expected_exit
                and row["unicode_budget_test_present"] is True
                and row["watchdog_marker_present"] == ("X09-DRIVER-002" in log)
                and row["watchdog_marker_present"] == (expected_exit == 101),
                "退出码或看门狗标记不符")
        section_match = re.search(r"Running tests/20o_b1_imp_resources[.]rs.*?(?=\n\s*Running |\Z)", log, re.S)
        require(section_match is not None, "资源目标日志缺失")
        section = section_match.group()
        expected = "16 passed; 1 failed; 0 ignored;" if expected_exit else "17 passed; 0 failed; 0 ignored;"
        require(expected in section, "资源目标汇总不符")
        expected_url = "url_decoding_reaches_the_unicode_result_budget ... FAILED" if expected_exit else "url_decoding_reaches_the_unicode_result_budget ... ok"
        require(expected_url in section, "URL 目标结论不符")
        counts = workspace_counts(log)
        expected_counts = ({"targets": 188, "passed": 1748, "failed": 1, "ignored": 54}
                           if expected_exit else
                           {"targets": 187, "passed": 1747, "failed": 0, "ignored": 54})
        require(counts == expected_counts, "完整workspace目标计数不符")
        require("Doc-tests xiao_xar" in log and counts["failed"] == (1 if expected_exit else 0),
                "完整workspace目标或失败数不符")
        require("error: 1 target failed:" in log if expected_exit else "test result: FAILED." not in log,
                "完整workspace退出汇总不符")
        rows.append({"label": label, "commit": commit, "exit_code": expected_exit,
                     "elapsed_seconds": row["elapsed_seconds"], "url_passed": expected_exit == 0,
                     "workspace": counts})
    first = load(root / "pre-b5-cfa79381.json")
    second = load(root / "b5-idle-replay.json")
    require(datetime.fromisoformat(second["started_utc"]) > datetime.fromisoformat(first["finished_utc"])
            and first["host"] == second["host"]
            and first["cargo_lock_sha256"] == second["cargo_lock_sha256"],
            "两次运行未按顺序或共同环境执行")
    summary = load(root / "summary.json")
    require(summary == {
        "same_default_command": True, "sequential_idle_replay": True,
        "baseline_commit": RUNS[0][2], "b5_test_commit": RUNS[1][2],
        "baseline_timed_out": False, "b5_replay_timed_out": True,
        "runs": ["pre-b5-cfa79381.json", "b5-idle-replay.json"],
    }, "摘要不符")
    result = {"passed": False, "evidence_valid": True, "stage_passed": False,
              "blocked": True, "b5_closed": False, "b6_started": False,
              "causal_attribution_proven": False, "runs": rows}
    if sealed:
        require((root / "docs-final.json").is_file(), "最终文档门禁缺失")
        gate = load(root / "docs-final.json")
        documents = json.loads(object_bytes(root, gate["documents"]))
        require(gate["argv"] == ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"]
                and gate["exit_code"] == 0 and gate["source_unchanged"] is True
                and gate["source_before"] == gate["source_after"] == second["source_identity_sha256"]
                and gate["runner_sha256"] == sha((root / "seal.py").read_bytes())
                and gate["pid"] > 0
                and datetime.fromisoformat(gate["started_utc"]) > datetime.fromisoformat(second["finished_utc"])
                and datetime.fromisoformat(gate["finished_utc"]) > datetime.fromisoformat(gate["started_utc"])
                and gate["docs_unchanged"] is True
                and gate["docs_before"] == gate["docs_after"] == record.identity(documents)
                and documents == docs_snapshot(record, gate["tools_commit"]), "文档门禁输入身份不符")
        for path in root.iterdir():
            if path.suffix in {".py", ".md"} or path.name == ".gitattributes":
                require(sha(record.canonical(path.read_bytes())) == documents.get(PREFIX + "/" + path.name),
                        "归因工具或说明偏离定稿")
        gate_output = json.loads(object_bytes(root, gate["log"]))
        require(gate_output["passed"] is True
                and not any(row["severity"] == "error" for row in gate_output["diagnostics"]),
                "文档门禁未通过")
        result["docs_gate_passed"] = True
        proof = load(root / "self-negative.json")
        expected_reasons = {"baseline": "", "manifest": "归因证据清单不符",
                            "source": "产品来源身份不符", "command": "命令或标签不符",
                            "summary": "摘要不符"}
        require(proof["passed"] is True
                and [case["case"] for case in proof["cases"]] == list(expected_reasons),
                "自反例集合不符")
        for case in proof["cases"]:
            reason = expected_reasons[case["case"]]
            require(case["exit_code"] == (1 if reason else 0)
                    and case["reason"] == reason and case["stderr"].strip() == reason
                    and case["read_only"] is True
                    and case["before_sha256"] == case["after_sha256"], "自反例结果不符")
        result["self_negative_passed"] = True
        require(load(root / "review.json") == result, "审查结论不符")
    objects = [path for path in (root / "objects/sha256").rglob("*") if path.is_file()]
    require(len(objects) == len({sha(path.read_bytes()) for path in objects})
            and all(path.stem == sha(path.read_bytes()) for path in objects), "内容对象重复或损坏")
    require(before == tree(root), "审计改写证据")
    return result


PREFIX = ROOT.relative_to(REPOSITORY).as_posix()
GENERATED = {"docs-final.json", "review.json", "evidence.json", "self-negative.json"}


def docs_snapshot(record, revision=None):
    if revision:
        names = record.git(REPOSITORY, "ls-tree", "-r", "--name-only", "-z",
                           revision, "--", "docs/DevDocs")
    else:
        names = record.git(REPOSITORY, "ls-files", "-z", "--cached", "--others",
                           "--exclude-standard", "--", "docs/DevDocs")
    paths = sorted({name.decode("utf-8") for name in names.split(b"\0") if name})
    paths = [path for path in paths if "/assets/" not in path or path.startswith(PREFIX + "/")
             and path.removeprefix(PREFIX + "/") not in GENERATED and "/objects/" not in path]
    if not revision:
        return {path: sha(record.canonical((REPOSITORY / path).read_bytes())) for path in paths}
    stream = record.git(REPOSITORY, "cat-file", "--batch",
                        data=b"".join((revision + ":" + path + "\n").encode("utf-8") for path in paths))
    result, offset = {}, 0
    for path in paths:
        end = stream.index(b"\n", offset)
        header = stream[offset:end].split()
        require(header[1] == b"blob", "文档Git对象无效")
        size = int(header[2])
        result[path] = sha(record.canonical(stream[end + 1:end + size + 1]))
        offset = end + size + 2
    require(offset == len(stream), "文档Git对象流长度不符")
    return result


def self_negative(root=ROOT):
    reasons = {"manifest": "归因证据清单不符", "source": "产品来源身份不符",
               "command": "命令或标签不符", "summary": "摘要不符"}
    cases = []
    with tempfile.TemporaryDirectory(prefix="b5-watchdog-negative-") as temp:
        for case in ["baseline", *reasons]:
            copy = Path(temp) / case
            shutil.copytree(root, copy)
            if not (copy / "evidence.json").exists():
                (copy / "evidence.json").write_text(
                    json.dumps({"files": inventory(copy)}, ensure_ascii=False), encoding="utf-8")
            if case != "baseline":
                target = "summary.json" if case in {"manifest", "summary"} else "b5-idle-replay.json"
                value = load(copy / target)
                if case in {"manifest", "summary"}:
                    value["baseline_timed_out"] = True
                elif case == "source":
                    value["expected_product_commit"] = RUNS[0][2]
                else:
                    value["argv"] += ["--", "--test-threads=1"]
                (copy / target).write_text(json.dumps(value, ensure_ascii=False), encoding="utf-8")
                if case != "manifest":
                    (copy / "evidence.json").write_text(json.dumps({"files": inventory(copy)}), encoding="utf-8")
            before = tree(copy)
            run = subprocess.run([sys.executable, "-B", "-X", "utf8", str(Path(__file__).resolve()),
                                  "--root", str(copy), "--unsealed"],
                                 capture_output=True, encoding="utf-8")
            expected = reasons.get(case, "")
            after = tree(copy)
            require(run.returncode == (1 if expected else 0)
                    and run.stderr.strip() == expected and after == before,
                    "自反例拒绝或只读性不符：" + case)
            cases.append({"case": case, "exit_code": run.returncode, "reason": expected,
                          "stderr": run.stderr,
                          "read_only": True, "before_sha256": before, "after_sha256": after})
    return {"passed": True, "cases": cases}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--unsealed", action="store_true")
    parser.add_argument("--self-negative", action="store_true")
    args = parser.parse_args()
    value = self_negative(args.root) if args.self_negative else check(args.root, sealed=not args.unsealed)
    print(json.dumps(value, ensure_ascii=False))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, IndexError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
