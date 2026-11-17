"""只读核验固定产品对象、真实运行日志、文档身份与实际文件清单。"""
import argparse
from datetime import datetime
import gzip
import hashlib
import io
import json
from pathlib import Path, PureWindowsPath
import re
import subprocess
import sys
import tarfile

sys.dont_write_bytecode = True
BASELINE = "cba346fc6f3039cbc76889f4907ea8539c38ff48"
CODE = "67319d61c18b2f02a57f974883c57600f70c724d"
RESOURCE = "core/rust/crates/xiao-driver/tests/20o_b1_imp_resources.rs"
PREFIX = "docs/DevDocs/assets/20o-b2-b/entry-20261117-url-optimization"
PRODUCT_ROOTS = [
    ".cargo", ".gitattributes", ".github/workflows", "core/rust", "stdlib",
    "tests/fixtures", "tests/benchmarks", "tools", "cli/ts", "package.json",
    "bun.lock", "docs/module-registry.json", "docs/UseDocs",
]


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def scan(root):
    return {path.relative_to(root).as_posix():
            {"bytes": len(raw := path.read_bytes()), "sha256": sha(raw)}
            for path in sorted(root.rglob("*")) if path.is_file()}


def git(repository, *args):
    child = subprocess.run(["cmd", "/c", "git", "-C", str(repository), *args],
                           capture_output=True)
    require(child.returncode == 0, "固定Git对象读取失败：" +
            child.stderr.decode("utf-8", errors="replace"))
    return child.stdout


def stored(root, ref):
    path = (root / ref["path"]).resolve()
    require(path.is_relative_to(root), "内容对象路径越界")
    packed = path.read_bytes()
    require(len(packed) == ref["bytes"] and sha(packed) == ref["sha256"],
            "内容对象存储摘要错误：" + ref["path"])
    require(ref["encoding"] in ("identity", "gzip"), "内容对象编码非法")
    raw = gzip.decompress(packed) if ref["encoding"] == "gzip" else packed
    require(len(raw) == ref["raw_bytes"] and sha(raw) == ref["raw_sha256"],
            "内容对象原文摘要错误：" + ref["path"])
    return raw


def archive(repository, commit, paths):
    raw = git(repository, "archive", "--format=tar", commit, "--", *paths)
    with tarfile.open(fileobj=io.BytesIO(raw)) as stream:
        return {item.name: stream.extractfile(item).read()
                for item in stream.getmembers() if item.isfile()}


def verify_sources(snapshot, blobs):
    require(set(snapshot) == set(blobs), "被测源码路径集合与固定Git对象不同")
    for name, content in blobs.items():
        require(snapshot[name]["lf_sha256"] == sha(content.replace(b"\r\n", b"\n")),
                "被测源码与固定Git对象不符：" + name)


def summaries(raw):
    matches = re.findall(
        rb"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; "
        rb"(\d+) measured; (\d+) filtered out; finished in ([0-9.]+)s", raw)
    return [tuple(int(value) for value in row[:5]) + (float(row[5]),) for row in matches]


