"""从实际 CI 作业、原始输出和固定 Git 对象核对 C4/C5。"""

import json
import re

from entry_audit import ANCHOR, CANDIDATE_RUNNER, PREFIX, PRODUCT, Evidence, base, blob
from performance_audit import TARGETS, identity

FIRST_STATE = "fc015cba2d96a9d796aeef5f9b01222b5e23ce96"
MIDDLE_STATE = "215fcb388c2899dbbe493b02ba594337017e0e6a"
STATE = "7753fd69c0158a9a61a76ddc79cd65296df67bbc"
FIRST_RUNNER = "4f3d705ba487fbb84d885a294f59678ad8d49d43"
MIDDLE_RUNNER = "bc219c29bbf411539a9fee244405772a24f6db90"
RUNNER = "3cf0c8390ef4b1682cb5489ebfd5f1f01f7934a3"
CHECKOUTS = {FIRST_RUNNER: FIRST_STATE, MIDDLE_RUNNER: MIDDLE_STATE, RUNNER: STATE}
MAINTENANCE = ".github/workflows/maintenance-regression.yml"
WORKSPACE = ".github/workflows/workspace-gates.yml"
RUNS = {
    45022312950: (FIRST_STATE, MAINTENANCE),
    45022312960: (FIRST_STATE, WORKSPACE),
    45022345634: (FIRST_RUNNER, MAINTENANCE),
    45022345662: (FIRST_RUNNER, WORKSPACE),
    45022879989: (MIDDLE_STATE, MAINTENANCE),
    45023049586: (MIDDLE_STATE, WORKSPACE),
    45022888476: (MIDDLE_RUNNER, MAINTENANCE),
    45022888537: (MIDDLE_RUNNER, WORKSPACE),
    45024197031: (STATE, MAINTENANCE),
    45024198150: (STATE, WORKSPACE),
    45024200879: (RUNNER, MAINTENANCE),
    45024200820: (RUNNER, WORKSPACE),
}
NATIVE = {f"Native differential regression ({label})": platform for label, platform in
          (("Windows", "windows"), ("Linux", "linux"), ("macOS", "macos"))}
DEFAULT = "Rust workspace default tests (Linux)"
REPOSITORY = "Linux workspace gates"
MAINTENANCE_JOBS = {DEFAULT, "Deterministic artifact size regression (Windows baseline)",
                    "Deterministic parser and artifact security regression", *NATIVE}
TUTORIAL = "docs/UseDocs/library/cmath.md"
TUTORIAL_FIXTURE = "tests/fixtures/docs/numeric/cmath-tutorial"
REGISTRY = "docs/module-registry.json"
DRIVER_TESTS = "core/rust/crates/xiao-driver/tests/"
STATE_CHANGES = {
    TUTORIAL, REGISTRY, "docs/UseDocs/library/README.md", "stdlib/README.md", "stdlib/cmath.md",
    "docs/UseDocs/language/basics/p1-expressions/operators.md",
    "docs/UseDocs/language/basics/types/conversions.md",
    f"{TUTORIAL_FIXTURE}.xiao", f"{TUTORIAL_FIXTURE}.stdout",
    *(DRIVER_TESTS + name for name in ("20o_b2_b3_wrapper.rs", "20o_b2_b4_cmath_library.rs", "20o_b2_a_support/mod.rs",
                                     "support/b2_divide.rs", "support/b4_cmath_library_native.rs")),
}
FEATURES = {
    "autocfg": ("1.5.1", ""),
    "bigdecimal": ("0.4.11", "std"),
    "libm": ("0.2.16", "arch,default"),
    "num-bigint": ("0.4.8", "std"),
    "num-integer": ("0.1.47", "i128,std"),
    "num-traits": ("0.2.19", "i128,std"),
}
COUNTS = re.compile(r"^test result: (?:ok\.|FAILED\.) (\d+) passed; (\d+) failed; (\d+) ignored;", re.M)
EXACT = "test result: ok. 1 passed; 0 failed; 0 ignored;"


