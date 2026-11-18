//! B4-3受控原生观察宿主；编译真实LLVM，不在测试里实现数学。
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use xiao_codegen_llvm::{CodegenOptions, EntryObservation, TargetDescription, Toolchain};
use xiao_driver::{FrontendCompiler, FrontendRequest};

/// 测试所需工具由调用方显式设置；无默认环境跳过路径。
fn required(name: &str) -> PathBuf {
    std::env::var_os(name)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("缺少{name}"))
}

/// 有限时间运行本批文件，输出小于管道上限；超时终止并回收进程。
fn execute(path: &Path) -> Output {
    let mut child = Command::new(path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if start.elapsed() > Duration::from_secs(60) {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("原生功能测试超时：{}", path.display());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Runtime门面作为共享内核参考；正式MPFR容差仍由Runtime原有向量测试验证。
fn expected(name: &str, a: f64, b: f64) -> Result<u64, String> {
    use xiao_runtime::*;
    let result = match name {
        "sqrt" => cmath_sqrt(a),
        "floor" => cmath_floor(a),
        "ceil" => cmath_ceil(a),
        "round" => cmath_round(a),
        "trunc" => cmath_trunc(a),
        "abs" => cmath_abs(a),
        "min" => cmath_min(a, b),
        "max" => cmath_max(a, b),
        "pow" => cmath_pow(a, b),
        "sin" => cmath_sin(a),
        "cos" => cmath_cos(a),
        "tan" => cmath_tan(a),
        "exp" => cmath_exp(a),
        "ln" => cmath_ln(a),
        "log10" => cmath_log10(a),
        _ => panic!("未知函数"),
    };
    result.map(f64::to_bits).map_err(|e| e.code().to_owned())
}

/// 正式输入全部经过LLVM路径，1075条pow额外经float **；参考为共享内核位模式。
fn cases() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .unwrap();
    let text =
        fs::read_to_string(root.join("tests/fixtures/docs/cmath/reference-vectors.tsv")).unwrap();
    let mut rows = String::new();
    let mut pow_count = 0;
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let cols: Vec<_> = line.split('\t').collect();
        let index = xiao_intrinsics::CMATH_DECLARATIONS
            .iter()
            .position(|d| d.0.strip_prefix("cmath_") == Some(cols[1]))
            .unwrap();
        let a = u64::from_str_radix(cols[2], 16).unwrap();
        let b = if cols[3] == "-" {
            0
        } else {
            u64::from_str_radix(cols[3], 16).unwrap()
        };
        let value = expected(cols[1], f64::from_bits(a), f64::from_bits(b));
        let (bits, error) = match value {
            Ok(bits) => (bits, 0),
            Err(code) => {
                assert_eq!(code, "X06-RUNTIME-009");
                (0, 1)
            }
        };
        rows.push_str(&format!(
            "{{{index},0x{a:016x}ULL,0x{b:016x}ULL,0x{bits:016x}ULL,{error}}},\n"
        ));
        if cols[1] == "pow" {
            rows.push_str(&format!(
                "{{15,0x{a:016x}ULL,0x{b:016x}ULL,0x{bits:016x}ULL,{error}}},\n"
            ));
            pow_count += 1;
        }
    }
    assert_eq!(pow_count, 1075);
    rows
}

/// 在单一宿主进程中调用真实静态/动态生成函数；仅替换测试模块报告/exit便于观察静态失败。
fn harness(dynamic: bool, ir: &xiao_ir::IrProgram, target: &TargetDescription) -> String {
    let mut declarations = String::new();
    let mut calls = String::new();
    for index in 0..16 {
        let count = if matches!(index, 6..=8 | 15) { 2 } else { 1 };
        let symbol = if dynamic {
            let statement = &ir.body[index];
            format!(
                "xiao.fn.{}",
                xiao_codegen_llvm::stable_hash(
                    format!(
                        "entry::probe{index}@{}..{}",
                        statement.span.start, statement.span.end
                    )
                    .as_bytes()
                )
            )
        } else {
            format!("xiao_fn_{index}_probe{index}")
        };
        // 显式汇编名绕过 C 编译器的符号修饰，Mach-O 仍需链接 LLVM 生成的前导下划线。
        let symbol = if target.object_format == xiao_codegen_llvm::ObjectFormat::MachO {
            format!("_{symbol}")
        } else {
            symbol
        };
        if dynamic {
            declarations.push_str(&format!(
                "extern void probe{index}(Value *out, const Value *a{} ) __asm__(\"{symbol}\");\n",
                if count == 2 { ", const Value *b" } else { "" }
            ));
            calls.push_str(&format!(
                "case {index}: probe{index}(&out,&av{}); break;\n",
                if count == 2 { ",&bv" } else { "" }
            ));
        } else {
            declarations.push_str(&format!(
                "extern double probe{index}(double a{}) __asm__(\"{symbol}\");\n",
                if count == 2 { ",double b" } else { "" }
            ));
            calls.push_str(&format!(
                "case {index}: result=probe{index}(a{}); break;\n",
                if count == 2 { ",b" } else { "" }
            ));
        }
    }
    include_str!("b4_cmath_harness.c")
        .replace("/*EXTRA_CHECKS*/", if dynamic {
            "xiao_runtime_error_clear(); Value wrong={2,42}, invalid_out={0,0}; probe0(&invalid_out,&wrong); if(wrong.tag!=2 || wrong.payload!=42 || invalid_out.tag!=0 || !error_code_is(\"X06-RUNTIME-002\")) return 24;"
        } else { "" })
        .replace("/*DECLARATIONS*/", &declarations)
        .replace("/*CALLS*/", &calls)
        .replace("/*CASE_DATA*/", &cases())
        .replace(
            "/*RESULT*/",
            if dynamic {
                "return out.payload;"
            } else {
                "uint64_t bits; memcpy(&bits,&result,8); return bits;"
            },
        )
}

/// 真实语言程序验证正常退出、catch与清理，不使用setjmp观察适配。
fn language_cases(root: &Path, target: &TargetDescription, toolchain: &Toolchain) {
    let rows = [
        (
            "static",
            "result = int(cmath_pow(exponent = 3.0, base = 2.0))\n",
            8,
            "",
        ),
        (
            "dynamic",
            "try\n    cmath_sqrt(-1.0)\ncatch err as ArithmeticError\n    print(error_code(err))\nfinally\n    print(\"clean\")\n",
            0,
            "clean\nX06-RUNTIME-009\n",
        ),
    ];
    for (label, source, code, stdout) in rows {
        let artifact = FrontendCompiler::new()
            .compile(&FrontendRequest::from_text(source))
            .unwrap();
        for level in [0, 3] {
            let module = xiao_codegen_llvm::lower_program(
                &artifact.ir,
                &CodegenOptions::for_target(target.clone())
                    .with_entry_observation(EntryObservation::ExitCode)
                    .with_optimization_level(level)
                    .unwrap(),
            )
            .unwrap();
            let path = root.join(format!(
                "language-{label}-o{level}{}",
                std::env::consts::EXE_SUFFIX
            ));
            toolchain
                .compile_with_runtime_components_at_level(
                    &module.text,
                    target,
                    &path,
                    None,
                    &module.runtime_components,
                    level,
                )
                .unwrap();
            let result = execute(&path);
            assert_eq!(result.status.code(), Some(code), "{label}: {result:?}");
            assert_eq!(
                String::from_utf8(result.stdout)
                    .unwrap()
                    .replace("\r\n", "\n"),
                stdout
            );
            assert!(
                result.stderr.is_empty(),
                "{label}: {:?}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
    }
}

/// 测试专用C返回值注入，证明0/6与1/2/3/8不会共享可恢复分支；不修改产品Runtime。
fn status_harness(matrix: &str, dynamic: bool) -> String {
    let suffix = r#"
static int injected_status;
static int out_untouched;
void test_fatal(void) { observed_exit=8; longjmp(failure,1); }
extern int xiao_runtime_error_raise_numeric(uint32_t,const unsigned char*,size_t,const unsigned char*,size_t,uint64_t,uint64_t);
int test_cmath_sqrt(const Value *input, Value *out) {
    if(!input || !out || out->tag!=0) return 3;
    if(injected_status==-2 && input->tag!=5) { out_untouched=(out->tag==0 && out->payload==0); return 2; }
    if(injected_status==0) {out->tag=5;out->payload=0x4000000000000000ULL;}
    if(injected_status==6) {
        const unsigned char text[]="injected semantic error";
        xiao_runtime_error_raise_numeric(0,0,0,text,sizeof(text)-1,0,1);
    }
    return injected_status;
}
int main(void) {
    int codes[]={0,1,2,3,6,8};
    for(unsigned i=0;i<6;++i) {
        injected_status=codes[i];observed_exit=0;xiao_runtime_error_clear();
        uint64_t bits=0;
        if(setjmp(failure)==0) bits=invoke(0,0x4010000000000000ULL,0);
        if(injected_status==0) {
            if(observed_exit || xiao_runtime_error_class() || bits!=0x4000000000000000ULL) return 21;
        } else if(injected_status==6) {
            if(!error_is_009() || observed_exit==8) return 22;
        } else {
            if(observed_exit!=8 || xiao_runtime_error_class()!=0) return 23;
        }
    }
/*WRONG_TAG_CASE*/
    puts("B4-3-STATUS statuses=0,1,2,3,6,8 semantic=009 structural=fatal result=passed");
    return 0;
}
"#;
    let wrong_tag = if dynamic {
        r#"    injected_status=-2;observed_exit=0;out_untouched=0;xiao_runtime_error_clear();
    Value wrong={2,42},out={0,0};
    if(setjmp(failure)==0) probe0(&out,&wrong);
    if(observed_exit!=8 || xiao_runtime_error_class()!=0 || !out_untouched) return 24;
    puts("B4-3-STATUS wrong_tag=2 out=preserved pending=untouched fatal=boundary result=passed");
"#
    } else {
        ""
    };
    format!(
        "{}{}",
        matrix.replace("int main(void) {", "int matrix_main(void) {"),
        suffix
    )
    .replace("/*WRONG_TAG_CASE*/", wrong_tag)
}

/// 本任务新Runtime及独立PID/时间戳目录；LLVM-as验证后分别O0/O3链接运行。
pub fn run() {
    let target = TargetDescription::host();
    assert_eq!(std::env::var("XIAO_TARGET_TRIPLE").unwrap(), target.triple);
    let clang = required("XIAO_CLANG");
    let runtime = required("XIAO_RUNTIME_LIBRARY");
    let toolchain = Toolchain::new(&clang)
        .with_llvm_as(required("XIAO_LLVM_AS"))
        .with_runtime_library(&runtime)
        .probe_native_static_libraries("rustc", &target)
        .unwrap();
    let root = std::env::temp_dir().join(format!(
        "b4-3-native-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    for dynamic in [false, true] {
        let ir = crate::probes(dynamic);
        let module =
            xiao_codegen_llvm::lower_program(&ir, &CodegenOptions::for_target(target.clone()))
                .unwrap();
        let text = module
            .text
            .replace("@main(", "@xiao_generated_main(")
            .replace("@exit(", "@test_exit(")
            .replace("@xiao_runtime_error_report(", "@test_report(");
        let c = root.join(format!("harness-{dynamic}.c"));
        let harness = harness(dynamic, &ir, &target);
        fs::write(&c, &harness).unwrap();
        let object = c.with_extension("obj");
        let compiled = Command::new(&clang)
            .args(["-target", &target.triple, "-c"])
            .arg(&c)
            .arg("-o")
            .arg(&object)
            .output()
            .unwrap();
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        let mut libraries =
            xiao_codegen_llvm::query_native_static_libraries(Path::new("rustc"), &target).unwrap();
        libraries.insert(0, object.display().to_string());
        let with_harness = toolchain.clone().with_native_static_libraries(libraries);
        for level in [0, 3] {
            let path = root.join(format!(
                "matrix-{dynamic}-o{level}{}",
                std::env::consts::EXE_SUFFIX
            ));
            fs::write(path.with_extension("ll"), &text).unwrap();
            with_harness
                .compile_with_runtime_components_at_level(
                    &text,
                    &target,
                    &path,
                    Some(&runtime),
                    &module.runtime_components,
                    level,
                )
                .unwrap();
            let result = execute(&path);
            assert!(
                result.status.success(),
                "dynamic={dynamic} O{level}: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert!(result.stderr.is_empty());
            let stdout = String::from_utf8(result.stdout).unwrap();
            assert!(stdout.contains("cases=18318"), "{stdout}");
            println!(
                "B4-3-MATRIX dynamic={dynamic} optimization={level} executable={} {stdout}",
                path.display()
            );
        }
        let status_c = root.join(format!("status-{dynamic}.c"));
        fs::write(&status_c, status_harness(&harness, dynamic)).unwrap();
        let status_obj = status_c.with_extension("obj");
        let compiled = Command::new(&clang)
            .args(["-target", &target.triple, "-c"])
            .arg(&status_c)
            .arg("-o")
            .arg(&status_obj)
            .output()
            .unwrap();
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        let mut libs =
            xiao_codegen_llvm::query_native_static_libraries(Path::new("rustc"), &target).unwrap();
        libs.insert(0, status_obj.display().to_string());
        let status_toolchain = toolchain.clone().with_native_static_libraries(libs);
        let status_ir = text
            .replace("@xiao_runtime_cmath_sqrt(", "@test_cmath_sqrt(")
            .replace("@xiao_runtime_fatal_abi(", "@test_fatal(");
        for level in [0, 3] {
            let path = root.join(format!(
                "status-{dynamic}-o{level}{}",
                std::env::consts::EXE_SUFFIX
            ));
            status_toolchain
                .compile_with_runtime_components_at_level(
                    &status_ir,
                    &target,
                    &path,
                    Some(&runtime),
                    &module.runtime_components,
                    level,
                )
                .unwrap();
            let output = execute(&path);
            assert!(
                output.status.success(),
                "dynamic={dynamic} O{level}: {output:?}"
            );
            println!(
                "dynamic={dynamic} optimization={level} {}",
                String::from_utf8(output.stdout).unwrap()
            );
        }
    }
    language_cases(&root, &target, &toolchain);
    crate::numeric_support::native_fixtures(crate::numeric_support::CASES);
    println!("B4-3-NATIVE language=actual-exit,catch A5=7-fixtures O0,O3 result=passed");
}
