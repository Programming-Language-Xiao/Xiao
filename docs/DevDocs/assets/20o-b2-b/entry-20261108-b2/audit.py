"""B2 单入口证据只读审计；固定 Git 代码对象，不要求当前 HEAD。"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

DEFAULT_ROOT=Path(__file__).resolve().parent
DEFAULT_SOURCE=DEFAULT_ROOT.parents[4]
ENTRY="faecd26d"
CODE_COMMIT="37b92c5338274121afe98320d4e076620744449f"
REQUIRED_CHECKS=[
 'b2-final2-check','b2-final2-runtime-build','b2-final2-unit-language','b2-final2-clippy',
 'b2-docs-close','b2-final2-regression','b2-final2-b1-vm-exact','b2-final2-vm-exact',
 'b2-final2-workspace-tests','b2-final2-benchmark','b2-final2-fmt','b2-final2-native-exact',
 'b2-final2-b1-native-exact','b2-final2-guards',
]
EXACT_CHECKS=['b2-final2-b1-vm-exact','b2-final2-vm-exact','b2-final2-native-exact','b2-final2-b1-native-exact']
CHECK_ROLES={'b2-final-regression':'b2-final2-regression','b2-final-b1-vm-exact':'b2-final2-b1-vm-exact','b2-final-b1-native-exact':'b2-final2-b1-native-exact'}
ALLOWED_PATHS={
 'core/rust/crates/xiao-codegen-llvm/src/dynamic/expression.rs',
 'core/rust/crates/xiao-codegen-llvm/src/dynamic/predicate.rs',
 'core/rust/crates/xiao-codegen-llvm/src/dynamic/runtime_abi.rs',
 'core/rust/crates/xiao-driver/tests/20o_b1_probe.rs',
 'core/rust/crates/xiao-driver/tests/20o_b2_b2_divide.rs',
 'core/rust/crates/xiao-driver/tests/support/b2_divide.rs',
 'core/rust/crates/xiao-intrinsics/build.rs',
 'core/rust/crates/xiao-intrinsics/intrinsics.json',
 'core/rust/crates/xiao-intrinsics/src/lib.rs',
 'core/rust/crates/xiao-runtime-abi/README.md',
 'core/rust/crates/xiao-runtime-abi/src/lib.rs',
 'core/rust/crates/xiao-runtime/src/abi.rs',
 'core/rust/crates/xiao-runtime/src/abi_lfloat_divide.rs',
 'core/rust/crates/xiao-runtime/src/lib.rs',
 'core/rust/crates/xiao-runtime/src/value/arithmetic/explicit_division.rs',
 'core/rust/crates/xiao-runtime/src/value/arithmetic/mod.rs',
 'core/rust/crates/xiao-runtime/src/value/mod.rs',
 'core/rust/crates/xiao-types/src/checker/expression.rs',
 'core/rust/crates/xiao-vm/src/semantics/exec.rs',
 'core/rust/crates/xiao-vm/src/semantics/numeric_intrinsics.rs',
 'docs/UseDocs/language/basics/types/conversions.md',
 'tests/fixtures/docs/numeric/b2_explicit_divide.stdout',
 'tests/fixtures/docs/numeric/b2_explicit_divide.xiao',
}
PRODUCT_ROOTS=["core/rust","stdlib","tests/fixtures","tests/benchmarks",".github/workflows",".cargo","tools","cli/ts","docs/UseDocs","docs/module-registry.json","package.json","bun.lock",".gitattributes"]

def require(condition,message):
    if not condition: raise SystemExit(message)

def digest(path): return hashlib.sha256(path.read_bytes()).hexdigest()

def canonical(raw):
    try: raw.decode('utf-8')
    except UnicodeDecodeError: return raw
    return raw if b'\0' in raw else raw.replace(b'\r\n',b'\n')

def main(root,source):
    def read(name): return json.loads((root/name).read_text(encoding='utf-8'))
    def git(*args): return subprocess.check_output(['cmd','/c','git',*args],cwd=source)
    manifest=read('evidence.json')
    paths=sorted(p for p in root.rglob('*') if p.is_file() and p!=root/'evidence.json' and '__pycache__' not in p.parts)
    rows=[{'path':p.relative_to(root).as_posix(),'bytes':p.stat().st_size,'sha256':digest(p)} for p in paths]
    require(manifest['file_count']==len(rows) and manifest['files']==rows,'B2证据清单或摘要不符')
    closeout=read('closeout.json');identity=read('code-identity.json');code=identity['code_commit']
    require(closeout['passed'] is True and closeout['code_commit']==code==CODE_COMMIT,'B2收口或代码身份不符')
    require(closeout.get('effective_checks')==REQUIRED_CHECKS,'B2有效执行点集合不完整')
    require(closeout.get('exact_checks')==EXACT_CHECKS,'B2精确入口集合不完整')
    require(closeout.get('check_roles')==CHECK_ROLES,'B2执行点角色映射不符')
    require((closeout.get('runtime_tests'),closeout.get('driver_unit'),closeout.get('intrinsic_tests'),closeout.get('guards'))==(259,64,7,'28/28'),'B2收口测试计数不符')
    require(closeout['abi']=='1.18' and closeout['codegen']==20 and closeout['cmath']=='planned','B2版本或文档状态越界')
    require(closeout.get('active_intrinsic_max')==33 and closeout.get('stdlib_interface')==1,'B2收口公开面不符')
    require(closeout['b1_721_executed'] is False and closeout['b2_c_started'] is False,'B2执行范围越界')
    changed=set(git('diff','--name-only',ENTRY,code,'--',*PRODUCT_ROOTS).decode().splitlines())
    require(changed==set(identity['allowed_paths'])==ALLOWED_PATHS,'B2产品改动不在冻结允许集合')
    def blob(path): return git('show',code+':'+path)
    before=json.loads(git('show',ENTRY+':core/rust/crates/xiao-intrinsics/intrinsics.json'))['intrinsics']
    after=json.loads(blob('core/rust/crates/xiao-intrinsics/intrinsics.json'))['intrinsics']
    require([row for row in after if row['id']!=33]==before,'B2改写了既有intrinsic或墓碑')
    new=[row for row in after if row['id']==33]
    require(len(new)==1 and new[0]=={
        'id':33,'name':'lfloat_divide','kind':'lfloat_divide','arity':'four','parameter_type':'lfloat_divide_parameters',
        'return_type':'lfloat','vm_binding':'lfloat_divide','abi_binding':'lfloat_divide','effect':'allocates',
        'capability':'none','error':'runtime.type_mismatch','status':'active'},'B2 intrinsic33签名不符')
    require(blob('docs/module-registry.json')==git('show',ENTRY+':docs/module-registry.json'),'B2 registry发生变化')
    require(not git('diff','--name-only',ENTRY,code,'--','stdlib','.github/workflows','.cargo','tools','cli/ts','package.json','bun.lock','core/rust/Cargo.lock','tests/benchmarks/Cargo.lock').strip(),'B2受保护路径发生变化')
    logs={};snapshots=set()
    for name in closeout['effective_checks']:
        record=read(name+'.json');log=root/record['log']
        require(record['exit_code']==0 and digest(log)==record['sha256'],'B2验收失败或日志摘要不符：'+name)
        require(record['source_unchanged'] and record['source_before']==record['source_after'],'B2执行期间源码变化：'+name)
        require(record['runner_sha256']==digest(root/'run-step.py'),'B2执行器身份不符')
        snapshots.add(record['source_before']['manifest']);logs[name]=log.read_text(encoding='utf-8',errors='strict')
    require(len(snapshots)==1,'B2有效验收没有绑定同一份源码')
    summary=r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;'
    for name in closeout['exact_checks']:
        record=read(name+'.json')
        require('--exact' in record['argv'] and re.findall(summary,logs[name],re.M)==[('1','0','0')],'B2精确入口计数不符：'+name)
    for role,actual in closeout['check_roles'].items():
        logs[role]=logs[actual]
    require(sorted(re.findall(summary,logs['b2-final2-unit-language'],re.M))==sorted([('64','0','1'),('2','0','1'),('7','0','0'),('259','0','0')]),'B2单元/语言测试计数不符')
    require(re.findall(summary,logs['b2-final-regression'],re.M)==[('9','0','0'),('17','0','0')],'B1契约/资源计数不符')
    guards=read('b1-guards-report.json')
    require(guards['passed']==guards['total']==len(guards['cases'])==28 and all(c['Passed'] for c in guards['cases']),'B1守卫不是28/28')
    require('B2-B2-DIVIDE backend=vm' in logs['b2-final-b1-vm-exact'] and 'B2-B2-DIVIDE backend=native' in logs['b2-final-b1-native-exact'],'B2示例未进入维护精确入口')
    snapshot=read(next(iter(snapshots)))
    tracked=git('ls-tree','-r','--name-only','-z',code,'--',*PRODUCT_ROOTS).decode('utf-8').split('\0')
    tracked={path for path in tracked if path}
    listed=[row['path'] for row in snapshot['files']]
    require(set(listed)==tracked and len(listed)==len(tracked),'B2源码清单范围不完整')
    raw_tree=hashlib.sha256(json.dumps(snapshot['files'],ensure_ascii=False,sort_keys=True,separators=(',',':')).encode()).hexdigest()
    canonical_tree=hashlib.sha256(json.dumps([[row['path'],row['utf8_lf_or_binary_sha256']] for row in snapshot['files']],ensure_ascii=False,separators=(',',':')).encode()).hexdigest()
    require(snapshot['raw_tree_sha256']==raw_tree and snapshot['utf8_lf_tree_sha256']==canonical_tree,'B2源码树摘要不符')
    require(closeout['source_manifest']==next(iter(snapshots)),'B2收口源码清单与执行记录不符')
    for name in REQUIRED_CHECKS:
        record=read(name+'.json')['source_before']
        require(record['files']==len(listed) and record['raw_tree_sha256']==snapshot['raw_tree_sha256'] and record['utf8_lf_tree_sha256']==snapshot['utf8_lf_tree_sha256'],'B2源码清单身份与执行记录不符')
    requests=''.join(code+':'+row['path']+'\n' for row in snapshot['files']).encode()
    raw=subprocess.check_output(['cmd','/c','git','cat-file','--batch'],cwd=source,input=requests)
    offset=0
    for row in snapshot['files']:
        end=raw.index(b'\n',offset);header=raw[offset:end].split()
        require(len(header)==3 and header[1]==b'blob','B2代码提交缺少源码：'+row['path'])
        length=int(header[2]);content=raw[end+1:end+1+length];offset=end+length+2
        require(hashlib.sha256(canonical(content)).hexdigest()==row['utf8_lf_or_binary_sha256'],'B2被测源码偏离代码提交：'+row['path'])
    require(offset==len(raw),'B2源码对象流长度不符')
    runtime=closeout['runtime'];runtime_path=Path(runtime['path']).resolve();bootstrap=read('bootstrap.json');task=Path(bootstrap['task_root']).resolve()
    require(runtime_path.is_relative_to(task) and runtime_path.is_file(),'B2Runtime不在独立任务根内')
    require(runtime_path.stat().st_size==runtime['bytes'] and digest(runtime_path)==runtime['sha256'],'B2Runtime摘要不符')
    override=read('environment.json')['overrides']
    require(Path(override['XIAO_RUNTIME_LIBRARY']).resolve()==runtime_path,'B2Runtime环境路径与收口身份不符')
    require(runtime_path.is_relative_to(Path(override['CARGO_TARGET_DIR']).resolve()),'B2Runtime不在独立target内')
    require(str(override['XIAO_B1_F10'])=='0' and str(override['XIAO_B2_A_PERF'])=='0','B2错误启用性能开关')
    for key in ['CARGO_TARGET_DIR','XIAO_HOME','TEMP','TMP','XIAO_RUNTIME_LIBRARY']:
        require(Path(override[key]).resolve().is_relative_to(task),'B2路径越出独立任务根：'+key)
    symbols=read('native-symbols.json')
    require(symbols['exit_code']==0 and symbols['runtime_sha256']==runtime['sha256'],'B2 C符号表不属于被测Runtime')
    require('xiao_runtime_lfloat_divide' in symbols['symbols'] and not any(name.startswith('xiao_runtime_cmath_') for name in symbols['symbols']),'B2 C符号边界不符')
    negative=read('self-negative-result.json')
    expected={'baseline':(0,None),'tampered_manifest':(1,'B2证据清单或摘要不符'),
              'missing_attributes':(1,'B2证据清单或摘要不符'),
              'semantic_tamper_after_temp_manifest_rebuild':(1,'B2版本或文档状态越界')}
    require(negative['passed'] is True and set(negative['cases'])==set(expected),'B2自反例集合不完整')
    for name,(exit_code,reason) in expected.items():
        case=negative['cases'][name]
        require(case['read_only'] is True and case['passed'] is True and case['exit_code']==exit_code,'B2自反例未证明只读拒绝：'+name)
        if reason:require(case['output']==reason,'B2自反例拒绝原因不符：'+name)
    print(json.dumps({'passed':True,'files':len(rows),'checks':len(closeout['effective_checks']),'exact_checks':len(closeout['exact_checks']),'code_commit':code,'abi':'1.18','codegen':20,'max_active_intrinsic':33,'scope':'B2内部单入口'},ensure_ascii=False))

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--root',type=Path,default=DEFAULT_ROOT);parser.add_argument('--source-root',type=Path,default=DEFAULT_SOURCE);args=parser.parse_args()
    main(args.root.resolve(),args.source_root.resolve())
