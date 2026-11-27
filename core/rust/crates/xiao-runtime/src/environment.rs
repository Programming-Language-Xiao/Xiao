//! 运行入口的环境接纳与单次运行内的局部读写。

use std::cmp::Ordering;
use std::collections::TryReserveError;
use std::ffi::OsString;
use std::fmt;
use std::sync::Mutex;

use xiao_intrinsics::{
    CAPABILITY_ENVIRONMENT, Capability, CapabilityMaskError, validate_capability_mask,
};

use crate::{
    ALLOCATION_CODE, ArrayHandle, INVALID_VALUE_CODE, RuntimeError, RuntimeErrorKind,
    RuntimeResult, RuntimeValue, StringHandle, resources,
};

const MAX_PAIRS: usize = 1_024;
const MAX_NAME_BYTES: usize = 256;
const MAX_VALUE_BYTES: usize = 65_536;
const MAX_TOTAL_BYTES: usize = 1_048_576;

/// 接纳失败，仅携带错误身份，不包含环境内容。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotError {
    /// 原生输入不能无损解码成 Unicode。
    InvalidEncoding,
    /// 名称为空或含等号或 NUL。
    InvalidName,
    /// 值含 NUL。
    InvalidValue,
    /// 已有按当前平台规则等价的名称。
    DuplicateName,
    /// 条目数、字段长度或总字节数超限。
    LimitExceeded,
    /// 无法为快照分配缓冲区。
    AllocationFailed,
    /// 读取来源前检测到未知的能力位。
    InvalidCapabilities(CapabilityMaskError),
}

impl SnapshotError {
    /// 返回接纳失败可用的稳定诊断编号。
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidEncoding
            | Self::InvalidName
            | Self::InvalidValue
            | Self::DuplicateName => "X09-DRIVER-006",
            Self::LimitExceeded => "X09-DRIVER-007",
            Self::AllocationFailed => ALLOCATION_CODE,
            Self::InvalidCapabilities(_) => INVALID_VALUE_CODE,
        }
    }
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidEncoding => "环境快照含无效的宿主编码",
            Self::InvalidName => "环境名称为空或含等号或 NUL",
            Self::InvalidValue => "环境值含 NUL",
            Self::DuplicateName => "环境快照含重复名称",
            Self::LimitExceeded => "环境快照超过接纳上限",
            Self::AllocationFailed => "无法分配环境快照",
            Self::InvalidCapabilities(error) => return fmt::Display::fmt(error, formatter),
        })
    }
}

impl std::error::Error for SnapshotError {}

/// 单次运行拥有的环境视图，同次运行的调用方可以通过 `Arc` 共享。
#[derive(Default)]
pub struct EnvironmentSnapshot {
    view: Mutex<View>,
}

impl fmt::Debug for EnvironmentSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EnvironmentSnapshot")
            .finish_non_exhaustive()
    }
}

#[derive(Default)]
struct View {
    /// 按平台名称比较顺序排列。
    entries: Vec<(String, String)>,
    total_bytes: usize,
}

impl View {
    /// 查找等价名称，或返回插入位置。
    fn position(&self, name: &str) -> Result<usize, usize> {
        self.entries
            .binary_search_by(|(existing, _)| compare_names(existing, name))
    }

    /// 按提交后的条目数和逻辑大小计算总字节数。
    fn updated_bytes(&self, old_index: Option<usize>, name: &str, value: &str) -> Option<usize> {
        if old_index.is_none() && self.entries.len() == MAX_PAIRS {
            return None;
        }
        let old_bytes = old_index.map_or(0, |index| {
            let (old_name, old_value) = &self.entries[index];
            old_name.len() + old_value.len()
        });
        let total = self.total_bytes - old_bytes + name.len() + value.len();
        (total <= MAX_TOTAL_BYTES).then_some(total)
    }

    /// 为写入准备拥有的名称和值，在插入前预留条目空间。
    fn prepare_entry(
        &mut self,
        name: &str,
        value: &str,
        inserting: bool,
    ) -> Result<(String, String), TryReserveError> {
        let entry = (copy_text(name)?, copy_text(value)?);
        if inserting {
            self.entries.try_reserve(1)?;
        }
        Ok(entry)
    }
}

/// 已按形参顺序绑定实参的环境操作。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Operation {
    /// 返回拥有的零项或一项字符串数组。
    Get,
    /// 插入或替换当前运行内的条目。
    Set,
    /// 删除当前运行内的条目并返回是否存在。
    Unset,
}

/// 保留可捕获的运行错误和调用方已有的运行控制通道。
#[derive(Debug)]
pub enum Failure<C> {
    /// 语言可捕获的运行错误。
    Runtime(RuntimeError),
    /// 运行取消、截止或任务取消。
    Interrupted(C),
}

