//! 协议子模块依赖方向的源码级回归测试。

/// 移除行注释，避免依赖断言被说明文字误触发。
fn code_without_line_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| line.split_once("//").map_or(line, |(code, _)| code))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 断言源码不包含指定的反向依赖。
fn assert_no_dependency(source_name: &str, source: &str, dependency: &str) {
    let code = code_without_line_comments(source);
    assert!(
        !code.contains(dependency),
        "{source_name} 不得依赖 {dependency}"
    );
}

#[test]
/// 验证协议门面、叶子模块和服务层的依赖方向保持无环。
fn module_dependency_direction_is_acyclic() {
    let facade = include_str!("protocol.rs");
    for declaration in [
        "mod frame;",
        "mod message;",
        "mod request;",
        "mod mapping;",
        "mod validate;",
        "mod run;",
        "mod build;",
        "mod config;",
        "mod service;",
        "mod test;",
    ] {
        assert!(facade.contains(declaration), "门面缺少 {declaration}");
    }
    for implementation in [
        "fn read_request",
        "fn dispatch",
        "fn worker_response",
        "fn serve",
        "fn build_response",
        "fn test_request_response",
    ] {
        assert!(
            !facade.contains(implementation),
            "门面不应保留实现 {implementation}"
        );
    }

    let frame = include_str!("protocol/frame.rs");
    let message = include_str!("protocol/message.rs");
    let request = include_str!("protocol/request.rs");
    for (name, source) in [
        ("frame.rs", frame),
        ("message.rs", message),
        ("request.rs", request),
    ] {
        assert_no_dependency(name, source, "super::service");
        assert_no_dependency(name, source, "super::run");
        assert_no_dependency(name, source, "super::build");
        assert_no_dependency(name, source, "crate::protocol");
    }
    assert_no_dependency("frame.rs", frame, "super::");
    assert_no_dependency("message.rs", message, "super::");
    assert_no_dependency("request.rs", request, "super::");

    let mapping = include_str!("protocol/mapping.rs");
    let validate = include_str!("protocol/validate.rs");
    for (name, source) in [("mapping.rs", mapping), ("validate.rs", validate)] {
        assert_no_dependency(name, source, "super::service");
        assert_no_dependency(name, source, "super::run");
        assert_no_dependency(name, source, "super::build");
        assert_no_dependency(name, source, "super::config");
        assert_no_dependency(name, source, "crate::protocol");
    }

    let run = include_str!("protocol/run.rs");
    let build = include_str!("protocol/build.rs");
    let config = include_str!("protocol/config.rs");
    let test = include_str!("protocol/test.rs");
    assert_no_dependency("run.rs", run, "super::service");
    assert_no_dependency("run.rs", run, "super::build");
    assert_no_dependency("run.rs", run, "super::config");
    assert_no_dependency("test.rs", test, "super::service");
    assert_no_dependency("test.rs", test, "super::build");
    assert_no_dependency("test.rs", test, "super::config");
    assert_no_dependency("test.rs", test, "super::validate");
    assert!(test.contains("super::run"));
    assert_no_dependency("build.rs", build, "super::service");
    assert_no_dependency("build.rs", build, "super::validate");
    assert_no_dependency("config.rs", config, "super::service");
    assert_no_dependency("config.rs", config, "super::run");
    assert_no_dependency("config.rs", config, "super::build");
    assert_no_dependency("config.rs", config, "super::mapping");

    let service = include_str!("protocol/service.rs");
    for dependency in [
        "super::frame",
        "super::message",
        "super::request",
        "super::mapping",
        "super::validate",
        "super::run",
        "super::build",
        "super::test",
    ] {
        assert!(
            service.contains(dependency),
            "service.rs 应显式依赖 {dependency}"
        );
    }
    assert!(service.contains("xiao_package"));
    assert!(service.contains("std::thread"));
    assert!(service.contains("Arc"));
    for (name, source) in [
        ("frame.rs", frame),
        ("message.rs", message),
        ("request.rs", request),
        ("mapping.rs", mapping),
        ("validate.rs", validate),
        ("run.rs", run),
        ("build.rs", build),
        ("config.rs", config),
        ("test.rs", test),
    ] {
        assert_no_dependency(name, source, "std::thread");
        assert_no_dependency(name, source, "Arc<");
        assert_no_dependency(name, source, "Arc::");
    }
}
