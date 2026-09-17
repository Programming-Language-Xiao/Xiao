//! Xiao 编译、运行和构建请求编排接口的 crate 入口。
//!
//! 当前提供 08-U0 统一前端：源码只经过一次解析、模块分析、类型检查和
//! 生命周期分析，成功后交给 `xiao-ir` 验证。执行后端和 CLI 接线留给后续阶段。

/// 统一前端请求、上下文、结果和编排器。
mod frontend;

/// 重导出统一前端公共接口。
pub use frontend::{
    ExternalModuleGraph, FRONTEND_VERSION, FrontendArtifact, FrontendCompiler, FrontendContext,
    FrontendError, FrontendIoError, FrontendRequest, compile, empty_type_results,
};
