//! X0-A 子进程协议、长度前缀帧和 Rust 核心入口。
//!
//! 协议层只负责传输、版本协商和结果映射；源码解析、类型检查、生命周期分析、
//! 字节码降低、VM 执行和 LLVM 构建仍分别由已有驱动器负责。帧的长度字段是 8 字节
//! 大端无符号整数，只计算 UTF-8 JSON 负载；解码器在分配前检查 16 MiB 上限。

/// 原生构建、工具链和附属产物整理。
mod build;
/// `config.xiao` 的静态固化与旁置配置写入。
mod config;
/// 长度前缀 JSON 帧的编解码。
mod frame;
/// 内部运行结果到协议类型的映射。
mod mapping;
/// 协议版本、消息和响应类型。
mod message;
/// 请求配置、请求消息和协议错误。
mod request;
/// 前端到 VM 的运行路径。
mod run;
/// 请求分发、取消登记和标准输入输出服务。
mod service;
/// 协议版本、源码和目标校验。
mod validate;

#[cfg(test)]
#[allow(unused_imports)]
use crate::run::{CancellationToken, ExitCode};
#[cfg(test)]
#[allow(unused_imports)]
use build::{diagnostics_component_path, stage_diagnostics_component};
#[cfg(test)]
#[allow(unused_imports)]
use config::FrozenRuntimeConfig;
#[cfg(test)]
#[allow(unused_imports)]
use config::{freeze_runtime_config, write_runtime_config};
/// 帧边界常量、错误类型以及 JSON 帧编解码函数。
pub use frame::{
    FRAME_ERROR_CODE, FRAME_LENGTH_BYTES, FrameError, MAX_FRAME_BYTES, decode_frame, encode_frame,
    read_frame, write_frame,
};
/// 将内部诊断和值参数转换为跨进程协议形状。
pub use mapping::{protocol_diagnostic, protocol_param};
/// 协议版本、响应和跨进程值类型。
pub use message::*;
/// 请求配置、请求消息和协议层错误类型。
pub use request::*;
#[cfg(test)]
#[allow(unused_imports)]
use serde_json::Value;
#[cfg(test)]
#[allow(unused_imports)]
use serde_json::json;
#[cfg(test)]
#[allow(unused_imports)]
use service::worker_response;
/// 请求分发、帧读取、服务循环和崩溃响应入口。
pub use service::{core_crash_response, dispatch, read_request, serve, serve_stdio};
#[cfg(test)]
#[allow(unused_imports)]
use std::fs;
#[cfg(test)]
#[allow(unused_imports)]
use std::io::{self, Write};
#[cfg(test)]
#[allow(unused_imports)]
use std::sync::Mutex;

#[cfg(test)]
#[path = "protocol_tests.rs"]
/// 覆盖帧边界、版本协商、取消和真实前端运行路径。
mod tests;

#[cfg(test)]
#[path = "protocol_architecture_tests.rs"]
/// 锁定协议子模块的源码级依赖方向。
mod architecture_tests;
