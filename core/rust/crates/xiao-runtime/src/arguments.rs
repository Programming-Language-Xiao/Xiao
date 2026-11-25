//! 运行入口与 args 原语共享的参数快照。

use std::ffi::OsString;
use std::fmt;

use crate::{
    ALLOCATION_CODE, ArrayHandle, RuntimeError, RuntimeErrorKind, RuntimeResult, RuntimeValue,
    StringHandle,
};

/// 程序名和参数通过接纳检查后由一次运行及其子任务共享。
#[derive(Default, Eq, PartialEq)]
pub struct ArgumentsSnapshot {
    program_name: String,
    argv: Vec<String>,
}

/// 嵌入方省略输入时读取的空快照。
pub static EMPTY_ARGUMENTS: ArgumentsSnapshot = ArgumentsSnapshot {
    program_name: String::new(),
    argv: Vec::new(),
};

/// 接纳阶段的失败不携带原始参数文本。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArgumentsError {
    /// 宿主编码不能无损转成 Unicode。
    InvalidEncoding,
    /// 字符串含宿主 argv 不允许的 NUL。
    InteriorNul,
    /// 条目数、单项或总字节数超过接纳上限。
    LimitExceeded,
}

impl ArgumentsError {
    /// 返回拒绝执行时的驱动器编号。
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidEncoding | Self::InteriorNul => "X09-DRIVER-004",
            Self::LimitExceeded => "X09-DRIVER-005",
        }
    }
}

impl fmt::Display for ArgumentsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidEncoding => "运行参数含无效的宿主编码",
            Self::InteriorNul => "运行参数含 NUL",
            Self::LimitExceeded => "运行参数超过接纳上限",
        })
    }
}

impl std::error::Error for ArgumentsError {}

impl fmt::Debug for ArgumentsSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ArgumentsSnapshot")
            .field("argument_count", &self.argv.len())
            .finish_non_exhaustive()
    }
}

impl ArgumentsSnapshot {
    /// 接纳嵌入或协议提供的 UTF-8 输入。
    pub fn new(program_name: String, argv: Vec<String>) -> Result<Self, ArgumentsError> {
        check_count(argv.len())?;
        let mut total = 0;
        for value in std::iter::once(&program_name).chain(&argv) {
            check_text(value, &mut total)?;
        }
        Ok(Self { program_name, argv })
    }

    /// 在有损转换之前接纳宿主已经拆分的程序名和参数。
    pub fn from_os(program_name: OsString, argv: Vec<OsString>) -> Result<Self, ArgumentsError> {
        Self::from_os_iter(program_name, argv.into_iter())
    }

    /// 独立原生入口读取宿主拆分结果，Windows 保留原始宽字符。
    pub(crate) fn from_host() -> Result<Self, ArgumentsError> {
        let mut argv = std::env::args_os();
        let program_name = argv.next().unwrap_or_default();
        Self::from_os_iter(program_name, argv)
    }

    fn from_os_iter(
        program_name: OsString,
        argv: impl ExactSizeIterator<Item = OsString>,
    ) -> Result<Self, ArgumentsError> {
        check_count(argv.len())?;
        let mut total = 0;
        let mut convert = |value: OsString| {
            let text = value
                .into_string()
                .map_err(|_| ArgumentsError::InvalidEncoding)?;
            check_text(&text, &mut total)?;
            Ok(text)
        };
        let program_name = convert(program_name)?;
        let argv = argv.map(convert).collect::<Result<_, ArgumentsError>>()?;
        Ok(Self { program_name, argv })
    }

    /// 经进程能力检查后交付拥有的程序名。
    pub fn program_name(&self, capabilities: u64) -> RuntimeResult<StringHandle> {
        crate::resources::require_process(capabilities)?;
        output_string(&self.program_name)
    }

    /// 经进程能力检查后交付独立数组，元素由数组持有。
    pub fn argv(&self, capabilities: u64) -> RuntimeResult<ArrayHandle> {
        crate::resources::require_process(capabilities)?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(self.argv.len())
            .map_err(|_| allocation_error())?;
        for item in &self.argv {
            values.push(RuntimeValue::Str(output_string(item)?));
        }
        ArrayHandle::new(values)
    }
}

/// 在检查单项前拒绝条目超限。程序名独立于条目数。
fn check_count(count: usize) -> Result<(), ArgumentsError> {
    if count > 1024 {
        return Err(ArgumentsError::LimitExceeded);
    }
    Ok(())
}

/// 按冻结顺序核对 NUL、单项和累计字节数。
fn check_text(text: &str, total: &mut usize) -> Result<(), ArgumentsError> {
    if text.contains('\0') {
        return Err(ArgumentsError::InteriorNul);
    }
    if text.len() > 4096 || *total + text.len() > 65536 {
        return Err(ArgumentsError::LimitExceeded);
    }
    *total += text.len();
    Ok(())
}

