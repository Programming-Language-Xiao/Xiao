"""固定入口源码的A2/A3审计、四包及B1基线；每条独立留档。"""
from pathlib import Path
import subprocess
import sys

task = Path(__file__).resolve().parents[1]
source = task / 'source'
cargo = ['D:/Program Files/.cargo/bin/cargo.exe', '+1.96.0', 'test', '--locked', '--manifest-path', 'core/rust/Cargo.toml']
checks = [
    ('entry-a2-a3-audit', [sys.executable, '-X', 'utf8', 'docs/DevDocs/assets/20o-b2-a2-a3/entry-20261106/audit.py']),
    ('entry-package-tests', cargo + ['-p', 'xiao-runtime', '-p', 'xiao-types', '-p', 'xiao-diagnostics', '-p', 'xiao-vm']),
    ('entry-b1-contract', cargo + ['-p', 'xiao-driver', '--test', '20o_b1_imp_contract']),
    ('entry-b1-resources', cargo + ['-p', 'xiao-driver', '--test', '20o_b1_imp_resources']),
]
for name, argv in checks:
    result = subprocess.run([sys.executable, '-X', 'utf8', '-u', str(task/'scripts/run-step.py'), name, '--', *argv], cwd=source)
    if result.returncode:
        raise SystemExit(result.returncode)
