//! 19B 的换行、路径、权限和进程终止语义回归。

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use xiao_artifacts::{ArtifactStore, ObjectKind};
use xiao_driver::{DriverOutcome, FrontendCompiler, FrontendRequest, run};
use xiao_optimizer::normalize_process_termination;
use xiao_vm::VmEvent;

/// 创建带稳定前缀的跨平台临时目录。
fn temporary_root(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("xiao-19b-{label}-{}", std::process::id()));
    fs::create_dir_all(&path).expect("创建 19B 临时目录");
    path
}

/// 提取运行结果中的用户输出事件。
fn observed_output(outcome: &DriverOutcome) -> String {
    outcome
        .as_executed()
        .map(|execution| {
            execution
                .events()
                .iter()
                .filter_map(|event| match event {
                    VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(unix)]
/// 读取 Unix 子进程的信号终止信息。
fn observed_signal(status: &std::process::ExitStatus) -> Option<String> {
    use std::os::unix::process::ExitStatusExt;
    status.signal().map(|value| format!("SIG{value}"))
}

#[cfg(not(unix))]
/// Windows 等平台没有 POSIX 信号字段，返回明确的缺省值。
fn observed_signal(_status: &std::process::ExitStatus) -> Option<String> {
    None
}

#[test]
/// LF 与 CRLF 源码应得到同一运行观察。
fn lf_and_crlf_sources_have_the_same_runtime_observation() {
    let lf = "print(\"line\")\nvalue = 1\n";
    let crlf = lf.replace('\n', "\r\n");
    let left = run(&xiao_driver::DriverRequest::new(
        FrontendRequest::from_text(lf),
    ));
    let right = run(&xiao_driver::DriverRequest::new(
        FrontendRequest::from_text(crlf),
    ));
    assert_eq!(left.is_success(), right.is_success());
    assert_eq!(left.code(), right.code());
    assert_eq!(left.exit_code(), right.exit_code());
    assert_eq!(observed_output(&left), observed_output(&right));
    assert_eq!(observed_output(&left), "line\n");
}

#[test]
/// 相对、绝对、空格和 Unicode 路径都应编译并运行到相同的具体输出。
fn absolute_relative_space_and_unicode_paths_are_accepted_by_runtime_boundary() {
    let root = temporary_root("路径 空格");
    let source_path = root.join("模块-中文.xiao");
    let source = "print(\"path\")\n";
    let mut request = FrontendRequest::from_text_at(source, &source_path);
    request.context.project_root = Some(root.clone());
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .expect("含空格和 Unicode 的绝对路径应可编译");
    assert!(!artifact.ir.body.is_empty());
    let absolute_run = run(&xiao_driver::DriverRequest::new(request));
    assert!(absolute_run.is_success());
    assert_eq!(observed_output(&absolute_run), "path\n");
    let relative = FrontendRequest::from_text_at(source, "模块-中文.xiao");
    let relative_artifact = FrontendCompiler::new()
        .compile(&relative)
        .expect("相对路径应可编译");
    let relative_run = run(&xiao_driver::DriverRequest::new(relative));
    assert!(relative_run.is_success());
    assert_eq!(observed_output(&relative_run), "path\n");
    assert_eq!(artifact.ir.body.len(), relative_artifact.ir.body.len());
    let _ = fs::remove_dir_all(root);
}

#[test]
/// 只读对象在当前宿主仍可完整校验。
fn read_only_content_addressed_objects_remain_verifiable_on_each_host() {
    let root = temporary_root("readonly-cache");
    let store = ArtifactStore::open(&root).expect("创建对象缓存");
    let reference = store
        .put(ObjectKind::Source, b"readonly-source")
        .expect("写入对象");
    let mut permissions = fs::metadata(&reference.path)
        .expect("读取对象权限")
        .permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&reference.path, permissions).expect("设置只读对象");
    assert_eq!(
        store.read(ObjectKind::Source, reference.digest).unwrap(),
        b"readonly-source"
    );
    assert!(
        fs::metadata(&reference.path)
            .unwrap()
            .permissions()
            .readonly()
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
/// 真实子进程的 Unix 信号和 Windows 中断退出码必须有明确替代表示。
fn process_termination_has_an_explicit_signal_or_exit_alternative() {
    let status = if cfg!(windows) {
        Command::new("cmd")
            .args(["/c", "exit", "130"])
            .status()
            .expect("应能启动 Windows 退出码探针")
    } else {
        Command::new("sh")
            .args(["-c", "kill -TERM $$"])
            .status()
            .expect("应能启动 Unix 信号探针")
    };
    let signal = observed_signal(&status);
    if cfg!(windows) {
        assert_eq!(status.code(), Some(130));
        assert_eq!(
            normalize_process_termination(status.code(), signal.as_deref()),
            "exit:130"
        );
    } else {
        assert!(signal.is_some(), "Unix 探针必须由信号终止：{status:?}");
        assert!(normalize_process_termination(None, signal.as_deref()).starts_with("signal:SIG"));
    }
}
