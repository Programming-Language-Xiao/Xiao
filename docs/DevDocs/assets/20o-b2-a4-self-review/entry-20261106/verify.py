"""只读复核本次自审证据；--manifest仅重建本目录文件摘要。"""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def require(value, message):
    if not value:
        raise SystemExit(message)


def path(name):
    result = (ROOT/name).resolve()
    require(not Path(name).is_absolute() and result.is_relative_to(ROOT), '自审引用越界：'+name)
    return result


def read(name):
    return json.loads(path(name).read_text(encoding='utf-8'))


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def inventory():
    return {item.relative_to(ROOT).as_posix(): item for item in ROOT.rglob('*')
            if item.is_file() and item != ROOT/'evidence.json'}


def manifest():
    rows = [{'path':name, 'bytes':len(item.read_bytes()), 'sha256':sha(item.read_bytes())}
            for name,item in sorted(inventory().items())]
    (ROOT/'evidence.json').write_text(json.dumps({'file_count':len(rows), 'files':rows}, ensure_ascii=False, indent=2)+'\n', encoding='utf-8', newline='\n')
    print(json.dumps({'files':len(rows)}))


def verify():
    listing = read('evidence.json')
    files = inventory()
    require(len(listing['files']) == listing['file_count'] == len(files), '自审文件计数不符')
    require({row['path'] for row in listing['files']} == set(files), '自审文件清单缺项或重复')
    for row in listing['files']:
        raw = path(row['path']).read_bytes()
        require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], '自审摘要不符：'+row['path'])
    for label, script in [('before-final', 'original-audit.py.txt'), ('after-final', 'fixed-audit.py.txt')]:
        report = read(label+'-cases.json')
        require(report['audit_sha256'] == sha(path(script).read_bytes()), '审计版本不符：'+label)
        require(report['harness_sha256'] == sha(path('audit-cases.py').read_bytes()), '反例执行器版本不符')
        require(report['passed'] and len(report['cases']) == 13 and len({row['case'] for row in report['cases']}) == 13, '反例清单不符')
        for row in report['cases']:
            actual = json.loads(path(label+'-'+row['case']+'.log').read_text(encoding='utf-8').splitlines()[-1])
            require(actual == row and row['matched'], '反例原始记录不符')
            expected = 'passed' if label == 'before-final' or row['case'] == 'baseline' else 'rejected'
            require(row['outcome'] == expected, '反例结果不符')
            if expected == 'rejected':
                require(row['reason'].startswith(row['expected_reason_prefix']), '反例拒绝原因不符')
    vectors = read('vectors.json')
    raw = path('conversion-vectors.txt').read_bytes()
    require(sha(raw) == vectors['sha256'] and len(raw.splitlines()) == vectors['vectors'] == 6046, '数值输入不符')
    result = read('review-conversion-differential.log')
    require(result == {'vectors':6046, 'comparisons':12092, 'mismatches':0}, '独立数值对照不符')
    for name in read('result.json')['checks']:
        row = read(name+'.json')
        require(row['exit_code'] == 0 and row['source_before'] == row['source_after'] and row['source_unchanged'], '自审命令未通过：'+name)
        require(sha(path(row['log']).read_bytes()) == row['sha256'], '自审命令日志不符')
        require(row['source_before']['utf8_lf_tree_sha256'] == read('result.json')['product_utf8_lf_tree_sha256'], '产品输入改变')
    require(read('boundary.json')['passed'], '自审边界不符')
    if 'fix_ci' in read('result.json'):
        ci = read(read('result.json')['fix_ci'])
        require(ci['head_sha'] == read('result.json')['fix_commit'] and ci['conclusion'] == 'success', '自审修复CI不符')
    print(json.dumps({'passed':True, 'positive_cases':1, 'negative_cases':12, 'numeric_comparisons':12092}, ensure_ascii=False))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--manifest', action='store_true')
    manifest() if parser.parse_args().manifest else verify()
