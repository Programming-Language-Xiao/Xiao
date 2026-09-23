//! 协议请求的边界校验。
//!
//! 校验层只检查协议版本、源码身份和目标描述；具体运行参数与构建工具链
//! 的阶段性校验留在各自职责模块，避免把执行语义倒灌到协议门面。

use super::{CORE_VERSION, PROTOCOL_VERSION, ProtocolError, ProtocolTarget, SourceIdentity};

/// 校验协议版本和统一核心版本。
pub(super) fn validate_versions(
    protocol_version: u16,
    core_version: u32,
) -> Result<(), ProtocolError> {
    if protocol_version != PROTOCOL_VERSION {
        return Err(ProtocolError::version(format!(
            "协议版本不兼容：需要 {}，收到 {}",
            PROTOCOL_VERSION, protocol_version
        )));
    }
    if core_version != CORE_VERSION {
        return Err(ProtocolError::version(format!(
            "核心版本不兼容：需要 {}，收到 {}",
            CORE_VERSION, core_version
        )));
    }
    Ok(())
}

/// 校验源码和模块身份的最小边界。
pub(super) fn validate_source(source: &SourceIdentity) -> Result<(), ProtocolError> {
    if source.module.trim().is_empty() {
        return Err(ProtocolError::request("source.module", "模块名不能为空"));
    }
    Ok(())
}

/// 校验目标字段而不复制 LLVM 目标语义。
pub(super) fn validate_target(target: &ProtocolTarget) -> Result<(), ProtocolError> {
    target.to_target().map(|_| ())
}
