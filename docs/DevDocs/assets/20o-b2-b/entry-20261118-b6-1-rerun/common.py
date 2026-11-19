"""B6-1重跑的固定范围、实际字节身份与内容寻址对象。"""
import gzip
import hashlib
import json
import os
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
CODE = "43f18274175ba09ac84e759999c40cde72eae1c5"
ENTRY = "1b95fe9b"
PREFIX = "docs/DevDocs/assets/20o-b2-b/entry-20261118-b6-1-rerun"
WINDOWS_TASK = "D:/project/Xiao/.b6/b6-1-rerun-20261118-01/windows"
LINUX_TASK = "/home/xiaoczx/.b6/b6-1-rerun-20261118-01"
PRODUCT_ROOTS = [
    ".cargo", ".gitattributes", ".github/workflows", "core/rust", "stdlib",
    "tests/fixtures", "tests/benchmarks", "tools", "cli/ts", "package.json",
    "bun.lock", "docs/module-registry.json", "docs/UseDocs",
]
DOC_FILES = ["docs/DevDocs/" + name for name in (
    "20o-b2-b6-1-rerun-plan.md", "20o-b2-b6-resource-performance-plan.md",
    "20o-b2-b7-closeout-plan.md", "20o-official-library-program.md", "README.md",
)]
DRIVER_TARGETS = [
    "20o_b2_b6_resources", "20o_b2_a_resources", "20o_b2_a_contract",
    "20o_b2_b2_divide", "20o_b2_b4_cmath_vm", "20o_b2_b4_cmath_library",
    "20o_b2_b4_cmath_boundary", "20o_b2_b4_numeric_range_routing",
]
NATIVE_TESTS = {
    "wrappers": ("20o_b2_b4_cmath_library", "cmath_library_matches_native_o0_o3"),
    "native-core": ("20o_b2_b4_cmath_llvm", "native_cmath_static_dynamic_o0_o3_and_shared_pow"),
    "native-errors": ("20o_b2_a_contract", "a5_language_values_match_native_golden_outputs"),
    "native-divide": ("20o_b2_b2_divide", "lfloat_divide_matches_native_golden_outputs"),
}
STAGES = ["runtime-build", "wrappers", "runtime-tests", "vm-regressions",
          "codegen-tests", "driver-regressions", "native-core", "native-errors",
          "native-divide", "fmt"]


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(",", ":")).encode("utf-8")


def digest(raw):
    return {"bytes": len(raw), "sha256": sha(raw)}


def identity(path):
    path = Path(path)
    return {"path": str(path.resolve()), **digest(path.read_bytes())}


def write_new(path, value):
    with Path(path).open("xb") as output:
        output.write((json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode("utf-8"))


def read(root, name):
    return json.loads((Path(root) / name).read_text(encoding="utf-8"))


def inventory(root):
    return {p.relative_to(root).as_posix(): digest(p.read_bytes())
            for p in sorted(Path(root).rglob("*")) if p.is_file()}


def source_snapshot(source):
    files = {}
    for name in PRODUCT_ROOTS:
        base = Path(source) / name
        paths = [base] if base.is_file() else sorted(base.rglob("*"))
        for path in paths:
            if path.is_file():
                raw = path.read_bytes()
                binary = b"\0" in raw
                files[path.relative_to(source).as_posix()] = {
                    **digest(raw), "binary": binary,
                    "lf_sha256": sha(raw if binary else raw.replace(b"\r\n", b"\n")),
                }
    return files


def store(raw):
    # 对象名由原文确定，同一内容只落一个对象；两平台共享此目录。
    raw_sha = sha(raw)
    packed = gzip.compress(raw, mtime=0) if len(raw) > 8192 else raw
    if len(raw) > 8192:
        packed = packed[:9] + b"\xff" + packed[10:]
    path = ROOT / "objects/sha256" / raw_sha[:2] / (raw_sha + ".bin")
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        with path.open("xb") as output:
            output.write(packed)
    except FileExistsError:
        require(path.read_bytes() == packed, "RERUN内容对象损坏或重复原文编码不一致")
    return {"path": path.relative_to(ROOT).as_posix(), **digest(packed),
            "encoding": "gzip" if len(raw) > 8192 else "identity",
            "raw_bytes": len(raw), "raw_sha256": raw_sha}


def unpack(root, ref):
    path = (Path(root) / ref["path"]).resolve()
    require(path.is_relative_to(Path(root).resolve()), "RERUN内容对象路径越界")
    packed = path.read_bytes()
    require(digest(packed) == {k: ref[k] for k in ("bytes", "sha256")},
            "RERUN内容对象摘要不符")
    require(ref["encoding"] in {"gzip", "identity"}, "RERUN内容对象编码不符")
    raw = gzip.decompress(packed) if ref["encoding"] == "gzip" else packed
    require(len(raw) == ref["raw_bytes"] and sha(raw) == ref["raw_sha256"],
            "RERUN内容对象原文摘要不符")
    return raw


def command(stage):
    cargo = ["cargo", "+1.96.0"]
    base = ["--locked", "--manifest-path", "core/rust/Cargo.toml"]
    if stage == "runtime-build":
        return cargo + ["build", *base, "-p", "xiao-runtime", "--release"]
    if stage in NATIVE_TESTS:
        target, name = NATIVE_TESTS[stage]
        return cargo + ["test", *base, "-p", "xiao-driver", "--release",
                        "--test", target, "--", "--ignored", "--exact", name, "--nocapture"]
    if stage in {"runtime-tests", "codegen-tests"}:
        package = "xiao-runtime" if stage == "runtime-tests" else "xiao-codegen-llvm"
        return cargo + ["test", *base, "-p", package, "--release", "--", "--nocapture"]
    if stage in {"vm-regressions", "driver-regressions"}:
        targets = DRIVER_TARGETS if stage == "vm-regressions" else [
            "20o_b2_b4_cmath_llvm", "n0_b_dynamic_native"]
        args = [arg for target in targets for arg in ("--test", target)]
        return cargo + ["test", *base, "-p", "xiao-driver", "--release", *args,
                        "--", "--nocapture"]
    if stage == "clippy":
        return cargo + ["clippy", *base, "-p", "xiao-driver", "-p", "xiao-codegen-llvm",
                        "--release", "--all-targets", "--", "-D", "warnings"]
    if stage == "fmt":
        return cargo + ["fmt", "--manifest-path", "core/rust/Cargo.toml", "--all", "--", "--check"]
    raise ValueError("未知RERUN阶段：" + stage)
