"""A7内部收口只读审计；固定Git源码与阶段原件，不运行产品或性能测量。"""
import argparse,hashlib,importlib.util,json,re,subprocess,sys
from pathlib import Path
sys.dont_write_bytecode=True
ROOT=Path(__file__).resolve().parent
REPOSITORY=ROOT.parents[4]
HELPER_REL='docs/DevDocs/assets/20o-b2-a5/entry-20261106/audit.py'
HELPER_SHA='7d9628ee3eb72629b385e88072b129edf00a59697d9119f18583421488b62525'
BASELINE='e7bf7a49ba3ed55679b69c97ac1bd5ce7f5777b6'
CANDIDATE='5749d6e0f277ba9aebf2bf8468e0c79a082e42a6'
DOC_PATHS={'docs/UseDocs/language/basics/p1-expressions/operators.md','docs/UseDocs/language/basics/types/conversions.md','docs/UseDocs/language/memory/runtime/README.md','docs/UseDocs/library/README.md','tests/fixtures/docs/numeric/README.md'}
PROTECTED=['core/rust','stdlib','tests/benchmarks','.github/workflows','.cargo','tools','cli/ts','docs/module-registry.json','package.json','bun.lock','.gitattributes']
EXACT={'b1-vm-exact':'b1_capabilities_have_vm_golden_outputs','b1-native-exact':'b1_capabilities_match_native_golden_outputs','a1-vm-exact':'a1_numeric_values_and_errors_have_vm_golden_outputs','a1-native-exact':'a1_numeric_values_and_errors_match_native_golden_outputs'}
B1_NAMES={'math':['abs','min','max','clamp','gcd','pow'],'hex':['encode','decode'],'base64':['encode','decode'],'url':['encode_component','decode_component'],'csv':['parse_record','format_record'],'fmt':['pad_left','pad_right','format_int'],'statistics':['sum','min','max']}
COUNTS={'b1-contract':[(9,0)],'b1-resources':[(17,0)],'b2-resources':[(3,0),(8,0)],'b2-language':[(12,1)],'driver-unit':[(64,1)],**{name:[(1,0)] for name in EXACT}}
HISTORICAL={'entry-a0-audit':'docs/DevDocs/assets/20o-b2-a0/entry-20261105/verify-evidence.py','entry-a1-audit':'docs/DevDocs/assets/20o-b2-a1/entry-20261106/audit.py','entry-a2-a3-audit':'docs/DevDocs/assets/20o-b2-a2-a3/entry-20261106/audit.py','entry-a4-audit':'docs/DevDocs/assets/20o-b2-a4/entry-20261106/audit.py','entry-a5-audit':HELPER_REL,'entry-a6-audit':'docs/DevDocs/assets/20o-b2-a6/entry-20261107/audit.py','entry-a6-review':'docs/DevDocs/assets/20o-b2-a6/self-review-20261107/verify-review.py'}
REQUIRED={'build-runtime','build-adapters','b1-contract','b1-resources','b2-resources','b2-language','package-tests','driver-unit','b1-vm-exact','b1-native-exact','a1-vm-exact','a1-native-exact','workspace-check','benchmarks-check','workspace-clippy','workspace-fmt','b1-guards','docs-final',*HISTORICAL}

def require(value,reason):
    if not value:raise SystemExit(reason)
def sha(raw):return hashlib.sha256(raw).hexdigest()
def path(name):
    require(isinstance(name,str) and name and not Path(name).is_absolute(),'A7证据引用不是相对路径')
    result=(ROOT/name).resolve();require(result.is_relative_to(ROOT.resolve()),'A7证据引用越界');return result
def read(name):return json.loads(path(name).read_text(encoding='utf-8'))
def git(*args):return subprocess.check_output(['cmd','/c','git',*args],cwd=REPOSITORY)
def helper():
    script=REPOSITORY/HELPER_REL;require(sha(script.read_bytes())==HELPER_SHA,'固定通用源码/作业审计函数发生变化')
    spec=importlib.util.spec_from_file_location('a7_common',script);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);module.ROOT=ROOT;module.REPOSITORY=REPOSITORY;return module

