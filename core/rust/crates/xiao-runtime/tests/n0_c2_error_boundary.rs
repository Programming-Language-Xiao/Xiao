//! N0-C-2 平台异常报告边界回归测试。

use std::process::Command;

#[cfg(any(unix, windows))]
#[test]
/// 平台异常必须在独立进程中终止，并输出统一机器字段。
fn platform_failure_report_is_machine_readable() {
    let executable = std::env::current_exe().expect("应能定位当前测试进程");
    let output = Command::new(executable)
        .args([
            "--exact",
            "platform_failure_child",
            "--ignored",
            "--nocapture",
        ])
        .env("XIAO_RUN_PLATFORM_FAILURE_CHILD", "1")
        .output()
        .expect("应能启动平台异常子进程");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(139),
        "平台异常应以统一退出码终止；stderr: {stderr}"
    );
    for marker in [
        "class=fatal",
        "code=X07-FATAL-005",
        "message_id=fatal.hardware",
        "exit_code=139",
    ] {
        assert!(
            stderr.contains(marker),
            "平台异常报告缺少 {marker}：{stderr}"
        );
    }
}

#[cfg(any(unix, windows))]
#[test]
#[ignore = "由 platform_failure_report_is_machine_readable 在子进程中调用"]
/// 子进程入口只用于触发真正的平台异常；异常不进入 Xiao catch 路径。
fn platform_failure_child() {
    if std::env::var_os("XIAO_RUN_PLATFORM_FAILURE_CHILD").is_none() {
        return;
    }
    xiao_runtime::install_platform_failure_reporter();

    #[cfg(unix)]
    unsafe {
        libc::raise(libc::SIGSEGV);
    }

    #[cfg(windows)]
    unsafe {
        std::ptr::write_volatile(std::ptr::null_mut::<u8>(), 1);
    }
}
