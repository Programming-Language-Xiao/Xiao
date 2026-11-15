//! B4-4：将FIX1的planned边界显式迁移为子VM/归档执行；未知模块拒绝仍保留。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use xiao_driver::{
    ArchiveBuildRequest, CancellationToken, DriverRequest, FrontendCompiler, FrontendRequest,
    FrontendVmDriver, build_archive,
};
use xiao_optimizer::OptimizationLevel;
use xiao_vm::VmEvent;

/// 同一进程内的每个夹具使用独立且不能复用的目录。
fn workspace() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "xiao-b4-fix1-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).expect("FIX1 夹具目录必须全新");
    root
}

/// 每个形态同时用于未使用导入与真实调用，防止只卡某个表面写法。
fn import_forms(prefix: &str) -> [(String, String); 4] {
    [
        (
            format!("from {prefix}.cmath import sqrt"),
            "sqrt(4.0)".into(),
        ),
        (
            format!("import {prefix}.cmath as math"),
            "math.sqrt(4.0)".into(),
        ),
        (
            format!("import {prefix}.cmath"),
            format!("{prefix}.cmath.sqrt(4.0)"),
        ),
        (
            format!("from {prefix}.cmath import sqrt as root"),
            "root(4.0)".into(),
        ),
    ]
}

/// 记录完整目录集合和文件字节，连空目录/临时文件残留也不能被忽略。
fn directory_snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn visit(root: &Path, current: &Path, result: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in std::fs::read_dir(current).unwrap() {
            let path = entry.unwrap().path();
            let key = path.strip_prefix(root).unwrap().to_path_buf();
            if path.is_dir() {
                result.insert(key, None);
                visit(root, &path, result);
            } else {
                result.insert(key, Some(std::fs::read(path).unwrap()));
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}

#[test]
/// 相同32条子VM导入/调用形态在O0/O3实际执行并保持规范模块身份。
fn child_executes_cmath_alias_imports_and_calls() {
    let mut checked = 0;
    for prefix in ["std", "stdlib"] {
        for (index, (import, call)) in import_forms(prefix).into_iter().enumerate() {
            for called in [false, true] {
                let value = if called { call.as_str() } else { "7.0" };
                let source = format!(
                    "from std.concurrent import current_runtime, spawn, join\ndef work() -> float\n    {import}\n    return {value}\nrt = current_runtime()\ntask = spawn(rt, work)\nprint(join(task))\n"
                );
                let request = FrontendRequest::from_text(source);
                let artifact = FrontendCompiler::new().compile(&request).unwrap();
                for level in [OptimizationLevel::O0, OptimizationLevel::O3] {
                    let outcome = FrontendVmDriver::new().run_artifact(
                        &artifact,
                        &DriverRequest::new(request.clone())
                            .with_timeout(Duration::from_secs(10))
                            .with_optimization_level(level),
                    );
                    assert!(
                        outcome.is_success(),
                        "{prefix} shape={index} called={called} {level:?}: {outcome:?}"
                    );
                    let events = outcome.as_executed().unwrap().events();
                    let output: String = events
                        .iter()
                        .filter_map(|event| match event {
                            VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect();
                    assert_eq!(output, if called { "2\n" } else { "7\n" });
                    assert!(!events.iter().any(|event| matches!(event,VmEvent::ModuleLoaded {module} if module=="std.cmath")));
                    checked += 1;
                }
            }
        }
    }
    assert_eq!(checked, 32);
    println!("B4-4-CHILD cases={checked} identity=stdlib.cmath output=exact result=passed");
}

#[test]
/// 48条直接/未调用函数/死分支导入均可归档；删除应用源码后只用归档与官方加载器。
fn archive_executes_cmath_without_source_or_bundled_official_bodies() {
    let mut checked = 0;
    for prefix in ["std", "stdlib"] {
        for (import, _) in import_forms(prefix) {
            for source in [
                format!("{import}\nprint(1)\n"),
                format!("def unused() -> int\n    {import}\n    return 7\nprint(1)\n"),
                format!("if false\n    {import}\nprint(1)\n"),
            ] {
                for level in [OptimizationLevel::O0, OptimizationLevel::O3] {
                    let root = workspace();
                    let input = root.join("main.xiao");
                    std::fs::write(&input, &source).unwrap();
                    let mut request = ArchiveBuildRequest::new(&input);
                    request.output = Some(root.join("out/main.xar"));
                    request.cache_root = Some(root.join("cache"));
                    request.optimization_level = level;
                    let archive = build_archive(&request, &CancellationToken::new()).unwrap();
                    assert!(
                        archive
                            .index
                            .entries
                            .iter()
                            .all(|entry| !entry.module.starts_with("stdlib.")
                                && !entry.module.starts_with("std."))
                    );
                    let raw = std::fs::read(&archive.output).unwrap();
                    std::fs::remove_file(&input).unwrap();
                    let outcome = xiao_driver::run_archive(&raw, Default::default()).unwrap();
                    assert!(outcome.result.is_success(), "{source:?}: {outcome:?}");
                    let output: String = outcome
                        .events
                        .iter()
                        .filter_map(|event| match event {
                            VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect();
                    assert_eq!(output, "1\n");
                    checked += 1;
                }
            }
        }
    }
    assert_eq!(checked, 48);
    println!(
        "B4-4-ARCHIVE cases={checked} source_removed=true bundled_official=false result=passed"
    );
}

#[test]
/// 未知及后置官方模块仍在归档写出前拒绝，无部分输出或缓存残留。
fn unknown_official_imports_fail_without_output_or_temporary_residue() {
    for prefix in ["std", "stdlib"] {
        for module in ["json", "toml", "missing_b4_4"] {
            let root = workspace();
            let input = root.join("main.xiao");
            std::fs::write(&input, format!("import {prefix}.{module}\nprint(1)\n")).unwrap();
            let before = directory_snapshot(&root);
            let mut request = ArchiveBuildRequest::new(&input);
            request.output = Some(root.join("out/main.xar"));
            request.cache_root = Some(root.join("cache"));
            let error = build_archive(&request, &CancellationToken::new()).unwrap_err();
            assert_eq!(error.code, "X11-PROTOCOL-007");
            assert_eq!(directory_snapshot(&root), before, "{error:?}");
        }
    }
}

#[test]
/// 未导入模块中的 cmath 与同名项目模块不会污染普通模块的执行/归档闭包。
fn unrelated_and_project_cmath_modules_keep_their_existing_boundaries() {
    let root = workspace();
    std::fs::write(
        root.join("cmath.xiao"),
        "def answer() -> int\n    return 7\n",
    )
    .unwrap();
    std::fs::write(root.join("unused.xiao"), "from std.cmath import sqrt\n").unwrap();
    let input = root.join("main.xiao");
    std::fs::write(&input, "from cmath import answer\nprint(answer())\n").unwrap();
    let mut frontend = FrontendRequest::from_file(&input).unwrap();
    frontend.context.project_root = Some(root.clone());
    let result = xiao_driver::run(&DriverRequest::new(frontend));
    assert!(result.is_success(), "{result:?}");
    let mut request = ArchiveBuildRequest::new(&input);
    request.output = Some(root.join("main.xar"));
    request.cache_root = Some(root.join("cache"));
    let archive = build_archive(&request, &CancellationToken::new()).unwrap();
    assert!(
        !archive
            .index
            .stdlib_modules
            .iter()
            .any(|module| module == "stdlib.cmath")
    );
    let replay =
        xiao_xar::run_archive(&std::fs::read(archive.output).unwrap(), Default::default()).unwrap();
    assert!(replay.result.is_success(), "{replay:?}");
    println!("FIX1-ISOLATION project_cmath=success unimported_cmath=excluded archive=success");
}
