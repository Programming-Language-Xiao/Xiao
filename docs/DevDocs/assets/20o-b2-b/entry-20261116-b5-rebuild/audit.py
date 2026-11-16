"""B5只读收口审计；固定Git身份、明确重建来源、真实运行和单摘要对象。"""
import argparse
from datetime import datetime
from functools import lru_cache
import json
from pathlib import Path,PureWindowsPath
import re
import shutil
import subprocess
import sys
import tempfile
sys.dont_write_bytecode=True
from record import ROOT,REPOSITORY,PREFIX,PRODUCT_ROOTS,canonical,fixed_files,git,identity,load,require,sha
from commands import expected as expected_command
BASE="cfa79381bbae027cec11136ca380e82f66720c34"
VERSIONS={"b1":"106b99c70e88a88afd0dd9a33f6a9c792e13324c","b4":BASE,
 "current":"71e611f82bbd4d7963c05d83db29b5c6ccb664bd",
 "final":"0dfa97ca3d4ab56a6d8437d35cf64055ad810ecc","final2":"01bde595896925f77e0136a5078eb7f0bb092edc",
 "final3":"5864cbde627f727483ab676c6f4bc1bf0a7819bb"}
FAILED={("b1","host-build"):1,("b1","host-build-verified"):1,("b4","host-build-verified"):1,
 ("current","c-abi"):1,("current","numeric-range"):101,("current","numeric-range-verified"):101,
 ("current","host-numeric-final"):1,("current","guards"):1,("current","workspace"):101,("final","size"):101,("final2","workspace"):101,("final3","workspace-complete"):101}
FAILED["b4","static-float-equality-reproduction"] = 101
REASONS={"manifest":"B5重建证据清单不符","missing":"B5重建已登记文件缺失","source":"B5重建来源身份不符",
 "historical":"B5不得把重建基线称为历史原件","matrix":"B5版本矩阵不符","state":"B5公共状态冒进","runtime":"B5Runtime身份不符"}
CASES=["baseline","manifest","missing","source","historical","matrix","state"]
EXPECTED_PRODUCT_PATHS = {
"core/rust/crates/xiao-bytecode/tests/b4_cmath_tac.rs",
"core/rust/crates/xiao-codegen-llvm/src/native_metadata.rs",
"core/rust/crates/xiao-driver/src/compatibility.rs",
"core/rust/crates/xiao-driver/tests/20o_b1_probe.rs",
"core/rust/crates/xiao-driver/tests/20o_b2_a_support/replay.rs",
"core/rust/crates/xiao-driver/tests/20o_b2_b2_divide.rs",
"core/rust/crates/xiao-driver/tests/20o_b2_b4_cmath_llvm.rs",
"core/rust/crates/xiao-driver/tests/20o_b2_b4_cmath_vm.rs",
"core/rust/crates/xiao-driver/tests/20o_b2_b4_frontend.rs",
"core/rust/crates/xiao-driver/tests/20o_b2_b5_versions.rs",
"core/rust/crates/xiao-driver/tests/artifact_size_regression.rs",
"core/rust/crates/xiao-driver/tests/compatibility_matrix.rs",
"core/rust/crates/xiao-driver/tests/22e_release.rs",
"core/rust/crates/xiao-driver/tests/support/b2_divide.rs",
"core/rust/crates/xiao-runtime-abi/src/lib.rs",
"core/rust/crates/xiao-runtime/src/abi_tests.rs",
"tests/fixtures/compatibility/b5/README.md",
"tests/fixtures/compatibility/b5/b1-main.meta",
"tests/fixtures/compatibility/b5/b1-main.xar",
"tests/fixtures/compatibility/b5/b1-main.xiaoc",
"tests/fixtures/compatibility/b5/b4-main.meta",
"tests/fixtures/compatibility/b5/b4-main.xar",
"tests/fixtures/compatibility/b5/b4-main.xiaoc",
}


