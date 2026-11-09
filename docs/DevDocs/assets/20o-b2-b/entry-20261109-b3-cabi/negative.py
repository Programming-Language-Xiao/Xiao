"""临时复制归档的精确拒绝反例，检查退出码、全文原因及只读性。"""
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
          'pow':'B3-CABI pow分类或数值距离不符'}


def write(path,data):
    path.write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')


def snapshot(root):
    return {p.relative_to(root).as_posix():hashlib.sha256(p.read_bytes()).hexdigest() for p in root.rglob('*') if p.is_file()}


def main():
    cases={}
    task=Path(json.loads((ROOT/'bootstrap.json').read_text(encoding='utf-8'))['task_root'])
    for name,reason in EXPECTED.items():
        with tempfile.TemporaryDirectory(prefix='b3-negative-'+name+'-',dir=task) as temporary:
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
            if name in ['target','runtime','pow']:
                rows=[{'path':p.relative_to(root).as_posix(),'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
                      for p in sorted(root.rglob('*')) if p.is_file() and p!=root/'evidence.json' and '__pycache__' not in p.parts]
                write(root/'evidence.json',{'file_count':len(rows),'files':rows})
            before=snapshot(root)
            argv=[sys.executable,'-X','utf8',str(ROOT/'audit.py'),'--root',str(root),'--source-root',str(SOURCE),'--core-only']
            run=subprocess.run(argv,capture_output=True,text=True,encoding='utf-8',env=dict(os.environ,PYTHONUTF8='1',PYTHONIOENCODING='utf-8'))
            output=(run.stdout+run.stderr).strip();readonly=before==snapshot(root)
            passed=readonly and run.returncode==(1 if reason else 0) and (output==reason if reason else json.loads(output)['passed'] is True)
            cases[name]={'exit_code':run.returncode,'output':output,'expected_reason':reason,'read_only':readonly,'passed':passed,'argv':argv}
            print(json.dumps({'case':name,**cases[name]},ensure_ascii=True),flush=True)
    result={'passed':all(c['passed'] for c in cases.values()),'cases':cases,'scope':'临时复制及三种明确重建清单的语义反例；不改真实证据'}
    write(ROOT/'negative-result.json',result)
    if not result['passed']:raise SystemExit('自反例失败')


if __name__=='__main__':main()
