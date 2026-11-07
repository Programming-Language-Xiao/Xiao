"""从固定代码、最终执行记录和实际Runtime字节生成B2内部收口。"""
import hashlib
import json
from pathlib import Path
import subprocess

root=Path(__file__).resolve().parent
source=root.parents[4]
code='37b92c5338274121afe98320d4e076620744449f'
entry='faecd26d'
product_roots=['core/rust','stdlib','tests/fixtures','tests/benchmarks','.github/workflows','.cargo','tools','cli/ts','docs/UseDocs','docs/module-registry.json','package.json','bun.lock','.gitattributes']
paths=subprocess.check_output(['cmd','/c','git','diff','--name-only',entry,code,'--',*product_roots],cwd=source,text=True).splitlines()
identity={'entry_commit':entry,'code_commit':code,'allowed_paths':paths}
(root/'code-identity.json').write_text(json.dumps(identity,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
checks=[
 'b2-final2-check','b2-final2-runtime-build','b2-final2-unit-language','b2-final2-clippy',
 'b2-docs-close','b2-final2-regression','b2-final2-b1-vm-exact','b2-final2-vm-exact',
 'b2-final2-workspace-tests','b2-final2-benchmark','b2-final2-fmt','b2-final2-native-exact',
 'b2-final2-b1-native-exact','b2-final2-guards',
]
snapshots=set()
for name in checks:
 record=json.loads((root/(name+'.json')).read_text(encoding='utf-8'))
 if record['exit_code']!=0 or not record['source_unchanged'] or record['source_before']!=record['source_after']:
  raise SystemExit('有效检查尚未通过：'+name)
 snapshots.add(record['source_before']['manifest'])
if len(snapshots)!=1: raise SystemExit('最终证据不是同一份源码')
env=json.loads((root/'environment.json').read_text(encoding='utf-8'))['overrides']
runtime=Path(env['XIAO_RUNTIME_LIBRARY']).resolve()
closeout={
 'passed':True,'scope':'B2内部lfloat_divide单入口；Windows本机','code_commit':code,'entry_commit':entry,
 'source_manifest':next(iter(snapshots)),'effective_checks':checks,
 'exact_checks':['b2-final2-b1-vm-exact','b2-final2-vm-exact','b2-final2-native-exact','b2-final2-b1-native-exact'],
 'check_roles':{'b2-final-regression':'b2-final2-regression','b2-final-b1-vm-exact':'b2-final2-b1-vm-exact','b2-final-b1-native-exact':'b2-final2-b1-native-exact'},
 'runtime':{'path':str(runtime),'bytes':runtime.stat().st_size,'sha256':hashlib.sha256(runtime.read_bytes()).hexdigest(),'build_record':'b2-final2-runtime-build.json'},
 'runtime_tests':259,'driver_unit':64,'intrinsic_tests':7,'guards':'28/28',
 'abi':'1.18','codegen':20,'active_intrinsic_max':33,'stdlib_interface':1,'cmath':'planned',
 'b1_721_executed':False,'b2_a_perf':False,'b2_c_started':False,'b3_started':False,'ci_executed':False,
 'platforms':{'windows':'本机VM与原生O0/O3已执行','linux':'尚未执行','macos':'尚未执行'},
 'historical_failures_retained':True,
}
(root/'closeout.json').write_text(json.dumps(closeout,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
print(json.dumps({'written':'closeout.json','checks':len(checks),'code_commit':code},ensure_ascii=False))
