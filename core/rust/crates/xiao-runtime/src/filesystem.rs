//! std.fs 的共享参数、资源和提交边界。

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant, UNIX_EPOCH};

use crate::{
    ALLOCATION_CODE, ArrayHandle, DiagnosticParam, DictHandle, DictKind, RuntimeError,
    RuntimeErrorKind, RuntimeResult, RuntimeValue, StringHandle, resources, time::TimeUnit,
};

#[path = "filesystem_host.rs"]
mod host;

const PATH_BYTES: usize = 4_096;
const TEXT_BYTES: usize = 8 * 1_024 * 1_024;
const DIR_NAMES: usize = 1_024;
const DIR_BYTES: usize = 65_536;
const OPERATION_LIMIT: Duration = Duration::from_secs(10);
const WRITE_CHUNK: usize = 64 * 1_024;

/// 冻结的路径操作，由调用方先按形参顺序绑定实参。
#[derive(Clone, Copy)]
pub enum Operation {
    /// 在目标目录完成文本暂存，再原子提交。
    WriteText,
    /// 枚举直接子项。
    ListDir,
    /// 非递归删除单个目录项。
    Remove,
    /// 同卷移动单个目录项。
    Rename,
    /// 读取末段本身的元数据。
    Metadata,
}

/// 同时保留可捕获 Runtime 错误和调用方的运行控制通道。
#[derive(Debug)]
pub enum Failure<C> {
    /// 文件系统或值错误。
    Runtime(RuntimeError),
    /// 取消或运行截止。
    Interrupted {
        /// 原有运行控制身份。
        control: C,
        /// 取消之后清理暂存失败的附带原因。
        cleanup: Option<RuntimeError>,
    },
}

impl<C> From<RuntimeError> for Failure<C> {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

type Result<T, C> = std::result::Result<T, Failure<C>>;

/// 检查所有语言类型、授权、路径参数及输入预算，再进入可中断宿主操作。
pub fn execute<C>(
    operation: Operation,
    arguments: &[&RuntimeValue],
    capabilities: u64,
    checkpoint: impl FnMut() -> std::result::Result<(), C>,
) -> Result<RuntimeValue, C> {
    match (operation, arguments) {
        (Operation::WriteText | Operation::Rename, [first, second, overwrite]) => {
            let first = text_argument(first)?;
            let second = text_argument(second)?;
            let RuntimeValue::Bool(overwrite) = overwrite else {
                return Err(RuntimeError::type_mismatch("bool", overwrite.type_name()).into());
            };
            resources::require_filesystem(capabilities)?;
            first.with_str(|first| {
                second.with_str(|second| {
                    check_path_syntax(first)?;
                    if matches!(operation, Operation::Rename) {
                        check_path_syntax(second)?;
                    }
                    check_bytes("路径", first.len(), PATH_BYTES)?;
                    if matches!(operation, Operation::WriteText) {
                        check_bytes("文本", second.len(), TEXT_BYTES)?;
                        let mut control = Control::new(checkpoint);
                        write_text(first, second, *overwrite, &mut control)?;
                    } else {
                        check_bytes("路径", second.len(), PATH_BYTES)?;
                        let mut control = Control::new(checkpoint);
                        control.check()?;
                        host::rename(first.as_ref(), second.as_ref(), *overwrite)
                            .map_err(|error| rename_error("重命名", error, *overwrite))?;
                    }
                    Ok(RuntimeValue::None)
                })?
            })?
        }
        (Operation::ListDir | Operation::Remove | Operation::Metadata, [path]) => {
            let path = text_argument(path)?;
            resources::require_filesystem(capabilities)?;
            path.with_str(|path| {
                check_path_syntax(path)?;
                check_bytes("路径", path.len(), PATH_BYTES)?;
                let mut control = Control::new(checkpoint);
                let path = Path::new(path);
                match operation {
                    Operation::ListDir => list_dir(path, &mut control),
                    Operation::Remove => {
                        remove(path, &mut control)?;
                        Ok(RuntimeValue::None)
                    }
                    Operation::Metadata => metadata(path, &mut control),
                    _ => unreachable!("一元 fs 操作"),
                }
            })?
        }
        _ => Err(RuntimeError::type_mismatch("fs 必填形参", "调用形态").into()),
    }
}

fn text_argument(value: &RuntimeValue) -> RuntimeResult<&StringHandle> {
    match value {
        RuntimeValue::Str(text) => Ok(text),
        _ => Err(RuntimeError::type_mismatch("str", value.type_name())),
    }
}

fn check_path_syntax(path: &str) -> RuntimeResult<()> {
    if path.is_empty() || path.contains('\0') {
        return Err(RuntimeError::invalid_value("路径为空或含 NUL"));
    }
    Ok(())
}

fn check_bytes(label: &str, bytes: usize, max: usize) -> RuntimeResult<()> {
    if bytes > max {
        return Err(limit(label, max));
    }
    Ok(())
}

fn limit(label: &str, max: usize) -> RuntimeError {
    resource_error(
        "FS_LIMIT_EXCEEDED",
        "runtime.fs_limit_exceeded",
        "文件系统操作超出限额",
    )
    .with_param("resource", DiagnosticParam::Text(label.to_owned()))
    .with_param("limit", DiagnosticParam::Integer(max as i128))
}

fn allocation_error() -> RuntimeError {
    resource_error(
        ALLOCATION_CODE,
        "runtime.allocation",
        "无法分配文件系统操作结果",
    )
}

fn resource_error(code: &str, message_id: &str, message: &str) -> RuntimeError {
    RuntimeError::new(RuntimeErrorKind::Resource, code, message_id, message)
}

fn unsupported(operation: &str) -> RuntimeError {
    resource_error(
        "FS_UNSUPPORTED",
        "runtime.fs_unsupported",
        "宿主无法保证所请求的文件系统语义",
    )
    .with_param("operation", DiagnosticParam::Text(operation.to_owned()))
}

fn io_error(operation: &str, error: io::Error) -> RuntimeError {
    let mut report = resource_error(
        "FS_IO_FAILED",
        "runtime.fs_io_failed",
        "宿主文件系统操作失败",
    )
    .with_param("operation", DiagnosticParam::Text(operation.to_owned()))
    .with_param("reason", DiagnosticParam::Text(error.to_string()));
    if let Some(code) = error.raw_os_error() {
        report = report.with_param("os_error", DiagnosticParam::Integer(i128::from(code)));
    }
    report
}

fn rename_error(operation: &str, error: io::Error, overwrite: bool) -> RuntimeError {
    match error.kind() {
        io::ErrorKind::AlreadyExists if !overwrite => {
            resource_error("FS_EXISTS", "runtime.fs_exists", "目标目录项已存在")
        }
        _ if host::cannot_rename_atomically(&error, overwrite) => {
            unsupported(operation).with_cause(io_error(operation, error))
        }
        _ => io_error(operation, error),
    }
}

struct Control<F> {
    started: Instant,
    checkpoint: F,
}

impl<C, F: FnMut() -> std::result::Result<(), C>> Control<F> {
    fn new(checkpoint: F) -> Self {
        Self {
            started: Instant::now(),
            checkpoint,
        }
    }

