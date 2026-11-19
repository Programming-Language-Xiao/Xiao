"""只读复核B6-1重跑：固定Git源码、真实回执、平台限定、清单与最终文档身份。"""
import argparse
import io
import json
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
import subprocess
import sys
import tarfile

sys.dont_write_bytecode = True
import common as c

REVIEW_PREFIX = "docs/DevDocs/assets/20o-b2-b/entry-20261118-b6-1-review"
REVIEW_FILES = [REVIEW_PREFIX+"/.gitattributes", REVIEW_PREFIX+"/README.md"]


def git(repository, *args):
    p = subprocess.run(["cmd","/c","git","-C",str(repository),*args],capture_output=True)
    c.require(p.returncode==0,"RERUN固定Git读取失败："+p.stderr.decode("utf-8",errors="replace"))
    return p.stdout


def fixed_files(repository, commit, paths):
    raw = git(repository,"archive","--format=tar",commit,"--",*paths)
    with tarfile.open(fileobj=io.BytesIO(raw)) as archive:
        return {m.name:archive.extractfile(m).read() for m in archive.getmembers() if m.isfile()}


def summary(raw):
    rows = re.findall(rb"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;",raw)
    return {"targets":len(rows),"passed":sum(int(r[0]) for r in rows),
            "failed":sum(int(r[1]) for r in rows),"ignored":sum(int(r[2]) for r in rows)}


def wrapper_rows(raw):
    rows=[]
    for line in raw.decode("utf-8").splitlines():
        if not line.startswith("B4-4-NATIVE "):
            continue
        match=re.fullmatch(
            r"B4-4-NATIVE path=([^ ]+) optimization=(\d+) executable=(.+?) "
            r"verification=([^ ]+) declared=(\[.*?\]) observed=(\[.*?\]) "
            r"stdout=([^ ]+) errors=([^ ]+) result=([^ ]+)",line)
        c.require(match is not None,"RERUN包装原始输出格式不符")
        row=dict(zip(("module","optimization","executable","verification","declared", "observed", "stdout","errors","result"),match.groups()))
        row["optimization"]=int(row["optimization"])
        for key in ("declared","observed"):
            row[key]=json.loads(row[key])
        rows.append(row)
    return rows


def verify_receipt(root, platform, stage, expected, runtime):
    retry=stage=="clippy"
    row=c.read(root,platform+"-"+stage+("-r2" if retry else "")+".json")
    c.require(row["exit_code"]==0 and row["pid"]>0 and row["elapsed_seconds"]>0,
              "RERUN命令失败或无真实进程："+row["label"])
    c.require(row["code_commit"]==c.CODE and row["argv"]==c.command(stage)
              and row["argv"]==c.read(root,"commands.json")[platform][stage],
              "RERUN实际命令或源码提交不符")
    task=PureWindowsPath(c.WINDOWS_TASK) if platform=="windows" else PurePosixPath(c.LINUX_TASK)
    path_type=type(task)
    c.require(path_type(row["task_root"])==task and path_type(row["cwd"])==task/"source"
              and path_type(row["source_root"])==task/"source","RERUN运行目录不在新任务根")
    env=row["environment_overrides"]
    for name in ("CARGO_TARGET_DIR","TEMP","TMP","XIAO_HOME","XIAO_STDLIB_ROOT","XIAO_RUNTIME_LIBRARY"):
        c.require(path_type(env[name]).is_relative_to(task),"RERUN隔离路径越界："+name)
    c.require(path_type(env["TEMP"])==task/"temp"/(stage+"-r2" if retry else stage)
              and path_type(env["XIAO_STDLIB_ROOT"])==task/"source/stdlib"
              and path_type(env["CARGO_TARGET_DIR"])==task/"target"
              and env["RUSTUP_TOOLCHAIN"]=="1.96.0"
              and env["XIAO_B1_F10"]==env["XIAO_B2_A_PERF"]=="0"
              and "RUST_TEST_THREADS" not in env,"RERUN加载根、线程或模式不符")
    if platform=="linux":
        c.require(path_type(env["TMPDIR"])==task/"temp"/stage,"RERUN Linux临时根越界")
    before=c.unpack(root,row["source_before"])
    c.require(before==c.unpack(root,row["source_after"]) and row["source_unchanged"] is True
              and json.loads(before)==expected,"RERUN前后源码偏离固定Git字节")
    c.require(row["tools_before"]==row["tools_after"],"RERUN工具身份运行期间变化")
    c.require(row["recorder_sha256"]==c.sha((root/("retry_clippy.py" if retry else "run.py")).read_bytes())
              and row["common_sha256"]==c.sha((root/"common.py").read_bytes()),"RERUN采集器身份不符")
    for name in ("rustc","cargo"):
        c.require(row["tools_before"][name]["version"].startswith(name+" 1.96.0"),"RERUN Rust工具链不符")
    for tool in row["tools_before"].values():
        c.require(tool["binary"]["bytes"]>0 and len(tool["binary"]["sha256"])==64,"RERUN工具摘要缺失")
    if platform=="linux":
        c.require("21.1.8" in row["tools_before"]["XIAO_CLANG"]["version"]
                  and "21.1.8" in row["tools_before"]["XIAO_LLVM_AS"]["version"],"RERUN Linux LLVM版本不符")
    c.require(row["runtime_after"]==runtime and
              (row["runtime_before"] is None if stage=="runtime-build" else row["runtime_before"]==runtime),
              "RERUN未冷构建Runtime或复用了不同Runtime")
    c.require(path_type(runtime["path"]).is_relative_to(task/"target/release"),"RERUN Runtime不在新任务根")
    return row,c.unpack(root,row["log"])


