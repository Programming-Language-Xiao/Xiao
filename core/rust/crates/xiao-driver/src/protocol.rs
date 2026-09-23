//! X0-A 子进程协议、长度前缀帧和 Rust 核心入口。
//!
//! 协议层只负责传输、版本协商和结果映射；源码解析、类型检查、生命周期分析、
//! 字节码降低、VM 执行和 LLVM 构建仍分别由已有驱动器负责。帧的长度字段是 8 字节
//! 大端无符号整数，只计算 UTF-8 JSON 负载；解码器在分配前检查 16 MiB 上限。

mod build;
mod config;
mod frame;
mod mapping;
mod message;
mod request;
mod run;
mod service;
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
pub use frame::{
    FRAME_ERROR_CODE, FRAME_LENGTH_BYTES, FrameError, MAX_FRAME_BYTES, decode_frame, encode_frame,
    read_frame, write_frame,
};
pub use mapping::{protocol_diagnostic, protocol_param};
pub use message::*;
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
mod architecture_tests;
