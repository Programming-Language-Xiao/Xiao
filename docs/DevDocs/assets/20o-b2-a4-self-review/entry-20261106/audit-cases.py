"""在隔离副本比较原/修正审计；检查真实退出原因，不搜索脚本源码文本。"""
import argparse
import contextlib
import hashlib
import io
import json
from pathlib import Path
import shutil
import sys
import tempfile

TASK = Path(__file__).resolve().parents[1]
SOURCE = TASK/'source'
ARCHIVE = SOURCE/'docs/DevDocs/assets/20o-b2-a4/entry-20261106'
EVIDENCE = SOURCE/'docs/DevDocs/assets/20o-b2-a4-self-review/entry-20261106'


def read(root, name):
    return json.loads((root/name).read_text(encoding='utf-8'))


def write(root, name, value):
    (root/name).write_text(json.dumps(value, ensure_ascii=False, indent=2)+'\n', encoding='utf-8', newline='\n')


def mutate(root, case):
    if case == 'duplicate-kernel-list':
        row = read(root, 'kernel-test-names.json')
        keep = row['tests'][0]
        removed = set(row['tests'][1:])
        row['tests'] = [keep]*17
        write(root, 'kernel-test-names.json', row)
        log = root/'ci-kernel-linux.log'
        raw = b''.join(line for line in log.read_bytes().splitlines(keepends=True)
                       if not any(('test '+name+' ... ok').encode() in line for name in removed))
        log.write_bytes(raw)
        row = read(root, 'ci-kernel-linux.json')
        row['log_sha256'] = hashlib.sha256(raw).hexdigest()
        write(root, 'ci-kernel-linux.json', row)
    elif case == 'duplicate-arithmetic-list':
        row = read(root, 'a2-a3-regression-test-names.json')
        row['tests'] = [row['tests'][0]]*21
        write(root, 'a2-a3-regression-test-names.json', row)
    elif case == 'missing-required-check':
        row = read(root, 'closeout.json')
        row['effective_checks'].remove('close-clippy')
        write(root, 'closeout.json', row)
    elif case == 'missing-required-ci-run':
        row = read(root, 'closeout.json')
        del row['ci_runs']['workspace']
        write(root, 'closeout.json', row)
    elif case == 'wrong-canonical-summary':
        row = read(root, 'closeout.json')
        row['utf8_lf_source_tree_sha256'] = '0'*64
        write(root, 'closeout.json', row)
    elif case == 'wrong-source-count':
        row = read(root, 'closeout.json')
        row['source_files'] = 1
        write(root, 'closeout.json', row)
    elif case == 'wrong-command-source-summary':
        row = read(root, 'close-clippy.json')
        row['source_before']['files'] = row['source_after']['files'] = 1
        write(root, 'close-clippy.json', row)
    elif case == 'native-run-id':
        row = read(root, 'ci-native-windows.json')
        row['run_id'] = 0
        write(root, 'ci-native-windows.json', row)
    elif case == 'stale-native-job':
        row = read(root, 'ci-native-linux.json')
        jobs = read(root, 'ci-maintenance-jobs.json')['jobs']
        old = [job for job in jobs if job['name'] == row['job_name'] and job['conclusion'] == 'success' and job['run_attempt'] == 1]
        assert old
        row.update(job_id=old[0]['id'], run_attempt=1)
        write(root, 'ci-native-linux.json', row)
    elif case == 'log-outside-archive':
        row = read(root, 'close-kernel-tests.json')
        row['log'] = str(ARCHIVE/'close-kernel-tests.log')
        write(root, 'close-kernel-tests.json', row)
    elif case == 'guard-case-result':
        row = read(root, 'close-b1-guards-report.json')
        row['cases'][0]['Passed'] = False
        write(root, 'close-b1-guards-report.json', row)
    elif case == 'nested-inventory-file':
        # 调用方在生成清单之后添加；它不是清单自身，必须被发现。
        (root/'unexpected').mkdir()
        (root/'unexpected/evidence.json').write_text('{"unexpected":true}\n', encoding='utf-8')
    elif case != 'baseline':
        raise AssertionError(case)


