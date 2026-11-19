"""B5每条采集回执的独立命令契约。"""
from pathlib import PureWindowsPath
from suite import COMMANDS
from record import PREFIX,REPOSITORY

ROOT_PATH=PureWindowsPath(str(REPOSITORY))/PREFIX
BASE=PureWindowsPath(r"D:\project\Xiao\.b5\20261116")
def python_script(name,*args):
 return ["python","-B","-X","utf8",str(ROOT_PATH/name),*args]
def expected(variant,label):
 actual="current" if variant in {"final","final2","final3"} else variant
 task=BASE/actual
 host=task/"b5-host.exe"
 if label=="url-resource-isolated":return ["cargo","+1.96.0","test","--locked","--manifest-path","core/rust/Cargo.toml","-p","xiao-driver","--test","20o_b1_imp_resources","url_decoding_reaches_the_unicode_result_budget","--exact","--nocapture"]
 if label in COMMANDS:return COMMANDS[label]
 if label=="libraries":return ["cargo","+1.96.0","build","--locked","--manifest-path","core/rust/Cargo.toml","--release","--lib","-p","xiao-runtime","-p","xiao-driver"]
 if label.startswith("host-"):
  extra={"host-extra":"b5-extra","host-extra2":"b5-extra2","host-numeric-final":"b5-numeric-final","host-numeric-ready":"b5-numeric-final","host-existing-issue":"b5-existing-issue"}
  return python_script("build_host.py",variant,*(["--name",extra[label]] if label in extra else []))
 if label=="known-static-float-equality":return [str(host),"generate",str(task/"artifacts/static-float-equality-proof"),str(task/"artifacts/static-float-equality.xiao")]
 if label=="static-float-equality-reproduction":return [str(task/"b5-existing-issue.exe"),"generate",str(task/"artifacts/static-float-equality-original"),str(task/"artifacts/static-float-equality.xiao")]
 if label=="freeze":return python_script("freeze.py",variant)
 if label=="generate":return [str(host),"generate",str(task/"artifacts/baseline")]
 if label.startswith("validate-"):
  _,source,kind=label.split("-")
  name={"xiaoc":"main.xiaoc","xar":"main.xar","native":"main.exe","meta":"main.meta"}[kind]
  accept=(actual=="current")==(source=="current") and not (variant=="b1" and source=="b4" and kind in {"native","meta"})
  return [str(host),"validate",kind,str(BASE/source/"artifacts/baseline"/name),"accept" if accept else "reject"]
 if label.startswith("cache-"):
  group="generator" if "generator" in label else "abi"
  new=label.endswith("-new")
  verified="verified" in label
  cache=BASE/(("sealed-cache-" if "sealed" in label or verified else "cache-")+group+("-verified" if verified else ""))
  runtime_variant="current" if group=="abi" and new else "b4"
  runtime=BASE/runtime_variant/"target/release/xiao_runtime.lib"
  return [str(host),"cache",str(cache),str(cache/("new.exe" if new else "old.exe")),str(20 if group=="generator" and not new else 21),str(runtime)]
 if label.startswith("new-entries-"):
  kind=label.removeprefix("new-entries-");name={"xiaoc":"main.xiaoc","xar":"main.xar","native":"main.exe","meta":"main.meta"}[kind]
  return [str(host),"validate",kind,str(BASE/"current/artifacts/new-entries"/name),"accept" if actual=="current" else "reject"]
 if label=="new-entries":return [str(task/"b5-extra.exe"),"generate",str(task/"artifacts/new-entries"),str(ROOT_PATH/"new-entries.xiao")]
 if label.startswith("c-abi"):return python_script("c_abi.py")
 if label.startswith("numeric-range"):
  name={"numeric-range":"b5-extra","numeric-range-verified":"b5-extra2","numeric-range-final":"b5-numeric-final"}[label]
  directory={"numeric-range":"numeric","numeric-range-verified":"numeric-verified","numeric-range-final":"numeric-final"}[label]
  return [str(task/(name+".exe")),"numeric",str(task/"artifacts"/directory)]
 if label=="versions":return ["cargo","+1.96.0","test","--locked","--manifest-path","core/rust/Cargo.toml","-p","xiao-driver","--release","--test","20o_b2_b5_versions","--test","compatibility_matrix","--test","20o_b2_b4_numeric_range_routing","--","--nocapture"]
 if label=="resources-serial":return ["cargo","+1.96.0","test","--locked","--manifest-path","core/rust/Cargo.toml","-p","xiao-driver","--test","20o_b1_imp_resources","--","--test-threads=1","--nocapture"]
 if label=="url-resource-isolated":return ["cargo","+1.96.0","test","--locked","--manifest-path","core/rust/Cargo.toml","-p","xiao-driver","--test","20o_b1_imp_resources","url_decoding_reaches_the_unicode_result_budget","--exact","--nocapture"]
 if label=="url-resource-isolated-fixed":return ["cargo","+1.96.0","test","--locked","--manifest-path","core/rust/Cargo.toml","-p","xiao-driver","--test","20o_b1_imp_resources","url_decoding_reaches_the_unicode_result_budget","--","--exact","--nocapture"]
 raise ValueError("B5未知运行标签："+variant+"/"+label)