def changes(before, after, *paths):
    raw = base.git("diff", "--name-only", "-z", before, after, "--", *paths)
    return set(raw.decode().split("\0")) - {""}


def tree(commit, *paths):
    return base.git("ls-tree", "-rz", commit, "--", *paths)


def header(commit):
    return base.git("cat-file", "-p", commit).splitlines()


def status_identity():
    return {"commit": STATE, "tree": header(STATE)[0].decode().removeprefix("tree "),
            "docs_tree_sha256": base.sha(tree(STATE, "docs", "stdlib")),
            "locks": {path: base.sha(blob(STATE, path)) for path in base.LOCKS}}


def verify_status(root):
    base.require(changes(PRODUCT, STATE, *base.PRODUCT_ROOTS) == STATE_CHANGES,
                 "C4 产品与状态提交差异越界")
    base.require(all(path in STATE_CHANGES or path.startswith(PREFIX + "/") for path in changes(ANCHOR, STATE)),
                 "C4 改动超出状态登记、夹具及当期证据")
    base.require(header(FIRST_STATE)[1] == f"parent {ANCHOR}".encode()
                 and header(MIDDLE_STATE)[1] == f"parent {FIRST_STATE}".encode()
                 and changes(FIRST_STATE, MIDDLE_STATE) == {DRIVER_TESTS + "20o_b2_b4_cmath_library.rs",
                                                           DRIVER_TESTS + "support/b4_cmath_library_native.rs"}
                 and header(STATE)[1] == f"parent {MIDDLE_STATE}".encode()
                 and changes(MIDDLE_STATE, STATE) == {DRIVER_TESTS + "20o_b2_a_support/mod.rs",
                                                     DRIVER_TESTS + "20o_b2_b3_wrapper.rs"},
                 "C4 状态及教程断言修正身份不符")
    original = json.loads(blob(PRODUCT, REGISTRY))
    released = json.loads(blob(STATE, REGISTRY))
    base.require(original.keys() == released.keys(), "登记表结构改变")
    module = None
    for before, after in zip(original["modules"], released["modules"], strict=True):
        if before["id"] == "stdlib.cmath":
            base.require(before["status"] == "planned" and after["id"] == before["id"]
                         and after["status"] == "verified" and after["stage"] == "20O-B2-C"
                         and after["code"] and after["tests"] and TUTORIAL in after["usedocs"],
                         "cmath 登记的状态或验收范围不符")
            for path in after["code"] + after["tests"] + after["usedocs"]:
                base.require(bool(blob(STATE, path)), f"cmath 登记的实际文件缺失：{path}")
            module = after
        else:
            base.require(before == after, f"既有登记改变：{before['id']}")
    base.require(module is not None and all(original[key] == released[key] for key in original if key != "modules"),
                 "登记表的元数据或 cmath 项缺失")

    tutorial = blob(STATE, TUTORIAL).decode().replace("\r\n", "\n")
    frontmatter = tutorial.split("---", 2)[1].splitlines()
    base.require([line for line in frontmatter if line.startswith("status: ")] == ["status: verified"]
                 and "stage: 20O-B2-C" in frontmatter, "教程状态与登记不符")
    examples = {}
    for extension, fence in (("xiao", "xiao"), ("stdout", "text")):
        archived = f"cmath-tutorial.{extension}"
        fixture = f"{TUTORIAL_FIXTURE}.{extension}"
        candidate = blob(CANDIDATE_RUNNER, f"{PREFIX}/{archived}")
        base.require(blob(STATE, fixture) == candidate == (root / archived).read_bytes()
                     and fixture in module["tests"]
                     and f"```{fence}\n{candidate.decode()}```" in tutorial,
                     f"教程、夹具和固定候选未逐字对应：{extension}")
        examples[fixture] = identity(candidate)
    converted = "docs/UseDocs/language/basics/types/conversions.md"
    operators = "docs/UseDocs/language/basics/p1-expressions/operators.md"
    for path in (converted, operators):
        blocks = r"^```(?:xiao|text)\n.*?^```$"
        base.require(re.findall(blocks, blob(PRODUCT, path).decode(), re.M | re.S)
                     == re.findall(blocks, blob(STATE, path).decode(), re.M | re.S),
                     f"已有语言示例字节变化：{path}")
    return status_identity() | {
        "files": len(base.tracked(tree(STATE, *base.PRODUCT_ROOTS))),
        "tree_list_sha256": base.sha(tree(STATE, *base.PRODUCT_ROOTS)),
        "changed_paths": sorted(STATE_CHANGES), "examples": examples, "cmath": module["status"],
    }


