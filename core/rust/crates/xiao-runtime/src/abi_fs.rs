//! fs 借读 C ABI，所有宿主语义委托同一个 Runtime。

use super::{abi_value_validation::validate_structure, *};
use crate::filesystem::{self, Failure, Operation};
use xiao_intrinsics::validate_capability_mask;

/// 先校验 ABI 结构，再委托共享入口，成功后才替换输出槽。
fn execute<const N: usize>(
    operation: Operation,
    inputs: [*const XiaoValue; N],
    tags: [XiaoValueTag; N],
    output: Option<*mut XiaoValue>,
    capabilities: u64,
) -> i32 {
    let run = || {
        if inputs.iter().any(|input| input.is_null()) || output.is_some_and(|out| out.is_null()) {
            return Err(XiaoAbiStatus::Null.code());
        }
        if output.is_some_and(|out| inputs.contains(&out.cast_const()))
            || validate_capability_mask(capabilities).is_err()
        {
            return Err(XiaoAbiStatus::InvalidArgument.code());
        }
        // SAFETY: 借读槽由调用方保证有效并持有强引用；这里已经拒绝空指针及同址 out。
        let inputs = inputs.map(|input| unsafe { &*input });
        for input in inputs {
            unsafe { validate_structure(input, false) }?;
        }
        if let Some(out) = output {
            unsafe { validate_structure(&*out, true) }?;
        }
        let mut values = std::array::from_fn::<_, N, _>(|_| RuntimeValue::None);
        for (slot, (input, tag)) in values.iter_mut().zip(inputs.into_iter().zip(tags)) {
            if input.tag != tag {
                let expected = if tag == XiaoValueTag::Bool {
                    "bool"
                } else {
                    "str"
                };
                return Err(status_from_error(&RuntimeError::type_mismatch(
                    expected,
                    format!("{:?}", input.tag),
                )));
            }
            *slot = unsafe { value_to_runtime(input) }?;
        }
        let result = filesystem::execute(operation, &values.each_ref(), capabilities, || {
            let status = super::abi_concurrency::cancellation_status();
            if status == XiaoAbiStatus::Ok.code() {
                Ok(())
            } else {
                Err(status)
            }
        });
        let value = match result {
            Ok(value) => value,
            Err(Failure::Runtime(error)) => return Err(status_from_error(&error)),
            Err(Failure::Interrupted { control, cleanup }) => {
                if let Some(cleanup) = cleanup {
                    PENDING_ERROR.with_borrow_mut(|pending| {
                        if let Some(PendingError::Recoverable(primary)) = pending.as_mut() {
                            primary.push_suppressed(cleanup);
                        }
                    });
                }
                return Err(control);
            }
        };
        if let Some(out) = output {
            let value = runtime_to_value(&value)?;
            unsafe { write_value(out, value) }?;
        }
        Ok(())
    };
    run().map_or_else(|status| status, |()| XiaoAbiStatus::Ok.code())
}

/// 借读路径、文本和覆盖标记并执行同目录原子写入。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_fs_write_text(
    path: *const XiaoValue,
    text: *const XiaoValue,
    overwrite: *const XiaoValue,
    capabilities: u64,
) -> i32 {
    execute(
        Operation::WriteText,
        [path, text, overwrite],
        [XiaoValueTag::Str, XiaoValueTag::Str, XiaoValueTag::Bool],
        None,
        capabilities,
    )
}

/// 成功时用拥有的目录项名称数组替换输出槽。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_fs_list_dir(
    path: *const XiaoValue,
    out: *mut XiaoValue,
    capabilities: u64,
) -> i32 {
    execute(
        Operation::ListDir,
        [path],
        [XiaoValueTag::Str],
        Some(out),
        capabilities,
    )
}

/// 按目录项语义删除借用路径。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_fs_remove(path: *const XiaoValue, capabilities: u64) -> i32 {
    execute(
        Operation::Remove,
        [path],
        [XiaoValueTag::Str],
        None,
        capabilities,
    )
}

/// 借读两个路径和覆盖标记并执行同卷重命名。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_fs_rename(
    source: *const XiaoValue,
    destination: *const XiaoValue,
    overwrite: *const XiaoValue,
    capabilities: u64,
) -> i32 {
    execute(
        Operation::Rename,
        [source, destination, overwrite],
        [XiaoValueTag::Str, XiaoValueTag::Str, XiaoValueTag::Bool],
        None,
        capabilities,
    )
}

