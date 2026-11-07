"""复核A6固定源码、实际命令、CI日志来源和逐批性能；不执行产品二进制。"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import zipfile

sys.dont_write_bytecode=True
ROOT=Path(__file__).resolve().parent
REPOSITORY=ROOT.parents[4]
HELPER=ROOT.parents[1]/'20o-b2-a5/entry-20261106/audit.py'
HELPER_SHA='7d9628ee3eb72629b385e88072b129edf00a59697d9119f18583421488b62525'


def require(value,reason):
    if not value: raise SystemExit(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def read(name):
    path=(ROOT/name).resolve()
    require(not Path(name).is_absolute() and path.is_relative_to(ROOT.resolve()), 'A6证据引用越界')
    return json.loads(path.read_text(encoding='utf-8'))


def helper():
    require(sha(HELPER.read_bytes())==HELPER_SHA, 'A5通用来源复核函数偏离固定版本')
    spec=importlib.util.spec_from_file_location('a5_audit_helper',HELPER)
    module=importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.ROOT=ROOT
    module.REPOSITORY=REPOSITORY
    return module


def inventory(write=False):
    observed={p.relative_to(ROOT).as_posix():p for p in ROOT.rglob('*') if p.is_file() and p!=ROOT/'evidence.json' and '__pycache__' not in p.parts}
    rows=[{'path':name,'bytes':p.stat().st_size,'sha256':sha(p.read_bytes())} for name,p in sorted(observed.items())]
    if write:
        (ROOT/'evidence.json').write_text(json.dumps({'scope':'本目录全部实际文件；清单不递归计自身或Python缓存','files':rows,'file_count':len(rows)},ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
    else:
        manifest=read('evidence.json')
        require(manifest['file_count']==len(rows) and manifest['files']==rows, 'A6归档文件集合或原始摘要不符')
    return len(rows)


def git(*arguments):
    return subprocess.check_output(['cmd','/c','git',*arguments],cwd=REPOSITORY)


def report_sources(report,commit):
    prefixes=['core/rust/crates/','stdlib/','tests/fixtures/20o-b2-a6/']
    singles={'core/rust/Cargo.toml','core/rust/Cargo.lock','core/rust/rust-toolchain.toml','.cargo/config.toml'}
    all_names=[n.decode() for n in git('ls-tree','-r','--name-only','-z',commit).split(b'\0') if n]
    expected={name for name in all_names if (name in singles or any(name.startswith(p) for p in prefixes)) and not any(part in {'target','node_modules','.git'} for part in Path(name).parts)}
    files=report['identity']['files']
    require(set(files)==expected, '性能报告源码输入集合缺项或混入外部目录')
    payload=''.join(commit+':'+name+'\n' for name in sorted(files)).encode()
    raw=subprocess.check_output(['cmd','/c','git','cat-file','--batch'],cwd=REPOSITORY,input=payload)
    offset=0
    for name in sorted(files):
        end=raw.index(b'\n',offset); header=raw[offset:end].split()
        require(len(header)==3 and header[1]==b'blob', '性能源码缺少固定Git对象')
        size=int(header[2]); content=raw[end+1:end+1+size]; offset=end+size+2
        # checkout可采用LF或CRLF；实际原始摘要必须精确匹配其中一种，不只比较文件名。
        candidates={sha(content)}
        try:
            content.decode('utf-8')
            if b'\0' not in content:
                lf=content.replace(b'\r\n',b'\n')
                candidates.update([sha(lf),sha(lf.replace(b'\n',b'\r\n'))])
        except UnicodeDecodeError:
            pass
        require(files[name] in candidates, '实际性能输入偏离固定提交：'+name)
    require(offset==len(raw), '性能源码Git对象流长度不符')
    tools=report['identity']['tools']
    require('rustc' in tools and tools['rustc']['version']==report['identity']['rustc'], '缺实际编译器文件与完整版本')
    if report['backend']=='native':
        require({'XIAO_CLANG','XIAO_LLVM_AS','XIAO_LLC'}<=set(tools), '缺实际LLVM工具身份')
    for value in [report['identity']['runtime'],report['identity']['worker'],*(tool['file'] for tool in tools.values())]:
        require(value and value['bytes']>0 and re.fullmatch('[0-9a-f]{64}',value['sha256']), 'Runtime或工具实际文件身份不完整')


def unpack(archive,directory):
    with zipfile.ZipFile(archive) as stream:
        for info in stream.infolist():
            target=(directory/info.filename).resolve()
            require(target.is_relative_to(directory.resolve()), '性能归档引用越界')
            require(not info.filename.startswith('/') and '\\' not in info.filename, '性能归档路径不规范')
        stream.extractall(directory)


def performance(item,commit,platform):
    archive=ROOT/item['archive']
    require(sha(archive.read_bytes())==item['sha256'], '性能原件ZIP摘要不符')
    with tempfile.TemporaryDirectory(prefix='a6-readonly-audit-') as temporary:
        directory=Path(temporary)
        unpack(archive,directory)
        path=directory/item['report']
        report=json.loads(path.read_text(encoding='utf-8'))
        require(report['backend']==item['backend'], '性能后端身份不符')
        suffix={'windows':'-pc-windows-msvc','linux':'-unknown-linux-gnu','macos':'-apple-darwin'}[platform]
        require(report['identity']['target'].endswith(suffix) and 'host: '+report['identity']['target'] in report['identity']['rustc'], '性能报告平台或实际编译器宿主不符')
        report_sources(report,commit)
        catalog=directory/'_fixed-source-catalog.json'
        with catalog.open('xb') as stream:
            stream.write(git('show',commit+':tests/fixtures/20o-b2-a6/catalog.json'))
        result=subprocess.run([sys.executable,'-X','utf8',str(ROOT/'verify-performance.py'),'--report',str(path),'--artifacts',str(directory/item['artifacts']),'--require-full','--catalog',str(catalog)],capture_output=True)
        require(result.returncode==0, '逐批性能复核失败：'+result.stderr.decode('utf-8',errors='replace'))
        return json.loads(result.stdout),report


def ci_artifact(role,item,closeout,commit,log):
    run=read(closeout['ci_runs']['performance'])
    metadata=read(closeout['ci_artifacts'])
    matches=[row for row in metadata['artifacts'] if row['id']==item['artifact_id']]
    require(len(matches)==1, '性能ZIP没有唯一的实际CI产物ID')
    artifact=matches[0]
    platform={'ci-vm':'Linux','ci-windows':'Windows','ci-linux':'Linux','ci-macos':'macOS'}[role]
    record=read(closeout['ci_records'][{'ci-vm':'perf-vm','ci-windows':'perf-windows','ci-linux':'perf-linux','ci-macos':'perf-macos'}[role]])
    prefix='b2-a6-vm-' if role=='ci-vm' else 'b2-a6-native-'
    expected=f"{prefix}{platform}-{run['id']}-{record['run_attempt']}"
    require(artifact['name']==expected and not artifact['expired'] and artifact['workflow_run']['id']==run['id'] and artifact['workflow_run']['head_sha']==commit, '性能ZIP平台、原始执行次数或源码不符')
    require(artifact['digest']=='sha256:'+item['sha256'] and Path(item['archive']).name==f"artifact-{artifact['id']}.zip" and Path(item['archive']).parent==Path(closeout['ci_artifacts']).parent, '性能ZIP未绑定API原件和远端摘要')
    raw=[line for line in log.splitlines() if line.startswith('B2-A6-PERF case=')]
    pattern=r'^B2-A6-PERF case=(\S+) profile=(\S+) optimization=(\d+) median=Some\(([^)]+)\) peak=Some\((\d+)\) ratio=(None|Some\([^)]+\)) result=passed$'
    rows=[]
    for line in raw:
        match=re.fullmatch(pattern,line)
        require(match is not None, '实际CI日志包含失败或不可验证的性能行')
        case,profile,level,median,peak,ratio=match.groups()
        rows.append(((case,profile,int(level)),float(median),int(peak),None if ratio=='None' else float(ratio[5:-1])))
    return rows


EXACT_TARGETS={
 'a6-b1-vm-exact-decimal':'b1_capabilities_have_vm_golden_outputs',
 'a6-b1-native-exact-decimal':'b1_capabilities_match_native_golden_outputs',
 'a6-a1-vm-exact-decimal':'a1_numeric_values_and_errors_have_vm_golden_outputs',
 'a6-a1-native-exact-decimal':'a1_numeric_values_and_errors_match_native_golden_outputs',
}
TEST_COUNTS={
 'a6-resources-decimal':[[3,0],[8,0]],
 'a6-driver-unit-decimal':[[64,1]],
 'a6-language-decimal':[[12,1]],
 'a6-b1-contract-decimal':[[9,0]],
 'a6-b1-resources-decimal':[[17,0]],
 **{name:[[1,0]] for name in EXACT_TARGETS},
}

def command(record,name,commit):
    argv=record['argv']; executable=Path(argv[0]).name.lower().removesuffix('.exe')
    manifest=['--locked','--manifest-path','core/rust/Cargo.toml']
    driver=['test',*manifest,'-p','xiao-driver','--test']
    cargo={
     'a6-resources-decimal':[*driver,'20o_b2_a_resources','--test','20o_b2_a_performance_protocol'],
     'a6-packages-decimal':['test',*manifest,'-p','xiao-types','-p','xiao-ir','-p','xiao-bytecode','-p','xiao-runtime','-p','xiao-vm','-p','xiao-codegen-llvm','-p','xiao-diagnostics','--lib','--tests'],
     'a6-driver-unit-decimal':['test',*manifest,'-p','xiao-driver','--lib'],
     'a6-language-decimal':[*driver,'20o_b2_a_contract','--','--nocapture'],
     'a6-b1-contract-decimal':[*driver,'20o_b1_imp_contract'],
     'a6-b1-resources-decimal':[*driver,'20o_b1_imp_resources'],
     'a6-b1-vm-exact-decimal':[*driver,'20o_b1_probe','--',EXACT_TARGETS['a6-b1-vm-exact-decimal'],'--exact','--nocapture'],
     'a6-b1-native-exact-decimal':[*driver,'20o_b1_probe','--','--ignored',EXACT_TARGETS['a6-b1-native-exact-decimal'],'--exact','--nocapture'],
     'a6-a1-vm-exact-decimal':[*driver,'20o_b2_a1_identity',EXACT_TARGETS['a6-a1-vm-exact-decimal'],'--','--exact','--nocapture'],
     'a6-a1-native-exact-decimal':[*driver,'20o_b2_a1_identity',EXACT_TARGETS['a6-a1-native-exact-decimal'],'--','--ignored','--exact','--nocapture'],
     'a6-workspace-check-decimal':['check',*manifest,'--workspace'],
     'a6-benchmarks-check-decimal':['check','--locked','--manifest-path','tests/benchmarks/Cargo.toml'],
     'a6-clippy-decimal':['clippy',*manifest,'--workspace','--all-targets','--','-D','warnings'],
    }
    if name in cargo:
        require(executable=='cargo' and argv[1:]==['+1.96.0',*cargo[name]], 'A6实际Cargo命令偏离执行点：'+name)
    elif name=='a6-docs-closeout':
        require(executable=='bun' and argv[1:]==['tools/repo-check/src/cli.ts','all'], 'A6实际文档命令不符')
    elif name=='a6-rustfmt-decimal':
        header=['run','1.96.0','rustfmt','--check','--edition','2024','--config','skip_children=true']
        require(executable=='rustup' and argv[1:9]==header, 'A6实际格式命令不符')
        changed={path for path in git('diff','--name-only','d4ae7877360f6446c0b3aa617432b8b6999c4e2a',commit,'--','core/rust').decode().splitlines() if path.endswith('.rs')}
        require(len(argv[9:])==len(changed) and set(argv[9:])==changed, 'A6格式检查输入没有覆盖全部本批Rust文件')
    elif name=='a6-selection-decimal':
        require(executable in {'python','python3'} and argv[1:]==['-X','utf8','tools/gates/b2-a6-selection.py','--self-test'], 'A6模式选择守卫命令不符')
    elif name=='a6-actionlint-decimal':
        require(executable=='actionlint' and argv[1:]==['.github/workflows/maintenance-regression.yml'], 'A6工作流静态检查命令不符')
    elif name=='a6-b1-guards-decimal':
        require(executable=='pwsh' and len(argv)==8 and argv[1:5]==['-NoProfile','-File','tools/gates/b1-probe-guards.ps1','-BashPath'] and argv[6]=='-ReportPath', 'A6原28/28守卫命令不符')
        require(Path(argv[7])==Path(record['cwd'])/'docs/DevDocs/assets/20o-b2-a6/entry-20261107/a6-b1-guards-decimal-report.json', 'A6守卫输出位置不是实际本轮根')
    else:
        require(False,'未登记A6执行点')


def boundaries(commit):
    entry='d4ae7877360f6446c0b3aa617432b8b6999c4e2a'
    changed=git('diff','--name-only',entry,commit).decode().splitlines()
    exact={'.github/workflows/maintenance-regression.yml','core/rust/crates/xiao-runtime/src/value/arithmetic/exact_division.rs','core/rust/crates/xiao-runtime/src/value/numeric.rs','core/rust/crates/xiao-runtime/src/value/numeric_tests.rs','core/rust/crates/xiao-runtime/src/value/arithmetic/common.rs','core/rust/crates/xiao-runtime/src/value/arithmetic/mod.rs','core/rust/crates/xiao-runtime/src/value/arithmetic/README.md','core/rust/crates/xiao-runtime/src/value/arithmetic/text_add.rs','core/rust/crates/xiao-runtime/src/value/arithmetic/text_add_tests.rs','core/rust/crates/xiao-runtime/src/value/arithmetic/comparison.rs','core/rust/crates/xiao-runtime/src/value/arithmetic/decimal.rs','core/rust/crates/xiao-runtime/src/value/arithmetic/decimal_tests.rs','core/rust/crates/xiao-runtime/src/value/arithmetic/resource_tests.rs','core/rust/crates/xiao-runtime/src/value/conversions/mod.rs','core/rust/crates/xiao-runtime/src/value/conversions/integer.rs','core/rust/crates/xiao-runtime/src/value/conversions/resource_tests.rs','core/rust/crates/xiao-driver/examples/b2_numeric_performance.rs','core/rust/crates/xiao-driver/tests/20o_b2_a_resources.rs','core/rust/crates/xiao-driver/tests/20o_b2_a_performance_protocol.rs','tools/gates/README.md','tools/gates/b2-a6-selection.py'}
    prefixes=['core/rust/crates/xiao-driver/examples/numeric_perf/','tests/fixtures/20o-b2-a6/','docs/DevDocs/']
    require(all(name in exact or any(name.startswith(p) for p in prefixes) for name in changed), 'A6越出产品/文档边界（包含UseDocs/registry/锁及ABI目录）')
    require('docs/DevDocs/20o-b2-freeze-draft.md' not in changed, '已签冻结正文被修改')


def main():
    common=helper()
    files=inventory()
    closeout=read('closeout.json')
    require(closeout['passed'] and closeout['scope']=='A6资源与受控性能', 'A6尚未有效收口')
    require(closeout['abi']=='1.18' and closeout['codegen']==20 and closeout['active_intrinsic_max']==32, 'A6版本边界漂移')
    require(closeout['cmath']==closeout['numeric_tutorial_scope']=='planned' and not closeout['a7_started'] and not closeout['b1_721_executed'] and not closeout['independent_debts_closed'], 'A6后续范围或独立债务越界')
    debt=read(closeout['independent_debt_record'])
    require(debt['closed'] is False and debt['test']=='native_concurrency_vm_native_differential_matches_from_same_ir', '独立22D债务被关闭或替换')
    debt_log=(ROOT/debt['log']).read_bytes()
    require(sha(debt_log)==debt['log_sha256'] and debt['test'] in debt_log.decode('utf-8') and debt['case'] in debt_log.decode('utf-8'), '独立22D原始失败日志不符')
    debt_jobs=read((Path(debt['log']).parent/'jobs.json').as_posix())['jobs']
    origins=[job for job in debt_jobs if job['id']==debt['job_id']]
    require(len(origins)==1 and origins[0]['run_id']==debt['run_id'] and origins[0]['head_sha']==debt['code_commit'] and origins[0]['conclusion']=='failure', '独立22D失败来源不符')
    debt22c=read('independent-22c-record.json')
    require(debt22c['closed'] is False and debt22c['test']=='queued_task_cancel_never_runs_body_and_wait_is_027', '独立22C债务被关闭或替换')
    raw22c=(ROOT/debt22c['log']).read_bytes()
    require(sha(raw22c)==debt22c['log_sha256'] and debt22c['test'] in raw22c.decode('utf-8'), '独立22C原始日志不符')
    jobs22c=read(debt22c['jobs'])['jobs']
    originals=[job for job in jobs22c if job['id']==debt22c['job_id']]
    require(len(originals)==1 and originals[0]['run_id']==debt22c['run_id'] and originals[0]['head_sha']==debt22c['code_commit'] and originals[0]['conclusion']=='failure', '独立22C失败来源不符')
    commit=closeout['code_commit']
    require(re.fullmatch('[0-9a-f]{40}',commit), 'A6固定源码提交不完整')
    boundaries(commit)
    snapshot=read(closeout['source_manifest'])
    common.source(snapshot,commit)
    functional=read(closeout['functional_source_manifest'])
    functional_commit=closeout['functional_code_commit']
    common.source(functional,functional_commit)
    delta=git('diff','--name-only',functional_commit,commit).decode().splitlines()
    require(delta==[] or delta==['tests/fixtures/20o-b2-a6/clock-shim.c'], '本地功能证据与最终快照之间存在测量C夹具之外的源码变化')
    before={row['path']:row['utf8_lf_or_binary_sha256'] for row in functional['files']}
    after={row['path']:row['utf8_lf_or_binary_sha256'] for row in snapshot['files']}
    require(set(before)==set(after) and [name for name in before if before[name]!=after[name]]==delta, '功能输入与最终输入的逐文件差异不符合声明')
    checks=read('checks.json')
    expected={'a6-resources-decimal','a6-packages-decimal','a6-driver-unit-decimal','a6-language-decimal','a6-b1-contract-decimal','a6-b1-resources-decimal','a6-b1-vm-exact-decimal','a6-b1-native-exact-decimal','a6-a1-vm-exact-decimal','a6-a1-native-exact-decimal','a6-workspace-check-decimal','a6-benchmarks-check-decimal','a6-clippy-decimal','a6-rustfmt-decimal','a6-docs-closeout','a6-b1-guards-decimal','a6-selection-decimal','a6-actionlint-decimal'}
    require(set(checks['effective'])==expected and len(checks['effective'])==len(expected), 'A6有效检查清单缺项/重复')
    require(checks['exact_targets']==EXACT_TARGETS and checks['test_counts']==TEST_COUNTS, 'A6精确入口或计数契约漂移')
    for name in checks['effective']:
        record=read(name+'.json'); raw=(ROOT/record['log']).read_bytes()
        require(record['name']==name and record['exit_code']==0 and sha(raw)==record['sha256'], 'A6实际命令失败或日志摘要不符：'+name)
        require(record['source_before']==record['source_after'] and record['source_unchanged'] and record['source_after']['utf8_lf_tree_sha256'] in {snapshot['utf8_lf_tree_sha256'],functional['utf8_lf_tree_sha256']}, 'A6实际命令不属于声明的功能或最终测量输入：'+name)
        command(record,name,commit)
        require(record['cwd']==snapshot['source_root'], 'A6实际cwd偏离固定源码根')
        argv=record['argv']
        if 'cargo' in Path(argv[0]).name.lower():
            require(argv[1]=='+1.96.0' and '--locked' in argv, 'A6实际Cargo版本或锁边界不符')
        text=common.clean_log(raw)
        if name in checks['exact_targets']:
            require('--exact' in argv, 'A6精确入口缺少--exact')
            section=common.exact_section(text,checks['exact_targets'][name])
            if 'b1-' in name:
                group='native' if 'native' in name else 'vm'
                common.markers(section,common.expected_markers()[group])
        if name in checks['test_counts']:
            summaries=re.findall(r'^test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;',text,re.M)
            require(sorted(summaries)==sorted(tuple(map(str,item)) for item in checks['test_counts'][name]), 'A6实际测试/忽略计数不符：'+name)
    guards=read('a6-b1-guards-decimal-report.json')
    require(guards['passed']==guards['total']==28 and all(item['Passed'] for item in guards['cases']), '原28/28守卫未通过')
    require(set(closeout['ci_runs'])=={'workspace','default','performance'}, 'A6代码CI角色清单缺项')
    for role,path in closeout['ci_runs'].items():
        run=read(path)
        require(run['head_sha']==commit and run['status']=='completed' and run['conclusion']=='success', 'A6代码CI未通过：'+role)
        workflow={
            'workspace':('Xiao workspace gates','.github/workflows/workspace-gates.yml','push'),
            'default':('Xiao maintenance regression','.github/workflows/maintenance-regression.yml','push'),
            'performance':('Xiao maintenance regression','.github/workflows/maintenance-regression.yml','workflow_dispatch'),
        }[role]
        require((run['name'],run['path'],run['event'])==workflow, 'CI工作流角色或触发方式不符：'+role)
    require(set(closeout['ci_records'])=={'default-vm','default-windows','default-linux','default-macos','perf-vm','perf-windows','perf-linux','perf-macos'}, 'A6实际日志作业清单缺项')
    performance_logs={}
    for role,path in closeout['ci_records'].items():
        record=read(path)
        group='performance' if role.startswith('perf-') else 'default'
        run=read(closeout['ci_runs'][group]); jobs=read(closeout['ci_jobs'][group])['jobs']
        require(type(record.get('job_id')) is int and record['job_id']>0, '实际日志作业被聚合作业ID替代或缺失')
        platform=role.rsplit('-',1)[-1]
        expected_name={
            'vm':'Rust workspace default tests (Linux)',
            'windows':'Native differential regression (Windows)',
            'linux':'Native differential regression (Linux)',
            'macos':'Native differential regression (macOS)',
        }[platform]
        require(record['job_name']==expected_name, 'CI平台角色与实际作业名称不符')
        log=common.origin_job(record,jobs,commit,run)
        target='b1_capabilities_have_vm_golden_outputs' if platform=='vm' else 'b1_capabilities_match_native_golden_outputs'
        common.markers(common.exact_section(log,target),common.expected_markers()['vm' if platform=='vm' else 'native'])
        if group=='performance':
            performance_logs[role]=log
            require('b2_numeric_performance' in log and 'B2-A6-PERF case=' in log, 'A6作业缺少真实测量输出')
            require('8 passed; 0 failed; 0 ignored;' in log and '3 passed; 0 failed; 0 ignored;' in log, 'A6测量前资源/协议测试未实际通过')
    require(set(closeout['performance'])=={'local-vm','local-native','ci-vm','ci-windows','ci-linux','ci-macos'}, 'A6完整性能来源缺项')
    results={}
    for role,item in closeout['performance'].items():
        platform='windows' if role.startswith('local-') else 'linux' if role=='ci-vm' else role.removeprefix('ci-')
        result,report=performance(item,commit,platform)
        if role.startswith('ci-'):
            log_role={'ci-vm':'perf-vm','ci-windows':'perf-windows','ci-linux':'perf-linux','ci-macos':'perf-macos'}[role]
            actual=ci_artifact(role,item,closeout,commit,performance_logs[log_role])
            expected=[((row['case'],row['profile'],row['optimization']),row['median_seconds'],row['resident_increment_upper_bytes'],row['ratio']) for row in report['records']]
            require(actual==expected, 'CI原始性能行与ZIP内报告不是同一次测量')
        results[role]=result
    print(json.dumps({'passed':True,'scope':closeout['scope'],'code_commit':commit,'files':files,'source_files':len(snapshot['files']),'checks':len(checks['effective']),'performance':results},ensure_ascii=False))

if __name__=='__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('--manifest',action='store_true')
    parser.add_argument('--archive-root',type=Path)
    parser.add_argument('--repository-root',type=Path)
    args=parser.parse_args()
    if args.archive_root: ROOT=args.archive_root.resolve()
    if args.repository_root: REPOSITORY=args.repository_root.resolve()
    if args.manifest: print(json.dumps({'files':inventory(True)}))
    else: main()
