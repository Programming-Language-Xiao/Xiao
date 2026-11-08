"""保存当前私有批次的公开面与 Runtime 身份，不修改产品。"""
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parent
SOURCE = ROOT.parents[4]


def write(path, data):
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')


def main(phase):
    assert phase in ['b0', 'b1']
    setup = json.loads((ROOT / 'environment.json').read_text(encoding='utf-8'))['overrides']
    table = json.loads((SOURCE / 'core/rust/crates/xiao-intrinsics/intrinsics.json').read_text(encoding='utf-8'))
    rows = table['intrinsics']
    active = sorted(row['id'] for row in rows if row['status'] == 'active')
    absent = [i for i in range(34, 49) if all(row['id'] != i for row in rows)]
    tombstones = [row for row in rows if row['id'] == 100]
    assert active == list(range(1, 34)) and absent == list(range(34, 49)) and tombstones[0]['status'] == 'tombstone'
    version_files = {'abi': 'core/rust/crates/xiao-runtime-abi/src/lib.rs',
                     'codegen': 'core/rust/crates/xiao-codegen-llvm/src/lib.rs',
                     'stdlib_interface': 'core/rust/crates/xiao-standard-library/src/lib.rs'}
    constants = {'abi': 'ABI_MINOR_VERSION', 'codegen': 'CODEGEN_VERSION', 'stdlib_interface': 'STDLIB_INTERFACE_VERSION'}
    versions = {key: int(re.search(r'pub const ' + constants[key] + r': u32 = (\d+);', (SOURCE / path).read_text(encoding='utf-8')).group(1)) for key, path in version_files.items()}
    assert versions == {'abi': 18, 'codegen': 20, 'stdlib_interface': 1}
    registry = json.loads((SOURCE / 'docs/module-registry.json').read_text(encoding='utf-8'))
    # registry 结构从现有JSON读取，不靠README计数。
    modules = registry['modules']
    cmath = next(row for row in modules if row['id'] == 'stdlib.cmath')
    assert cmath['status'] == 'planned' and not (SOURCE / 'stdlib/cmath.xiao').exists()
    runtime = Path(setup['XIAO_RUNTIME_LIBRARY'])
    raw = runtime.read_bytes()
    identity = {'path': str(runtime), 'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest(), 'mtime_ns': runtime.stat().st_mtime_ns}
    task = Path(json.loads((ROOT / 'bootstrap.json').read_text(encoding='utf-8'))['task_root'])
    archive = task / ('runtime-' + phase) / runtime.name
    archive.parent.mkdir(exist_ok=True)
    shutil.copy2(runtime, archive)
    assert archive.read_bytes() == raw
    identity['archived_copy'] = str(archive)
    result = subprocess.run(['D:/msys64/ucrt64/bin/llvm-nm.exe', '-g', '--defined-only', str(runtime)], capture_output=True)
    assert result.returncode == 0, result.stderr.decode('utf-8')
    symbols = sorted(set(re.findall(r'\b(xiao_runtime_\w+)\b', result.stdout.decode('utf-8'))))
    assert 'xiao_runtime_lfloat_divide' in symbols and not any(name.startswith('xiao_runtime_cmath_') for name in symbols)
    # 原始符号输出保留摘要；完整公开符号集合可在无Runtime的Git导出中复核。
    write(ROOT / phase / 'native-symbols.json', {'argv': result.args, 'exit_code': result.returncode,
          'raw_stdout_bytes': len(result.stdout), 'raw_stdout_sha256': hashlib.sha256(result.stdout).hexdigest(),
          'runtime': identity, 'symbols': symbols})
    write(ROOT / phase / 'public-surface.json', {'active_intrinsics': active, 'absent_intrinsics': absent,
          'tombstones': tombstones, 'versions': versions, 'cmath_status': cmath['status'], 'cmath_product_exists': False,
          'registry_sha256': hashlib.sha256((SOURCE / 'docs/module-registry.json').read_bytes()).hexdigest(),
          'runtime': identity, 'old_721_executed': False, 'b2_a_perf': False})
    print(json.dumps({'phase': phase, 'active_max': max(active), 'absent': absent, 'versions': versions,
                      'runtime_sha256': identity['sha256'], 'symbols': len(symbols), 'cmath': cmath['status']}, ensure_ascii=True))


if __name__ == '__main__':
    main(sys.argv[1])
