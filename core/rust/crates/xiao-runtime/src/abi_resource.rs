//! 首批 os 资源的 Runtime ABI；宿主语义统一委托 resources。

use xiao_runtime_abi::{XiaoAbiStatus, XiaoValue};

use super::{
    RuntimeError, RuntimeValue, runtime_to_value, set_pending_runtime_error, status_from_error,
    value_to_runtime,
};
use crate::{RuntimeResult, resources};

/// 借用 ABI 实参并转换为 Runtime 值；值返回入口不能静默吞掉无效指针/标签。
unsafe fn argument(value: *const XiaoValue) -> RuntimeResult<RuntimeValue> {
    if value.is_null() {
        return Err(RuntimeError::invalid_value("资源入口的参数指针为空"));
    }
    unsafe { value_to_runtime(&*value) }.map_err(|status| {
        RuntimeError::invalid_value(format!("资源入口的参数无效（状态码 {status}）"))
    })
}

/// 把共用操作的结果转换为拥有的 ABI 值，失败保留挂起错误。
fn result_value(result: RuntimeResult<RuntimeValue>) -> XiaoValue {
    match result {
        Ok(value) => match runtime_to_value(&value) {
            Ok(value) => value,
            Err(status) => {
                set_pending_runtime_error(RuntimeError::invalid_value(format!(
                    "资源返回值构造失败（状态码 {status}）"
                )));
                XiaoValue::none()
            }
        },
        Err(error) => {
            set_pending_runtime_error(error);
            XiaoValue::none()
        }
    }
}

/// 先查能力、再解码借用参数，最后调用共用 Runtime 操作。
fn resource_value_call(
    value: *const XiaoValue,
    capabilities: u64,
    operation: fn(&RuntimeValue, u64) -> RuntimeResult<RuntimeValue>,
) -> XiaoValue {
    resource_value_call_with_requirement(
        value,
        capabilities,
        resources::require_filesystem,
        operation,
    )
}

/// 按入口静态能力先校验，再解码参数；文件系统与进程使用同一 ABI 形状。
fn resource_value_call_with_requirement(
    value: *const XiaoValue,
    capabilities: u64,
    requirement: fn(u64) -> RuntimeResult<()>,
    operation: fn(&RuntimeValue, u64) -> RuntimeResult<RuntimeValue>,
) -> XiaoValue {
    result_value(requirement(capabilities).and_then(|()| {
        let value = unsafe { argument(value) }?;
        operation(&value, capabilities)
    }))
}

/// 打开文件并返回外部资源值；返回值由调用者释放。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_resource_open_file_with_capabilities(
    path: *const XiaoValue,
    capabilities: u64,
) -> XiaoValue {
    resource_value_call(path, capabilities, resources::open_file)
}

/// 验证目录并返回资源值；首版仅保存路径。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_resource_open_directory_with_capabilities(
    path: *const XiaoValue,
    capabilities: u64,
) -> XiaoValue {
    resource_value_call(path, capabilities, resources::open_directory)
}

/// 关闭借用资源；重复关闭成功，缺能力时不改变资源状态。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_resource_close_with_capabilities(
    value: *const XiaoValue,
    capabilities: u64,
) -> i32 {
    let result = resources::require_filesystem(capabilities).and_then(|()| {
        let value = unsafe { argument(value) }?;
        resources::close(&value, capabilities)
    });
    match result {
        Ok(()) => XiaoAbiStatus::Ok.code(),
        Err(error) => status_from_error(&error),
    }
}

/// 返回文件的完整 UTF-8 文本；失败返回 none 并设置错误槽。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_resource_read_text_with_capabilities(
    value: *const XiaoValue,
    capabilities: u64,
) -> XiaoValue {
    resource_value_call(value, capabilities, resources::read_text)
}

/// 启动子进程并返回拥有的统一资源值，参数仍以 XiaoValue 借用传递。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_process_spawn_with_capabilities(
    value: *const XiaoValue,
    capabilities: u64,
) -> XiaoValue {
    resource_value_call_with_requirement(
        value,
        capabilities,
        resources::require_process,
        resources::spawn,
    )
}

/// 返回缓存或阻塞取得的进程退出码；失败保留 ABI 错误快照。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_process_wait_with_capabilities(
    value: *const XiaoValue,
    capabilities: u64,
) -> XiaoValue {
    resource_value_call_with_requirement(
        value,
        capabilities,
        resources::require_process,
        resources::wait,
    )
}