def clean_log(raw):
    text = re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", raw.decode("utf-8-sig", "replace"))
    return re.sub(r"^\ufeff?\d{4}-\d\d-\d\dT[\d:.]+Z ", "", text.replace("\r\n", "\n"), flags=re.M)


def runs(book):
    effective = {}
    history = {}
    state = status_identity()
    for number, (revision, path) in RUNS.items():
        prefix = f"ci/{number}"
        attempts = sorted(int(match[1]) for row in book.entries
                          if (match := re.fullmatch(f"{prefix}/run-(\\d+).json", row)))
        base.require(bool(attempts) and attempts == list(range(1, len(attempts) + 1)), f"CI 尝试缺失：{number}")
        expected = MAINTENANCE_JOBS if path == MAINTENANCE else {REPOSITORY}
        results = {}
        history[str(number)] = []
        for attempt in attempts:
            run = book.read(f"{prefix}/run-{attempt}.json")
            jobs = book.read(f"{prefix}/jobs-{attempt}.json")["jobs"]
            base.require(run["id"] == number and run["head_sha"] == revision and run["path"] == path
                         and run["run_attempt"] == attempt and run["status"] == "completed"
                         and {job["name"] for job in jobs} <= expected, f"工程门禁运行身份不符：{number}/{attempt}")
            actual = {}
            for job in jobs:
                name = job["name"]
                if attempt > 1 and job["started_at"] < run["run_started_at"]:
                    base.require(name in results, f"没有可复核的前次作业：{name}")
                    continue
                job_id = job["id"]
                base.require(name not in actual and job["run_id"] == number and job["head_sha"] == revision
                             and job["run_attempt"] == attempt and job["status"] == "completed"
                             and run["run_started_at"] <= job["started_at"] < job["completed_at"],
                             f"工程门禁实际作业身份不符：{job_id}")
                raw = book.raw(f"{prefix}/{job_id}.log")
                log = clean_log(raw)
                base.require("Current runner version:" in log and CHECKOUTS.get(revision, revision) in log,
                             f"工程门禁原始作业日志不符：{job_id}")
                row = {"job": job_id, "attempt": attempt, "conclusion": job["conclusion"], "log": identity(raw)}
                actual[name] = row
                results[name] = (row, job, log)
            history[str(number)].append({"attempt": attempt, "conclusion": run["conclusion"], "jobs": actual})
        base.require(results.keys() == expected, f"工程门禁作业集合不符：{number}")
        if revision == RUNNER:
            base.require(run["conclusion"] == "success", f"C4 最终补验未通过：{number}")
            for name, (row, job, log) in results.items():
                base.require(row["conclusion"] == "success" and name not in effective,
                             f"C4 最终作业未通过或重复：{name}")
                markers = [(phase, json.loads(value)) for phase, value in re.findall(
                    r"^B2-C-STATE (before|after) (\{.*\})$", log, re.M)]
                base.require(markers == [(phase, state) for phase in ("before", "after")],
                             f"C4 作业前后源码、文档或锁身份不符：{name}")
                effective[name] = (job, log)
    base.require(effective.keys() == MAINTENANCE_JOBS | {REPOSITORY}, "最终工程门禁作业集合不符")
    return effective, history


def verify_runner(before, runner):
    base.require(header(runner)[1] == f"parent {before}".encode()
                 and changes(before, runner) == {MAINTENANCE, WORKSPACE}, "取证 Runner 差异越界")
    for path in (MAINTENANCE, WORKSPACE):
        original = [row for row in blob(before, path).decode().splitlines() if row]
        supplemented = blob(runner, path).decode()
        sequential = iter(supplemented.splitlines())
        base.require(all(any(line == expected for line in sequential) for expected in original)
                     and f"ref: '{before}'" in supplemented
                     and supplemented.count("timeout-minutes:") == sum("timeout-minutes:" in row for row in original),
                     "Runner 未保留现有门禁或时限")