def inventory(root):
 out={}
 for p in sorted(root.rglob("*")):
  require(not p.is_symlink() and not getattr(p,"is_junction",lambda:False)(),"B5证据含链接")
  require("__pycache__" not in p.parts,"B5证据含Python缓存")
  if p.is_file() and p.relative_to(root).as_posix()!="evidence.json":
   raw=p.read_bytes();out[p.relative_to(root).as_posix()]={"bytes":len(raw),"sha256":sha(raw)}
 return out
def tree(root):
 return sha(json.dumps({p.relative_to(root).as_posix():sha(p.read_bytes()) if p.is_file() else "directory"
                        for p in sorted(root.rglob("*"))},sort_keys=True).encode())
def obj(root,row):
 require(re.fullmatch(r"objects/sha256/[0-9a-f]{2}/[0-9a-f]{64}\.bin",row["path"]),"B5内容对象路径不符")
 raw=(root/row["path"]).read_bytes()
 require(len(raw)==row["bytes"] and sha(raw)==row["sha256"] and row["path"]==f"objects/sha256/{sha(raw)[:2]}/{sha(raw)}.bin",REASONS["manifest"])
 return raw
@lru_cache(maxsize=None)
def products(repository,commit):return fixed_files(repository,commit)
@lru_cache(maxsize=None)
def doc_tree(repository,commit):
 names=sorted({p.decode("utf-8") for p in git(repository,"ls-tree","-r","--name-only","-z",commit,"--","docs/DevDocs").split(b"\0") if p})
 names=[p for p in names if "/assets/" not in p or p.startswith(PREFIX+"/") and Path(p).suffix in {".py",".rs",".c",".md",".xiao"}]
 stream=git(repository,"cat-file","--batch",data=b"".join((commit+":"+p+"\n").encode("utf-8") for p in names))
 out={};offset=0
 for name in names:
  end=stream.index(b"\n",offset);header=stream[offset:end].split();require(header[1]==b"blob","B5文档对象无效")
  length=int(header[2]);out[name]=sha(canonical(stream[end+1:end+1+length]));offset=end+length+2
 require(offset==len(stream),"B5文档对象流长度不符")
 return out