def recompute(root,repository):
    source=c.read(root,"source.json")
    blobs=fixed_files(repository,c.CODE,c.PRODUCT_ROOTS)
    expected={name:{**c.digest(raw),"binary":b"\0" in raw,
                   "lf_sha256":c.sha(raw if b"\0" in raw else raw.replace(b"\r\n",b"\n"))}
              for name,raw in blobs.items()}
    c.require(source["files"]==expected and source["file_count"]==len(expected)
              and source["code_commit"]==c.CODE,"RERUN源码清单偏离固定Git对象")
    c.require(b"pub const ABI_MINOR_VERSION: u32 = 19;" in blobs["core/rust/crates/xiao-runtime-abi/src/lib.rs"]
              and b"pub const CODEGEN_VERSION: u32 = 22;" in blobs["core/rust/crates/xiao-codegen-llvm/src/lib.rs"],
              "RERUN ABI或生成器越界")
    c.require(c.read(root,"commands.json")=={
        p:{s:c.command(s) for s in c.STAGES+(["clippy"] if p=="windows" else [])}
        for p in ("windows","linux")},"RERUN冻结命令清单被改变")
    gates=c.read(root,"entry-gates.json")
    c.require(gates["passed"] and gates["historical_read_only"]
              and len(gates["runs"])==6,"RERUN开工审计不完整")
    for run in gates["runs"]:
        actual=json.loads(c.unpack(root,run["stdout"]))
        c.require(run["exit_code"]==0 and run["pid"]>0 and not c.unpack(root,run["stderr"])
                  and actual==run["outcome"],"RERUN开工审计原始输出不符")
        if run["label"]=="fix2-negative":
            c.require(actual["passed"] and actual["archive_read_only"] and actual["case_count"]==8,
                      "RERUN FIX.2八项反例未通过")
    original_clippy=c.read(root,"windows-clippy.json")
    c.require(original_clippy["exit_code"]==101 and original_clippy["source_unchanged"] is True
              and original_clippy["argv"]==c.command("clippy")
              and b"tests/spec/11x0-protocol" in c.unpack(root,original_clippy["log"]),
              "RERUN首次Clippy缺失夹具的失败回执被改写")
    supplement=c.read(root,"supplemental-source.json")
    extra=fixed_files(repository,c.CODE,["tests/spec"])
    c.require(supplement["code_commit"]==c.CODE and supplement["files"]==
              {name:c.digest(raw) for name,raw in extra.items()},"RERUN补充编译夹具不等于固定Git字节")
    retry=c.read(root,"windows-clippy-r2.json")
    c.require(retry["argv"]==original_clippy["argv"] and
              c.unpack(root,retry["supplemental_before"])==c.unpack(root,retry["supplemental_after"])
              and json.loads(c.unpack(root,retry["supplemental_after"]))==supplement["files"],
              "RERUN Clippy补验改动命令范围或编译夹具")
    availability=c.read(root,"linux-tool-availability.json")
    c.require(availability["exit_code"]==0 and availability["pid"]>0
              and b"clippy" not in c.unpack(root,availability["stdout"])
              and b"rustfmt" in c.unpack(root,availability["stdout"]),"RERUN Linux工具缺项未核实")
    platforms={}
    for platform in ("windows","linux"):
        setup=c.read(root,platform+"-setup.json")
        c.require(setup["task_root_existed_before"] is False and setup["target_existed_before"] is False
                  and setup["runtime_existed_before"] is False and setup["source_matches_fixed_git_bytes"]
                  and json.loads(c.unpack(root,setup["source"]))==expected,"RERUN任务根不是全新")
        runtime=c.read(root,platform+"-runtime-build.json")["runtime_after"]
        rows,logs={},{}
        stages=c.STAGES+(["clippy"] if platform=="windows" else [])
        for stage in stages:
            rows[stage],logs[stage]=verify_receipt(root,platform,stage,expected,runtime)
        rt=summary(logs["runtime-tests"])
        c.require(rt=={"targets":7,"passed":305,"failed":0,"ignored":1},"RERUN Runtime测试摘要不符")
        abi=re.findall(rb"B3-CMATH-ABI function=([^ ]+) cases=(\d+) result=success",logs["runtime-tests"])
        c.require(len(abi)==15 and sum(int(r[1]) for r in abi)==17243
                  and b"reference_inputs=17243 facades=15 kernel_identity=exact" in logs["runtime-tests"],
                  "RERUN正式参考或C ABI回归缺失")
        for module in (b"std.cmath",b"stdlib.cmath"):
            c.require(b"B6-1-LANGUAGE module="+module in logs["vm-regressions"],"RERUN语言双路径缺失")
        vm=summary(logs["vm-regressions"])
        c.require(vm=={"targets":8,"passed":42 if platform=="windows" else 43,"failed":0,"ignored":3},
                  "RERUN资源、VM和归档回归摘要不符")
        a6=re.findall(rb"test result: ok\. 8 passed; 0 failed; 0 ignored;",logs["vm-regressions"])
        c.require(a6,"RERUN A6资源8/8缺失")
        codegen=summary(logs["codegen-tests"])
        c.require(codegen=={"targets":7,"passed":109,"failed":0,"ignored":15},"RERUN codegen回归摘要不符")
        c.require(summary(logs["driver-regressions"])=={"targets":2,"passed":17,"failed":0,"ignored":3},
                  "RERUN driver额外回归摘要不符")
        for stage in ("wrappers","native-core","native-errors","native-divide"):
            c.require(summary(logs[stage])["passed"]==1 and summary(logs[stage])["failed"]==0,
                      "RERUN原生精确目标未通过："+stage)
        c.require(b"B4-3-NATIVE" in logs["native-core"],"RERUN Cmath核心原生参考未运行")
        for level in (b"0",b"3"):
            c.require(b"B2-A5-ALLOCATION optimization="+level+b" code=X06-RUNTIME-011 out=unchanged source_unchanged=true" in logs["native-errors"],
                      "RERUN原生011或输出/输入原子性缺失")
        c.require(b"B2-B2-DIVIDE backend=native optimization=0,3 intrinsic=33 result=success" in logs["native-divide"],
                  "RERUN显式除法原生O0/O3缺失")
        native=wrapper_rows(logs["wrappers"])
        c.require(len(native)==4 and {(r["module"],r["optimization"]) for r in native}==
                  {(p,l) for p in ("std.cmath","stdlib.cmath") for l in (0,3)},"RERUN包装四格缺失")
        artifacts=json.loads(c.unpack(root,rows["wrappers"]["artifacts_after"]))
        path_type=PureWindowsPath if platform=="windows" else PurePosixPath
        by_path={path_type(a["path"]):a for a in artifacts}
        for row in native:
            expected_status="UnverifiedCoffExports" if platform=="windows" else "Complete"
            c.require(row["verification"]==expected_status,"RERUN平台组件验证等级被冒进升级")
            c.require(row["declared"]==["containers","rc","value","weak"]
                      and (platform=="windows" or row["observed"]==row["declared"]),"RERUN Linux完整组件集合不一致")
            c.require(row["stdout"]=="golden" and row["errors"]=="009" and row["result"]=="passed",
                      "RERUN包装黄金或错误恢复不符")
            c.require(path_type(row["executable"]) in by_path,"RERUN实际包装执行文件缺少摘要")
            row["executable_identity"]=by_path[path_type(row["executable"])]
            row["cargo_pid"]=rows["wrappers"]["pid"]
        platforms[platform]={
            "functional_status":"passed","export_integrity":"unverifiable" if platform=="windows" else "passed",
            "artifact_verification":"UnverifiedCoffExports" if platform=="windows" else "Complete",
            "runtime":runtime,"runtime_tests":rt,"formal_reference_cases":17243,
            "vm_resource_archive_tests":vm,"a6_resource_tests":8,"codegen_tests":codegen,
            "extra_driver_tests":summary(logs["driver-regressions"]),"native_wrapper_rows":native,
            "native_core":"passed","native_errors_atomicity":"passed","native_explicit_divide":"passed",
            "fmt":"passed","clippy":"passed" if platform=="windows" else "not_run: Rust1.96工具链未安装Clippy",
            "receipts":[platform+"-"+stage+("-r2" if stage=="clippy" else "")+".json" for stage in stages],
        }
    return {
        "stage":"B6-1-RERUN","rerun_completed":True,"rerun_closed":True,
        "stage_passed":False,"b6_1_status":"restricted","linux_original_blocker_resolved":True,
        "code_commit":c.CODE,"runtime_abi":"1.19","codegen_version":22,"stdlib_interface_version":1,
        "source_files":len(expected),"platforms":platforms,
        "reference_fixture":{"path":"tests/fixtures/docs/cmath/reference-vectors.tsv",
                             **expected["tests/fixtures/docs/cmath/reference-vectors.tsv"]},
        "retained_clippy_failure":"windows-clippy.json; missing exported tests/spec; exit=101",
        "compile_fixture_supplement":"supplemental-source.json; fixed Git original bytes",
        "macos":{"status":"unverifiable","reason":"无macOS主机或已配置远程Mac Runner"},
        "three_platform_passed":False,"b6_closed":False,"b6_2_started":False,
        "b6_2_entry":{"status":"restricted_windows_linux","requires_frozen_b6_protocol":True,
                      "macos_unverified":True,"full_three_platform_gate_satisfied":False},
        "performance":"not_started","url_full_escaped_maximum_measured":False,
        "remote_ci":"not_run","full_721":"not_run","b7_started":False,"b2_c_started":False,
        "historical_b6_1_status":"blocked; stage_passed=false; unchanged",
        "historical_files":len(c.read(root,"historical.json")),
    }


