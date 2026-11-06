"""按同一源码快照串行运行真实回归，各命令独立保存原始记录。"""
import pathlib
import subprocess
import sys

task = pathlib.Path(__file__).resolve().parents[1]
source = task / 'source'
cargo = ['D:/Program Files/.cargo/bin/cargo.exe', '+1.96.0']
manifest = ['--locked', '--manifest-path', 'core/rust/Cargo.toml']
checks = [
    ('final-b1-contract', cargo + ['test', *manifest, '-p', 'xiao-driver', '--test', '20o_b1_imp_contract']),
    ('final-b1-resources', cargo + ['test', *manifest, '-p', 'xiao-driver', '--test', '20o_b1_imp_resources']),
    ('final-b1-vm-exact', cargo + ['test', *manifest, '-p', 'xiao-driver', '--test', '20o_b1_probe', '--', 'b1_capabilities_have_vm_golden_outputs', '--exact', '--nocapture']),
    ('final-b1-native-exact', cargo + ['test', *manifest, '-p', 'xiao-driver', '--test', '20o_b1_probe', '--', '--ignored', 'b1_capabilities_match_native_golden_outputs', '--exact', '--nocapture']),
    ('final-workspace-check', cargo + ['check', *manifest, '--workspace']),
    ('final-benchmarks-check', cargo + ['check', '--locked', '--manifest-path', 'tests/benchmarks/Cargo.toml']),
]
for name, argv in checks:
    if len(sys.argv) > 1:
        name = sys.argv[1] + name.removeprefix('final')
    result = subprocess.run([sys.executable, '-X', 'utf8', '-u', str(task / 'scripts/run-step.py'), name, '--', *argv], cwd=source)
    if result.returncode:
        raise SystemExit(result.returncode)