def audit(root, repository, prepare):
    actual = scan(root)
    manifest = load(root / "evidence.json")
    require(".gitattributes" in actual, "已登记文件缺失：.gitattributes")
    expected = manifest["files"]
    for name, identity in expected.items():
        require(name in actual, "已登记文件缺失：" + name)
        require(actual[name] == identity, "证据文件字节与清单不符：" + name)
    require(set(actual) - {"evidence.json"} == set(expected)
            and manifest["file_count_excluding_manifest"] == len(expected),
            "证据清单文件集合或数量不符")
    manifest_commit = None
    if not prepare:
        manifest_commit = git(repository, "log", "-1", "--format=%H", "--",
                              PREFIX + "/evidence.json").decode().strip()
        require(manifest_commit, "清单尚未提交，不能作为封存证据")
        require(git(repository, "show", manifest_commit + ":" + PREFIX + "/evidence.json")
                == (root / "evidence.json").read_bytes(), "清单字节偏离固定Git对象")
    result = load(root / "result.json")
    require(result["baseline_commit"] == BASELINE and result["code_commit"] == CODE,
            "产品基线或测试提交身份不符")
    changed = git(repository, "diff", "--name-only", BASELINE, CODE, "--",
                  *PRODUCT_ROOTS).decode().splitlines()
    require(changed == [RESOURCE] and result["allowed_code_changes"] == changed,
            "产品差异越出获批测试文件")
    blobs = archive(repository, CODE, PRODUCT_ROOTS)
    source = json.loads(stored(root, result["source"]))
    verify_sources(source, blobs)
    resource = blobs[RESOURCE].decode("utf-8")
    require(".with_timeout(Duration::from_secs(60))" in resource
            and 'let input = "🚀".repeat(16_383) + "%F0%9F%9A%80";' in resource
            and 'let expected = "🚀".repeat(16_384);' in resource
            and '&"%F0%9F%9A%80".repeat(256)' in resource,
            "URL用例表示、黄金输出或60秒窗口变化")
    require(resource.count("#[test]") == 18 and "#[ignore]" not in resource,
            "资源用例数或忽略策略变化")
    workspace = None
    for name in ("workspace-final.json", "clippy-final.json", "fmt-final.json"):
        receipt = load(root / name)
        require(receipt["exit_code"] == 0 and receipt["source_unchanged"],
                "真实命令未通过或源码改变：" + name)
        require(receipt["pid"] > 0 and receipt["elapsed_seconds"] > 0
                and datetime.fromisoformat(receipt["finished_utc"])
                >= datetime.fromisoformat(receipt["started_utc"]), "命令PID或时间不符")
        for key in ("source_before", "source_after"):
            require(json.loads(stored(root, receipt[key])) == source,
                    "命令未绑定同一产品源码：" + name)
        require(receipt["recorder"]["sha256"] == sha((root / "record.py").read_bytes()),
                "采集器字节与真实回执不符")
        env = receipt["environment_overrides"]
        task = PureWindowsPath(env["CARGO_TARGET_DIR"]).parent
        require(task == PureWindowsPath(receipt["cwd"]).parent /
                ".b5/url-case-opt-20261118-01"
                and PureWindowsPath(env["XIAO_STDLIB_ROOT"])
                == PureWindowsPath(receipt["cwd"]) / "stdlib", "任务根或标准库加载根不符")
        for key in ("CARGO_TARGET_DIR", "TEMP", "TMP", "XIAO_HOME"):
            require(PureWindowsPath(env[key]).is_relative_to(task), "任务路径未隔离：" + key)
        require(env["XIAO_B1_F10"] == env["XIAO_B2_A_PERF"] == "0"
                and env["RUSTUP_TOOLCHAIN"] == "1.96.0"
                and "RUST_TEST_THREADS" not in env, "测试模式或工具链不符")
        raw = stored(root, receipt["log"])
        if name == "workspace-final.json":
            require(receipt["argv"] == [
                "cargo", "+1.96.0", "test", "--locked", "--manifest-path",
                "core/rust/Cargo.toml", "--workspace"], "完整默认workspace命令被替换")
            require(receipt["runtime_and_test_binaries_after"], "缺少Runtime/测试二进制身份")
            workspace = receipt
            rows = summaries(raw)
            totals = tuple(sum(row[i] for row in rows) for i in range(3))
            require(len(rows) == 191 and totals == (1754, 0, 55)
                    and not re.search(rb"test result: FAILED|panicked at", raw),
                    "完整workspace实际日志统计不符")
            for marker in ("url_decoding_reaches_the_unicode_result_budget",
                           "url_decoding_resets_utf8_state_between_escaped_codepoints"):
                require(("test " + marker + " ... ok").encode() in raw,
                        "完整workspace缺少URL成功记录：" + marker)
        elif name == "clippy-final.json":
            require(receipt["argv"] == [
                "cargo", "+1.96.0", "clippy", "--locked", "--manifest-path",
                "core/rust/Cargo.toml", "--workspace", "--all-targets", "--", "-D", "warnings"],
                "Clippy命令不符")
        else:
            require(receipt["argv"] == [
                "cargo", "+1.96.0", "fmt", "--manifest-path",
                "core/rust/Cargo.toml", "--all", "--", "--check"], "格式检查命令不符")
    require(result["workspace"] == {
        "receipt": "workspace-final.json", "target_count": 191, "passed": 1754,
        "failed": 0, "ignored": 55, "test_case_count": 1809, "exit_code": 0,
        "elapsed_seconds": workspace["elapsed_seconds"], "pid": workspace["pid"],
    }, "派生workspace统计与真实日志不符")
    for field, expected_passed in (("resource_target", 18), ("unicode_budget_single", 1)):
        item = result[field]
        rows = summaries(stored(root, item["log"]))
        require(len(rows) == 1 and rows[0][0] == expected_passed
                and rows[0][1:3] == (0, 0) and item["test_seconds"] == rows[0][5],
                "补充运行统计不符：" + field)
    require(result["runtime_abi"] == 65555 and result["codegen_version"] == 22
            and result["stdlib_interface_version"] == 1
            and result["input_bytes"] == 65544 and result["expected_utf8_bytes"] == 65536
            and result["expected_codepoints"] == 16384
            and result["separate_utf8_reset_codepoints"] == 256
            and result["driver_timeout_seconds"] == 60, "版本或URL精确预算不符")
    for key in ("url_implementation_changed", "resource_limits_changed",
                "workflow_or_f10_thresholds_changed", "b5_historical_1_19_21_evidence_rewritten",
                "b5_closed", "b6_started", "multi_platform_or_ci_passed"):
        require(result[key] is False, "擅自扩大验收结论：" + key)
    docs = load(root / "docs-final.json")
    require(docs["exit_code"] == 0 and docs["source_unchanged"] and docs["pid"] > 0
            and docs["argv"][1:] == ["tools/repo-check/src/cli.ts", "all", "--format", "json"],
            "完整文档门禁回执未通过或命令不符")
    docs_before = json.loads(stored(root, docs["docs_before"]))
    require(docs_before == json.loads(stored(root, docs["docs_after"])),
            "文档门禁前后字节不同")
    required_docs = {"docs/DevDocs/" + name for name in (
        "20o-b2-b-implementation-plan.md", "20o-b2-b5-version-matrix-plan.md",
        "20o-b2-b6-resource-performance-plan.md", "20o-b2-b7-closeout-plan.md",
        "20o-b2-c-final-closeout-plan.md", "20o-official-library-program.md", "README.md",
    )}
    require(required_docs <= docs_before.keys() and PREFIX + "/README.md" in docs_before,
            "文档门禁没有绑定本批DevDocs")
    doc_blobs = ({name: (repository / name).read_bytes() for name in docs_before}
                if prepare else archive(repository, manifest_commit, list(docs_before)))
    verify_sources(docs_before, doc_blobs)
    for key in ("product_before", "product_after"):
        require(json.loads(stored(root, docs[key])) == source, "文档门禁的产品源码不符")
    report = json.loads(stored(root, docs["log"]))
    require(report["passed"] is True and not any(
        item["severity"] == "error" for item in report["diagnostics"]),
        "文档门禁实际报告未通过")
    return {
        "passed": True, "sealed": not prepare, "files_excluding_manifest": len(expected),
        "baseline_commit": BASELINE, "code_commit": CODE,
        "workspace_passed": 1754, "workspace_failed": 0, "workspace_ignored": 55,
        "resource_tests": 18, "b5_closed": False, "b6_started": False,
        "platform": "Windows本机", "multi_platform_or_ci_passed": False,
    }


def main():
    parser = argparse.ArgumentParser()
    default = Path(__file__).resolve().parent
    parser.add_argument("--root", type=Path, default=default)
    parser.add_argument("--repository-root", type=Path, default=default.parents[4])
    parser.add_argument("--prepare", action="store_true",
                        help="仅封存前预检；不证明清单已提交")
    args = parser.parse_args()
    root = args.root.resolve()
    before = scan(root)
    try:
        result = audit(root, args.repository_root.resolve(), args.prepare)
        code = 0
    except Exception as error:
        result, code = {"passed": False, "reason": str(error)}, 1
    result["read_only"] = scan(root) == before
    if not result["read_only"]:
        result.update(passed=False, reason="审计修改了证据目录")
        code = 1
    sys.stdout.reconfigure(encoding="utf-8")
    print(json.dumps(result, ensure_ascii=False))
    return code


if __name__ == "__main__":
    raise SystemExit(main())