impl<C> From<RuntimeError> for Failure<C> {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl EnvironmentSnapshot {
    /// 接纳显式提供的有序 UTF-8 名值对，任一条目失败时整份拒绝。
    pub fn from_pairs(pairs: &[(String, String)]) -> Result<Self, SnapshotError> {
        Self::admit(pairs.iter().map(|(name, value)| Ok((name, value))))
    }

    /// 先校验能力，再最多调用一次原生数据来源。缺少环境能力时返回空视图。
    ///
    /// 启动适配层提供已过滤 Windows 驱动器 cwd 伪变量的原生名值对。
    pub fn from_host<I>(capabilities: u64, read: impl FnOnce() -> I) -> Result<Self, SnapshotError>
    where
        I: IntoIterator<Item = (OsString, OsString)>,
    {
        validate_capability_mask(capabilities).map_err(SnapshotError::InvalidCapabilities)?;
        if capabilities & CAPABILITY_ENVIRONMENT == 0 {
            return Ok(Self::default());
        }
        Self::admit(read().into_iter().map(|(name, value)| {
            let name = name
                .into_string()
                .map_err(|_| SnapshotError::InvalidEncoding)?;
            let value = value
                .into_string()
                .map_err(|_| SnapshotError::InvalidEncoding)?;
            Ok((name, value))
        }))
    }

    /// 复用显式与原生来源的排序、去重和预算口径。
    fn admit<S: AsRef<str>>(
        pairs: impl IntoIterator<Item = Result<(S, S), SnapshotError>>,
    ) -> Result<Self, SnapshotError> {
        let mut view = View::default();
        for pair in pairs {
            let (name, value) = pair?;
            let (name, value) = (name.as_ref(), value.as_ref());
            check_name(name)?;
            check_value(value)?;
            if !pair_fits(name, value) {
                return Err(SnapshotError::LimitExceeded);
            }
            let index = match view.position(name) {
                Ok(_) => return Err(SnapshotError::DuplicateName),
                Err(index) => index,
            };
            let bytes = view
                .updated_bytes(None, name, value)
                .ok_or(SnapshotError::LimitExceeded)?;
            let entry = view
                .prepare_entry(name, value, true)
                .map_err(|_| SnapshotError::AllocationFailed)?;
            view.entries.insert(index, entry);
            view.total_bytes = bytes;
        }
        Ok(Self {
            view: Mutex::new(view),
        })
    }

    /// 按类型、授权、参数、预算和运行控制顺序执行一次操作。
    ///
    /// 安全点回调在持有视图锁时执行，只用于检查控制状态。
    pub fn execute<C>(
        &self,
        operation: Operation,
        arguments: &[&RuntimeValue],
        capabilities: u64,
        checkpoint: impl FnOnce() -> Result<(), C>,
    ) -> Result<RuntimeValue, Failure<C>> {
        match (operation, arguments) {
            (Operation::Set, [name, value]) => {
                let name = text_argument(name)?;
                let value = text_argument(value)?;
                resources::require_capability(capabilities, Capability::Environment)?;
                name.with_str(|name| value.with_str(|value| self.set(name, value, checkpoint)))??
            }
            (Operation::Get | Operation::Unset, [name]) => {
                let name = text_argument(name)?;
                resources::require_capability(capabilities, Capability::Environment)?;
                name.with_str(|name| self.read_or_unset(operation, name, checkpoint))?
            }
            _ => Err(RuntimeError::type_mismatch("env 必填形参", "调用形态").into()),
        }
    }

    /// 在同一临界区核算、准备，然后提交新条目。
    fn set<C>(
        &self,
        name: &str,
        value: &str,
        checkpoint: impl FnOnce() -> Result<(), C>,
    ) -> Result<RuntimeValue, Failure<C>> {
        check_name(name).map_err(invalid_value)?;
        check_value(value).map_err(invalid_value)?;
        if !pair_fits(name, value) {
            return Err(limit_error().into());
        }
        let mut view = self.view.lock().expect("环境视图锁未中毒");
        let position = view.position(name);
        let bytes = view
            .updated_bytes(position.ok(), name, value)
            .ok_or_else(limit_error)?;
        let entry = view
            .prepare_entry(name, value, position.is_err())
            .map_err(allocation_error)?;
        checkpoint().map_err(Failure::Interrupted)?;
        match position {
            Ok(index) => view.entries[index] = entry,
            Err(index) => view.entries.insert(index, entry),
        }
        view.total_bytes = bytes;
        Ok(RuntimeValue::None)
    }