    fn check(&mut self) -> Result<(), C> {
        (self.checkpoint)().map_err(|control| Failure::Interrupted {
            control,
            cleanup: None,
        })?;
        if self.started.elapsed() >= OPERATION_LIMIT {
            return Err(resource_error(
                "FS_TIMEOUT",
                "runtime.fs_timeout",
                "文件系统操作达到十秒期限",
            )
            .into());
        }
        Ok(())
    }

    /// 可逆操作在返回后再次检查。进入目录项提交后直接报告宿主结果。
    fn read<T>(&mut self, label: &str, call: impl FnOnce() -> io::Result<T>) -> Result<T, C> {
        self.check()?;
        let result = call();
        self.check()?;
        result.map_err(|error| io_error(label, error).into())
    }
}

fn write_text<C>(
    path: &str,
    text: &str,
    overwrite: bool,
    control: &mut Control<impl FnMut() -> std::result::Result<(), C>>,
) -> Result<(), C> {
    control.check()?;
    let destination = Path::new(path);
    let parent = destination
        .parent()
        .ok_or_else(|| unsupported("写入文本"))?;
    let (temporary, mut file) = stage_text(parent, destination, control)?;
    let written = text
        .as_bytes()
        .chunks(WRITE_CHUNK)
        .try_for_each(|chunk| control.read("写入暂存文件", || file.write_all(chunk)));
    drop(file);
    let committed = written.and_then(|()| {
        control.check()?;
        host::rename(&temporary, destination, overwrite).map_err(|error| {
            if overwrite && error.kind() == io::ErrorKind::IsADirectory {
                unsupported("以文本替换目录")
                    .with_cause(io_error("原子提交", error))
                    .into()
            } else {
                rename_error("原子提交", error, overwrite).into()
            }
        })
    });
    match committed {
        Ok(()) => Ok(()),
        Err(mut failure) => {
            if let Err(error) = fs::remove_file(&temporary) {
                let cleanup_error = io_error("清理暂存文件", error);
                match &mut failure {
                    Failure::Runtime(error) => error.push_suppressed(cleanup_error),
                    Failure::Interrupted { cleanup, .. } => *cleanup = Some(cleanup_error),
                }
            }
            Err(failure)
        }
    }
}

fn stage_text<C>(
    directory: &Path,
    destination: &Path,
    control: &mut Control<impl FnMut() -> std::result::Result<(), C>>,
) -> Result<(PathBuf, File), C> {
    // 短文件名也适用于启用 8.3 别名的 Windows 文件系统。
    static SEQUENCE: AtomicU32 = AtomicU32::new(0);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);