/// 先以可恢复分配复制字节，再交给 Runtime 字符串。
fn output_string(text: &str) -> RuntimeResult<StringHandle> {
    let mut value = String::new();
    value
        .try_reserve_exact(text.len())
        .map_err(|_| allocation_error())?;
    value.push_str(text);
    StringHandle::new(value)
}

/// 受控输出分配失败沿用 Runtime 011。
fn allocation_error() -> RuntimeError {
    RuntimeError::new(
        RuntimeErrorKind::Resource,
        ALLOCATION_CODE,
        "runtime.allocation",
        "无法分配参数快照输出",
    )
}

#[cfg(test)]
/// 参数接纳、进程能力和拥有输出。
mod tests {
    use super::*;
    use xiao_intrinsics::CAPABILITY_PROCESS;

    #[test]
    fn admission_checks_count_then_each_item_in_order() {
        assert_eq!(
            ArgumentsSnapshot::new("\0".into(), vec![String::new(); 1025]).unwrap_err(),
            ArgumentsError::LimitExceeded
        );
        assert_eq!(
            ArgumentsSnapshot::new("\0".into(), vec!["a".repeat(4097)])
                .unwrap_err()
                .code(),
            "X09-DRIVER-004"
        );
        let full = "🧪".repeat(1024);
        assert!(ArgumentsSnapshot::new(full.clone(), vec![full.clone(); 15]).is_ok());
        assert_eq!(
            ArgumentsSnapshot::new(full.clone(), vec![full.clone(); 16])
                .unwrap_err()
                .code(),
            "X09-DRIVER-005"
        );
        assert_eq!(
            ArgumentsSnapshot::new(format!("{full}a"), Vec::new()).unwrap_err(),
            ArgumentsError::LimitExceeded
        );
        assert!(ArgumentsSnapshot::new("name".into(), vec![String::new(); 1024]).is_ok());
    }

    #[test]
    fn host_encoding_is_strict_and_checks_items_in_order() {
        #[cfg(unix)]
        let invalid = {
            use std::os::unix::ffi::OsStringExt;
            OsString::from_vec(vec![0xff])
        };
        #[cfg(windows)]
        let invalid = {
            use std::os::windows::ffi::OsStringExt;
            OsString::from_wide(&[0xd800])
        };
        assert_eq!(
            ArgumentsSnapshot::from_os("name".into(), vec![invalid.clone()]).unwrap_err(),
            ArgumentsError::InvalidEncoding
        );
        assert_eq!(
            ArgumentsSnapshot::from_os("name".into(), vec!["a".repeat(4097).into(), invalid])
                .unwrap_err(),
            ArgumentsError::LimitExceeded
        );
        assert!(ArgumentsSnapshot::from_os("name".into(), vec!["\u{fffd}".into()]).is_ok());
    }

    #[test]
    fn owned_results_keep_the_snapshot_private_and_independent() {
        let snapshot = ArgumentsSnapshot::new(
            "private-name".into(),
            vec!["".into(), "参数".into(), "参数".into()],
        )
        .unwrap();
        assert!(!format!("{snapshot:?}").contains("private-name"));
        assert_eq!(
            snapshot
                .program_name(CAPABILITY_PROCESS)
                .unwrap()
                .to_string()
                .unwrap(),
            "private-name"
        );
        assert_eq!(
            snapshot.program_name(0).err().unwrap().code(),
            "X06-RUNTIME-026"
        );
        assert_eq!(snapshot.argv(0).err().unwrap().code(), "X06-RUNTIME-026");
        let first = snapshot.argv(CAPABILITY_PROCESS).unwrap();
        first.push(RuntimeValue::Int(1)).unwrap();
        assert_eq!(first.len(), 4);
        let second = snapshot.argv(CAPABILITY_PROCESS).unwrap();
        let values = second
            .with_elements(|values| {
                values
                    .iter()
                    .map(|value| value.display_text().unwrap())
                    .collect::<Vec<_>>()
            })
            .unwrap();
        assert_eq!(values, ["", "参数", "参数"]);
        assert!(EMPTY_ARGUMENTS.argv(CAPABILITY_PROCESS).unwrap().is_empty());
        assert!(
            EMPTY_ARGUMENTS
                .program_name(CAPABILITY_PROCESS)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn allocation_failure_does_not_publish_a_partial_result() {
        let snapshot =
            ArgumentsSnapshot::new("name".into(), vec!["one".into(), "two".into()]).unwrap();
        let failure = crate::allocation_testing::StringAllocationFailure::after(1);
        assert_eq!(
            snapshot.argv(CAPABILITY_PROCESS).err().unwrap().code(),
            ALLOCATION_CODE
        );
        assert!(failure.was_triggered());
        assert_eq!(snapshot.argv(CAPABILITY_PROCESS).unwrap().len(), 2);
    }
}