def inventory(write=False):
    files={p.relative_to(ROOT).as_posix():p for p in ROOT.rglob('*') if p.is_file() and p!=ROOT/'evidence.json' and '__pycache__' not in p.parts}
    rows=[{'path':name,'bytes':p.stat().st_size,'sha256':sha(p.read_bytes())} for name,p in sorted(files.items())]
    if write:(ROOT/'evidence.json').write_text(json.dumps({'scope':'A7本目录完整原件；清单不递归计自身或Python缓存','file_count':len(rows),'files':rows},ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
    else:
        value=read('evidence.json');require(value['file_count']==len(rows) and value['files']==rows,'A7原件清单或字节摘要不符')
    return len(rows)

def cargo_arguments():
    m=['--locked','--manifest-path','core/rust/Cargo.toml'];driver=['test',*m,'-p','xiao-driver','--test']
    return {'build-runtime':['build',*m,'--release','-p','xiao-runtime'],'build-adapters':['build',*m,'-p','xiao-doc-coverage-rust','-p','xiao-diagnostics'],'b1-contract':[*driver,'20o_b1_imp_contract'],'b1-resources':[*driver,'20o_b1_imp_resources'],'b2-resources':[*driver,'20o_b2_a_resources','--test','20o_b2_a_performance_protocol'],'b2-language':[*driver,'20o_b2_a_contract','--','--nocapture'],'package-tests':['test',*m,'-p','xiao-types','-p','xiao-ir','-p','xiao-bytecode','-p','xiao-runtime','-p','xiao-vm','-p','xiao-codegen-llvm','-p','xiao-diagnostics','--lib','--tests'],'driver-unit':['test',*m,'-p','xiao-driver','--lib'],'b1-vm-exact':[*driver,'20o_b1_probe','--',EXACT['b1-vm-exact'],'--exact','--nocapture'],'b1-native-exact':[*driver,'20o_b1_probe','--','--ignored',EXACT['b1-native-exact'],'--exact','--nocapture'],'a1-vm-exact':[*driver,'20o_b2_a1_identity',EXACT['a1-vm-exact'],'--','--exact','--nocapture'],'a1-native-exact':[*driver,'20o_b2_a1_identity',EXACT['a1-native-exact'],'--','--ignored','--exact','--nocapture'],'workspace-check':['check',*m,'--workspace'],'benchmarks-check':['check','--locked','--manifest-path','tests/benchmarks/Cargo.toml'],'workspace-clippy':['clippy',*m,'--workspace','--all-targets','--','-D','warnings']}

def command(name,record):
    args=record['argv'];exe=Path(args[0]).name.lower().removesuffix('.exe')
    if name in cargo_arguments():require(exe=='cargo' and args[1:]==['+1.96.0',*cargo_arguments()[name]],'A7实际Cargo参数偏离冻结入口：'+name)
    elif name in HISTORICAL:require(exe in {'python','python3'} and args[1:]==['-X','utf8',HISTORICAL[name]],'A7阶段审计命令不符')
    elif name=='workspace-fmt':require(exe=='rustup' and args[1:]==['run','1.96.0','cargo','fmt','--manifest-path','core/rust/Cargo.toml','--all','--','--check'],'A7格式门禁命令不符')
    elif name in {'docs-final','docs-closeout'}:require(exe=='bun' and args[1:]==['tools/repo-check/src/cli.ts','all'],'A7文档门禁命令不符')
    elif name=='b1-guards':require(exe=='pwsh' and len(args)==8 and args[1:5]==['-NoProfile','-File','tools/gates/b1-probe-guards.ps1','-BashPath'] and args[6]=='-ReportPath' and Path(args[7])==Path(record['cwd'])/'docs/DevDocs/assets/20o-b2-a7/entry-20261107/b1-guards-report.json','A7原28守卫命令不符')
    else:require(False,'A7执行点未登记')

def state(source):
    registry=json.loads(git('show',source+':docs/module-registry.json'));states={r['id']:r['status'] for r in registry['modules']}
    require(states['stdlib.cmath']=='planned' and all(states['stdlib.'+m]=='verified' for m in ['math','hex','base64','url','csv','fmt','statistics']),'A7库状态越界')
    for page in ['docs/UseDocs/language/basics/types/conversions.md','docs/UseDocs/language/basics/p1-expressions/operators.md']:
        raw=git('show',source+':'+page).decode();require('status: verified' in raw and '（planned）' in raw and '内部' in raw and '最终' in raw,'A7数值教程内部/最终边界缺失')
    for module,names in B1_NAMES.items():
        code=git('show',source+':stdlib/'+module+'.xiao').decode()
        require(re.findall(r'^def (\w+)\(',code,re.M)==names,'B1实际源码导出集合偏离20项冻结入口')
    require(not git('diff','--name-only',BASELINE,source,'--',*PROTECTED),'A7修改受保护产品/锁/registry/工作流')
    changed=git('diff','--name-only',BASELINE,source,'--','tests/fixtures','docs/UseDocs').decode().splitlines();require(set(changed)==DOC_PATHS,'A7教程或夹具改动超出文档范围')

def main():
    common=helper();files=inventory();closeout=read('closeout.json')
    require(closeout['passed'] and closeout['scope']=='B2-A内部1.18/20收口','A7尚未有效收口')
    require(closeout['code_commit']==CANDIDATE and closeout['product_commit']==BASELINE,'A7源码快照身份不符')
    require(closeout['abi']=='1.18' and closeout['codegen']==20 and closeout['active_intrinsic_max']==32 and closeout['standard_library_interface']==1,'A7版本边界漂移')
    require(not closeout['b1_721_executed'] and not closeout['b2_b_c_started'] and not closeout['public_release'] and not closeout['independent_debts_closed'] and closeout['cmath']==closeout['final_numeric_tutorial_status']=='planned','A7后续范围或债务状态越界')
    state(CANDIDATE)
    checks=read('checks.json');require(len(checks['effective'])==len(REQUIRED) and set(checks['effective'])==REQUIRED,'A7有效检查清单缺项或重复')
    snapshot=read(closeout['source_manifest']);common.source(snapshot,CANDIDATE)
    effective_snapshot=snapshot['utf8_lf_tree_sha256'];logs={}
    early={'entry-a0-audit','entry-a1-audit','entry-a2-a3-audit','entry-a4-audit','entry-a5-audit','entry-a6-audit','entry-a6-review'}
    for name in checks['effective']:
        record=read(name+'.json');raw=path(record['log']).read_bytes()
        require(record['name']==name and record['exit_code']==0 and sha(raw)==record['sha256'] and record['source_unchanged'] and record['source_before']==record['source_after'],'A7实际命令失败或执行期间源码变化：'+name)
        if name not in early:require(record['source_after']['utf8_lf_tree_sha256']==effective_snapshot,'A7有效功能/文档门禁不是同一快照：'+name)
        command(name,record);text=common.clean_log(raw);logs[name]=text
        if name in COUNTS:
            summaries=re.findall(r'^test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;',text,re.M)
            require(sorted(summaries)==sorted(tuple(map(str,r)) for r in COUNTS[name]),'A7实际测试或忽略计数不符：'+name)
        if name in EXACT:
            section=common.exact_section(text,EXACT[name])
            if name.startswith('b1-'):common.markers(section,common.expected_markers()['native' if 'native' in name else 'vm'])
            else:
                for case in ['a1_identity','a1_admission_error','a1_pending_finally']:
                    variants=[f'B2-A1-IDENTITY case={case} backend=vm'] if 'vm' in name else [f'B2-A1-IDENTITY case={case} backend=native optimization={level} artifact=' for level in [0,3]]
                    common.markers(section,variants)
        if name in HISTORICAL:require(json.loads(text.strip())['passed'],'阶段历史审计未实际通过')
    guards=read('b1-guards-report.json');require(guards['passed']==guards['total']==len(guards['cases'])==28 and all(r['Passed'] for r in guards['cases']),'A7原28守卫未全部通过')
    final_docs=read('docs-closeout.json');command('docs-closeout',final_docs)
    require(final_docs['exit_code']==0 and final_docs['source_unchanged'] and final_docs['source_before']==final_docs['source_after']
            and final_docs['source_after']['utf8_lf_tree_sha256']==effective_snapshot and sha(path(final_docs['log']).read_bytes())==final_docs['sha256'],'最终收口文档门禁或输入不符')
    stage=read('stage-and-boundary.json');require(stage['passed'] and not stage['a7_registry_changed'] and len(stage['stage_closeouts'])==6,'A7阶段对账不完整')
    for row in stage['stage_closeouts']:
        p=REPOSITORY/row['closeout'];v=json.loads(p.read_text());require(v['passed'] and sha(p.read_bytes())==row['closeout_sha256'] and v.get('code_commit',v.get('source_commit'))==row['code_commit'],'阶段有效收口或源码身份不符')
    debts=read('independent-debts.json')
    for row in debts['carried_records']:
        p=REPOSITORY/row['source'];v=json.loads(p.read_text());require(sha(p.read_bytes())==row['sha256'] and v['closed'] is False and row['closed'] is False,'历史独立债务被改写或关闭')
    original=json.loads((REPOSITORY/'docs/DevDocs/assets/20o-b2-a0/entry-20261105/result.json').read_text());require(original['passed'] is False,'A0原始失败被改写')
    require(set(closeout['ci_runs'])=={'workspace','maintenance'},'A7工作区/维护CI缺项')
    runs={role:read(name) for role,name in closeout['ci_runs'].items()}
    for role,r in runs.items():
        expected=('Xiao workspace gates','.github/workflows/workspace-gates.yml') if role=='workspace' else ('Xiao maintenance regression','.github/workflows/maintenance-regression.yml')
        require(r['head_sha']==CANDIDATE and r['conclusion']=='success' and r['status']=='completed' and (r['name'],r['path'])==expected and r['event']=='push','A7实际CI角色、触发方式或源码不符：'+role)
    require(set(closeout['ci_records'])=={'vm','windows','linux','macos'},'A7三平台实际作业缺项')
    jobs=read(closeout['ci_jobs'])['jobs']
    for role,name in closeout['ci_records'].items():
        record=read(name);jobname={'vm':'Rust workspace default tests (Linux)','windows':'Native differential regression (Windows)','linux':'Native differential regression (Linux)','macos':'Native differential regression (macOS)'}[role]
        require(record['job_name']==jobname,'A7平台角色与实际作业不符')
        text=common.origin_job(record,jobs,CANDIDATE,runs['maintenance'])
        require('XIAO_B1_F10: 0' in text and 'XIAO_B2_A_PERF: 0' in text,'A7运行启用了性能测量开关或缺少关闭证据')
        section=common.exact_section(text,'b1_capabilities_have_vm_golden_outputs' if role=='vm' else 'b1_capabilities_match_native_golden_outputs')
        common.markers(section,common.expected_markers()['vm' if role=='vm' else 'native'])
        require(not any(line.startswith(('B1-PERF-','B1-F10','B2-A6-PERF')) for line in text.splitlines()),'A7不应执行旧F10或A6新测量')
    artifacts=read('local-artifacts.json');require(artifacts['source_manifest']==closeout['source_manifest'] and artifacts['runtime_build_record']=='build-runtime.json' and artifacts['runtime']['bytes']>0 and len(artifacts['runtime']['sha256'])==64,'A7 Runtime身份不完整')
    setup=read('environment.json')['overrides'];bootstrap=read('bootstrap.json')
    require(setup['XIAO_B1_F10']==setup['XIAO_B2_A_PERF']=='0' and artifacts['runtime']['path']==setup['XIAO_RUNTIME_LIBRARY'] and setup['XIAO_STDLIB_ROOT']==str(Path(bootstrap['source_root'])/'stdlib'),'A7构建/stdlib/开关身份不符')
    require(Path(artifacts['runtime']['path']).is_relative_to(Path(bootstrap['task_root'])) and artifacts['fresh_target']==setup['CARGO_TARGET_DIR'],'A7 Runtime不是本任务独占输出')
    require(bool(artifacts['artifacts']) and len({row['path'] for row in artifacts['artifacts']})==len(artifacts['artifacts']) and all(row['bytes']>0 and re.fullmatch('[0-9a-f]{64}',row['sha256']) for row in artifacts['artifacts']),'A7真实原生产物身份不完整')
    print(json.dumps({'passed':True,'scope':closeout['scope'],'code_commit':CANDIDATE,'product_commit':BASELINE,'checks':len(REQUIRED),'source_files':len(snapshot['files']),'files':files,'platforms':['windows','linux','macos'],'b1_entries':20,'historical_stage_audits':7,'b1_721_executed':False},ensure_ascii=False))

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--manifest',action='store_true');parser.add_argument('--archive-root',type=Path);parser.add_argument('--repository-root',type=Path);args=parser.parse_args()
    if args.archive_root:ROOT=args.archive_root.resolve()
    if args.repository_root:REPOSITORY=args.repository_root.resolve()
    if args.manifest:print(json.dumps({'files':inventory(True)}))
    else:main()
