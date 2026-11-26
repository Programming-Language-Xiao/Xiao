"""只读复核 B3-3 本机证据及其固定 Git 输入。"""

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import zipfile

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
BASE = "879ce625c84965dc4fe4bb11523434635be55826"
IDS = "core/rust/crates/xiao-intrinsics/intrinsics.json"
SYMBOLS = sorted("xiao_runtime_fs_" + name for name in (
    "write_text", "list_dir", "remove", "rename", "metadata",
))
PLATFORMS = {"windows": "x86_64-pc-windows-msvc", "linux-wsl": "x86_64-unknown-linux-gnu"}


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def git(repo, *args):
    return subprocess.check_output(["cmd", "/d", "/c", "git", *args], cwd=repo)


def identity(data):
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def inventory(root):
    return {path.relative_to(root).as_posix(): identity(path.read_bytes())
            for path in sorted(root.rglob("*"))
            if path.is_file() and path != root / "evidence.json"}


def verify_inputs(repo, root, product):
    def blob(commit, path):
        return git(repo, "show", f"{commit}:{path}")

    baseline = json.loads((root / "baseline.json").read_bytes())
    require(baseline["product_commit"] == BASE, "B3-2 基线提交不符")
    require(git(repo, "show", "-s", "--format=%T", BASE).decode().strip()
            == baseline["product_tree"], "B3-2 基线树不符")
    for path, expected in baseline["locks"].items():
        require(identity(blob(BASE, path)) == expected["git_blob"]
                and blob(BASE, path) == blob(product, path), "固定锁文件不符")
    for path, expected in baseline["inputs"].items():
        require(identity(blob(baseline["entry_commit"], path)) == expected, "B3-3 入口文档不符")
    for path, expected in baseline["test_only_delta"].items():
        marker = b"#[cfg(test)]"
        prefix = blob(BASE, path).split(marker, 1)[0]
        require(identity(prefix) == expected
                and prefix == blob(baseline["maintenance_commit"], path).split(marker, 1)[0],
                "B3-2 维护增量包含生产变动")
    baseline_ids = {row["id"]: row for row in json.loads(blob(BASE, IDS))["intrinsics"]}
    product_ids = {row["id"]: row for row in json.loads(blob(product, IDS))["intrinsics"]}
    require(set(baseline_ids) == set(product_ids), "intrinsic 编号集合变化")
    for id_, row in product_ids.items():
        if 55 <= id_ <= 59:
            require(row["status"] == "active" and row["capability"] == "filesystem"
                    and "xiao_runtime_" + row["abi_binding"] in SYMBOLS, "fs 裁定不符")
        else:
            require(row == baseline_ids[id_], f"超出本批的 intrinsic 发生变化 {id_}")
    table = json.loads(blob(product, "docs/module-registry.json"))
    require([row["status"] for row in table["modules"] if row["id"] == "stdlib.fs"] == ["planned"],
            "fs registry 状态不符")
    candidates = "docs/DevDocs/assets/20o-b3/entry-20261124-b3-0e/candidates/"
    for name in ("tutorial.xiao", "tutorial.stdout"):
        path = f"tests/fixtures/docs/fs/{name}"
        require(blob(product, path) == blob(baseline["entry_commit"], candidates + path),
                "fs 教程字节与冻结候选不符")
    version = blob(product, "core/rust/crates/xiao-codegen-llvm/src/lib.rs")
    require(b"pub const CODEGEN_VERSION: u32 = 24;" in version, "生成器版本不符")


def successes(text):
    return [int(count) for count in re.findall(
        r"test result: ok\. (\d+) passed; 0 failed; 0 ignored;", text,
    )]


