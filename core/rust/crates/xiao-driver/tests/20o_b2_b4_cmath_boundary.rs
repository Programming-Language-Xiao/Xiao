//! B4-2A-FIX1：planned cmath 的子 VM 和归档拒绝，保留 B4-1 类型/IR 能力。

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
/// 子 VM 在加载模块前拒绝未使用导入/直接调用，无输出、模块实例或部分导入值释放。
fn child_rejects_planned_cmath_imports_before_module_execution() {
    let mut checked = 0;
    for prefix in ["std", "stdlib"] {
        for (index, (import, call)) in import_forms(prefix).into_iter().enumerate() {
            for called in [false, true] {
                let value = if called { call.as_str() } else { "7.0" };
                let source = format!(
                    "from std.concurrent import current_runtime, spawn, join\n\
                     def work() -> float\n    {import}\n    return {value}\n\
                     rt = current_runtime()\ntask = spawn(rt, work)\nprint(join(task))\n"
                );
                let request = FrontendRequest::from_text(source);
                let artifact = FrontendCompiler::new()
                    .compile(&request)
                    .expect("planned 模块的合法类型/IR 必须保留");
                let child_scopes = artifact
                    .ir
                    .ownership
                    .scopes
                    .iter()
                    .filter(|scope| scope.kind == "function")
                    .map(|scope| scope.id)
                    .collect::<Vec<_>>();
                assert!(!child_scopes.is_empty());
                for level in [OptimizationLevel::O0, OptimizationLevel::O3] {
                    let outcome = FrontendVmDriver::new().run_artifact(
                        &artifact,
                        &DriverRequest::new(request.clone())
                            .with_timeout(Duration::from_secs(10))
                            .with_optimization_level(level),
                    );
                    assert_eq!(outcome.code(), Some("X06-RUNTIME-012"), "{outcome:?}");
                    assert!(
                        outcome.report().is_some_and(|report| {
                            report.message.contains("官方标准库模块 cmath 加载失败")
                        }),
                        "{prefix} shape={index} called={called} {level:?}: {outcome:?}"
                    );
                    let events = outcome.as_executed().unwrap().events();
                    assert!(!events.iter().any(|event| matches!(event,
                        VmEvent::ModuleLoaded { module } if module == "stdlib.cmath")));
                    assert!(!events.iter().any(|event| matches!(
                        event,
                        VmEvent::IntrinsicOutput { .. } | VmEvent::BoundaryCopyReleased { .. }
                    )));
                    assert!(
                        !events.iter().any(|event| matches!(event,
                        VmEvent::ValueReleased { scope, .. } if child_scopes.contains(scope))),
                        "失败导入不能产生部分子 VM 值释放：{events:?}"
                    );
                    checked += 1;
                }
            }
        }
    }
    assert_eq!(checked, 32);
    println!(
        "FIX1-CHILD cases={checked} code=X06-RUNTIME-012 loaded=false output=false partial_release=false"
    );
}

#[test]
/// 可达、未调用函数和死分支的导入都在输出/缓存创建前拒绝，两条别名不能绕过。
fn archive_rejects_planned_cmath_with_no_output_or_temporary_residue() {
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
                    let before = directory_snapshot(&root);
                    let mut request = ArchiveBuildRequest::new(&input);
                    request.output = Some(root.join("out/main.xar"));
                    request.cache_root = Some(root.join("cache"));
                    request.optimization_level = level;
                    let error = build_archive(&request, &CancellationToken::new())
                        .expect_err("planned cmath 不能进入归档");
                    assert_eq!(error.code, "X11-PROTOCOL-007");
                    assert!(error.message.contains("cmath_sqrt"), "{source:?}: {error}");
                    assert_eq!(
                        directory_snapshot(&root),
                        before,
                        "拒绝不能留下部分归档、缓存或临时文件"
                    );
                    checked += 1;
                }
            }
        }
    }
    assert_eq!(checked, 48);
    println!("FIX1-ARCHIVE cases={checked} code=X11-PROTOCOL-007 output=false residue=false");
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
