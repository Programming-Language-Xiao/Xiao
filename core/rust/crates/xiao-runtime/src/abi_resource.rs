//! 首批 `os` 资源 intrinsic 的 Runtime ABI 实现。

use std::fs::{File, metadata};
use std::path::Path;

use xiao_intrinsics::{Capability, validate_capability_mask};
use xiao_runtime_abi::{XiaoAbiStatus, XiaoValue};

use super::{
    RuntimeError, RuntimeValue, runtime_to_value, set_pending_runtime_error, status_from_error,
    value_to_runtime,
};
use crate::value::ExternalResourceHandle;

/// 在任何宿主访问前验证能力上下文。
fn require_filesystem(capabilities: u64) -> Result<(), i32> {
    if validate_capability_mask(capabilities).is_err() {
        return Err(status_from_error(&RuntimeError::invalid_value(
            "能力掩码包含未知位",
        )));
    }
    if capabilities & Capability::Filesystem.bit().unwrap_or(0) == 0 {
        return Err(status_from_error(&RuntimeError::capability_unavailable(
            Capability::Filesystem.as_str(),
        )));
    }
    Ok(())
}

/// 将宿主路径错误映射为带 `reason` 参数的稳定 Runtime 错误。
fn path_error(operation: &str, error: impl std::fmt::Display) -> RuntimeError {
    RuntimeError::invalid_value(format!("{operation}失败")).with_param(
        "reason",
        crate::errors::DiagnosticParam::Text(error.to_string()),
    )
}

/// 从 Xiao 字符串值中借用宿主路径。
unsafe fn path_from_value(value: *const XiaoValue) -> Result<String, i32> {
    if value.is_null() {
        return Err(XiaoAbiStatus::Null.code());
    }
    let value = unsafe { value_to_runtime(&*value) }?;
    let RuntimeValue::Str(value) = value else {
        let error = RuntimeError::type_mismatch("str", value.type_name());
        return Err(status_from_error(&error));
    };
    value.to_string().map_err(|error| status_from_error(&error))
}

/// 打开文件并返回外部资源值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_resource_open_file_with_capabilities(
    path: *const XiaoValue,
    capabilities: u64,
) -> XiaoValue {
    if require_filesystem(capabilities).is_err() {
        return XiaoValue::none();
    }
    let path = match unsafe { path_from_value(path) } {
        Ok(path) => path,
        Err(_) => return XiaoValue::none(),
    };
    match std::fs::metadata(Path::new(&path)) {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => {
            set_pending_runtime_error(path_error("打开文件", "目标不是文件"));
            return XiaoValue::none();
        }
        Err(error) => {
            set_pending_runtime_error(path_error("打开文件", error));
            return XiaoValue::none();
        }
    }
    let file = match File::open(Path::new(&path)) {
        Ok(file) => file,
        Err(error) => {
            set_pending_runtime_error(path_error("打开文件", error));
            return XiaoValue::none();
        }
    };
    let resource = match ExternalResourceHandle::from_file(file) {
        Ok(resource) => RuntimeValue::ExternalResource(resource),
        Err(error) => {
            set_pending_runtime_error(error);
            return XiaoValue::none();
        }
    };
    match runtime_to_value(&resource) {
        Ok(value) => value,
        Err(status) => {
            set_pending_runtime_error(RuntimeError::invalid_value(format!(
                "打开文件返回值构造失败（状态码 {status}）"
            )));
            XiaoValue::none()
        }
    }
}

/// 验证目录路径并返回目录资源值。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_resource_open_directory_with_capabilities(
    path: *const XiaoValue,
    capabilities: u64,
) -> XiaoValue {
    if require_filesystem(capabilities).is_err() {
        return XiaoValue::none();
    }
    let path = match unsafe { path_from_value(path) } {
        Ok(path) => path,
        Err(_) => return XiaoValue::none(),
    };
    match metadata(Path::new(&path)) {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => {
            set_pending_runtime_error(path_error("打开目录", "目标不是目录"));
            return XiaoValue::none();
        }
        Err(error) => {
            set_pending_runtime_error(path_error("打开目录", error));
            return XiaoValue::none();
        }
    };
    let resource = match ExternalResourceHandle::from_directory(path.into()) {
        Ok(resource) => RuntimeValue::ExternalResource(resource),
        Err(error) => {
            set_pending_runtime_error(error);
            return XiaoValue::none();
        }
    };
    match runtime_to_value(&resource) {
        Ok(value) => value,
        Err(status) => {
            set_pending_runtime_error(RuntimeError::invalid_value(format!(
                "打开目录返回值构造失败（状态码 {status}）"
            )));
            XiaoValue::none()
        }
    }
}

