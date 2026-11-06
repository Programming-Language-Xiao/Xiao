"""记录本轮文档门禁的实际命令、环境来源与产品源码边界。"""
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
SCOPE = ['core/rust', 'stdlib', 'tests/fixtures', 'tests/benchmarks', '.github/workflows',
         '.cargo', 'tools', 'cli/ts', 'docs/UseDocs', 'docs/module-registry.json',
         'package.json', 'bun.lock', '.gitattributes']


def boundary():
    return subprocess.check_output(['cmd', '/c', 'git', 'diff', '--name-only', '3dd492d1', '--', *SCOPE], cwd=REPOSITORY).decode().splitlines()


def main():
    name = sys.argv[1]
    assert name.replace('-', '').isalnum()
    log = ROOT/(name+'.log')
    assert not log.exists(), '原始门禁记录不可覆盖'
    environment_source = ROOT.parent/'entry-20261106/environment.json'
    setup = json.loads(environment_source.read_text(encoding='utf-8'))
    removed = [key for key in os.environ if key.startswith('RUST_BIGDECIMAL_')]
    env = {key: value for key, value in os.environ.items() if not key.startswith(('RUST_BIGDECIMAL_', 'XIAO_'))}
    env.update(setup['msvc_whitelist'])
    env.update(setup['overrides'])
    env['XIAO_STDLIB_ROOT'] = str(REPOSITORY/'stdlib')
    before = boundary()
    assert not before
    argv = ['C:/Users/xiaoc/node_modules/bun/bin/bun.exe', 'tools/repo-check/src/cli.ts', 'all']
    started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    clock = time.monotonic()
    with log.open('wb') as stream:
        child = subprocess.Popen(argv, cwd=REPOSITORY, env=env, stdout=stream, stderr=subprocess.STDOUT)
        code = child.wait()
    after = boundary()
    result = {'argv': argv, 'cwd': str(REPOSITORY), 'pid': child.pid, 'started_utc': started,
              'finished_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
              'duration_seconds': round(time.monotonic()-clock, 3), 'exit_code': code,
              'environment_source': str(environment_source), 'removed_environment_names': removed,
              'product_changes_before': before, 'product_changes_after': after,
              'log': log.name, 'log_sha256': hashlib.sha256(log.read_bytes()).hexdigest()}
    (ROOT/(name+'.json')).write_text(json.dumps(result, ensure_ascii=False, indent=2)+'\n', encoding='utf-8', newline='\n')
    print(json.dumps(result, ensure_ascii=False))
    assert not after, '执行期间出现产品源码变化'
    raise SystemExit(code)


if __name__ == '__main__':
    main()
