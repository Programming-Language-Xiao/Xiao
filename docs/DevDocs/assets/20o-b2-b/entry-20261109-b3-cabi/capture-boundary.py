"""保存当前私有批次的公开面与 Runtime 身份，不修改产品。"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess

ROOT = Path(__file__).resolve().parent
SOURCE = ROOT.parents[4]


def write(path, data):
    with path.open('x', encoding='utf-8', newline='\n') as stream:
        stream.write(json.dumps(data, ensure_ascii=False, indent=2) + '\n')


def main(phase, prefix=''):
    assert phase in ['baseline', 'pow', 'abi']
    if prefix and not re.fullmatch(r'[A-Za-z0-9_-]+-', prefix):
        raise SystemExit('结果前缀只允许字母、数字、下划线和连字符，并以连字符结尾')
    outputs = {name: ROOT / phase / (prefix + name + '.json') for name in ['native-symbols', 'public-surface']}
    if any(path.exists() for path in outputs.values()):
        raise SystemExit('拒绝覆盖已有公开面证据')
    maximum = 48 if phase == 'abi' else 33
    setup = json.loads((ROOT / 'environment.json').read_text(encoding='utf-8'))['overrides']
    table = json.loads((SOURCE / 'core/rust/crates/xiao-intrinsics/intrinsics.json').read_text(encoding='utf-8'))
    rows = table['intrinsics']
    active = sorted(row['id'] for row in rows if row['status'] == 'active')
    absent = [i for i in range(34, 49) if all(row['id'] != i for row in rows)]
    tombstones = [row for row in rows if row['id'] == 100]
    assert active == list(range(1, maximum + 1)) and absent == ([] if phase == 'abi' else list(range(34, 49))) and tombstones[0]['status'] == 'tombstone'
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
    assert cmath['status'] == 'planned' and (SOURCE / 'stdlib/cmath.xiao').exists() == False
    runtime = Path(setup['XIAO_RUNTIME_LIBRARY'])
    raw = runtime.read_bytes()
    identity = {'path': str(runtime), 'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest(), 'mtime_ns': runtime.stat().st_mtime_ns}
    task = Path(json.loads((ROOT / 'bootstrap.json').read_text(encoding='utf-8'))['task_root'])
    archive = task / ('runtime-' + phase) / runtime.name
    archive.parent.mkdir(exist_ok=True)
    if archive.exists():
        assert archive.read_bytes() == raw, '拒绝覆盖已有Runtime存档'
    else:
        shutil.copy2(runtime, archive)
    assert archive.read_bytes() == raw
    identity['archived_copy'] = str(archive)
    result = subprocess.run(['D:/msys64/ucrt64/bin/llvm-nm.exe', '-g', '--defined-only', str(runtime)], capture_output=True)
    assert result.returncode == 0, result.stderr.decode('utf-8')
    symbols = sorted(set(re.findall(r'\b(xiao_runtime_\w+)\b', result.stdout.decode('utf-8'))))
    expected = {'xiao_runtime_cmath_' + name for name in ['sqrt','floor','ceil','round','trunc','abs','min','max','pow','sin','cos','tan','exp','ln','log10']} if phase == 'abi' else set()
    assert 'xiao_runtime_lfloat_divide' in symbols and {name for name in symbols if name.startswith('xiao_runtime_cmath_')} == expected
    # 原始符号输出保留摘要；完整公开符号集合可在无Runtime的Git导出中复核。
    write(outputs['native-symbols'], {'argv': result.args, 'exit_code': result.returncode,
          'raw_stdout_bytes': len(result.stdout), 'raw_stdout_sha256': hashlib.sha256(result.stdout).hexdigest(),
          'runtime': identity, 'symbols': symbols})
    write(outputs['public-surface'], {'active_intrinsics': active, 'absent_intrinsics': absent,
          'tombstones': tombstones, 'versions': versions, 'cmath_status': cmath['status'], 'cmath_product_exists': False,
          'registry_sha256': hashlib.sha256((SOURCE / 'docs/module-registry.json').read_bytes()).hexdigest(),
          'runtime': identity, 'old_721_executed': False, 'b2_a_perf': False})
    print(json.dumps({'phase': phase, 'active_max': max(active), 'absent': absent, 'versions': versions,
                      'runtime_sha256': identity['sha256'], 'symbols': len(symbols), 'cmath': cmath['status']}, ensure_ascii=True))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase', choices=['baseline', 'pow', 'abi'])
    parser.add_argument('--prefix', default='')
    args = parser.parse_args()
    main(args.phase, args.prefix)