/// 成功时用拥有的元数据字典替换输出槽。
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_fs_metadata(
    path: *const XiaoValue,
    out: *mut XiaoValue,
    capabilities: u64,
) -> i32 {
    execute(
        Operation::Metadata,
        [path],
        [XiaoValueTag::Str],
        Some(out),
        capabilities,
    )
}

#[cfg(test)]
/// 验证借读实参、拥有槽和挂起错误。
mod tests {
    use super::*;
    use xiao_intrinsics::CAPABILITY_FILESYSTEM;

    /// 创建由测试负责释放的 ABI 字符串。
    fn string(text: &str) -> XiaoValue {
        runtime_to_value(&RuntimeValue::new_string(text).unwrap()).unwrap()
    }

    /// 读取当前线程的主错误编号。
    fn pending_code() -> String {
        PENDING_ERROR.with_borrow(|error| match error {
            Some(PendingError::Recoverable(error)) => error.code().to_owned(),
            _ => panic!("预期可恢复的挂起错误"),
        })
    }

    #[test]
    /// 核对结构错误不改变挂起状态，并验证输出槽替换规则。
    fn fs_abi_shapes_types_capabilities_and_output_slot() {
        let mut out = string("旧值");
        let previous = unsafe { out.payload.raw };
        let mut empty = string("");
        let mut current = string(".");
        let aliased = &raw mut current;
        let bad_handle = XiaoValue {
            tag: XiaoValueTag::Str,
            payload: XiaoValuePayload {
                handle: std::ptr::null_mut(),
            },
        };
        set_pending_runtime_error(RuntimeError::resource_closed());
        let original = pending_code();
        for (path, out, capabilities, expected) in [
            (std::ptr::null(), &raw mut out, 0, 1),
            (
                &raw const current,
                std::ptr::null_mut(),
                CAPABILITY_FILESYSTEM,
                1,
            ),
            (aliased.cast_const(), aliased, CAPABILITY_FILESYSTEM, 2),
            (&raw const current, &raw mut out, 0x100, 2),
            (
                &raw const bad_handle,
                &raw mut out,
                CAPABILITY_FILESYSTEM,
                3,
            ),
        ] {
            assert_eq!(xiao_runtime_fs_metadata(path, out, capabilities), expected);
            assert_eq!(pending_code(), original);
        }
        xiao_runtime_error_clear();
        assert_eq!(xiao_runtime_fs_metadata(&XiaoValue::int(0), &mut out, 0), 6);
        assert_eq!(pending_code(), "X06-RUNTIME-002");
        xiao_runtime_error_clear();
        assert_eq!(xiao_runtime_fs_metadata(&empty, &mut out, 0), 6);
        assert_eq!(pending_code(), "X06-RUNTIME-026");
        xiao_runtime_error_clear();
        assert_eq!(
            xiao_runtime_fs_metadata(&empty, &mut out, CAPABILITY_FILESYSTEM),
            6
        );
        assert_eq!(pending_code(), "X06-RUNTIME-012");
        assert_eq!(unsafe { out.payload.raw }, previous);
        assert_eq!(
            xiao_runtime_fs_metadata(&current, &mut out, CAPABILITY_FILESYSTEM),
            0
        );
        let RuntimeValue::DictTable(metadata) = (unsafe { value_to_runtime(&out) }).unwrap() else {
            panic!("元数据返回拥有的字典");
        };
        assert_eq!(metadata.value("kind").unwrap(), Some(RuntimeValue::Int(2)));
        assert_eq!(metadata.value("size").unwrap(), Some(RuntimeValue::Int(-1)));
        assert_eq!(pending_code(), "X06-RUNTIME-012");
        assert_eq!(
            xiao_runtime_fs_list_dir(&current, &mut out, CAPABILITY_FILESYSTEM),
            0
        );
        assert!(matches!(
            unsafe { value_to_runtime(&out) }.unwrap(),
            RuntimeValue::Array(_)
        ));
        for value in [&mut empty, &mut current, &mut out] {
            xiao_runtime_value_release_strong(value);
        }
        xiao_runtime_error_clear();
    }
}
