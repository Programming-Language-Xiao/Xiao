"""同一最终源码的四包回归和任务内新产物构建。"""
from pathlib import Path
import subprocess
import sys

task = Path(__file__).resolve().parents[1]
source = task / 'source'
cargo = ['D:/Program Files/.cargo/bin/cargo.exe', '+1.96.0']
manifest = ['--locked', '--manifest-path', 'core/rust/Cargo.toml']
checks = [
    ('close-package-tests', cargo + ['test', *manifest, '-p', 'xiao-runtime', '-p', 'xiao-types', '-p', 'xiao-diagnostics', '-p', 'xiao-vm']),
    ('close-build-runtime-adapter', cargo + ['build', *manifest, '-p', 'xiao-runtime', '-p', 'xiao-doc-coverage-rust']),
    ('close-build-runtime-release', cargo + ['build', *manifest, '--release', '-p', 'xiao-runtime']),
]
for name, argv in checks:
    result = subprocess.run([sys.executable, '-X', 'utf8', '-u', str(task/'scripts/run-step.py'), name, '--', *argv], cwd=source)
    if result.returncode:
        raise SystemExit(result.returncode)