    loop {
        control.check()?;
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name = format!("XF{:06X}.$$$", (sequence ^ std::process::id()) & 0xff_ffff);
        if host::matches_target_name(destination, &name) {
            continue;
        }
        let path = directory.join(name);
        match options.open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                control.check()?;
                return Err(io_error("创建暂存文件", error).into());
            }
        }
    }
}

fn list_dir<C>(
    path: &Path,
    control: &mut Control<impl FnMut() -> std::result::Result<(), C>>,
) -> Result<RuntimeValue, C> {
    let mut children = control.read("打开目录枚举", || fs::read_dir(path))?;
    let mut names = Vec::new();
    let mut bytes = 0;
    while let Some(entry) = control.read("读取目录项", || children.next().transpose())? {
        if names.len() == DIR_NAMES {
            return Err(limit("枚举项", DIR_NAMES).into());
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| RuntimeError::invalid_value("目录项名称含无效 Unicode"))?;
        check_bytes("枚举名称", name.len(), PATH_BYTES)?;
        bytes += name.len();
        check_bytes("枚举名称合计", bytes, DIR_BYTES)?;
        names.try_reserve(1).map_err(|_| allocation_error())?;
        names.push(name);
    }
    names.sort_unstable();
    let mut values = Vec::new();
    values
        .try_reserve_exact(names.len())
        .map_err(|_| allocation_error())?;
    for name in names {
        control.check()?;
        values.push(RuntimeValue::new_string(name)?);
    }
    let result = ArrayHandle::new(values)?;
    control.check()?;
    Ok(RuntimeValue::Array(result))
}

/// 末段链接即使附带尾分隔符也按链接处理。其他目录项保留宿主解析规则。
fn entry_metadata<'a, C>(
    path: &'a Path,
    control: &mut Control<impl FnMut() -> std::result::Result<(), C>>,
) -> Result<(&'a Path, fs::Metadata), C> {
    if let Some(raw) = path.to_str() {
        let trimmed = Path::new(raw.trim_end_matches(std::path::is_separator));
        if trimmed.as_os_str() != path.as_os_str() && path.file_name().is_some() {
            let entry =
                control.read("读取目录项元数据", || fs::symlink_metadata(trimmed))?;
            if entry.is_symlink() {
                return Ok((trimmed, entry));
            }
        }
    }
    control
        .read("读取目录项元数据", || fs::symlink_metadata(path))
        .map(|entry| (path, entry))
}

fn remove<C>(
    path: &Path,
    control: &mut Control<impl FnMut() -> std::result::Result<(), C>>,
) -> Result<(), C> {
    let (path, entry) = entry_metadata(path, control)?;
    if !(entry.is_file() || entry.is_dir() || entry.is_symlink()) {
        return Err(unsupported("删除特殊目录项").into());
    }
    control.check()?;
    host::remove(path, entry.file_type()).map_err(|error| io_error("删除目录项", error).into())
}

fn metadata<C>(
    path: &Path,
    control: &mut Control<impl FnMut() -> std::result::Result<(), C>>,
) -> Result<RuntimeValue, C> {
    let (_, entry) = entry_metadata(path, control)?;
    let (kind, size) = if entry.is_file() {
        let size = i64::try_from(entry.len())
            .map_err(|_| RuntimeError::numeric_overflow("文件尺寸超出 int 范围"))?;
        (1, size)
    } else if entry.is_dir() {
        (2, -1)
    } else if entry.is_symlink() {
        (3, -1)
    } else {
        (4, -1)
    };
    let modified = entry
        .modified()
        .map_err(|error| io_error("读取修改时间", error))?;
    let nanos = match modified.duration_since(UNIX_EPOCH) {
        Ok(after) => after.as_nanos() as i128,
        Err(before) => -(before.duration().as_nanos() as i128),
    };
    let modified_ms = TimeUnit::Milliseconds.convert(nanos)?;
    let mut entries = Vec::new();
    entries
        .try_reserve_exact(3)
        .map_err(|_| allocation_error())?;
    for (field, value) in [("kind", kind), ("size", size), ("modified_ms", modified_ms)] {
        let mut key = String::new();
        key.try_reserve_exact(field.len())
            .map_err(|_| allocation_error())?;
        key.push_str(field);
        entries.push((key, RuntimeValue::Int(value)));
    }
    let result = DictHandle::new(DictKind::Table, entries)?;
    control.check()?;
    Ok(RuntimeValue::DictTable(result))
}

#[cfg(test)]
#[path = "filesystem_tests.rs"]
mod tests;
