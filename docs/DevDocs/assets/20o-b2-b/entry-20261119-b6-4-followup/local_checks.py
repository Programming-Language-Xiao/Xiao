"""复核 780398e5 的 Linux ABI 导出对账和 Windows 定向 Clippy 回执。"""

from datetime import datetime
from io import BytesIO
from pathlib import PurePosixPath, PureWindowsPath
import re
from zipfile import ZipFile

from evidence import Evidence, LOCKS, blob, digest, git, require


COMMIT = "780398e5b657b698962e22e1959b7de85272b907"
ARCHIVE_SHA256 = "70718a0fc01050804846fe31a30d3320c2a5c2ef282ef5b4374288688fe2fe8f"
SOURCE_PATHS = (".gitattributes", ".cargo", "core/rust", "tests", "stdlib", "docs/UseDocs", "docs/module-registry.json")
CODEGEN = "core/rust/crates/xiao-codegen-llvm/src"
INPUTS = (".cargo/config.toml", *LOCKS, f"{CODEGEN}/artifact.rs", f"{CODEGEN}/artifact_link_tests.rs")
TARGETS = {"linux": "x86_64-unknown-linux-gnu", "windows": "x86_64-pc-windows-msvc"}
LLVM_TOOLS = ("XIAO_CLANG", "XIAO_LLVM_AS", "XIAO_LLVM_NM")
TEST = "artifact::link_tests::runtime_components_match_release_archive_exports"
LOCKED = ("--locked", "--manifest-path", "core/rust/Cargo.toml")
PACKAGE = ("-p", "xiao-codegen-llvm")
BUILD = ("build", *LOCKED, "--release", "-p", "xiao-runtime")
EXPORTS = ("test", *LOCKED, *PACKAGE, "--lib", "--", "--ignored", "--exact", TEST, "--nocapture")
CLIPPY = ("clippy", *LOCKED, *PACKAGE, "--all-targets", "--", "-D", "warnings")


def source_identity():
    manifest = git("ls-tree", "-rz", COMMIT, "--", *SOURCE_PATHS)
    return {"files": manifest.count(b"\0"), "manifest": digest(manifest),
            "inputs": {name: digest(blob(COMMIT, name)) for name in INPUTS}}


def valid_digest(item):
    return item is not None and item["bytes"] > 0 and re.fullmatch(r"[0-9a-f]{64}", item["sha256"]) is not None


def verify_environment(bundle, host, source, tree):
    environment = bundle.read(f"{host}/environment.json")
    export = environment["source_export"]
    expected_export = ["cmd", "/c", "git", "-c", "core.autocrlf=false", "-c", "core.eol=lf", "archive", "--format=tar"]
    argv = export["argv"]
    require((export["commit"], export["tree"]) == (COMMIT, tree) and argv[:len(expected_export)] == expected_export
            and argv[len(expected_export)].startswith("--output=")
            and argv[len(expected_export) + 1:] == [COMMIT, *SOURCE_PATHS]
            and environment["source_before"] == source == bundle.read(f"{host}/source-after.json"),
            f"补验固定源码身份不符：{host}")

    path = PurePosixPath if host == "linux" else PureWindowsPath
    root = path(environment["task_root"])
    settings, tools = environment["environment"], environment["tools"]
    temp_names = ("TMPDIR",) if host == "linux" else ("TEMP", "TMP")
    require(environment["host"] == host and environment["machine"].lower() in ("x86_64", "amd64")
            and environment["kernel"] and environment["os_version"] and root.is_absolute()
            and path(environment["source_root"]) == root / "source"
            and path(settings["CARGO_TARGET_DIR"]) == root / "target"
            and all(path(settings[name]) == root / "tmp" for name in temp_names)
            and settings["RUSTUP_TOOLCHAIN"] == "1.96.0"
            and all(settings[name.upper()] == tools[name]["path"] for name in ("rustc", "rustdoc")),
            f"补验平台或构建路径不符：{host}")

    extra_tools = (*LLVM_TOOLS, "ld") if host == "linux" else ("cargo-clippy", "clippy-driver")
    require(set(tools) == {"cargo", "rustc", "rustdoc", *extra_tools}
            and all(valid_digest(tool) and path(tool["path"]).is_absolute()
                    and path(tool["resolved_path"]).is_absolute() and tool["version"] for tool in tools.values())
            and all(tools[name]["version"].startswith(f"{name} 1.96.0 ") for name in ("cargo", "rustc", "rustdoc"))
            and all(f"host: {TARGETS[host]}" in tools[name]["version"].splitlines() for name in ("cargo", "rustc")),
            f"补验工具身份不符：{host}")
    if host == "linux":
        require(settings["XIAO_TARGET_TRIPLE"] == TARGETS[host]
                and path(settings["XIAO_RUNTIME_LIBRARY"]) == root / "target/release/libxiao_runtime.a"
                and all(settings[name] == tools[name]["path"] and "21.1.8" in tools[name]["version"]
                        for name in LLVM_TOOLS), "Linux Runtime 路径或 LLVM 身份不符")
    else:
        require(all(tools[name]["version"].startswith("clippy 0.1.96 ") for name in extra_tools),
                "Windows Clippy 版本不符")
    return environment