/// 只释放进程句柄；缺能力时不改变共享关闭状态。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_process_close_with_capabilities(
    value: *const XiaoValue,
    capabilities: u64,
) -> i32 {
    let result = resources::require_process(capabilities).and_then(|()| {
        let value = unsafe { argument(value) }?;
        resources::close_process(&value, capabilities)
    });
    match result {
        Ok(()) => XiaoAbiStatus::Ok.code(),
        Err(error) => status_from_error(&error),
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
        let mut alias = XiaoValue::none();
        assert_eq!(
            super::super::xiao_runtime_value_copy(&resource, &mut alias),
            XiaoAbiStatus::Ok.code()
        );
        assert_eq!(
            xiao_runtime_resource_close_with_capabilities(&alias, 0),
            XiaoAbiStatus::RuntimeError.code()
        );
        assert_eq!(error_code(), xiao_diagnostics::CAPABILITY_UNAVAILABLE_CODE);
        super::super::xiao_runtime_error_clear();
        let mut text =
            xiao_runtime_resource_read_text_with_capabilities(&resource, KNOWN_CAPABILITY_MASK);
        let runtime = unsafe { value_to_runtime(&text) }.expect("读取返回值");
        let RuntimeValue::Str(text_handle) = runtime else {
            panic!("read_text 应返回 str");
        };
        assert_eq!(text_handle.to_string().expect("文本"), "小雪\n");
        assert_eq!(
            xiao_runtime_resource_close_with_capabilities(&alias, KNOWN_CAPABILITY_MASK),
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
        release(&mut alias);
        release(&mut resource);
        release(&mut path_value);
        let _ = fs::remove_file(path);
        super::super::xiao_runtime_error_clear();
    }

    #[test]
    /// 四条入口都先检查能力；值返回入口对无效参数必须留下可读取的错误。
    fn every_entry_checks_capabilities_before_decoding_arguments() {
        for operation in [
            xiao_runtime_resource_open_file_with_capabilities,
            xiao_runtime_resource_open_directory_with_capabilities,
            xiao_runtime_resource_read_text_with_capabilities,
        ] {
            let value = operation(std::ptr::null(), 0);
            assert_eq!(value.tag, XiaoValueTag::None);
            assert_eq!(error_code(), xiao_diagnostics::CAPABILITY_UNAVAILABLE_CODE);
            super::super::xiao_runtime_error_clear();
            let value = operation(std::ptr::null(), KNOWN_CAPABILITY_MASK);
            assert_eq!(value.tag, XiaoValueTag::None);
            assert_eq!(error_code(), xiao_diagnostics::INVALID_VALUE_CODE);
            super::super::xiao_runtime_error_clear();
        }
        assert_eq!(
            xiao_runtime_resource_close_with_capabilities(std::ptr::null(), 0),
            XiaoAbiStatus::RuntimeError.code()
        );
        assert_eq!(error_code(), xiao_diagnostics::CAPABILITY_UNAVAILABLE_CODE);
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

    #[test]
    /// 进程能力必须先于参数解码；filesystem 位不能替代 process 位。
    fn process_abi_checks_capabilities_before_arguments() {
        for operation in [
            xiao_runtime_process_spawn_with_capabilities,
            xiao_runtime_process_wait_with_capabilities,
        ] {
            let value = operation(std::ptr::null(), xiao_intrinsics::CAPABILITY_FILESYSTEM);
            assert_eq!(value.tag, XiaoValueTag::None);
            assert_eq!(error_code(), xiao_diagnostics::CAPABILITY_UNAVAILABLE_CODE);
            super::super::xiao_runtime_error_clear();
            let value = operation(std::ptr::null(), xiao_intrinsics::CAPABILITY_PROCESS);
            assert_eq!(value.tag, XiaoValueTag::None);
            assert_eq!(error_code(), xiao_diagnostics::INVALID_VALUE_CODE);
            super::super::xiao_runtime_error_clear();
        }
        assert_eq!(
            xiao_runtime_process_close_with_capabilities(std::ptr::null(), 0),
            XiaoAbiStatus::RuntimeError.code()
        );
        assert_eq!(error_code(), xiao_diagnostics::CAPABILITY_UNAVAILABLE_CODE);
        super::super::xiao_runtime_error_clear();
    }

    #[test]
    /// 标签 17 可往返进程种类；错误关闭入口不能改变进程状态。
    fn process_abi_roundtrip_and_close_preserve_capability_and_alias_state() {
        let handle = crate::ExternalResourceHandle::new(crate::ExternalResourceKind::Process)
            .expect("进程承载");
        let mut value =
            runtime_to_value(&RuntimeValue::ExternalResource(handle.clone())).expect("ABI 值");
        assert_eq!(value.tag, XiaoValueTag::ExternalResource);
        let roundtrip = unsafe { value_to_runtime(&value) }.expect("ABI 往返");
        assert_eq!(roundtrip.type_name(), "resource<process>");
        assert_eq!(
            xiao_runtime_resource_close_with_capabilities(
                &value,
                xiao_intrinsics::CAPABILITY_FILESYSTEM
            ),
            XiaoAbiStatus::RuntimeError.code()
        );
        assert!(!handle.is_closed().unwrap());
        super::super::xiao_runtime_error_clear();
        assert_eq!(
            xiao_runtime_process_close_with_capabilities(
                &value,
                xiao_intrinsics::CAPABILITY_FILESYSTEM
            ),
            XiaoAbiStatus::RuntimeError.code()
        );
        assert_eq!(error_code(), xiao_diagnostics::CAPABILITY_UNAVAILABLE_CODE);
        assert!(!handle.is_closed().unwrap());
        super::super::xiao_runtime_error_clear();
        for _ in 0..2 {
            assert_eq!(
                xiao_runtime_process_close_with_capabilities(
                    &value,
                    xiao_intrinsics::CAPABILITY_PROCESS
                ),
                XiaoAbiStatus::Ok.code()
            );
        }
        assert!(handle.is_closed().unwrap());
        let closed = xiao_runtime_process_wait_with_capabilities(
            &value,
            xiao_intrinsics::CAPABILITY_PROCESS,
        );
        assert_eq!(closed.tag, XiaoValueTag::None);
        assert_eq!(error_code(), xiao_diagnostics::RESOURCE_CLOSED_CODE);
        release(&mut value);
        super::super::xiao_runtime_error_clear();
    }
}