    /// 在临界区检查控制状态后读取或删除一次。
    fn read_or_unset<C>(
        &self,
        operation: Operation,
        name: &str,
        checkpoint: impl FnOnce() -> Result<(), C>,
    ) -> Result<RuntimeValue, Failure<C>> {
        check_name(name).map_err(invalid_value)?;
        if name.len() > MAX_NAME_BYTES {
            return Err(RuntimeError::invalid_value("环境名称超过 256 UTF-8 字节").into());
        }
        let mut view = self.view.lock().expect("环境视图锁未中毒");
        let index = view.position(name).ok();
        checkpoint().map_err(Failure::Interrupted)?;
        if operation == Operation::Unset {
            if let Some(index) = index {
                let (name, value) = view.entries.remove(index);
                view.total_bytes -= name.len() + value.len();
            }
            return Ok(RuntimeValue::Bool(index.is_some()));
        }
        let result = index
            .map(|index| copy_text(&view.entries[index].1))
            .transpose()
            .map_err(allocation_error)?;
        drop(view);
        let mut values = Vec::new();
        if let Some(value) = result {
            values.try_reserve_exact(1).map_err(allocation_error)?;
            values.push(RuntimeValue::Str(StringHandle::new(value)?));
        }
        Ok(RuntimeValue::Array(ArrayHandle::new(values)?))
    }
}

/// 借读已有字符串，先完成类型检查。
fn text_argument(value: &RuntimeValue) -> RuntimeResult<&StringHandle> {
    match value {
        RuntimeValue::Str(text) => Ok(text),
        _ => Err(RuntimeError::type_mismatch("str", value.type_name())),
    }
}

/// 快照和运行操作共用的名称语法。
fn check_name(name: &str) -> Result<(), SnapshotError> {
    if name.is_empty() || name.contains(['=', '\0']) {
        return Err(SnapshotError::InvalidName);
    }
    Ok(())
}

/// 值允许空串和等号。
fn check_value(value: &str) -> Result<(), SnapshotError> {
    if value.contains('\0') {
        return Err(SnapshotError::InvalidValue);
    }
    Ok(())
}

/// 单项预算包含上限，按原始 UTF-8 字节计数。
fn pair_fits(name: &str, value: &str) -> bool {
    name.len() <= MAX_NAME_BYTES && value.len() <= MAX_VALUE_BYTES
}

/// 准备新的字符串缓冲区，保留分配错误。
fn copy_text(text: &str) -> Result<String, TryReserveError> {
    let mut copy = String::new();
    copy.try_reserve_exact(text.len())?;
    copy.push_str(text);
    Ok(copy)
}

fn invalid_value(error: SnapshotError) -> RuntimeError {
    RuntimeError::invalid_value(error.to_string())
}

fn allocation_error(_: TryReserveError) -> RuntimeError {
    RuntimeError::new(
        RuntimeErrorKind::Resource,
        ALLOCATION_CODE,
        "runtime.allocation",
        "无法分配环境操作所需内存",
    )
}

fn limit_error() -> RuntimeError {
    RuntimeError::new(
        RuntimeErrorKind::Resource,
        "ENV_LIMIT_EXCEEDED",
        "runtime.env_limit_exceeded",
        "环境写入超过当前运行的限额",
    )
}

/// Linux 和 macOS 按原始 UTF-8 精确比较。
#[cfg(not(windows))]
fn compare_names(left: &str, right: &str) -> Ordering {
    left.cmp(right)
}

/// 使用 Windows 的不区分大小写序号比较，不依赖区域设置。
#[cfg(windows)]
fn compare_names(left: &str, right: &str) -> Ordering {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CompareStringOrdinal(
            left: *const u16,
            left_len: i32,
            right: *const u16,
            right_len: i32,
            ignore_case: i32,
        ) -> i32;
    }

    fn encode_name(name: &str, buffer: &mut [u16; MAX_NAME_BYTES]) -> i32 {
        let mut len = 0;
        for unit in name.encode_utf16() {
            buffer[len] = unit;
            len += 1;
        }
        len as i32
    }

    // 此处的名称已通过预算检查，UTF-16 码元数至多等于 UTF-8 字节数。
    let (mut left_wide, mut right_wide) = ([0; MAX_NAME_BYTES], [0; MAX_NAME_BYTES]);
    let left_len = encode_name(left, &mut left_wide);
    let right_len = encode_name(right, &mut right_wide);
    // SAFETY: 指针指向已编码的栈缓冲区，长度覆盖相应的码元范围。
    match unsafe {
        CompareStringOrdinal(
            left_wide.as_ptr(),
            left_len,
            right_wide.as_ptr(),
            right_len,
            1,
        )
    } {
        1 => Ordering::Less,
        2 => Ordering::Equal,
        3 => Ordering::Greater,
        _ => unreachable!("已验证的环境名称应当可以进行序号比较"),
    }
}

#[cfg(test)]
#[path = "environment_tests.rs"]
mod tests;
