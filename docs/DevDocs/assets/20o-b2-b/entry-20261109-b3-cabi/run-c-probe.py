"""全新目录链接本批release Runtime，外部C在O0/O3调用15个导出符号。"""
import argparse
import datetime
import hashlib
import json
from pathlib import Path
import re
import subprocess
import uuid

ROOT = Path(__file__).resolve().parent


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def identity(path):
    if not path.is_file():
        return {'path': str(path), 'exists': False}
    return {'path': str(path), 'sha256': sha(path), 'bytes': path.stat().st_size,
            'mtime_ns': path.stat().st_mtime_ns}


def main(result_name):
    destination = ROOT / 'abi' / result_name
    if Path(result_name).name != result_name or destination.exists():
        raise SystemExit('结果必须是新文件名，拒绝覆盖既有证据')
    setup = json.loads((ROOT / 'environment.json').read_text(encoding='utf-8'))['overrides']
    task = Path(json.loads((ROOT / 'bootstrap.json').read_text(encoding='utf-8'))['task_root'])
    directory = task / 'c-probes' / str(uuid.uuid4())
    directory.mkdir(parents=True)
    runtime = Path(setup['XIAO_RUNTIME_LIBRARY'])
    before = identity(runtime)
    if before.get('exists') is False:
        raise RuntimeError('被测Runtime不存在')
    c_source = ROOT / 'cmath-abi-probe.c'
    input_files = [c_source, Path(__file__).resolve()]
    inputs_before = {path.name: sha(path) for path in input_files}
    records = []

    def run(name, argv):
        started = datetime.datetime.now(datetime.timezone.utc).isoformat()
        with (directory / (name + '.log')).open('wb') as stream:
            child = subprocess.Popen(argv, stdout=stream, stderr=subprocess.STDOUT)
            code = child.wait()
        text = (directory / (name + '.log')).read_text(encoding='utf-8')
        records.append({'name': name, 'argv': argv, 'pid': child.pid, 'started_utc': started,
                        'finished_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
                        'exit_code': code, 'output': text})
        if code:
            raise RuntimeError(name + ': ' + text)
        return text

    libraries = []
    completed = False
    failure = None
    try:
        # 系统库探测也纳入结果记录；子进程失败同样生成明确的passed=false。
        source = directory / 'native-libs.rs'
        source.write_text('pub fn b3_native_libraries_probe() {}\n', encoding='utf-8')
        native = run('native-libs', [setup['RUSTC'], '--crate-type=staticlib', '--print=native-static-libs',
            '--target=' + setup['XIAO_TARGET_TRIPLE'], '-o', str(directory / 'native-libs.lib'), str(source)])
        found = re.search(r'native-static-libs:\s*([^\r\n]+)', native)
        if not found:
            raise RuntimeError('未取得Rust原生库清单')
        for token in found.group(1).split():
            if token.lower().startswith(('/defaultlib:', '-defaultlib:')):
                continue
            libraries.append('-l' + token[:-4] if token.lower().endswith('.lib') else token)
        for level in [0, 3]:
            executable = directory / f'cmath-o{level}.exe'
            run(f'compile-o{level}', [setup['XIAO_CLANG'], '--target=' + setup['XIAO_TARGET_TRIPLE'],
                '-std=c11', '-Wall', '-Wextra', '-Werror', '-O' + str(level), str(c_source), str(runtime),
                '-o', str(executable), *libraries])
            output = run(f'run-o{level}', [str(executable)])
            if not re.fullmatch(r'B3-CMATH-C-PROBE entries=15 checks=231 statuses=0,1,2,3,6 result=success\s*', output):
                raise RuntimeError('外部C输出不符：' + output)
            records[-1]['executable_sha256'] = sha(executable)
            records[-1]['executable_bytes'] = executable.stat().st_size
        completed = True
    except Exception as error:
        failure = str(error)
    finally:
        after = identity(runtime)
        inputs_after = {path.name: sha(path) if path.is_file() else None for path in input_files}
        runtime_unchanged = before == after
        inputs_unchanged = inputs_before == inputs_after
        passed = completed and failure is None and runtime_unchanged and inputs_unchanged and len(records) == 5 and all(r['exit_code'] == 0 for r in records)
        if not passed and failure is None:
            failure = 'C ABI探测期间Runtime或探测输入身份变化'
        result = {'passed': passed, 'directory': str(directory), 'failure': failure,
                  'runtime_before': before, 'runtime_after': after, 'runtime_unchanged': runtime_unchanged,
                  'inputs_before': inputs_before, 'inputs_after': inputs_after, 'inputs_unchanged': inputs_unchanged,
                  'source_sha256': inputs_before[c_source.name], 'tool_sha256': inputs_before[Path(__file__).name], 'native_libraries': libraries,
                  'records': records, 'scope': '外部C ABI调用；不代表Xiao VM/LLVM语言全链路'}
        with destination.open('x', encoding='utf-8', newline='\n') as stream:
            stream.write(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    if not result['passed']:
        raise RuntimeError(result['failure'])
    print(json.dumps({'passed': True, 'optimizations': [0, 3], 'entries': 15, 'directory': str(directory)}, ensure_ascii=True))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--result', default='c-probe-result.json')
    main(parser.parse_args().result)