def verify_command(bundle, host, stem, command, environment):
    prefix = f"{host}/{stem}"
    receipt = bundle.read(prefix + ".json")
    require(receipt["argv"] == [environment["tools"]["cargo"]["path"], *command]
            and receipt["cwd"] == environment["source_root"] and receipt["pid"] > 0 and receipt["exit"] == 0
            and datetime.fromisoformat(receipt["started_utc"]) < datetime.fromisoformat(receipt["finished_utc"])
            and receipt["outputs"] == {stream: bundle.entries[f"{prefix}.{stream}"] for stream in ("stdout", "stderr")},
            f"补验命令或原始输出回执不符：{prefix}")
    return receipt


def verify_local_checks(archive):
    source = source_identity()
    tree = git("show", "-s", "--format=%T", COMMIT).decode("ascii").strip()
    raw = archive.read_bytes()
    with ZipFile(BytesIO(raw)) as zipped:
        bundle = Evidence(zipped)
        linux = verify_environment(bundle, "linux", source, tree)
        build = verify_command(bundle, "linux", "build-runtime", BUILD, linux)
        test = verify_command(bundle, "linux", "exports", EXPORTS, linux)
        runtime = build["runtime_after"]
        require(build["runtime_before"] is None and valid_digest(runtime)
                and runtime == test["runtime_before"] == test["runtime_after"]
                and datetime.fromisoformat(build["finished_utc"]) <= datetime.fromisoformat(test["started_utc"])
                and b"Compiling xiao-runtime " in bundle.raw("linux/build-runtime.stderr")
                and b"Finished `release` profile" in bundle.raw("linux/build-runtime.stderr"),
                "Linux release Runtime 构建或链接身份不符")
        output = bundle.raw("linux/exports.stdout").decode("utf-8")
        exports = re.findall(r"^RUNTIME-COMPONENTS exports=(\d+) target=(\S+) result=passed$", output, re.M)
        require(exports == [("164", TARGETS["linux"])] and f"test {TEST} ... ok" in output
                and "test result: ok. 1 passed; 0 failed; 0 ignored;" in output, "Linux 导出对账输出不符")

        windows = verify_environment(bundle, "windows", source, tree)
        clippy = verify_command(bundle, "windows", "clippy", CLIPPY, windows)
        clippy_output = bundle.raw("windows/clippy.stderr")
        require(clippy["runtime_before"] is None and clippy["runtime_after"] is None
                and b"Checking xiao-codegen-llvm " in clippy_output and b"Finished `dev` profile" in clippy_output,
                "Windows 定向 Clippy 输出不符")
        result = {"commit": COMMIT, "files": len(bundle.entries), "objects": bundle.unique,
                  "linux_abi_exports": {"exports": int(exports[0][0]), "target": exports[0][1], "runtime": runtime},
                  "windows_clippy": {"target": TARGETS["windows"], "package": "xiao-codegen-llvm", "all_targets": True}}
    require(digest(raw)["sha256"] == ARCHIVE_SHA256, "补验归档与封存摘要不符")
    return result