def check(root,repository,prepare=False):
    before=c.inventory(root)
    c.require(".gitattributes" in before,"RERUN证据缺少.gitattributes")
    if not prepare:
        manifest=c.read(root,"evidence.json")
        files={k:v for k,v in before.items() if k!="evidence.json"}
        c.require(not set(manifest["files"])-set(files),"RERUN清单文件缺失")
        c.require(manifest["file_count_excluding_manifest"]==len(files),"RERUN清单文件数不符")
        c.require(manifest["files"]==files,"RERUN清单与实际字节不符")
    result=c.read(root,"result.json")
    c.require(result["three_platform_passed"] is False and result["stage_passed"] is False
              and result["macos"]["status"]=="unverifiable" and result["b6_2_started"] is False,
              "RERUN未验证平台或B6-2状态被冒进升级")
    c.require(result["platforms"]["windows"]["artifact_verification"]=="UnverifiedCoffExports"
              and result["platforms"]["windows"]["export_integrity"]=="unverifiable",
              "RERUN Windows导出完整性被冒进升级")
    c.require(result==recompute(root,repository),"RERUN结果与真实回执重算不一致")
    if not prepare:
        commit=git(repository,"log","-1","--format=%H","--",c.PREFIX+"/evidence.json").decode().strip()
        c.require(commit and git(repository,"show",commit+":"+c.PREFIX+"/evidence.json")==
                  (root/"evidence.json").read_bytes(),"RERUN固定Git清单字节不符")
        docs=c.read(root,"docs-final.json")
        c.require(docs["exit_code"]==0 and docs["source_unchanged"] is True
                  and docs["result_sha256"]==c.sha((root/"result.json").read_bytes()),"RERUN定稿文档门禁未通过")
        c.require(json.loads(c.unpack(root,docs["log"]))["passed"] is True,"RERUN原始文档门禁报告未通过")
        docs_raw=c.unpack(root,docs["docs_before"])
        c.require(docs_raw==c.unpack(root,docs["docs_after"]),"RERUN文档门禁前后字节变化")
        product_raw=c.unpack(root,docs["product_before"])
        product=json.loads(product_raw)
        c.require(product_raw==c.unpack(root,docs["product_after"])
                  and {k:v["lf_sha256"] for k,v in product.items()}==
                  {k:v["lf_sha256"] for k,v in c.read(root,"source.json")["files"].items()},
                  "RERUN文档门禁产品身份不符")
        docs_inputs=json.loads(docs_raw)
        required=set(c.DOC_FILES+REVIEW_FILES)|{c.PREFIX+"/"+n for n in before
                  if not n.startswith("objects/") and n not in {"docs-final.json","evidence.json"}}
        c.require(set(docs_inputs)==required,"RERUN门禁DevDocs输入集合不全")
        fixed=fixed_files(repository,commit,list(docs_inputs))
        c.require(set(fixed)==set(docs_inputs),"RERUN封存文档Git路径集合不全")
        for name,raw in fixed.items():
            c.require(docs_inputs[name]["lf_sha256"]==c.sha(raw.replace(b"\r\n",b"\n")),
                      "RERUN门禁输入不等于封存Git文档："+name)
    c.require(c.inventory(root)==before,"RERUN审计修改证据")
    return {"passed":True,"read_only":True,"sealed":not prepare,"rerun_closed":True,
            "stage_passed":False,"b6_1_status":"restricted","linux_blocker_resolved":True,
            "windows_export_integrity":"unverifiable","macos":"unverifiable",
            "b6_2_started":False,"files_excluding_manifest":len(before)-(1 if "evidence.json" in before else 0)}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root",type=Path,default=c.ROOT)
    parser.add_argument("--repository-root",type=Path,default=c.REPOSITORY)
    parser.add_argument("--prepare",action="store_true",help="只读封存前语义核对，不证明最终清单")
    args=parser.parse_args()
    root=args.root.resolve()
    before=c.inventory(root)
    try:
        value=check(root,args.repository_root.resolve(),args.prepare)
        code=0
    except Exception as error:
        value={"passed":False,"reason":str(error)}
        code=1
    value["read_only"]=c.inventory(root)==before
    if not value["read_only"]:
        value.update(passed=False,reason="RERUN审计修改证据")
        code=1
    print(json.dumps(value,ensure_ascii=False))
    return code


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    raise SystemExit(main())
