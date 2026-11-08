"""复跑默认审计两次并验证Git证据导出；结果只写显式指定的外部路径。"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parent
PREFIX = 'docs/DevDocs/assets/20o-b2-b/entry-20261108-b3'


def snapshot(root):
    return {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in root.rglob('*') if p.is_file()}


def main(source, evidence_commit, output):
    output = output.resolve()
    assert not output.is_relative_to(ROOT), '复核输出必须在被审计目录之外'
    assert not output.exists(), '拒绝覆盖既有复核结果'
    output.parent.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, PYTHONUTF8='1', PYTHONIOENCODING='utf-8')
    commit = subprocess.check_output(['cmd', '/c', 'git', 'rev-parse', evidence_commit], cwd=source).decode().strip()
    runs = []

    def audit(root, name, live=False):
        before = snapshot(root)
        argv = [sys.executable, '-X', 'utf8', str(root / 'audit.py'), '--source-root', str(source)]
        if live:
            argv.append('--live-runtime')
        result = subprocess.run(argv, capture_output=True, text=True, encoding='utf-8', env=env)
        raw = (result.stdout + result.stderr).strip()
        read_only = before == snapshot(root)
        row = {'name': name, 'argv': argv, 'exit_code': result.returncode, 'read_only': read_only, 'output': raw}
        runs.append(row)
        assert result.returncode == 0 and read_only and json.loads(raw)['passed'] is True, row
        print(json.dumps(row, ensure_ascii=True), flush=True)

    audit(ROOT, 'default-first')
    audit(ROOT, 'default-second')
    audit(ROOT, 'live-runtime', True)
    with tempfile.TemporaryDirectory(prefix='b3-git-evidence-export-', dir=output.parent) as temporary:
        temporary = Path(temporary)
        archive = temporary / 'evidence.tar'
        argv = ['cmd', '/c', 'git', 'archive', commit, '--format=tar', '--output=' + str(archive), '--', PREFIX]
        subprocess.run(argv, cwd=source, check=True)
        with tarfile.open(archive) as stream:
            stream.extractall(temporary / 'export', filter='data')
        exported = temporary / 'export' / PREFIX
        audit(exported, 'git-evidence-export')
        result = {'passed': True, 'evidence_commit': commit, 'source_repository': str(source),
                  'archive_command': argv, 'archive_sha256': hashlib.sha256(archive.read_bytes()).hexdigest(),
                  'export_scope': '只导出本批证据目录；完整产品源码通过固定Git对象逐文件校验',
                  'runs': runs, 'ci_executed': False}
    output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')
    print(json.dumps({'passed': True, 'evidence_commit': commit, 'output': str(output)}, ensure_ascii=True))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path, default=ROOT.parents[4])
    parser.add_argument('--evidence-commit', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    main(args.source_root.resolve(), args.evidence_commit, args.output)
