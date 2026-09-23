//! Xiao N0-A LLVM 原生后端。
//!
//! 后端只消费已经由 `xiao-driver` 生成并验证的 [`xiao_ir::IrProgram`]，把 N0-A 支持的
//! 静态标量和控制流写成 LLVM IR 文本。LLVM 开发库不是 Rust 编译期依赖；验证、目标文件
//! 生成和链接都通过调用方显式注入的外部工具链完成。

/// 原生产物构建请求和运行观察适配。
mod build;
/// 动态值、容器、表和正常释放计划的 Runtime ABI 降低。
mod dynamic;
/// 后端结构化错误定义。
mod error;
/// 类型化 IR 到 LLVM 文本的降低实现。
mod ir;
/// 规范化目标描述和固定宽度约束。
mod target;
/// 外部 LLVM 工具链调用和指纹。
mod toolchain;

/// 构建请求、原生产物和运行观察接口。
pub use build::{BuildRequest, NativeArtifact, NativeBuild, NativeRun, NativeRunResult};
/// 后端失败类型和统一结果别名。
pub use error::{CodegenError, Result};
/// LLVM 文本降低选项、入口观察策略和降低入口。
pub use ir::{CodegenOptions, EntryObservation, LlvmModule, NativeStartup, validate_program};
/// 目标字节序、对象格式和规范化目标描述。
pub use target::{Endian, ObjectFormat, TargetDescription};
/// 外部 LLVM 工具链描述、版本和构建指纹。
pub use toolchain::{
    Toolchain, ToolchainFingerprint, ToolchainVersions, parse_native_static_libraries,
    query_native_static_libraries,
};

/// 后端接口版本；参与原生产物指纹。
///
/// 版本 2 固定了 COFF 目标的 Runtime `sret`/间接聚合参数调用约定；旧版本生成的动态
/// LLVM 文本不能与当前 MSVC Runtime ABI 混用。
pub const CODEGEN_VERSION: u32 = 2;

/// 将同一份类型化 IR 降低为静态标量或 Runtime ABI LLVM 模块。
pub fn lower_program(program: &xiao_ir::IrProgram, options: &CodegenOptions) -> Result<LlvmModule> {
    if dynamic::program_uses_runtime(program) {
        dynamic::lower_program(program, options)
    } else {
        ir::lower_static_program(program, options)
    }
}