EXPECTED = {
    'duplicate-kernel-list': '内核测试名单不完整或重复',
    'duplicate-arithmetic-list': 'A2/A3回归标识清单不完整或重复',
    'missing-required-check': '有效检查清单不完整或重复',
    'missing-required-ci-run': 'CI工作流清单不完整',
    'wrong-canonical-summary': '收口规范源码身份不符',
    'wrong-source-count': '收口源码文件计数不符',
    'wrong-command-source-summary': '命令源码身份不符：close-clippy',
    'native-run-id': '平台运行号不符：windows',
    'stale-native-job': '平台未使用最新成功作业：linux',
    'log-outside-archive': '归档引用路径越界：',
    'guard-case-result': 'B1守卫明细不完整或存在失败',
    'nested-inventory-file': '证据清单计数不符',
}


def main():
    global TASK, SOURCE, ARCHIVE, EVIDENCE
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--audit', required=True, type=Path)
    parser.add_argument('--label', required=True)
    parser.add_argument('--mode', choices=['before', 'after'], required=True)
    parser.add_argument('--repository-root', type=Path)
    parser.add_argument('--work-root', type=Path)
    parser.add_argument('--output-root', type=Path)
    args = parser.parse_args()
    if args.repository_root is not None:
        SOURCE = args.repository_root.resolve()
        ARCHIVE = SOURCE/'docs/DevDocs/assets/20o-b2-a4/entry-20261106'
        TASK = args.work_root.resolve() if args.work_root else Path(tempfile.mkdtemp(prefix='xiao-a4-review-'))
        TASK.mkdir(parents=True, exist_ok=True)
        EVIDENCE = args.output_root.resolve() if args.output_root else TASK/'results'
        EVIDENCE.mkdir(parents=True, exist_ok=True)
    code = args.audit.read_bytes()
    results = []
    for case in ['baseline', *EXPECTED]:
        root = TASK/'cases'/args.label/case
        assert not root.exists(), root
        shutil.copytree(ARCHIVE, root)
        namespace = {'__file__': str(args.audit.resolve()), '__name__': 'review_audit'}
        exec(compile(code, str(args.audit), 'exec'), namespace)
        namespace.update(ROOT=root.resolve(), REPOSITORY=SOURCE)
        if case != 'nested-inventory-file':
            mutate(root, case)
        with contextlib.redirect_stdout(io.StringIO()):
            namespace['inventory']()
        if case == 'nested-inventory-file':
            mutate(root, case)
        stream = io.StringIO()
        reason = ''
        try:
            with contextlib.redirect_stdout(stream), contextlib.redirect_stderr(stream):
                namespace['main']()
            outcome = 'passed'
        except SystemExit as error:
            outcome, reason = 'rejected', str(error.code)
        except Exception as error:
            outcome, reason = 'exception', type(error).__name__+': '+str(error)
        expected = 'passed' if case == 'baseline' or args.mode == 'before' else 'rejected'
        matched = outcome == expected and (expected != 'rejected' or reason.startswith(EXPECTED[case]))
        row = {'case': case, 'outcome': outcome, 'reason': reason, 'expected_outcome': expected,
               'expected_reason_prefix': EXPECTED.get(case), 'matched': matched}
        results.append(row)
        (EVIDENCE/(args.label+'-'+case+'.log')).write_text(stream.getvalue()+
            json.dumps(row, ensure_ascii=False)+'\n', encoding='utf-8', newline='\n')
    report = {'audit_path': str(args.audit.resolve()), 'audit_sha256': hashlib.sha256(code).hexdigest(),
              'harness_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'argv': sys.argv, 'repository_root': str(SOURCE), 'work_root': str(TASK), 'output_root': str(EVIDENCE),
              'mode': args.mode, 'scope': '隔离副本的审计反例，不是新产品执行',
              'note': 'before模式的通过表示原脚本错误放行；after模式要求实际SystemExit理由匹配，普通异常不算通过',
              'passed': all(row['matched'] for row in results), 'cases': results}
    write(EVIDENCE, args.label+'-cases.json', report)
    print(json.dumps({'passed': report['passed'], 'mode': args.mode, 'cases': len(results),
                      'output_root': str(EVIDENCE),
                      'outcomes': {row['case']: row['outcome'] for row in results}}, ensure_ascii=False))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
