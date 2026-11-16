"""B5-0只读来源检查的固定范围与摘要函数。"""
import hashlib
import json
from pathlib import Path
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
ENTRY = "6d32761477dd6e09fe4074a7b8b76a9416c0826c"
PRODUCT = "cfa79381bbae027cec11136ca380e82f66720c34"
B1 = "106b99c70e88a88afd0dd9a33f6a9c792e13324c"
PRODUCT_ROOTS = ["core/rust", "stdlib", "tests/fixtures", "tests/benchmarks", ".github/workflows",
                 ".cargo", "tools", "cli/ts", "docs/UseDocs", "docs/module-registry.json", "package.json",
                 "bun.lock", ".gitattributes"]
B1_ROOTS = ["docs/DevDocs/assets/20o-b1-close", "docs/DevDocs/assets/20o-b1-imp",
            "tests/fixtures/20o-b1"]
ARTIFACT_SUFFIXES = (".xiaoc", ".xar", ".exe", ".o", ".obj", ".bin", ".lib", ".a", ".native-meta", ".xiao.meta")
RUNS = ["41498029910", "41464439400"]
REQUIRED = ["B1 original ABI1.18 .xiaoc", "B1 original ABI1.18 .xar", "B1 original ABI1.18 native metadata"]


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def git(repository, *args):
    return subprocess.check_output(["cmd", "/c", "git", *args], cwd=repository)


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path, value):
    with path.open("x", encoding="utf-8", newline="\n") as output:
        output.write(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def describe(raw):
    return {"bytes": len(raw), "sha256": sha(raw)}


def candidate(name):
    return name.lower().endswith(ARTIFACT_SUFFIXES)


def zip_inventory(path):
    with zipfile.ZipFile(path) as archive:
        return [{"path": row.filename, "bytes": row.file_size, "sha256": sha(archive.read(row))}
                for row in archive.infolist() if not row.is_dir()]


def fixed_paths(repository, commit, roots):
    return sorted(set(git(repository, "ls-tree", "-r", "--name-only", commit, "--", *roots).decode("utf-8").splitlines()))


def source_facts(repository):
    abi_path = "core/rust/crates/xiao-runtime-abi/src/lib.rs"
    codegen_path = "core/rust/crates/xiao-codegen-llvm/src/lib.rs"
    registry_path = "docs/module-registry.json"
    abi = git(repository, "show", ENTRY + ":" + abi_path)
    codegen = git(repository, "show", ENTRY + ":" + codegen_path)
    registry = git(repository, "show", ENTRY + ":" + registry_path)
    changed = git(repository, "diff", "--name-only", PRODUCT, ENTRY, "--", *PRODUCT_ROOTS).decode().splitlines()
    require(not changed and b"ABI_MINOR_VERSION: u32 = 18;" in abi and b"CODEGEN_VERSION: u32 = 21;" in codegen,
            "B5入口版本或产品树不符")
    require(next(row for row in json.loads(registry)["modules"] if row["id"] == "stdlib.cmath")["status"] == "planned",
            "B5公共状态冒进")
    return {"entry_commit": ENTRY, "product_commit": PRODUCT, "b1_commit": B1, "product_diff": changed,
            "runtime_abi": "1.18", "codegen_version": 21, "stdlib_interface": 1, "public_cmath": "planned",
            "fixed_sources": {path: describe(raw) for path, raw in [(abi_path, abi), (codegen_path, codegen), (registry_path, registry)]}}


def inventory(root):
    result = {}
    for path in sorted(root.rglob("*")):
        require(not path.is_symlink() and not getattr(path, "is_junction", lambda: False)(), "B5证据含链接")
        require("__pycache__" not in path.parts, "B5证据含Python缓存")
        if path.is_file() and path.relative_to(root).as_posix() != "evidence.json":
            result[path.relative_to(root).as_posix()] = describe(path.read_bytes())
    return result


def tree_digest(root):
    tree = {path.relative_to(root).as_posix(): describe(path.read_bytes()) if path.is_file() else "directory"
            for path in sorted(root.rglob("*"))}
    return sha(json.dumps(tree, sort_keys=True, separators=(",", ":")).encode("utf-8"))


def docs_snapshot(repository, commit=None):
    prefix = ROOT.relative_to(REPOSITORY).as_posix() + "/"
    args = ["ls-tree", "-r", "--name-only", "-z", commit, "--", "docs/DevDocs"] if commit else [
        "ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", "docs/DevDocs"]
    paths = sorted({row.decode("utf-8") for row in git(repository, *args).split(b"\0") if row})
    paths = [path for path in paths if "/assets/" not in path or
             path.startswith(prefix) and Path(path).suffix in {".py", ".md"}]
    if commit:
        process = subprocess.run(["cmd", "/c", "git", "cat-file", "--batch"], cwd=repository,
                                 input=b"".join((commit + ":" + path + "\n").encode("utf-8") for path in paths),
                                 capture_output=True, check=True)
        stream, offset, blobs = process.stdout, 0, {}
        for path in paths:
            end = stream.index(b"\n", offset)
            header = stream[offset:end].split()
            require(len(header) == 3 and header[1] == b"blob", "B5文档Git对象缺失")
            length = int(header[2])
            blobs[path] = stream[end + 1:end + 1 + length]
            offset = end + length + 2
        require(offset == len(stream), "B5文档Git流长度不符")
    else:
        blobs = {path: (repository / path).read_bytes() for path in paths}
    fixed = {path: sha(raw.replace(b"\r\n", b"\n")) for path, raw in blobs.items()}
    return {"canonical_files": fixed,
            "raw_sha256": sha(json.dumps({path: describe(raw) for path, raw in blobs.items()}, sort_keys=True).encode("utf-8"))}
