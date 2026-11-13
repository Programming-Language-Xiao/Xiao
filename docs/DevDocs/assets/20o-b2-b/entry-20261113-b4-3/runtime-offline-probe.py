"""验证默认B4-3审计只依赖捕获的Runtime身份，而live模式检查本机归档。"""
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parent
REPOSITORY = Path(r"D:\project\Xiao\Xiao")
AUDIT = ROOT / "audit.py"
ENVIRONMENT = json.loads((ROOT / "fixed-final2" / "environment.json").read_text(encoding="utf-8"))


def main():
    runtime = Path(ENVIRONMENT["overrides"]["XIAO_RUNTIME_LIBRARY"])
    held = runtime.with_name(runtime.name + ".offline-probe-hold")
    if not runtime.is_file() or runtime.is_symlink() or held.exists():
        raise SystemExit("Runtime offline probe precondition failed")

    runtime.rename(held)
    try:
        command = [sys.executable, "-B", "-X", "utf8", str(AUDIT),
                   "--root", str(ROOT), "--repository-root", str(REPOSITORY)]
        default = subprocess.run(command, capture_output=True, text=True, encoding="utf-8")
        live = subprocess.run(command + ["--live-runtime"], capture_output=True, text=True, encoding="utf-8")
    finally:
        held.rename(runtime)

    if default.returncode != 0 or json.loads(default.stdout).get("passed") is not True:
        raise SystemExit("default audit unexpectedly required the live Runtime: " + default.stdout + default.stderr)
    if live.returncode != 1 or live.stdout.strip() != "B4-3 task Runtime artifact missing" or live.stderr:
        raise SystemExit("live audit failed to identify missing Runtime: " + live.stdout + live.stderr)
    if not runtime.is_file() or held.exists():
        raise SystemExit("Runtime archive was not restored")
    print(json.dumps({"passed": True, "default_without_live_runtime": True,
                      "live_mode_rejected_missing_runtime": True, "runtime_restored": True}, sort_keys=True))


if __name__ == "__main__":
    main()
