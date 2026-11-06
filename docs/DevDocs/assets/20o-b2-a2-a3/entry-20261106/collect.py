"""收集实际源码边界和产物/工具身份，不执行产品测试。"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

TASK = Path(__file__).resolve().parents[1]
SOURCE = TASK / 'source'
EVIDENCE = SOURCE / 'docs/DevDocs/assets/20o-b2-a2-a3/entry-20261106'
ENTRY = json.loads((EVIDENCE / 'bootstrap.json').read_text(encoding='utf-8'))['entry_head']


def git(*args):
    return subprocess.check_output(['cmd', '/c', 'git', *args], cwd=SOURCE)


def write(name, value):
    (EVIDENCE / name).write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical(raw):
    try:
        raw.decode('utf-8')
        return raw if b'\0' in raw else raw.replace(b'\r\n', b'\n')
    except UnicodeDecodeError:
        return raw


def boundary():
    changed = {x.decode('utf-8') for x in git('diff', '--name-only', '-z', ENTRY, '--').split(b'\0') if x}
    changed.update(x.decode('utf-8') for x in git('ls-files', '--others', '--exclude-standard', '-z').split(b'\0') if x)
    product = {'core/rust/crates/xiao-runtime/src/value/' + name for name in ['mod.rs', 'ops.rs', 'numeric.rs']}
    documents = {'docs/DevDocs/' + name for name in [
        '20o-b2-a2-a3-arithmetic-plan.md', '20o-b2-a2-a3-arithmetic-progress.md',
        '20o-b2-a-implementation-plan.md', '20o-b2-numeric-foundation-plan.md',
        '20o-official-library-program.md', 'README.md']}
    prefixes = ['core/rust/crates/xiao-runtime/src/value/arithmetic/', 'docs/DevDocs/assets/20o-b2-a2-a3/']
    allowed = lambda path: path in product | documents or any(path.startswith(prefix) for prefix in prefixes)
    protected = ['core/rust/Cargo.toml', 'core/rust/Cargo.lock', 'tests/benchmarks/Cargo.lock',
                 'core/rust/crates/xiao-runtime/Cargo.toml', 'docs/module-registry.json',
                 'docs/DevDocs/20o-b2-freeze-draft.md']
    rows = []
    for relative in protected:
        original = git('show', ENTRY + ':' + relative)
        current = (SOURCE / relative).read_bytes()
        rows.append({'path': relative, 'entry_utf8_lf_sha256': sha(canonical(original)),
                     'current_raw_sha256': sha(current), 'current_utf8_lf_sha256': sha(canonical(current)),
                     'unchanged': canonical(original) == canonical(current)})
    unclassified = [path for path in sorted(changed) if not allowed(path)]
    protected_changes = [row['path'] for row in rows if not row['unchanged']]
    write('boundary.json', {'passed': not unclassified and not protected_changes, 'entry_head': ENTRY,
        'recorded_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'scope': '全部Git已跟踪差异及全部未跟踪路径；没有过滤产品/UseDocs/工具路径',
        'commands': [['cmd', '/c', 'git', 'diff', '--name-only', '-z', ENTRY, '--'],
                     ['cmd', '/c', 'git', 'ls-files', '--others', '--exclude-standard', '-z']],
        'changed_paths': sorted(changed), 'unclassified_paths': unclassified,
        'protected_changes': protected_changes, 'protected_file_hashes': rows,
        'preserved_boundaries': ['ABI1.18', 'CODEGEN20', 'intrinsic最大32', 'XCON不变',
            'F-7依赖及锁不变', 'stdlib与UseDocs不变', 'registry不变', 'A0/A1归档不变',
            'A4～A7未开始', '旧721条未执行', '22C/22D及19/11/22债务独立']})
    assert not unclassified and not protected_changes


def identity():
    setup = json.loads((TASK / 'environment.json').read_text(encoding='utf-8'))
    env = {key: value for key, value in os.environ.items() if not key.startswith(('XIAO_', 'RUST_BIGDECIMAL_'))}
    env.update(setup['msvc_whitelist'])
    env.update(setup['overrides'])
    commands = [
        ('rustc', [env['RUSTC'], '-vV']),
        ('cargo', ['D:/Program Files/.cargo/bin/cargo.exe', '+1.96.0', '--version']),
        ('clang', [env['XIAO_CLANG'], '--version']),
        ('bun', ['C:/Users/xiaoc/node_modules/bun/bin/bun.exe', '--version']),
        ('python', [sys.executable, '--version']),
    ]
    versions = []
    for name, argv in commands:
        log = EVIDENCE / ('tool-version-' + name + '.log')
        assert not log.exists()
        with log.open('wb') as stream:
            process = subprocess.Popen(argv, cwd=SOURCE, env=env, stdout=stream, stderr=subprocess.STDOUT)
            status = process.wait()
        versions.append({'name': name, 'argv': argv, 'cwd': str(SOURCE), 'pid': process.pid,
                         'exit_code': status, 'log': log.name, 'sha256': sha(log.read_bytes())})
        assert status == 0
    artifacts = []
    for role in ['XIAO_RUNTIME_LIBRARY', 'XIAO_RUST_DOC_ADAPTER']:
        path = Path(env[role])
        raw = path.read_bytes()
        artifacts.append({'role': role, 'path': str(path), 'bytes': len(raw), 'sha256': sha(raw)})
    write('product-identity.json', {'code_commit': git('rev-parse', 'HEAD').decode().strip(),
        'recorded_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'source_root': str(SOURCE), 'task_root': str(TASK),
        'versions_scope': '只读工具版本探测；构建/测试/检查另按--locked执行',
        'tool_versions': versions, 'artifacts': artifacts,
        'internal_only': True, 'abi': '1.18', 'codegen': 20, 'active_intrinsic_max': 32,
        'typescript': {'version': json.loads((SOURCE / 'tools/doc-coverage/node_modules/typescript/package.json').read_text(encoding='utf-8'))['version'],
            'strategy': '从主工作区实体复制到本源码根内；无junction、不改锁',
            'source': 'D:/project/Xiao/Xiao/tools/doc-coverage/node_modules/typescript',
            'destination': str(SOURCE / 'tools/doc-coverage/node_modules/typescript')}})


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('action', choices=['boundary', 'identity'])
globals()[parser.parse_args().action]()
