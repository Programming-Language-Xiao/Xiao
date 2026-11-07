"""只读复核第二轮的误接受、修复后拒绝、原件保持与实际命令；不重新测量产品。"""
import argparse,hashlib,json,subprocess,sys
from pathlib import Path
sys.dont_write_bytecode=True
ROOT=Path(__file__).resolve().parent
ARCHIVE=ROOT.parent/'entry-20261107'
SOURCE=ROOT.parents[4]
def require(condition,reason):
    if not condition:raise SystemExit(reason)
def read(path):return json.loads(path.read_text(encoding='utf-8'))
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def inventory(write):
    paths=sorted(p for p in ROOT.rglob('*') if p.is_file() and p!=ROOT/'evidence.json' and '__pycache__' not in p.parts)
    rows=[{'path':p.relative_to(ROOT).as_posix(),'bytes':p.stat().st_size,'sha256':sha(p)} for p in paths]
    if write:
        (ROOT/'evidence.json').write_text(json.dumps({'scope':'第二轮自审全部实际文件；不递归计清单自身或Python缓存','files':rows,'file_count':len(rows)},ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
    else:
        manifest=read(ROOT/'evidence.json');require(manifest['file_count']==len(rows) and manifest['files']==rows,'第二轮归档集合或摘要不符')
    return len(rows)
def main():
    files=inventory(False)
    entry=read(ROOT/'before-evidence.json')
    allowed={'audit.py','verify-performance.py','validate-audit.py','verify-negative-cases.py','README.md'}
    changed=set()
    for item in entry['files']:
        if sha(ARCHIVE/item['path'])!=item['sha256']:changed.add(item['path'])
    require(changed==allowed,'第二轮修改了工具/说明之外的历史字节')
    for item in read(ROOT/'before-tools.json')['tools']:
        require(sha(ROOT/item['copy'])==item['sha256'],'旧工具备份摘要不符')
    before=read(ROOT/'before/cases.json');after=read(ROOT/'after/cases.json')
    require(len(before['cases'])==len(after['cases'])==4,'本轮四项误接受复现集合不完整')
    for phase,cases in [('before',before['cases']),('after',after['cases'])]:
        for row in cases:
            stdout=(ROOT/phase/row['stdout']).read_text(encoding='utf-8');stderr=(ROOT/phase/row['stderr']).read_text(encoding='utf-8')
            if phase=='before':require(row['exit_code']==0 and not stderr and json.loads(stdout)['passed'],'旧版没有实际复现误接受')
            else:require(row['exit_code']==1 and not stdout and stderr.strip()==row['expected_after_error'],'修复后的拒绝原因不符')
    repeat=read(ROOT/'before/fabricated-repeat.json')
    require(repeat['exit_code']==0 and repeat['accepted'] and json.loads((ROOT/'before/fabricated-repeat.stdout.log').read_text())['passed'],'旧校准次数误接受未实际复现')
    controls=read(ROOT/'final-process-controls/cases.json')
    require(controls['passed'] and len(controls['cases'])==6 and controls['archive_manifest_unchanged'],'逐进程与输出隔离反例不完整')
    for row in controls['cases']:
        require(row['exit_code']==1 and not (ROOT/'final-process-controls'/row['stdout']).read_bytes()
                and (ROOT/'final-process-controls'/row['stderr']).read_text(encoding='utf-8').strip()==row['expected_error'],'逐进程反例实际原因不符')
    negative=read(ROOT/'final-process-controls/report-negative-cases.json')
    require(negative['passed'] and len(negative['cases'])==11 and all(row['passed'] and row['exit_code']==1 and not row['actual_stdout'] and row['actual_stderr'].strip()==row['expected_error'] for row in negative['cases']),'11项报告反例没有按实际原因拒绝')
    archive=read(ROOT/'archive-controls/negative-cases.json')
    require(archive['passed'] and len(archive['cases'])==10,'完整归档1正9反不完整')
    for row in archive['cases']:
        out=(ROOT/'archive-controls'/row['stdout']).read_text(encoding='utf-8');error=(ROOT/'archive-controls'/row['stderr']).read_text(encoding='utf-8')
        if row['case']=='valid':
            value=json.loads(out)
            require(row['exit_code']==0 and not error and value['passed'] and value['checks']==18 and value['source_files']==1029 and len(value['performance'])==6,'完整归档正例未通过')
        else:require(row['exit_code']==1 and not out and error.strip()==row['expected_error'],'完整归档反例实际原因不符')
    for name in ['after-reproduction','final-process-and-report-controls','archive-validation','final-docs-gate-2','final-archive-audit','ci-completion']:
        record=read(ROOT/(name+'.json'))
        require(record['exit_code']==0 and record['product_before']==record['product_after'] and record['product_after']['no_product_difference'],'实际复核命令失败或产品输入改变：'+name)
        require(sha(ROOT/record['stdout'])==record['stdout_sha256'] and sha(ROOT/record['stderr'])==record['stderr_sha256'],'实际复核输出摘要不符：'+name)
    ci=json.loads((ROOT/'ci-completion.stdout.log').read_text(encoding='utf-8').strip().splitlines()[-1])
    require(ci['run_id']==42340653748 and ci['conclusion']=='success' and ci['code_commit']=='bbf9feaa76e1fd5083bf7924a363a31baf4c69a9','第二轮自审CI回执不符')
    print(json.dumps({'passed':True,'scope':'A6第二轮自审','files':files,'product_commit':'e7bf7a49ba3ed55679b69c97ac1bd5ce7f5777b6','reproduced_false_acceptances':5,'archive_cases':10,'report_negative_cases':11,'process_and_output_negative_cases':6,'original_measurements_unchanged':True},ensure_ascii=False))
if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--manifest',action='store_true');args=parser.parse_args()
    if args.manifest:print(json.dumps({'files':inventory(True)}))
    else:main()
