//! N0-C-2 平台异常报告边界回归测试。

use std::process::Command;

#[cfg(any(unix, windows))]
#[test]
/// 平台异常必须在独立进程中终止，并输出统一机器字段。
fn platform_failure_report_is_machine_readable() {
    assert_platform_failure("segv", 139);
    #[cfg(unix)]
    {
        assert_platform_failure("bus", 139);
        assert_platform_failure("trap", 132);
    }
}

#[cfg(any(unix, windows))]
/// 用真实信号验证处理器的报告与冻结退出码，避免只验证映射函数。
fn assert_platform_failure(kind: &str, exit_code: i32) {
    let executable = std::env::current_exe().expect("应能定位当前测试进程");
    let output = Command::new(executable)
        .args([
            "--exact",
            "platform_failure_child",
            "--ignored",
            "--nocapture",
        ])
        .env("XIAO_RUN_PLATFORM_FAILURE_CHILD", "1")
        .env("XIAO_PLATFORM_FAILURE_KIND", kind)
        .output()
        .expect("应能启动平台异常子进程");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(exit_code),
        "平台异常应以统一退出码终止；stderr: {stderr}"
    );
    for marker in [
        "class=fatal",
        "code=X07-FATAL-005",
        "message_id=fatal.hardware",
    ] {
        assert!(
            stderr.contains(marker),
            "平台异常报告缺少 {marker}：{stderr}"
        );
    }
    assert!(
        stderr.contains(&format!("exit_code={exit_code}")),
        "{stderr}"
    );
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
        let signal = match std::env::var("XIAO_PLATFORM_FAILURE_KIND").as_deref() {
            Ok("bus") => libc::SIGBUS,
            Ok("trap") => libc::SIGTRAP,
            _ => libc::SIGSEGV,
        };
        libc::raise(signal);
    }

    #[cfg(windows)]
    unsafe {
        std::ptr::write_volatile(std::ptr::null_mut::<u8>(), 1);
    }
}
