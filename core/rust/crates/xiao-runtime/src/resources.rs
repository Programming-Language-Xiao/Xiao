//! VM 与原生 ABI 共用的文件系统资源操作；所有宿主访问都先验证能力。

use std::fs::{File, metadata};

use xiao_intrinsics::{CAPABILITY_FILESYSTEM, validate_capability_mask};

use crate::{
    DiagnosticParam, ExternalResourceHandle, ExternalResourceKind, RuntimeError, RuntimeResult,
    RuntimeValue,
};

/// 在读取宿主状态或关闭句柄前验证冻结的 filesystem 能力位。
pub(crate) fn require_filesystem(capabilities: u64) -> RuntimeResult<()> {
    if validate_capability_mask(capabilities).is_err() {
        return Err(RuntimeError::invalid_value("能力掩码包含未知位"));
    }
    if capabilities & CAPABILITY_FILESYSTEM == 0 {
        return Err(RuntimeError::capability_unavailable("filesystem"));
    }
    Ok(())
}

/// 把宿主 IO 或 UTF-8 失败转换为稳定身份，并保留宿主原因参数。
pub(crate) fn io_error(operation: &str, error: impl std::fmt::Display) -> RuntimeError {
    RuntimeError::invalid_value(format!("{operation}失败"))
        .with_param("reason", DiagnosticParam::Text(error.to_string()))
}

/// 从 Runtime 字符串读取原始路径；不规范化、不更换相对路径基准。
fn path(value: &RuntimeValue) -> RuntimeResult<String> {
    match value {
        RuntimeValue::Str(value) => value.to_string(),
        _ => Err(RuntimeError::type_mismatch("str", value.type_name())),
    }
}

/// 在当前进程 cwd 下打开文件，调用者拥有返回的资源强引用。
pub fn open_file(value: &RuntimeValue, capabilities: u64) -> RuntimeResult<RuntimeValue> {
    require_filesystem(capabilities)?;
    let path = path(value)?;
    match metadata(&path) {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => return Err(io_error("打开文件", "目标不是文件")),
        Err(error) => return Err(io_error("打开文件", error)),
    }
    let file = File::open(&path).map_err(|error| io_error("打开文件", error))?;
    ExternalResourceHandle::from_file(file).map(RuntimeValue::ExternalResource)
}

/// 验证目录并保存原始路径；首版不持有平台目录句柄。
pub fn open_directory(value: &RuntimeValue, capabilities: u64) -> RuntimeResult<RuntimeValue> {
    require_filesystem(capabilities)?;
    let path = path(value)?;
    match metadata(&path) {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => return Err(io_error("打开目录", "目标不是目录")),
        Err(error) => return Err(io_error("打开目录", error)),
    }
    ExternalResourceHandle::from_directory(path.into()).map(RuntimeValue::ExternalResource)
}

/// 关闭文件或目录，别名共享关闭状态；缺能力时保持资源状态不变。
pub fn close(value: &RuntimeValue, capabilities: u64) -> RuntimeResult<()> {
    require_filesystem(capabilities)?;
    match value {
        RuntimeValue::ExternalResource(resource) => resource.close(),
        _ => Err(RuntimeError::type_mismatch("resource", value.type_name())),
    }
}

/// 读取完整的严格 UTF-8 文件文本；任何路径均先验证能力与资源种类。
pub fn read_text(value: &RuntimeValue, capabilities: u64) -> RuntimeResult<RuntimeValue> {
    require_filesystem(capabilities)?;
    match value {
        RuntimeValue::ExternalResource(resource)
            if resource.kind()? == ExternalResourceKind::File =>
        {
            RuntimeValue::new_string(resource.read_text()?)
        }
        _ => Err(RuntimeError::type_mismatch("file", value.type_name())),
    }
}
