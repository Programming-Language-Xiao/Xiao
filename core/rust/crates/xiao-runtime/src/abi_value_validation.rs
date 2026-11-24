//! 借读 C ABI 入口的标签和拥有槽结构校验。

use super::*;

/// 只校验载荷结构和句柄种类，不解析lint/lfloat文本或读写挂起错误。
pub(super) unsafe fn validate_structure(value: &XiaoValue, owned: bool) -> Result<(), i32> {
    let invalid = XiaoAbiStatus::InvalidHandle.code();
    let expected = match value.tag {
        XiaoValueTag::None
        | XiaoValueTag::Int
        | XiaoValueTag::Sint
        | XiaoValueTag::Float
        | XiaoValueTag::Sfloat => return Ok(()),
        XiaoValueTag::Bool => {
            return if unsafe { value.payload.bool_value } <= 1 {
                Ok(())
            } else {
                Err(XiaoAbiStatus::InvalidArgument.code())
            };
        }
        XiaoValueTag::Str | XiaoValueTag::Lint | XiaoValueTag::Lfloat => RuntimeTypeTag::String,
        XiaoValueTag::Table => RuntimeTypeTag::Table,
        XiaoValueTag::Array => RuntimeTypeTag::Array,
        XiaoValueTag::Tuple => RuntimeTypeTag::Tuple,
        XiaoValueTag::DictTable => RuntimeTypeTag::DictTable,
        XiaoValueTag::DictColumn => RuntimeTypeTag::DictColumn,
        XiaoValueTag::Set => RuntimeTypeTag::Set,
        XiaoValueTag::ExternalResource => EXTERNAL_RESOURCE_RUNTIME_TAG,
        XiaoValueTag::Error => {
            let boxed = unsafe { error_ref(value.payload.handle) }.map_err(|_| invalid)?;
            return if boxed.inners.borrow().is_empty() {
                Err(invalid)
            } else {
                Ok(())
            };
        }
        XiaoValueTag::TableDropView => {
            if owned {
                return Err(XiaoAbiStatus::InvalidArgument.code());
            }
            let boxed = unsafe { weak_ref(value.payload.weak_handle) }.map_err(|_| invalid)?;
            return if boxed
                .inners
                .borrow()
                .last()
                .is_some_and(|h| h.type_tag() == RuntimeTypeTag::Table)
            {
                Ok(())
            } else {
                Err(invalid)
            };
        }
        XiaoValueTag::Concurrency => {
            let kind = abi_concurrency::xiao_runtime_concurrency_kind(
                unsafe { value.payload.handle }.cast(),
            );
            return if kind == 0 { Err(invalid) } else { Ok(()) };
        }
        _ => return Err(XiaoAbiStatus::InvalidArgument.code()),
    };
    let boxed = unsafe { strong_ref(value.payload.handle) }.map_err(|_| invalid)?;
    if boxed
        .inners
        .borrow()
        .last()
        .is_some_and(|h| h.type_tag() == expected)
    {
        Ok(())
    } else {
        Err(invalid)
    }
}