def verify_engineering(archive, dependencies):
    for runner, checkout in CHECKOUTS.items():
        verify_runner(checkout, runner)
    effective, history = runs(Evidence(archive))
    expected_dependencies = {name: row["version"] for name, row in dependencies["packages"].items()}
    base.require({name: info[0] for name, info in FEATURES.items()} == expected_dependencies,
                 "F-7 实测版本与冻结包不符")
    native = {}
    for name, host in NATIVE.items():
        job, log = effective[name]
        tree_rows = set(re.findall(r"^([a-z-]+) v([\d.]+):([a-z0-9,]*)(?: \(\*\))?$", log, re.M))
        expected_rows = {(name, version, features) for name, (version, features) in FEATURES.items()}
        base.require(tree_rows == expected_rows and "F7-CARGO command=tree locked=true" in log
                     and "rustc 1.96.0 " in log and f"host: {TARGETS[host]}" in log,
                     f"F-7 实际闭包、特性或平台工具链不符：{host}")
        runtime = re.findall(r"^B2-C-RUNTIME ([a-f0-9]{64})$", log, re.M)
        levels = {int(level) for level in re.findall(
            r"^B4-4-NATIVE path=tutorial\.cmath optimization=([03]) .* stdout=golden errors=009 result=passed$",
            log, re.M)}
        base.require(len(runtime) == 1 and levels == {0, 3} and EXACT in log
                     and "B2-C-TUTORIAL vm,xiaoc,xar=passed source_removed=true" in log
                     and "test cmath_library_matches_native_o0_o3 ... ok" in log,
                     f"C4 三平台精确教程或 Runtime 回执不符：{host}")
        native[host] = {"job": job["id"], "attempt": job["run_attempt"], "target": TARGETS[host],
                        "runtime_sha256": runtime[0], "tutorial": ["vm", "xiaoc", "xar", "o0", "o3"]}
    job, log = effective[DEFAULT]
    command = "##[group]Run cargo test --locked --manifest-path core/rust/Cargo.toml --workspace"
    base.require(log.count(command) == 1, "未实际执行完整默认工作区")
    default = log.split(command, 1)[1].split("##[group]Run ", 1)[0]
    counts = [tuple(map(int, result)) for result in COUNTS.findall(default)]
    base.require(len(counts) == 192 and all(row[1] == 0 for row in counts)
                 and "F7-CARGO command=test locked=true" in default
                 and "10Z-PERFORMANCE-DRIVER self-test" in log, "默认工作区或基准门禁不完整")

    _, log = effective[REPOSITORY]
    markers = (
        "bun install v1.4.1 ", "-- cargo fmt --check --", "-- bun run check --",
        "-- bun run check:coverage --", "-- bunx tsc --noEmit --", "-- benchmark cargo check --",
        "-- benchmark cargo test --no-fail-fast --", "-- benchmark performance self-test --",
        "-- xiao-driver cargo test --no-fail-fast --", "-- bun test --",
        "F7-CARGO command=clippy locked=true", "10Z-PERFORMANCE-DRIVER self-test",
    )
    base.require(all(value in log for value in markers), "最终仓库或文档门禁缺少实际执行输出")
    base.require(b"bun install --frozen-lockfile" in blob(STATE, "tools/gates/run.sh")
                 and "##[group]Run bash tools/gates/run.sh" in log, "Bun 锁定仓库门禁没有使用固定入口")
    return {"runner": RUNNER, "runs": history, "native": native, "workspace": {
        "job": job["id"], "attempt": job["run_attempt"], "targets": len(counts),
        "passed_failed_ignored": [sum(values) for values in zip(*counts)]},
        "repository_job": effective[REPOSITORY][0]["id"], "dependency_features": FEATURES}