def derive(root, repo):
    platforms = {}
    with zipfile.ZipFile(root / "runs.zip") as archive:
        names = archive.namelist()
        require(len(names) == len(set(names)), "日志对象重复")
        product = None
        expected_names = {
            "runs/windows.json", "runs/linux-wsl.json", "runs/linux-wsl-first.json",
            "runs/linux-wsl-missing-temp.json", "runs/linux-wsl-export-option.json",
        }

        def verified_stdout(row):
            streams = {}
            for stream in ("stdout", "stderr"):
                digest = row[stream]
                require(re.fullmatch("[0-9a-f]{64}", digest) is not None, "日志摘要格式不符")
                key = "objects/" + digest
                data = archive.read(key)
                require(identity(data)["sha256"] == digest, "日志对象摘要不符")
                expected_names.add(key)
                streams[stream] = data.decode("utf-8", errors="replace")
            return streams

        for platform, target in PLATFORMS.items():
            record = json.loads(archive.read(f"runs/{platform}.json"))
            require(record["target"] == target and record["scope"] == "local-vm-and-runtime",
                    "本机采集目标不符")
            product = product or record["product_commit"]
            require(record["product_commit"] == product, "两平台产品提交不一致")
            commands = record["commands"]
            require(set(commands) == {"rustc", "nm-version", "runtime", "fs", "vm", "release", "exports"}
                    and all(row["exit"] == 0 for row in commands.values()), "本机命令未全部通过")
            stdout = {label: verified_stdout(row)["stdout"] for label, row in commands.items()}
            require(f"host: {target}" in stdout["rustc"] and "release: 1.96.0" in stdout["rustc"],
                    "Rust 工具链目标不符")
            runtime = record["runtime"]
            require(re.fullmatch("[0-9a-f]{64}", runtime["sha256"])
                    and runtime["path"].startswith(record["task_root"] + "/target/" + target + "/release/")
                    and runtime["exports"] == SYMBOLS
                    and sorted(set(stdout["exports"].splitlines()) & set(SYMBOLS)) == SYMBOLS,
                    "release Runtime 的五个 C ABI 导出不符")
            require(successes(stdout["runtime"]) == [10, 289], "Runtime 或 intrinsic 测试计数不符")
            require(successes(stdout["vm"]) == [7, 2, 4], "VM 测试计数不符")
            for alias in ("std", "stdlib"):
                require(f"B3-3-VM module=fs alias={alias} exact=true demo_empty=true" in stdout["vm"],
                        "fs 双路径教程回执缺项")
            require(successes(stdout["fs"]) == [8], "fs 与 C ABI 专项计数不符")
            platforms[platform] = {
                "runtime_tests": 289, "intrinsic_tests": 10, "vm_tests": 13,
                "fs_and_c_abi_tests": 8, "runtime_sha256": runtime["sha256"],
                "c_abi_exports": len(SYMBOLS),
                "symlinks": "passed" if "B3-3-FS-SYMLINK passed" in stdout["fs"] else "unverifiable",
                "native_product": "pending",
            }
            if platform == "linux-wsl":
                require("B3-3-FS-CROSS-VOLUME passed" in stdout["fs"], "Linux 跨卷拒绝回执缺项")
        first = json.loads(archive.read("runs/linux-wsl-first.json"))
        require(first["product_commit"] == product and first["target"] == PLATFORMS["linux-wsl"]
                and set(first["commands"]) == {"rustc"}
                and first["commands"]["rustc"]["exit"] == 127
                and "No such file or directory" in verified_stdout(first["commands"]["rustc"])["stderr"],
                "WSL 首次采集失败回执不符")
        missing = json.loads(archive.read("runs/linux-wsl-missing-temp.json"))
        require(missing["product_commit"] == product and missing["target"] == PLATFORMS["linux-wsl"]
                and set(missing["commands"]) == {"rustc", "nm-version", "runtime"}
                and missing["commands"]["runtime"]["exit"] == 101, "WSL 临时目录缺失回执不符")
        missing_streams = {label: verified_stdout(row) for label, row in missing["commands"].items()}
        require("No such file or directory" in missing_streams["runtime"]["stdout"]
                and "280 passed; 9 failed" in missing_streams["runtime"]["stdout"],
                "WSL 临时目录缺失日志不符")
        export_option = json.loads(archive.read("runs/linux-wsl-export-option.json"))
        require(export_option["product_commit"] == product
                and export_option["target"] == PLATFORMS["linux-wsl"], "WSL 导出工具失败回执不符")
        option_commands = export_option["commands"]
        require(option_commands["release"]["exit"] == 0
                and option_commands["exports"]["exit"] == 1, "WSL 导出工具状态不符")
        option_streams = {label: verified_stdout(row) for label, row in option_commands.items()}
        require("unrecognized option '--just-symbol-name'" in option_streams["exports"]["stderr"],
                "WSL 导出工具失败日志不符")
        require(set(names) == expected_names, "归档中有未登记日志对象")
    require(isinstance(product, str) and re.fullmatch("[0-9a-f]{40}", product), "产品提交格式不符")
    verify_inputs(repo, root, product)
    return {
        "stage": "20O-B3-3", "product_commit": product,
        "b3_3_started": True, "b3_3a_entry_and_baseline_verified": True,
        "b3_3b_fs_intrinsics_implemented": True,
        "b3_3c_vm_tac_llvm_c_abi_native_consistent": False,
        "b3_3d_fs_tutorial_and_boundaries_passed": False,
        "b3_3e_evidence_audit_passed": False, "b3_3_passed": False,
        "b3_4_entry_ready": False, "generator": 24, "generator_25_decision": "pending",
        "windows_product_coff": "pending", "macos": "unverifiable",
        "registry_status": "planned", "tutorial_status": "planned",
        "platforms": platforms,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=HERE)
    parser.add_argument("--source-root", type=Path, default=REPO)
    args = parser.parse_args()
    try:
        files = inventory(args.root)
        require(".gitattributes" in files, "缺少 .gitattributes")
        require(json.loads((args.root / "evidence.json").read_bytes()) == {"files": files},
                "封存清单与当前文件字节不一致")
        status = derive(args.root, args.source_root)
        require(json.loads((args.root / "status.json").read_bytes()) == status, "阶段状态与实测回执不一致")
    except (KeyError, ValueError, OSError, subprocess.SubprocessError, zipfile.BadZipFile) as error:
        raise SystemExit(f"B3-3 拒绝：{error}") from None
    print(json.dumps({**status, "audit_files": len(files)}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
