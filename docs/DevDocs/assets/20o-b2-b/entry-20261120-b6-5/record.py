"""绑定固定源码和历史证据，并从独立任务根归档 B6-5 实际回执。"""

import argparse
import hashlib
import json
from pathlib import Path
import sys
from tempfile import TemporaryDirectory

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[4]
PREFIX = ROOT.relative_to(REPO).as_posix()
BASE = ROOT.parent.relative_to(REPO).as_posix()
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT.with_name("entry-20261119-b6-4")))

from evidence import blob, collect, digest, git, require

PRODUCT = "e79fab3cdd3ea9d3a7eff31f529dec2498434114"
CLOSEOUT = "69d009e876531c3b17acc6f2a19a2c3d9c51d929"
LOCAL_CHECKS = "780398e5b657b698962e22e1959b7de85272b907"
PERFORMANCE = "79ea0375d7f2d823a5bdde73cb2c9a1c33c3e4c3"
BOUNDARY = "58176ff5299441f921f3b54fbdc300e2e46b9bb0"
PROTECTED = (
    ".gitattributes", ".cargo", ".github/workflows", "LICENSE", "core", "stdlib", "tests",
    "tools", "cli", "resources", "package.json", "bun.lock", "tsconfig.json", "docs/UseDocs",
    "docs/module-registry.json",
)
LOCKS = ("core/rust/Cargo.lock", "tests/benchmarks/Cargo.lock")
DOCS = tuple("docs/DevDocs/" + name for name in (
    "README.md", "20o-official-library-program.md", "20o-b2-b-implementation-plan.md",
    "20o-b2-b6-resource-performance-plan.md", "20o-b2-b6-5-closeout-plan.md",
    "20o-b2-b7-closeout-plan.md", "20o-b2-c-final-closeout-plan.md",
)) + (f"{BASE}/README.md", f"{PREFIX}/README.md")
DOC_INPUTS = (*DOCS, f"{PREFIX}/handoff.json")
HISTORY = {
    "b5": ("entry-20261117-b5-closeout", "closeout.json", "evidence.json"),
    "b6-1": ("entry-20261118-b6-1-rerun", "result.json", "evidence.json", "linux-tool-availability.json"),
    "b6-1-macos": ("entry-20261118-b6-1-macos", "evidence.json"),
    "b6-2": ("entry-20261118-b6-2", "measurements.zip", "summary.json"),
    "b6-2-source": ("entry-20261118-b6-2/review", "source-boundary.json"),
    "b6-2-review": ("entry-20261118-b6-2/review", "negative-cases.json"),
    "b6-3": ("entry-20261118-b6-3", "boundaries.zip", "matrix.json", "summary.json", "repo-check.zip"),
    "b6-3-review": ("entry-20261118-b6-3/review", "negative-cases.json", "macos-artifacts.json"),
    "b6-4": ("entry-20261119-b6-4", "ci.zip", "summary.json", "docs-check.zip"),
    "b6-4-followup": ("entry-20261119-b6-4-followup", "status.json", "followup.zip", "local-checks.zip"),
    "b6-4-review": ("entry-20261119-b6-4-followup", "negative-cases.json"),
}
ENTRYPOINTS = {
    "b6-2-source": ("verify_boundary.py", ["--revision", "58176ff5"]),
    "b6-4": ("evidence.py", []),
}
DRIVER = "core/rust/crates/xiao-driver"
LLVM = "core/rust/crates/xiao-codegen-llvm"
VM = "core/rust/crates/xiao-vm"
IMPACTS = {
    ".github/workflows/b2-b6-boundary.yml": "新增边界采集入口",
    f"{LLVM}/src/artifact.rs": "扩展 Runtime 组件声明与传递依赖。普通原生构建消费声明",
    f"{LLVM}/src/artifact_link_tests.rs": "新增 Linux release 静态库逐导出对账",
    f"{DRIVER}/tests/20o_static_float_equality.rs": "增加四格原生组件观测",
    f"{DRIVER}/examples/b2_b6_performance.rs": "测量入口增加 Rustdoc",
    f"{DRIVER}/examples/numeric_perf/b6_cases.rs": "测量负载增加 Rustdoc",
    f"{VM}/src/semantics/exec.rs": "加载器方法移出",
    f"{VM}/src/semantics/module_exec.rs": "原样接收加载器方法",
    "tools/gates/b2-b6-performance-baseline.json": "登记历史测量快照",
}
COMPONENT_DELTA = {
    ".github/workflows/b2-b6-boundary.yml", f"{LLVM}/src/artifact.rs",
    f"{LLVM}/src/artifact_link_tests.rs", f"{DRIVER}/tests/20o_static_float_equality.rs",
}