def count_targets(text):
 out={};nested=[];key=None;rows=[]
 def flush():
  if key:
   require(rows and key not in out,"B5workspace缺少或重复目标汇总")
   out[key]=rows[-1];nested.extend({"parent":key,"counts":r} for r in rows[:-1])
 for line in text.splitlines():
  match=re.search(r"Running .*\((?:.*[\\/])([^\\/]+)-([0-9a-f]+)\.exe\)",line)
  next_key=match[1]+"@"+match[2] if match else None
  if line.lstrip().startswith("Doc-tests "):next_key=line.split("Doc-tests ",1)[1].strip()+":doc"
  if next_key:flush();key=next_key;rows=[]
  match=re.match(r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;",line)
  if match:
   row=list(map(int,match.groups()));require(key and row[1]==0,"B5workspace包含失败");rows.append(row)
 flush();require(out,"B5没有目标汇总")
 return {"targets":out,"nested":nested,"passed":sum(v[0] for v in out.values()),"ignored":sum(v[2] for v in out.values())}
def data(root,row):return json.loads(obj(root,row))
def handoff(result):
 return {"passed":False,"stage":"B5-blocked","runtime_abi":"1.19","codegen_version":21,"stdlib_interface":1,
   "product_commit":VERSIONS["current"],"final_test_commit":VERSIONS["final3"],"cmath":"planned","b6_started":False,
   "baseline_kind":"reconstructed_from_B1_source","historical_originals_available":False,
   "b6_started":False,"blocker":result["default_workspace"],
   "next_step":"resolve or explicitly disposition the default parallel workspace watchdog before closing B5; do not start B6",
   "b6_required":["resource/error/atomicity and three-platform performance evidence","retain default workspace watchdog failure and remeasure under controlled conditions"],
   "b2_c_required":["full 721 F10 on final snapshot","three-platform public artifacts","tutorial/registry closeout"],
   "independent_issues":[result["known_independent_issue"],result["default_workspace"]],
   "retained_debts":["22C","22D","19/11/22"]}
def check(root=ROOT,repository=REPOSITORY,final_docs=True,sealed=True):
 before=tree(root)
 changed=set(git(repository,"diff","--name-only",BASE,VERSIONS["final3"],"--",*PRODUCT_ROOTS).decode().splitlines())
 require(changed==EXPECTED_PRODUCT_PATHS,"B5产品路径超出允许集合")
 if (root/"closeout.json").exists():
  state=load(root/"closeout.json")
  require(state.get("cmath")=="planned" and state.get("b6_started") is False and state.get("historical_originals_available") is False,REASONS["state"])
 if sealed:require(all((root/name).is_file() for name in ["evidence.json","closeout.json","self-negative.json","b6-handoff.json"]),REASONS["missing"])
 if (root/"evidence.json").exists():
  manifest=load(root/"evidence.json")
  require(all((root/name).is_file() for name in manifest["files"]),REASONS["missing"])
  require(manifest=={"files":inventory(root)},REASONS["manifest"])
 require((root/".gitattributes").read_bytes()==b"* -text -whitespace\n*.log -diff\n*.bin -diff\n",REASONS["manifest"])
 objects=[p for p in (root/"objects/sha256").rglob("*") if p.is_file()]
 require(len(objects)==len({sha(p.read_bytes()) for p in objects}),"B5存在重复内容对象")
 for p in objects:require(p.stem==sha(p.read_bytes()),REASONS["manifest"])
 sources={};records={};logs={}
 for variant,commit in VERSIONS.items():
  source=load(root/variant/"source.json");env=load(root/variant/"environment.json");sources[variant]=source
  require(source["revision"]==commit and source["sha256"]==identity(source["files"])
    and {p:r["canonical_sha256"] for p,r in source["files"].items()}==products(repository,commit),REASONS["source"])
  task=PureWindowsPath(env["task_root"]);base=PureWindowsPath(r"D:\project\Xiao\.b5\20261116")
  require(task==base/(variant if variant in {"b1","b4"} else "current"),REASONS["source"])
  for key,suffix in [("CARGO_TARGET_DIR","target"),("XIAO_HOME","home"),("TEMP","temp"),("TMP","temp"),("XIAO_RUNTIME_LIBRARY","target/release/xiao_runtime.lib")]:
   require(PureWindowsPath(env["overrides"][key])==task/suffix,REASONS["source"])
  require(env["overrides"]["XIAO_B1_F10"]==env["overrides"]["XIAO_B2_A_PERF"]=="0" and env["overrides"]["RUSTUP_TOOLCHAIN"]=="1.96.0",REASONS["source"])
  if variant in {"final","final2","final3"}:
   changed=set(git(repository,"diff","--name-only",VERSIONS["current"],commit,"--",*PRODUCT_ROOTS).decode().splitlines())
   inherited=env["inherited_test_task"]
   require(changed==set(inherited["test_only_paths"]) and all("/tests/" in p for p in changed)
    and inherited["source_commit"]==VERSIONS["current"],REASONS["source"])
  for path in sorted((root/variant).glob("*.json")):
   row=load(path)
   if "label" not in row:continue
   label=row["label"];key=(variant,label);records[key]=row;logs[key]=obj(root,row["log"]).decode("utf-8")
   require(path.name==label+".json" and row["variant"]==variant and row["cwd"]==env["source_root"]
    and row["source_before"]==row["source_after"]==source["sha256"] and row["source_unchanged"]
    and row["docs_unchanged"] and row["docs_before"]==row["docs_after"]
    and row["pid"]>0 and datetime.fromisoformat(row["started_utc"])<datetime.fromisoformat(row["finished_utc"])
    and row["environment_sha256"]==sha((root/variant/"environment.json").read_bytes()),REASONS["source"])
   require(row["argv"]==expected_command(variant,label),"B5运行命令不符："+variant+"/"+label)
   require(row["exit_code"]==FAILED.get(key,0),"B5运行退出码不符："+variant+"/"+label)
   require(row["runner_sha256"]==sha(git(repository,"show",row["tools_commit"]+":"+PREFIX+"/record.py")),REASONS["source"])
   docs=data(root,row["docs_before"]["files"])
   require(docs==doc_tree(repository,row["tools_commit"]) and identity(docs)==row["docs_before"]["canonical_sha256"],"B5文档字节不符")
 for variant in ["b1","b4","current"]:
  cold=records[variant,"libraries"];runtime=cold["runtime_after"]
  require(cold["runtime_before"]["exists"] is False and runtime["exists"] and runtime["bytes"]>0,REASONS["runtime"])
  for (v,label),row in records.items():
   if (v==variant or variant=="current" and v in {"final","final2","final3"}) and label!="libraries":
    require(row["runtime_before"]==row["runtime_after"]==runtime,REASONS["runtime"])
 baselines={v:load(root/v/"baseline.json") for v in ["b1","b4","current"]}
 for v,baseline in baselines.items():
  require(baseline["historical_originals_available"] is False
   and baseline["baseline_kind"]==("reconstructed_from_B1_source" if v=="b1" else "internal_source_build"),REASONS["historical"])
  require(baseline["source_commit"]==VERSIONS[v] and baseline["source_identity"]==sources[v]["sha256"],REASONS["source"])
  report=baseline["report"];expected=65555 if v=="current" else 65554
  require(report==data(root,baseline["artifacts"]["report.json"]) and report["abi"]==expected
   and report["codegen"]==(20 if v=="b1" else 21) and report["native"]["runtime_abi_min"]==report["native"]["runtime_abi_max"]==expected
   and report["xiaoc"]["runtime_abi_min"]==report["xiaoc"]["runtime_abi_max"]==report["xiaoc"]["instruction_abi"]==expected
   and report["xar"]["runtime_abi_min"]==report["xar"]["runtime_abi_max"]==expected
   and report["native"]["stdlib"]["stdlib_interface_version"]==report["xar"]["stdlib_interface"]==1
   and [report["xiaoc"]["format_major"],report["xiaoc"]["format_minor"]]==[1,0]
   and [report["xar"]["schema_major"],report["xar"]["schema_minor"]]==[1,0],REASONS["matrix"])
  for name in ["main.xiaoc","main.xar","main.meta","main.exe"]:
   require(sha(obj(root,baseline["artifacts"][name]))==report["artifacts"][name],REASONS["manifest"])
 matrices=[]
 for v in ["b1","b4","current"]:
  for source in ["b1","b4","current"]:
   for kind,name in [("xiaoc","main.xiaoc"),("xar","main.xar"),("native","main.exe"),("meta","main.meta")]:
    row=json.loads(logs[v,"validate-"+source+"-"+kind])
    accepted=(v=="current")== (source=="current") and not (v=="b1" and source=="b4" and kind in {"native","meta"})
    require(row["accepted"]==accepted and row["input_unchanged"] is True
     and row["input_sha256"]==baselines[source]["artifacts"][name]["sha256"]
     and row["validator_abi"]==(65555 if v=="current" else 65554)
     and row["validator_codegen"]==(20 if v=="b1" else 21),REASONS["matrix"])
    if not accepted:
     require(row["executed"] is False,REASONS["matrix"])
     response=row["response"]
     require(response.get("phase")=="native_metadata_before_execution" if kind in {"native","meta"}
      else response["error"]["phase"] in {"archive_validation","artifact_validation"},REASONS["matrix"])
    elif kind!="meta":require(row["executed"] is True,REASONS["matrix"])
    matrices.append({"validator":v,"input":source,"kind":kind,"accepted":accepted})
 new=load(root/"new-entry-matrix.json")
 require(new["passed"] is True and new["cases"]==len(new["rows"])==12,REASONS["matrix"])
 for row in new["rows"]:
  result=json.loads(logs[row["variant"],"new-entries-"+row["kind"]])
  require(result["accepted"]==(row["variant"]=="final3") and result["input_unchanged"] is True
    and result["input_sha256"]==sha(obj(root,row["input"])),REASONS["matrix"])
  if row["variant"]!="final3":require(result["executed"] is False,REASONS["matrix"])
 cache=load(root/"cache-matrices.json");require(cache["passed"] is True and len(cache["rows"])==4,"B5缓存矩阵缺失")
 for group in ["generator","abi"]:
  rows=[row for row in cache["rows"] if row["group"]==group]
  old,new=rows
  a,b=old["result"],new["result"]
  require(a["fingerprint"]!=b["fingerprint"] and a["request_key"]!=b["request_key"]
   and a["input_sha256"]==b["input_sha256"] and a["optimization_plan"]==b["optimization_plan"]
   and any(row["key"]==a["request_key"] for row in b["previous"])
   and a["cache_hit"] is False and b["cache_hit"] is False and a["regenerated"] is True and b["regenerated"] is True
   and a["executed"] is True and b["executed"] is True,"B5缓存未证明失配重生成")
  require(a["runtime_sha256"]==b["runtime_sha256"] and a["abi"]==b["abi"]==65554 and [a["actual_codegen"],b["actual_codegen"]]==[20,21]
   if group=="generator" else a["runtime_sha256"]!=b["runtime_sha256"] and [a["abi"],b["abi"]]==[65554,65555]
     and a["actual_codegen"]==b["actual_codegen"]==21,"B5缓存变化归因不符")
  for row in rows:
   result=json.loads(obj(root,load(root/row["receipt"])["log"]).decode())
   require(result==row["result"] and row["receipt_sha256"]==sha((root/row["receipt"]).read_bytes())
    and sha(obj(root,row["native_object"]))==sha(obj(root,row["executable"]))==result["sha256"]
    and sha(obj(root,row["index_bytes"]))==result["index_sha256"],"B5缓存字节快照不符")
 abi=load(root/"c-abi.json")
 require(abi["passed"] is True and abi["historical_originals_available"] is False
  and abi["baseline_kind"]=="reconstructed_from_B1_source" and abi["unchanged_b4_signatures"] is True,REASONS["historical"])
 old=baselines["b1"]["c_abi_signatures"];current=baselines["current"]["c_abi_signatures"]
 require(baselines["b4"]["c_abi_signatures"]==current and all(current.get(k)==v for k,v in old.items())
  and set(baselines["b1"]["exported_c_symbols"])<=set(baselines["current"]["exported_c_symbols"])
  and abi["preserved_historical_signatures"]==len(old)
  and set(abi["added_signatures"])==set(current)-set(old) and len(abi["added_signatures"])==16,"B5 C接口并非加法")
 require(len(abi["runs"])==3 and all(row["caller_object_sha256"]==abi["caller_object"]["sha256"] and row["exit_code"]==0
  for row in abi["runs"]) and len(obj(root,abi["caller_object"]))>0,"B5旧调用方未真实运行")
 numeric=json.loads(logs["current","numeric-range-final"])
 require(numeric["passed"] is True and numeric["cases"]==len(numeric["rows"])==20,"B5数值差分不完整")
 for row in numeric["rows"]:
  require(row["lowering_dynamic"]==(row["forced_dynamic"] or row["case"].startswith(("integer","narrow")))
   and row["optimization"] in [0,3] and row["source_sha256"]==sha(row["source"].encode()),"B5数值路由不符")
  if row["expected_error"]:
   require(row["expected_error"]=="X06-RUNTIME-009" and row["span"] is not None
    and "code=X06-RUNTIME-009" in row["native_stderr"]
    and ("span=%s..%s"%tuple(row["span"])) in row["native_stderr"],"B5数值span差分不符")
 issues=load(root/"known-issues.json")
 require(issues["preexisting_proven"] is True and issues["fixed_in_B5"] is False
  and issues["old_source_commit"]==BASE
  and issues["old_receipt_sha256"]==sha((root/issues["old_receipt"]).read_bytes())
  and issues["new_receipt_sha256"]==sha((root/issues["new_receipt"]).read_bytes())
  and "fcmp eq double" in logs["current","numeric-range-verified"]
  and "fcmp eq double" in logs["b4","static-float-equality-reproduction"],"B5既有LLVM比较缺陷直接复现记录缺失")
 for label in ["regressions","native-matrix","native-library","b1-vm","b1-native"]:
  require(records["current",label]["exit_code"]==0,"B5产品回归未通过")
 require(len(re.findall(r"B4-3-MATRIX dynamic=(?:true|false) optimization=[03].*cases=18318 error_cases=3153 pow_inputs=1075 bit_or_code=exact",logs["current","native-matrix"]))==4,"B5原生矩阵缺项")
 guard=load(root/"guards-report.json")
 require(guard["passed"]==guard["total"]==len(guard["cases"])==28
  and guard["platforms_executed"]==["Windows"] and all(r["passed"] for r in guard["cases"])
  and len([r for r in guard["cases"] if r["case"]=="actual"])==4,"B5真实输出守卫不完整")
 for row in guard["cases"]:
  v="b1-vm" if row["path"]=="default" else "b1-native";receipt=root/"current"/(v+".json")
  require(row["source_receipt_sha256"]==sha(receipt.read_bytes())
   and row["source_log_sha256"]==records["current",v]["log"]["sha256"]
   and (row["exit_code"]==0 if row["case"]=="actual" else row["exit_code"]!=0),"B5守卫未绑定真实输出")
 require("url_decoding_reaches_the_unicode_result_budget" in logs["final3","workspace-complete"]
  and "X09-DRIVER-002" in logs["final3","workspace-complete"] and "error: 1 target failed:" in logs["final3","workspace-complete"],"B5默认并发失败记录不符")
 require(records["final3","resources-serial"]["exit_code"]==0 and "17 passed; 0 failed; 0 ignored;" in logs["final3","resources-serial"],"B5资源串行复核未通过")
 isolated_url=records["final3","url-resource-isolated"]
 require(isolated_url["exit_code"] in {0,101},"B5独立URL用例退出码无效")
 if isolated_url["exit_code"]==0:require("1 passed; 0 failed; 0 ignored;" in logs["final3","url-resource-isolated"],"B5独立URL成功摘要不符")
 else:require("X09-DRIVER-002" in logs["final3","url-resource-isolated"],"B5独立URL失败身份不符")
 workspace=count_targets(logs["final3","workspace-serial"])
 require(records["final2","size"]["exit_code"]==0,"B5体积门禁失败")
 for label in ["workspace-serial","clippy","fmt","lock"]:require(records["final3",label]["exit_code"]==0,"B5最终门禁失败")
 if final_docs:
  record=records["final3","docs-final"];value=json.loads(logs["final3","docs-final"])
  require(value["passed"] is True and not any(row["severity"]=="error" for row in value["diagnostics"]),"B5最终docs失败")
  for path in root.iterdir():
   if path.suffix in {".py",".rs",".c",".md",".xiao"}:
    require(sha(canonical(path.read_bytes()))==doc_tree(repository,record["tools_commit"]).get(PREFIX+"/"+path.name),"B5最终工具未定稿")
 result={"passed":False,"evidence_valid":True,"blocked":True,"stage_passed":False,"stage":"B5","b5_closed":False,"runtime_abi":"1.19","codegen_version":21,"stdlib_interface":1,
   "cmath":"planned","language_available":True,"baseline_kind":"reconstructed_from_B1_source","historical_originals_available":False,
   "product_commit":VERSIONS["current"],"final_test_commit":VERSIONS["final3"],"version_matrix_cases":len(matrices),
   "new_entry_matrix_cases":12,"cache_matrix_groups":2,"numeric_cases":20,"c_preserved_signatures":len(old),"c_added_signatures":len(current)-len(old),
   "guards":{"passed":28,"actual_output_cases":4,"derived_negatives":24,"scope":"Windows replay of four workflow paths"},
   "default_workspace":{"passed":False,"failure":"URL Unicode result budget hit unchanged 60s watchdog","receipt":"final3/workspace-complete.json"},
   "resource_serial_replay_passed":True,"isolated_url_exit_code":isolated_url["exit_code"],
   "workspace_execution":"all targets serial; unchanged watchdogs; default parallel failure retained",
   "workspace":workspace,"platforms_executed":["Windows x86_64-pc-windows-msvc"],
   "not_executed":["Linux","macOS","remote CI","performance","721 F10"],"b6_started":False,"b2_c_started":False,
   "known_independent_issue":{"owner":"static LLVM Float comparison","evidence":"known-issues.json","historical_reproduction":"b4/static-float-equality-reproduction.json",
     "message":"float equality emits fcmp eq; numeric value proof uses explicit C bit observer; product fix deferred to independent batch"}}
 if sealed:
  report=load(root/"closeout.json")
  require(report.get("cmath")=="planned" and report.get("b6_started") is False and report.get("historical_originals_available") is False,REASONS["state"])
  require(report==result and load(root/"b6-handoff.json")==handoff(result),"B5收口或交接与重算不符")
  review=load(root/"self-negative.json")
  require(review["passed"] is True and [r["case"] for r in review["cases"]]==CASES,"B5反例集合不符")
  for row in review["cases"]:
   reason=None if row["case"]=="baseline" else REASONS[row["case"]]
   require(row["exit_code"]==(0 if reason is None else 1) and row["reason"]==reason and row["stderr"].strip()==(reason or "")
    and row["read_only"] and row["before_sha256"]==row["after_sha256"],"B5反例原因或只读摘要不符")
 require(before==tree(root),"B5审计改写证据")
 return result

def negatives(root=ROOT,repository=REPOSITORY):
 rows=[]
 with tempfile.TemporaryDirectory(prefix="b5-final-negative-") as temp:
  for name in CASES:
   copy=Path(temp)/name;shutil.copytree(root,copy)
   if name=="manifest":(copy/"c-abi.json").write_bytes(b"tampered")
   elif name=="missing":(copy/".gitattributes").unlink()
   elif name!="baseline":
    target={"source":"b1/source.json","historical":"b1/baseline.json","matrix":"current/baseline.json","state":"closeout.json"}[name]
    value=load(copy/target)
    if name=="source":value["revision"]=BASE
    elif name=="historical":value["historical_originals_available"]=True
    elif name=="matrix":value["report"]["codegen"]=20
    else:value["cmath"]="verified"
    (copy/target).write_text(json.dumps(value,ensure_ascii=False),encoding="utf-8")
    (copy/"evidence.json").write_text(json.dumps({"files":inventory(copy)}),encoding="utf-8")
   before=tree(copy)
   run=subprocess.run([sys.executable,"-B","-X","utf8",str(Path(__file__).resolve()),"--root",str(copy),
      "--repository-root",str(repository),"--core-only"],capture_output=True,encoding="utf-8")
   reason=None if name=="baseline" else REASONS[name]
   require(run.returncode==(0 if reason is None else 1) and run.stderr.strip()==(reason or "") and before==tree(copy),
      "B5反例实际拒绝不符："+name+"；"+run.stderr)
   rows.append({"case":name,"exit_code":run.returncode,"reason":reason,"stderr":run.stderr,"read_only":True,
      "before_sha256":before,"after_sha256":tree(copy)})
 return {"passed":True,"cases":rows}
def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument("--root",type=Path,default=ROOT);parser.add_argument("--repository-root",type=Path,default=REPOSITORY)
 parser.add_argument("--core-only",action="store_true");parser.add_argument("--self-negative",action="store_true");args=parser.parse_args()
 if args.self_negative:print(json.dumps(negatives(args.root,args.repository_root),ensure_ascii=False));return
 value=check(args.root,args.repository_root,final_docs=not args.core_only,sealed=not args.core_only)
 print(json.dumps({k:value[k] for k in ["passed","evidence_valid","blocked","stage_passed","b5_closed","runtime_abi","codegen_version","baseline_kind","historical_originals_available"]},ensure_ascii=False))
if __name__=="__main__":
 try:main()
 except (ValueError,KeyError,OSError,AssertionError) as e:print(str(e),file=sys.stderr);raise SystemExit(1)
