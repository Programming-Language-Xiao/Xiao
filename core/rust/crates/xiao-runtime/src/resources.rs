//! VM 与原生 ABI 共用的文件系统及进程操作；所有宿主访问都先验证能力。

use std::fs::{File, metadata};

use xiao_intrinsics::{Capability, validate_capability_mask};

use crate::{
    DiagnosticParam, ExternalResourceHandle, ExternalResourceKind, RuntimeError, RuntimeResult,
    RuntimeValue,
};

/// 在读取宿主状态或关闭句柄前验证冻结的 filesystem 能力位。
pub(crate) fn require_filesystem(capabilities: u64) -> RuntimeResult<()> {
    require_capability(capabilities, Capability::Filesystem)
}

/// 在创建、等待或释放子进程之前验证 process 位。
pub(crate) fn require_process(capabilities: u64) -> RuntimeResult<()> {
    require_capability(capabilities, Capability::Process)
}

/// 按静态契约验证冻结的能力位，不执行任何宿主操作。
fn require_capability(capabilities: u64, capability: Capability) -> RuntimeResult<()> {
    if validate_capability_mask(capabilities).is_err() {
        return Err(RuntimeError::invalid_value("能力掩码包含未知位"));
    }
    if capability.bit().is_some_and(|bit| capabilities & bit == 0) {
        return Err(RuntimeError::capability_unavailable(capability.as_str()));
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
        RuntimeValue::ExternalResource(resource)
            if matches!(
                resource.kind()?,
                ExternalResourceKind::File | ExternalResourceKind::Directory
            ) =>
        {
            resource.close()
        }
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

/// 无 shell、无参数地启动程序，继承 stdio、环境和 cwd，由宿主解析 PATH。
pub fn spawn(value: &RuntimeValue, capabilities: u64) -> RuntimeResult<RuntimeValue> {
    require_process(capabilities)?;
    let program = path(value)?;
    #[cfg(windows)]
    reject_implicit_shell(&program)?;
    let mut command = std::process::Command::new(&program);
    #[cfg(unix)]
    configure_unix_no_shell(&mut command, &program)
        .map_err(|error| io_error("启动子进程", error))?;
    let child = command
        .spawn()
        .map_err(|error| io_error("启动子进程", error))?;
    ExternalResourceHandle::from_process(child).map(RuntimeValue::ExternalResource)
}

#[cfg(unix)]
/// execvp/部分 posix_spawnp 会在 ENOEXEC 时启动 shell；直接 execv 保留继承与 PATH，
/// 并让 Command 的错误管道把实际执行失败返回父进程。
fn configure_unix_no_shell(
    command: &mut std::process::Command,
    program: &str,
) -> std::io::Result<()> {
    use std::ffi::{CString, OsString};
    use std::os::unix::{ffi::OsStringExt, process::CommandExt};

    let argv0 = CString::new(program)?;
    // 所有字符串和搜索候选在 fork 前准备；子进程钩子只调用 async-signal-safe execv。
    let candidates = if program.contains('/') || program.is_empty() {
        vec![argv0.clone()]
    } else {
        let search_path = if let Some(path) = std::env::var_os("PATH") {
            path
        } else {
            // PATH 未设置时使用宿主的标准搜索路径。
            // SAFETY: 空指针与零长度仅查询所需空间；第二次调用写入完整缓冲区。
            let length = unsafe { libc::confstr(libc::_CS_PATH, std::ptr::null_mut(), 0) };
            if length == 0 {
                return Err(std::io::Error::last_os_error());
            }
            let mut path = vec![0_u8; length];
            let actual = unsafe { libc::confstr(libc::_CS_PATH, path.as_mut_ptr().cast(), length) };
            if actual == 0 || actual > length {
                return Err(std::io::Error::last_os_error());
            }
            path.truncate(actual - 1);
            OsString::from_vec(path)
        };
        std::env::split_paths(&search_path)
            .map(|directory| CString::new(directory.join(program).into_os_string().into_vec()))
            .collect::<Result<Vec<_>, _>>()?
    };
    // SAFETY: 钩子不分配、不获取锁、不调用用户代码；指针均来自捕获的有效 CString。
    // execv 成功不返回，失败后仅构造无分配的 OS 错误并交给 Command 退出子进程。
    unsafe {
        command.pre_exec(move || {
            let arguments = [argv0.as_ptr(), std::ptr::null()];
            let mut permission_denied = false;
            for candidate in &candidates {
                libc::execv(candidate.as_ptr(), arguments.as_ptr());
                let error = std::io::Error::last_os_error();
                match error.raw_os_error() {
                    Some(libc::ENOENT | libc::ENOTDIR) => {}
                    Some(libc::EACCES) => permission_denied = true,
                    _ => return Err(error),
                }
            }
            Err(std::io::Error::from_raw_os_error(if permission_denied {
                libc::EACCES
            } else {
                libc::ENOENT
            }))
        });
    }
    Ok(())
}

#[cfg(windows)]
/// Windows 的 Command 会为批处理隐式启动 cmd；在交给宿主前按无 shell 契约拒绝。
fn reject_implicit_shell(program: &str) -> RuntimeResult<()> {
    let trimmed = program.trim_end_matches([' ', '.']);
    let extension = std::path::Path::new(trimmed)
        .extension()
        .and_then(|value| value.to_str());
    if extension.is_some_and(|extension| {
        extension.eq_ignore_ascii_case("cmd") || extension.eq_ignore_ascii_case("bat")
    }) {
        return Err(io_error(
            "启动子进程",
            "不允许通过 shell 隐式执行批处理文件",
        ));
    }
    Ok(())
}

/// 阻塞等待进程结束，重复调用返回缓存；缺能力时不等待或读取载荷。
pub fn wait(value: &RuntimeValue, capabilities: u64) -> RuntimeResult<RuntimeValue> {
    require_process(capabilities)?;
    match value {
        RuntimeValue::ExternalResource(resource)
            if resource.kind()? == ExternalResourceKind::Process =>
        {
            resource.wait_process().map(RuntimeValue::Int)
        }
        _ => Err(RuntimeError::type_mismatch("process", value.type_name())),
    }
}

/// 幂等释放进程句柄，不终止、不阻塞、不回收；未等待子进程可在 Unix 上留下僵尸。
pub fn close_process(value: &RuntimeValue, capabilities: u64) -> RuntimeResult<()> {
    require_process(capabilities)?;
    match value {
        RuntimeValue::ExternalResource(resource)
            if resource.kind()? == ExternalResourceKind::Process =>
        {
            resource.close()
        }
        _ => Err(RuntimeError::type_mismatch("process", value.type_name())),
    }
}

/// 只读取 Runtime 种类，不触碰宿主或关闭状态；资源参数集合按成员检查。
pub fn resource_type_matches(value: &RuntimeValue, check: &str, expected: &str) -> bool {
    let member_matches = |value: &RuntimeValue| match value {
        RuntimeValue::ExternalResource(resource) => resource.kind().is_ok_and(|actual| {
            actual.as_str() == expected
                || expected == "resource"
                    && matches!(
                        actual,
                        ExternalResourceKind::File | ExternalResourceKind::Directory
                    )
        }),
        _ => false,
    };
    match check {
        "resource_type" => member_matches(value),
        "resource_array" => match value {
            RuntimeValue::Array(array) => array
                .with_elements(|values| values.iter().all(member_matches))
                .unwrap_or(false),
            _ => false,
        },
        "resource_dictionary" => match value {
            RuntimeValue::DictTable(dictionary) | RuntimeValue::DictColumn(dictionary) => {
                dictionary
                    .with_entries(|entries| entries.iter().all(|(_, value)| member_matches(value)))
                    .unwrap_or(false)
            }
            _ => false,
        },
        _ => false,
    }
}
