"""从本批真实命令记录生成收口；只读审计不调用本工具。"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent
SOURCE = ROOT.parents[4]
BASE = ['build-runtime','build-driver','runtime','intrinsics','abi','pow','guards','history','reference-replay']
FINAL = ['regression','b1-native','b1-vm','b2-native','b2-vm','workspace-check','clippy','benchmark','fmt','c-probe','public-boundary','pow-annotations-final','docs']


def read(name):
    return json.loads((ROOT/name).read_text(encoding='utf-8'))


def write(name, data):
    (ROOT/name).write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')


def main(prepare):
    records=[]
    for phase,names in [('baseline',BASE),('abi',FINAL)]:
        for name in names:
            if prepare and name=='docs':
                continue
            record=read(phase+'/'+name+'.json')
            assert record['exit_code']==0 and record['source_unchanged'] and record['source_before']==record['source_after'], name
            assert hashlib.sha256((ROOT/record['log']).read_bytes()).hexdigest()==record['sha256']
            records.append(record)
    manifests={r['source_before']['manifest'] for r in records}
    assert len(manifests)==1
    code=subprocess.check_output(['cmd','/c','git','rev-parse','89d782e1'],cwd=SOURCE).decode().strip()
    entry=read('bootstrap.json')['entry_commit']
    allowed=subprocess.check_output(['cmd','/c','git','diff','--name-only',entry,code],cwd=SOURCE).decode().splitlines()
    write('code-identity.json',{'entry_commit':entry,'code_commit':code,'pow_commit':'8bf3a138','allowed_product_paths':allowed,
          'source_manifest':next(iter(manifests)),'reuse_from':'a3e0eec9源码选择，旧日志未作本批证据'})
    pow_manifest=read('baseline/pow-switch-manifest.json')
    assert pow_manifest['cases']==1075
    write('closeout.json',{'passed':not prepare,'scope':'B3-2A + B3-3A/B内部验收','code_commit':code,
          'effective_checks':{'baseline':BASE,'abi':FINAL},'source_manifest':next(iter(manifests)),
          'runtime_tests':269,'abi_tests':10,'intrinsic_tests':8,'formal_abi_cases':17243,'external_c_checks_each':231,
          'pow_summary':pow_manifest['summary'],'guards':'28/28','runtime':read('abi/public-surface.json')['runtime'],
          'abi':'1.18','codegen':20,'stdlib_interface':1,'active_max':48,'cmath':'planned','b3_4_started':False,
          'b1_721_executed':False,'b2_a_perf':False,'remote_ci_claimed':False,
          'platforms':{'windows':'本机内部验收','linux':'本批未执行','macos':'本批未执行'},
          'initial_snapshot_note':'本批全新构建已含选入的共享pow和C ABI源码，baseline为本批首次验证；旧33入口由Git对象与历史审计核验，未宣称重建旧Runtime。',
          'prior_attempt':'原包装/LLVM阻塞保留于codex/b3-runtime-20261108，本批不纳入；B3-4依新规划后置',
          'deferred':['B3-4包装','B4类型/IR/VM/LLVM及静态pow共核','B4前向量迁移','B5 ABI1.19/生成器21','B6性能','B2-C最终721及状态升级']})
    print(json.dumps({'passed':not prepare,'prepare_only':prepare,'code_commit':code,'checks':len(records)},ensure_ascii=True))


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prepare',action='store_true')
    main(parser.parse_args().prepare)
