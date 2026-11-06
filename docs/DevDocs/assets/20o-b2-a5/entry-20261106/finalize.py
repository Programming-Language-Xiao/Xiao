"""只在真实17项本地门禁与代码CI通过后构造A5有效收口。"""
from pathlib import Path
import datetime,hashlib,json,sys
TASK=Path(__file__).resolve().parents[1]
ROOT=TASK/'source/docs/DevDocs/assets/20o-b2-a5/entry-20261106'
COMMIT='f21715d38069923653438fce8a7f3f8445f49164'
def read(name):return json.loads((ROOT/name).read_text(encoding='utf-8'))
def write(name,data):(ROOT/name).write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
checks=read('checks.json')
for name in checks['effective']:
 record=read(name+'.json')
 assert record['exit_code']==0 and record['source_unchanged'],name
 assert record['source_before']==record['source_after']
 assert record['source_after']['utf8_lf_tree_sha256']=='9140e710d94af85638df126a1552fc1d35b2b75b328dedf67d0647bb9c006068',name
 assert hashlib.sha256((ROOT/record['log']).read_bytes()).hexdigest()==record['sha256']
for name in ['ci-workspace-run.json','ci-maintenance-run.json']:
 run=read(name);assert run['head_sha']==COMMIT and run['status']=='completed' and run['conclusion']=='success'
records=[]
for p in sorted(ROOT.glob('*.json')):
 value=read(p.name)
 if isinstance(value,dict) and {'argv','pid','source_before','source_after','exit_code','log','sha256'} <= value.keys():records.append({'record':p.name,**value})
write('commands.json',{'scope':'全部实际run-step记录，包括入口、开发失败、中间成功与最终检查；有效检查另由checks.json列出。','records':records})
closeout={'passed':True,'scope':'A5语言全链路','recorded_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),
 'code_commit':COMMIT,'entry_commit':read('bootstrap.json')['entry_head'],
 'product_manifest':read('close-b1-native-exact.json')['source_after']['manifest'],
 'effective_checks':checks['effective'],'exact_test_counts':checks['test_counts'],
 'abi':'1.18','codegen':20,'active_intrinsic_max':32,'xcon_version':1,'stdlib_interface':1,
 'cmath':'planned','numeric_tutorial_scope':'planned','a6_a7_started':False,'b1_721_executed':False,
 'ci_runs':{'workspace':'ci-workspace-run.json','maintenance':'ci-maintenance-run.json'},
 'ci_jobs':'ci-maintenance-jobs.json',
 'ci_records':{'vm':'ci-vm-linux.json','windows':'ci-native-windows.json','linux':'ci-native-linux.json','macos':'ci-native-macos.json'},
 'ci_maintenance_latest_attempt':read('ci-maintenance-run.json')['run_attempt'],
 'ci_attempt_history':{'first_run':'ci-attempt1-maintenance-run.json','first_jobs':'ci-attempt1-maintenance-jobs.json',
  'first_macos':'ci-attempt1-macos.json','first_macos_log':'ci-attempt1-macos.log','rerun_request':'ci-rerun-macos.json','scheduling':'ci-attempt2-scheduling.json','independent_debt':'independent-22c-record.json'},
 'independent_debts_closed':False,
 'ci_scope':'代码f21715d3的真实VM精确入口及Windows/Linux/macOS原生精确入口；保留的绿色执行使用原始日志作业ID，新映射不冒充新执行。',
 'documentation_ci_scope':'代码CI只绑定f21715d3；后续文档证据提交的CI单独登记，不声称由本代码运行覆盖。',
 'next_entry':'A6资源/性能规划与测量；A7整批收口、B2-B/C仍后置。'}
write('closeout.json',closeout)
print(json.dumps({'product_gates_passed':True,'checks':len(checks['effective']),'code_commit':COMMIT,'archive_audit':'还须独立执行audit.py与正反例自审'},ensure_ascii=False))