/// 关闭文件或目录资源；重复关闭沿用 Runtime 的幂等语义。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_resource_close_with_capabilities(
    value: *const XiaoValue,
    capabilities: u64,
) -> i32 {
    if let Err(status) = require_filesystem(capabilities) {
        return status;
    }
    if value.is_null() {
        return XiaoAbiStatus::Null.code();
    }
    let value = match unsafe { value_to_runtime(&*value) } {
        Ok(value) => value,
        Err(status) => return status,
    };
    let RuntimeValue::ExternalResource(resource) = value else {
        return status_from_error(&RuntimeError::type_mismatch("resource", value.type_name()));
    };
    // 目录资源的已验证路径属于承载状态；读取一次也让 ABI close 路径覆盖该载荷。
    let _ = resource.directory_path();
    match resource.close() {
        Ok(()) => XiaoAbiStatus::Ok.code(),
        Err(error) => status_from_error(&error),
    }
}

/// 读取文件的完整 UTF-8 文本。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_resource_read_text_with_capabilities(
    value: *const XiaoValue,
    capabilities: u64,
) -> XiaoValue {
    if require_filesystem(capabilities).is_err() {
        return XiaoValue::none();
    }
    if value.is_null() {
        set_pending_runtime_error(RuntimeError::invalid_value("read_text 的资源参数为空"));
        return XiaoValue::none();
    }
    let value = match unsafe { value_to_runtime(&*value) } {
        Ok(value) => value,
        Err(status) => {
            set_pending_runtime_error(RuntimeError::invalid_value(format!(
                "read_text 资源参数无效（状态码 {status}）"
            )));
            return XiaoValue::none();
        }
    };
    let RuntimeValue::ExternalResource(resource) = value else {
        set_pending_runtime_error(RuntimeError::type_mismatch("file", value.type_name()));
        return XiaoValue::none();
    };
    let text = match resource.read_text() {
        Ok(text) => text,
        Err(error) => {
            set_pending_runtime_error(error);
            return XiaoValue::none();
        }
    };
    let value = match RuntimeValue::new_string(text) {
        Ok(value) => value,
        Err(error) => {
            set_pending_runtime_error(error);
            return XiaoValue::none();
        }
    };
    match runtime_to_value(&value) {
        Ok(value) => value,
        Err(status) => {
            set_pending_runtime_error(RuntimeError::invalid_value(format!(
                "read_text 返回值构造失败（状态码 {status}）"
            )));
            XiaoValue::none()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;
    use xiao_intrinsics::KNOWN_CAPABILITY_MASK;
    use xiao_runtime_abi::{XiaoAbiErrorSnapshot, XiaoAbiStatus, XiaoValueTag};

    fn temporary_path(label: &str) -> PathBuf {
        let thread = std::thread::current()
            .name()
            .unwrap_or("test")
            .replace([':', '/'], "_");
        std::env::temp_dir().join(format!(
            "xiao-20d-{label}-{}-{}",
            std::process::id(),
            thread
        ))
    }

    fn path_value(path: &std::path::Path) -> XiaoValue {
        let value =
            RuntimeValue::new_string(path.to_string_lossy().to_string()).expect("路径字符串");
        runtime_to_value(&value).expect("路径值")
    }

    fn error_code() -> String {
        let mut snapshot = unsafe { std::mem::zeroed::<XiaoAbiErrorSnapshot>() };
        assert_eq!(
            super::super::xiao_runtime_error_snapshot(&mut snapshot),
            XiaoAbiStatus::Ok.code()
        );
        unsafe {
            String::from_utf8_lossy(std::slice::from_raw_parts(
                snapshot.code.ptr,
                snapshot.code.len,
            ))
            .into_owned()
        }
    }

    fn release(value: &mut XiaoValue) {
        super::super::xiao_runtime_value_release(value);
    }

    #[test]
    fn open_read_close_and_closed_alias_are_stable() {
        let path = temporary_path("read");
        fs::write(&path, "小雪\n").expect("写测试文件");
        let mut path_value = path_value(&path);
        let mut resource =
            xiao_runtime_resource_open_file_with_capabilities(&path_value, KNOWN_CAPABILITY_MASK);
        assert_eq!(resource.tag, XiaoValueTag::ExternalResource);
        let mut text =
            xiao_runtime_resource_read_text_with_capabilities(&resource, KNOWN_CAPABILITY_MASK);
        let runtime = unsafe { value_to_runtime(&text) }.expect("读取返回值");
        let RuntimeValue::Str(text_handle) = runtime else {
            panic!("read_text 应返回 str");
        };
        assert_eq!(text_handle.to_string().expect("文本"), "小雪\n");
        assert_eq!(
            xiao_runtime_resource_close_with_capabilities(&resource, KNOWN_CAPABILITY_MASK),
            XiaoAbiStatus::Ok.code()
        );
        assert_eq!(
            xiao_runtime_resource_close_with_capabilities(&resource, KNOWN_CAPABILITY_MASK),
            XiaoAbiStatus::Ok.code()
        );
        release(&mut text);
        let mut closed =
            xiao_runtime_resource_read_text_with_capabilities(&resource, KNOWN_CAPABILITY_MASK);
        assert_eq!(closed.tag, XiaoValueTag::None);
        assert_eq!(error_code(), xiao_diagnostics::RESOURCE_CLOSED_CODE);
        release(&mut closed);
        release(&mut resource);
        release(&mut path_value);
        let _ = fs::remove_file(path);
        super::super::xiao_runtime_error_clear();
    }

    #[test]
    fn capability_is_checked_before_open_and_io_errors_keep_reason() {
        let path = temporary_path("missing");
        let mut path_value = path_value(&path);
        let mut denied = xiao_runtime_resource_open_file_with_capabilities(&path_value, 0);
        assert_eq!(denied.tag, XiaoValueTag::None);
        assert_eq!(error_code(), xiao_diagnostics::CAPABILITY_UNAVAILABLE_CODE);
        release(&mut denied);
        super::super::xiao_runtime_error_clear();
        let mut missing =
            xiao_runtime_resource_open_file_with_capabilities(&path_value, KNOWN_CAPABILITY_MASK);
        assert_eq!(missing.tag, XiaoValueTag::None);
        assert_eq!(error_code(), xiao_diagnostics::INVALID_VALUE_CODE);
        let mut snapshot = unsafe { std::mem::zeroed::<XiaoAbiErrorSnapshot>() };
        super::super::xiao_runtime_error_snapshot(&mut snapshot);
        assert!(snapshot.param_count >= 1);
        release(&mut missing);
        release(&mut path_value);
        super::super::xiao_runtime_error_clear();
    }

    #[test]
    fn directory_open_validates_directory_and_non_utf8_is_rejected() {
        let directory = temporary_path("directory");
        fs::create_dir(&directory).expect("创建目录");
        let mut path = path_value(&directory);
        let mut resource =
            xiao_runtime_resource_open_directory_with_capabilities(&path, KNOWN_CAPABILITY_MASK);
        assert_eq!(resource.tag, XiaoValueTag::ExternalResource);
        assert_eq!(
            xiao_runtime_resource_close_with_capabilities(&resource, KNOWN_CAPABILITY_MASK),
            XiaoAbiStatus::Ok.code()
        );
        release(&mut resource);
        release(&mut path);
        let _ = fs::remove_dir(&directory);

        let file = temporary_path("binary");
        fs::write(&file, [0xff_u8, 0xfe_u8]).expect("写二进制测试文件");
        let mut file_path = path_value(&file);
        let mut file_resource =
            xiao_runtime_resource_open_file_with_capabilities(&file_path, KNOWN_CAPABILITY_MASK);
        let mut text = xiao_runtime_resource_read_text_with_capabilities(
            &file_resource,
            KNOWN_CAPABILITY_MASK,
        );
        assert_eq!(text.tag, XiaoValueTag::None);
        assert_eq!(error_code(), xiao_diagnostics::INVALID_VALUE_CODE);
        release(&mut text);
        release(&mut file_resource);
        release(&mut file_path);
        let _ = fs::remove_file(file);
        super::super::xiao_runtime_error_clear();
    }
}