def json_bytes(value):
    return (json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode("utf-8")


def emit(path, value):
    with path.open("xb") as output:
        output.write(json_bytes(value))


def git_tree(revision, *roots):
    return git("ls-tree", "-rz", revision, "--", *roots)


def entries(raw):
    result = {}
    for row in raw.split(b"\0")[:-1]:
        header, path = row.split(b"\t", 1)
        mode, kind, object_id = header.decode("ascii").split()
        require(mode in ("100644", "100755") and kind == "blob", "Git 清单包含非普通文件")
        result[path.decode("utf-8")] = object_id
    return result


def source_identity(manifest):
    return {
        "commit": PRODUCT,
        "files": len(entries(manifest)),
        "tree_list_sha256": digest(manifest)["sha256"],
        "locks": {name: digest(blob(PRODUCT, name)) for name in LOCKS},
    }


def git_blob_id(raw):
    return hashlib.sha1(f"blob {len(raw)}\0".encode("ascii") + raw).hexdigest()


def text_identity(raw):
    canonical = raw.decode("utf-8").replace("\r\n", "\n").encode("utf-8")
    return {"blob": git_blob_id(canonical), "utf8_lf": digest(canonical), "raw": digest(raw)}


def recorded_argv(name):
    folder = HISTORY[name][0]
    default = "self_negative.py" if name.endswith("-review") else "audit.py"
    script, args = ENTRYPOINTS.get(name, (default, []))
    return f"{BASE}/{folder}/{script}", args


def history_identity():
    observed = {}
    for name, (folder, *files) in HISTORY.items():
        path = f"{BASE}/{folder}"
        tree = git("rev-parse", f"{PRODUCT}:{path}").decode("ascii").strip()
        script, _ = recorded_argv(name)
        paths = [script, *(f"{path}/{file}" for file in files)]
        identities = {}
        for file in paths:
            raw = blob(PRODUCT, file)
            identities[file] = {"blob": git_blob_id(raw), **digest(raw)}
        observed[name] = {
            "commit": PRODUCT,
            "directory": path,
            "tree": tree,
            "files": identities,
        }
    path = "docs/DevDocs/22d-llvm-concurrency-differential.md"
    raw = blob(PRODUCT, path)
    observed["22d"] = {"commit": PRODUCT, "path": path, "blob": git_blob_id(raw), **digest(raw)}
    return observed


def delta(before, final):
    paths = sorted(name for name in before.keys() | final.keys() if before.get(name) != final.get(name))
    require(set(paths) <= IMPACTS.keys(), "最终树出现未对账的受保护路径")
    return {name: {"before": before.get(name), "after": final.get(name), "impact": IMPACTS[name]} for name in paths}


def product_record():
    manifest = git_tree(PRODUCT, *PROTECTED)
    final = entries(manifest)
    require(manifest == git_tree(LOCAL_CHECKS, *PROTECTED), "最终受保护产品树与 164 导出补验不符")
    return {
        "source": source_identity(manifest),
        "tree": git("cat-file", "-p", PRODUCT).decode("utf-8").splitlines()[0].removeprefix("tree "),
        "local_checks_source": LOCAL_CHECKS,
        "changes": {rev: delta(entries(git_tree(rev, *PROTECTED)), final)
                    for rev in (PERFORMANCE, BOUNDARY)},
    }


def markdown_identity(root):
    files = {path.relative_to(root).as_posix(): git_blob_id(path.read_bytes())
             for path in (root / "docs/DevDocs").rglob("*.md")}
    return {"files": len(files), **digest(json_bytes(files))}


def collect_records(capture, archive):
    recipes = ("capture-linux.py", "run-language.py")
    manifest = (capture / "product.tree").read_bytes()
    require(manifest == git_tree(PRODUCT, *PROTECTED), "采集使用的固定 Git 路径清单不符")
    with TemporaryDirectory(prefix="xiao-b6-5-record-") as task:
        task = Path(task)
        emit(task / "product.json", product_record())
        emit(task / "history.json", history_identity())
        emit(task / "recipes.json", {name: (capture / name).read_text(encoding="utf-8") for name in recipes})
        collect(archive, [("identity", task), *((name, capture / name) for name in ("linux", "stage-audits"))])


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture", type=Path)
    parser.add_argument("archive", type=Path)
    args = parser.parse_args()
    collect_records(args.capture, args.archive)
