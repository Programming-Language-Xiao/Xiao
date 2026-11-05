"""为CI安装任务级Cargo代理：锁定依赖并清理数值构建环境，不改宿主配置。"""

import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import uuid


LOCKED_COMMANDS = {
    "build", "check", "test", "clippy", "bench", "doc", "metadata", "fetch", "tree",
    "package", "publish", "update", "generate-lockfile", "rustc", "rustdoc", "run", "install",
}


def prepare(arguments, inherited):
    arguments = list(arguments)
    removed = sorted(key for key in inherited if key.startswith("RUST_BIGDECIMAL_"))
    environment = {key: value for key, value in inherited.items() if key not in removed}
    index = 0
    while index < len(arguments):
        argument = arguments[index]
        if argument.startswith("+"):
            index += 1
        elif argument in {"--color", "--config", "--manifest-path", "--target-dir"}:
            index += 2
        elif argument.startswith("-"):
            index += 1
        else:
            break
    command = arguments[index] if index < len(arguments) else "version-or-help"
    prefix = arguments[:arguments.index("--")] if "--" in arguments else arguments
    locked = command in LOCKED_COMMANDS
    if locked and "--locked" not in prefix:
        arguments.insert(index + 1, "--locked")
    return arguments, environment, command, locked, removed


def run(delegate, arguments, inherited):
    arguments, environment, command, locked, removed = prepare(arguments, inherited)
    print(f"F7-CARGO command={command} locked={str(locked).lower()} removed={len(removed)}", file=sys.stderr, flush=True)
    return subprocess.run([*delegate, *arguments], env=environment).returncode


def install():
    delegate = shutil.which("cargo")
    if delegate is None:
        raise SystemExit("安装Cargo控制代理前必须完成Rust工具链设置")
    directory = Path(os.environ["RUNNER_TEMP"]) / ("xiao-controlled-cargo-" + uuid.uuid4().hex)
    directory.mkdir(parents=True, exist_ok=False)
    script = str(Path(__file__).resolve())
    if os.name == "nt":
        wrapper = directory / "cargo.cmd"
        wrapper.write_text(f'@"{sys.executable}" "{script}" --delegate "{delegate}" %*\n@exit /b %errorlevel%\n', encoding="utf-8", newline="\r\n")
    else:
        wrapper = directory / "cargo"
        wrapper.write_text("#!/bin/sh\nexec " + " ".join(shlex.quote(value) for value in [sys.executable, script, "--delegate", delegate]) + ' "$@"\n', encoding="utf-8", newline="\n")
        wrapper.chmod(0o755)
    with Path(os.environ["GITHUB_PATH"]).open("a", encoding="utf-8", newline="\n") as path_file:
        path_file.write(str(directory) + "\n")
    print(json.dumps({"cargo_control_directory": str(directory), "delegate": delegate,
                      "scope": "仅当前CI作业后续步骤；不改全局环境或Cargo配置"}, ensure_ascii=True))


def self_test(directory):
    directory = Path(directory).resolve()
    directory.mkdir(parents=True, exist_ok=False)
    fake = directory / "cargo-probe.py"
    output = directory / "observed.json"
    fake.write_text('import json,os,sys\nfrom pathlib import Path\nPath(os.environ["F7_PROBE_OUTPUT"]).write_text(json.dumps({"argv":sys.argv[1:],"numeric_environment":[k for k in os.environ if k.startswith("RUST_BIGDECIMAL_")],"sentinel":os.environ.get("F7_PROBE_SENTINEL")}),encoding="utf-8")\nraise SystemExit(int(os.environ["F7_PROBE_EXIT"]))\n', encoding="utf-8", newline="\n")
    cases = [
        (["+1.96.0", "test", "--", "--locked"], ["+1.96.0", "test", "--locked", "--", "--locked"], 17),
        (["build", "--locked", "--release"], ["build", "--locked", "--release"], 0),
        (["--version"], ["--version"], 0),
    ]
    for arguments, expected, exit_code in cases:
        inherited = dict(os.environ, RUST_BIGDECIMAL_DEFAULT_PRECISION="invalid", RUST_BIGDECIMAL_UNKNOWN="invalid",
                         F7_PROBE_OUTPUT=str(output), F7_PROBE_SENTINEL="unchanged", F7_PROBE_EXIT=str(exit_code))
        actual_exit = run([sys.executable, str(fake)], arguments, inherited)
        observed = json.loads(output.read_text(encoding="utf-8"))
        assert actual_exit == exit_code and observed["argv"] == expected
        assert observed["numeric_environment"] == [] and observed["sentinel"] == "unchanged"
    print("F7-CARGO-SELF-TEST passed=3 total=3")


if __name__ == "__main__":
    if sys.argv[1:] == ["--install"]:
        install()
    elif len(sys.argv) == 3 and sys.argv[1] == "--self-test":
        self_test(sys.argv[2])
    elif len(sys.argv) >= 3 and sys.argv[1] == "--delegate":
        raise SystemExit(run([sys.argv[2]], sys.argv[3:], os.environ))
    else:
        raise SystemExit("用法：controlled-cargo.py --install | --delegate <cargo> [args...] | --self-test <new-directory>")
