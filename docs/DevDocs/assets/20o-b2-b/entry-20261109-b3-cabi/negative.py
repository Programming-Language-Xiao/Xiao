"""临时复制归档的精确拒绝反例，检查退出码、全文原因及只读性。"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT=Path(__file__).resolve().parent
SOURCE=ROOT.parents[4]
EXPECTED={'baseline':None,'manifest':'B3-CABI证据清单或摘要不符','attributes':'B3-CABI证据清单或摘要不符',
          'target':'B3-CABI精确入口命令或测试名不符','runtime':'B3-CABI Runtime身份与构建记录不符',
          'pow':'B3-CABI pow分类或数值距离不符',
          'closeout_counts':'B3-CABI收口统计或平台状态不符',
          'pow_rule_distance':'B3-CABI pow规则、可达性或数值距离不符',
          'c_optimization':'B3-CABI外部C编译或执行参数不符',
          'source_record_identity':'B3-CABI执行记录源码身份不符'}


def write(path,data):
    path.write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')


def snapshot(root):
    return {p.relative_to(root).as_posix():hashlib.sha256(p.read_bytes()).hexdigest() for p in root.rglob('*') if p.is_file()}


def main(source):
    cases={}
    for name,reason in EXPECTED.items():
        with tempfile.TemporaryDirectory(prefix='b3-negative-'+name+'-') as temporary:
            root=Path(temporary)/'evidence'
            shutil.copytree(ROOT,root,ignore=shutil.ignore_patterns('__pycache__'))
            def mutate(path,fn):
                document=json.loads((root/path).read_text(encoding='utf-8'));fn(document);write(root/path,document)
            if name=='manifest':
                mutate('evidence.json',lambda d:d.update(file_count=d['file_count']+1))
            elif name=='attributes':
                (root/'.gitattributes').unlink()
            elif name=='target':
                def change(d):d['argv'][d['argv'].index('lfloat_divide_has_vm_golden_outputs')]='unrelated_test'
                mutate('abi/b2-vm.json',change)
            elif name=='runtime':
                mutate('abi/public-surface.json',lambda d:d['runtime'].update(sha256='0'*64))
            elif name=='pow':
                def change(d):
                    next(r for r in d['rows'] if r['difference']=='value_bits')['old_new_ulp']=0
                mutate('baseline/pow-switch-manifest.json',change)
            elif name=='closeout_counts':
                mutate('closeout.json',lambda d:d.update(runtime_tests=0,formal_abi_cases=1,external_c_checks_each=0))
            elif name=='pow_rule_distance':
                def change(d):
                    row=next(r for r in d['rows'] if r['difference']=='same' and r['new_reference_ulp'] is not None)
                    row.update(frozen_rule='F-2:invented-unfrozen-rule',new_reference_ulp=999)
                mutate('baseline/pow-switch-manifest.json',change)
            elif name=='c_optimization':
                def change(d):
                    row=next(r for r in d['records'] if r['name']=='compile-o0')
                    row['argv'][row['argv'].index('-O0')]='-O3'
                mutate('abi/c-probe-result.json',change)
            elif name=='source_record_identity':
                def change(d):
                    d['source_before']['raw_tree_sha256']='0'*64
                    d['source_after']['raw_tree_sha256']='0'*64
                mutate('abi/b2-vm.json',change)
            if name not in ['baseline','manifest','attributes']:
                rows=[{'path':p.relative_to(root).as_posix(),'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
                      for p in sorted(root.rglob('*')) if p.is_file() and p!=root/'evidence.json' and '__pycache__' not in p.parts]
                write(root/'evidence.json',{'file_count':len(rows),'files':rows})
            before=snapshot(root)
            argv=[sys.executable,'-X','utf8',str(ROOT/'audit.py'),'--root',str(root),'--source-root',str(source),'--core-only']
            run=subprocess.run(argv,capture_output=True,text=True,encoding='utf-8',env=dict(os.environ,PYTHONUTF8='1',PYTHONIOENCODING='utf-8'))
            output=(run.stdout+run.stderr).strip();readonly=before==snapshot(root)
            passed=readonly and run.returncode==(1 if reason else 0) and (output==reason if reason else json.loads(output)['passed'] is True)
            cases[name]={'exit_code':run.returncode,'output':output,'expected_reason':reason,'read_only':readonly,'passed':passed,'argv':argv}
            print(json.dumps({'case':name,**cases[name]},ensure_ascii=True),flush=True)
    result={'passed':all(c['passed'] for c in cases.values()),'cases':cases,'scope':'临时复制及明确重建清单的语义反例；不改真实证据'}
    write(ROOT/'negative-result.json',result)
    if not result['passed']:raise SystemExit('自反例失败')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root',type=Path,default=SOURCE)
    main(parser.parse_args().source_root.resolve())
