import json,subprocess,hashlib
from pathlib import Path
repo=Path.cwd();root=repo/'docs/DevDocs/assets/20o-b2-a7/entry-20261107'
def git(*args):return subprocess.check_output(['cmd','/c','git',*args])
code=git('rev-parse','HEAD').decode().strip()
base='e7bf7a49ba3ed55679b69c97ac1bd5ce7f5777b6';a0='d0e4a5fb9043aa6c93dce7892dbe1ef333d3b276'
products=['core/rust','stdlib','tests/benchmarks','.github/workflows','.cargo','tools','cli/ts','docs/module-registry.json','package.json','bun.lock','.gitattributes']
assert not git('diff','--name-only',base,code,'--',*products)
allowed=['docs/UseDocs/language/basics/p1-expressions/operators.md','docs/UseDocs/language/basics/types/conversions.md','docs/UseDocs/language/memory/runtime/README.md','docs/UseDocs/library/README.md','tests/fixtures/docs/numeric/README.md']
changed=git('diff','--name-only',base,code,'--','core/rust','stdlib','tests/fixtures','tests/benchmarks','.github/workflows','.cargo','tools','cli/ts','docs/UseDocs','docs/module-registry.json','package.json','bun.lock','.gitattributes').decode().splitlines();assert set(changed)==set(allowed)
phase_changes=git('diff','--numstat',a0,base,'--','core/rust','tests/fixtures','stdlib','.github/workflows','tools/gates','docs/UseDocs','docs/module-registry.json').decode().splitlines()
rows=[]
for line in phase_changes:
 additions,deletions,path=line.split('\t');raw=git('show',base+':'+path) if path in git('ls-tree','-r','--name-only',base).decode().splitlines() else b''
 rows.append({'path':path,'added':additions,'deleted':deletions,'final_sha256':hashlib.sha256(raw).hexdigest() if raw else None})
stages=[]
for stage,path in [('A0','20o-b2-a0/entry-20261105'),('A1','20o-b2-a1/entry-20261106'),('A2/A3','20o-b2-a2-a3/entry-20261106'),('A4','20o-b2-a4/entry-20261106'),('A5','20o-b2-a5/entry-20261106'),('A6','20o-b2-a6/entry-20261107')]:
 p=repo/'docs/DevDocs/assets'/path/'closeout.json';v=json.loads(p.read_text(encoding='utf-8'));assert v['passed'];r={'stage':stage,'closeout':p.relative_to(repo).as_posix(),'closeout_sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'code_commit':v.get('code_commit',v.get('source_commit')),'internal_only':True}
 if stage=='A0':
  original=json.loads(p.with_name('result.json').read_text());assert not original['passed'];r.update(original_result_passed=False,effective_result='closeout.json',local_platforms=['windows'],linux_macos='历史本地未验证')
 stages.append(r)
registry=json.loads((repo/'docs/module-registry.json').read_text());states={m['id']:m['status'] for m in registry['modules']};assert states['stdlib.cmath']=='planned';assert all(states['stdlib.'+name]=='verified' for name in ['math','hex','base64','url','csv','fmt','statistics'])
record={'passed':True,'candidate_commit':code,'product_baseline':base,'a0_baseline':a0,'product_paths_unchanged_from_a6':products,'a7_input_doc_changes':allowed,'a7_registry_changed':False,'registry_statuses':states,'stage_closeouts':stages,'b2_a_legal_changes':rows,'abi':'1.18','codegen':20,'active_intrinsic_max':32,'b1_721_executed':False,'b2_b_c_started':False,'public_release':False}
(root/'stage-and-boundary.json').write_text(json.dumps(record,ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
print(json.dumps({'stage_closeouts':len(stages),'b2a_changed_paths':len(rows),'a7_product_changed':False,'a7_input_document_paths':len(allowed)},ensure_ascii=False))
