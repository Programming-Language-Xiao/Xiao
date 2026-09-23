//! 动态降低器的容器构造与表描述符发射。

use xiao_ir::{IrExpression, IrSpan};

use super::predicate::abi_field_type;
use super::text::escape_bytes;
use super::{BYTES_TYPE, DynamicGenerator, TABLE_DESCRIPTOR_TYPE, TABLE_FIELD_TYPE, VALUE_TYPE};
use crate::error::{CodegenError, Result};

impl<'a> DynamicGenerator<'a> {
    /// 发射数组或元组构造。
    pub(super) fn emit_sequence(
        &mut self,
        elements: &[IrExpression],
        tuple: bool,
    ) -> Result<String> {
        let array = self.next_temp();
        self.emit(format!(
            "  {array} = alloca {VALUE_TYPE}, i64 {}",
            elements.len()
        ));
        let mut temporaries = Vec::with_capacity(elements.len());
        for (index, element) in elements.iter().enumerate() {
            let value = self.emit_expression(element)?;
            let ptr = self.next_temp();
            self.emit(format!(
                "  {ptr} = getelementptr {VALUE_TYPE}, ptr {array}, i64 {index}"
            ));
            self.emit(format!("  store {VALUE_TYPE} {value}, ptr {ptr}"));
            temporaries.push(value);
        }
        let handle = self.next_temp();
        self.emit(format!("  {handle} = alloca ptr"));
        self.emit(format!("  store ptr null, ptr {handle}"));
        let function = if tuple {
            "xiao_runtime_tuple_new"
        } else {
            "xiao_runtime_array_new"
        };
        self.checked_status_call(format!(
            "@{function}(ptr {array}, i64 {}, ptr {handle})",
            elements.len()
        ));
        for value in temporaries {
            self.release_value(value);
        }
        let raw = self.next_temp();
        self.emit(format!("  {raw} = load ptr, ptr {handle}"));
        let constructor = if tuple {
            "xiao_runtime_value_tuple"
        } else {
            "xiao_runtime_value_array"
        };
        let value = self.emit_value_call(constructor, &format!("ptr {raw}"));
        self.emit(format!("  call void @xiao_runtime_release(ptr {raw})"));
        Ok(value)
    }

    /// 发射字典构造及键字符串描述数组。
    pub(super) fn emit_dictionary(
        &mut self,
        entries: &[xiao_ir::IrDictEntry],
        kind: u32,
    ) -> Result<String> {
        let keys = self.next_temp();
        self.emit(format!(
            "  {keys} = alloca {BYTES_TYPE}, i64 {}",
            entries.len()
        ));
        let values = self.next_temp();
        self.emit(format!(
            "  {values} = alloca {VALUE_TYPE}, i64 {}",
            entries.len()
        ));
        let mut temporaries = Vec::with_capacity(entries.len());
        for (index, entry) in entries.iter().enumerate() {
            let key_global = format!("@.xiao.key{}", self.next_global);
            self.next_global += 1;
            let key = entry.key.as_bytes();
            self.globals.push(format!(
                "{key_global} = private unnamed_addr constant [{} x i8] c\"{}\\00\"",
                key.len() + 1,
                escape_bytes(key)
            ));
            let key_ptr = self.next_temp();
            self.emit(format!(
                "  {key_ptr} = getelementptr inbounds [{} x i8], ptr {key_global}, i64 0, i64 0",
                key.len() + 1
            ));
            let key_value = self.next_temp();
            self.emit(format!(
                "  {key_value} = insertvalue {BYTES_TYPE} zeroinitializer, ptr {key_ptr}, 0"
            ));
            let key_value2 = self.next_temp();
            self.emit(format!(
                "  {key_value2} = insertvalue {BYTES_TYPE} {key_value}, i64 {}, 1",
                key.len()
            ));
            let key_slot = self.next_temp();
            self.emit(format!(
                "  {key_slot} = getelementptr {BYTES_TYPE}, ptr {keys}, i64 {index}"
            ));
            self.emit(format!("  store {BYTES_TYPE} {key_value2}, ptr {key_slot}"));
            let value = self.emit_expression(&entry.value)?;
            let value_slot = self.next_temp();
            self.emit(format!(
                "  {value_slot} = getelementptr {VALUE_TYPE}, ptr {values}, i64 {index}"
            ));
            self.emit(format!("  store {VALUE_TYPE} {value}, ptr {value_slot}"));
            temporaries.push(value);
        }
        let handle = self.next_temp();
        self.emit(format!("  {handle} = alloca ptr"));
        self.emit(format!("  store ptr null, ptr {handle}"));
        self.checked_status_call(format!(
            "@xiao_runtime_dict_new(i32 {kind}, ptr {keys}, ptr {values}, i64 {}, ptr {handle})",
            entries.len()
        ));
        for value in temporaries {
            self.release_value(value);
        }
        let raw = self.next_temp();
        self.emit(format!("  {raw} = load ptr, ptr {handle}"));
        let value =
            self.emit_value_call("xiao_runtime_value_dict", &format!("ptr {raw}, i32 {kind}"));
        self.emit(format!("  call void @xiao_runtime_release(ptr {raw})"));
        Ok(value)
    }

    /// 发射集合构造。
    pub(super) fn emit_set(&mut self, elements: &[IrExpression]) -> Result<String> {
        let array = self.next_temp();
        self.emit(format!(
            "  {array} = alloca {VALUE_TYPE}, i64 {}",
            elements.len()
        ));
        let mut temporaries = Vec::with_capacity(elements.len());
        for (index, element) in elements.iter().enumerate() {
            let value = self.emit_expression(element)?;
            let ptr = self.next_temp();
            self.emit(format!(
                "  {ptr} = getelementptr {VALUE_TYPE}, ptr {array}, i64 {index}"
            ));
            self.emit(format!("  store {VALUE_TYPE} {value}, ptr {ptr}"));
            temporaries.push(value);
        }
        let handle = self.next_temp();
        self.emit(format!("  {handle} = alloca ptr"));
        self.emit(format!("  store ptr null, ptr {handle}"));
        self.checked_status_call(format!(
            "@xiao_runtime_set_new(ptr {array}, i64 {}, ptr {handle})",
            elements.len()
        ));
        for value in temporaries {
            self.release_value(value);
        }
        let raw = self.next_temp();
        self.emit(format!("  {raw} = load ptr, ptr {handle}"));
        let value = self.emit_value_call("xiao_runtime_value_set", &format!("ptr {raw}"));
        self.emit(format!("  call void @xiao_runtime_release(ptr {raw})"));
        Ok(value)
    }

    /// 发射最小表构造调用；字段描述元数据来自 IR 单一来源。
    pub(super) fn emit_new_call(
        &mut self,
        callee: &IrExpression,
        arguments: &[xiao_ir::IrCallArgument],
        span: IrSpan,
    ) -> Result<String> {
        let xiao_ir::IrExpressionKind::Name { name } = &callee.kind else {
            return Err(CodegenError::Unsupported {
                feature: "动态表构造目标".to_owned(),
                span: Some(span),
            });
        };
        let signature = self
            .program
            .table_signatures
            .iter()
            .find(|signature| signature.name == name.text)
            .cloned()
            .ok_or_else(|| CodegenError::InvalidIr {
                message: format!("表 {} 没有登记签名", name.text),
            })?;
        if !arguments.is_empty() {
            return Err(CodegenError::Unsupported {
                feature: "动态表构造参数（需运行时 init 调用支持）".to_owned(),
                span: Some(span),
            });
        }
        let descriptor = self.emit_table_descriptor(&signature)?;
        let handle = self.next_temp();
        self.emit(format!("  {handle} = alloca ptr"));
        self.emit(format!("  store ptr null, ptr {handle}"));
        self.checked_status_call(format!(
            "@xiao_runtime_table_new(ptr {descriptor}, ptr {handle})"
        ));
        let raw = self.next_temp();
        self.emit(format!("  {raw} = load ptr, ptr {handle}"));
        let initializers = self
            .table_initializers
            .get(&name.text)
            .cloned()
            .unwrap_or_default();
        for (field, initializer) in initializers {
            let value = self.emit_expression(&initializer)?;
            let field_bytes = self.emit_bytes_value(field.as_bytes());
            let field_argument = self.emit_bytes_argument(&field_bytes);
            let value_slot = self.next_temp();
            self.emit(format!("  {value_slot} = alloca {VALUE_TYPE}"));
            self.emit(format!("  store {VALUE_TYPE} {value}, ptr {value_slot}"));
            self.checked_status_call(format!(
                "@xiao_runtime_table_set(ptr {raw}, {field_argument}, ptr {value_slot})"
            ));
            self.release_value(value);
        }
        let value = self.emit_value_call("xiao_runtime_value_table", &format!("ptr {raw}"));
        self.emit(format!("  call void @xiao_runtime_release(ptr {raw})"));
        Ok(value)
    }

    /// 发射静态表描述符及字段数组。
    fn emit_table_descriptor(&mut self, signature: &xiao_ir::IrTableSignature) -> Result<String> {
        // 先还原一次 Runtime 签名，确保 ABI 描述仍由 xiao-ir 的唯一表接口来源校验。
        // 字段描述 ABI 目前不携带方法函数表，因此含方法的表必须显式拒绝，不能把方法
        // 静默伪装成字段。
        if signature.runtime_signature().is_none() {
            return Err(CodegenError::InvalidIr {
                message: format!("表 {} 的 IR 签名无法还原为 Runtime 签名", signature.name),
            });
        }
        if signature.members.iter().any(|member| member.method) {
            return Err(CodegenError::Unsupported {
                feature: "动态表方法（ABI 尚未携带函数表）".to_owned(),
                span: Some(signature.span),
            });
        }

        let field_count = signature.members.len();
        let fields = if field_count == 0 {
            "null".to_owned()
        } else {
            let fields = self.next_temp();
            self.emit(format!(
                "  {fields} = alloca {TABLE_FIELD_TYPE}, i64 {field_count}"
            ));
            fields
        };
        for (index, member) in signature.members.iter().enumerate() {
            let field_type =
                abi_field_type(&member.ty).ok_or_else(|| CodegenError::Unsupported {
                    feature: "动态表字段类型".to_owned(),
                    span: Some(member.span),
                })?;
            let key = member.name.as_bytes();
            let global = format!("@.xiao.field{}", self.next_global);
            self.next_global += 1;
            self.globals.push(format!(
                "{global} = private unnamed_addr constant [{} x i8] c\"{}\\00\"",
                key.len() + 1,
                escape_bytes(key)
            ));
            let ptr = self.next_temp();
            self.emit(format!(
                "  {ptr} = getelementptr inbounds [{} x i8], ptr {global}, i64 0, i64 0",
                key.len() + 1
            ));
            let bytes = self.next_temp();
            self.emit(format!(
                "  {bytes} = insertvalue {BYTES_TYPE} zeroinitializer, ptr {ptr}, 0"
            ));
            let bytes_with_len = self.next_temp();
            self.emit(format!(
                "  {bytes_with_len} = insertvalue {BYTES_TYPE} {bytes}, i64 {}, 1",
                key.len()
            ));
            let field_value0 = self.next_temp();
            self.emit(format!(
                "  {field_value0} = insertvalue {TABLE_FIELD_TYPE} zeroinitializer, {BYTES_TYPE} {bytes_with_len}, 0"
            ));
            let field_value1 = self.next_temp();
            self.emit(format!(
                "  {field_value1} = insertvalue {TABLE_FIELD_TYPE} {field_value0}, i32 {field_type}, 1"
            ));
            let field_value2 = self.next_temp();
            self.emit(format!(
                "  {field_value2} = insertvalue {TABLE_FIELD_TYPE} {field_value1}, i8 {}, 2",
                u8::from(member.public)
            ));
            let field = self.next_temp();
            self.emit(format!(
                "  {field} = getelementptr {TABLE_FIELD_TYPE}, ptr {fields}, i64 {index}"
            ));
            self.emit(format!(
                "  store {TABLE_FIELD_TYPE} {field_value2}, ptr {field}"
            ));
        }
        let name = signature.name.as_bytes();
        let name_global = format!("@.xiao.table{}", self.next_global);
        self.next_global += 1;
        self.globals.push(format!(
            "{name_global} = private unnamed_addr constant [{} x i8] c\"{}\\00\"",
            name.len() + 1,
            escape_bytes(name)
        ));
        let name_ptr = self.next_temp();
        self.emit(format!(
            "  {name_ptr} = getelementptr inbounds [{} x i8], ptr {name_global}, i64 0, i64 0",
            name.len() + 1
        ));
        let name_bytes0 = self.next_temp();
        self.emit(format!(
            "  {name_bytes0} = insertvalue {BYTES_TYPE} zeroinitializer, ptr {name_ptr}, 0"
        ));
        let name_bytes1 = self.next_temp();
        self.emit(format!(
            "  {name_bytes1} = insertvalue {BYTES_TYPE} {name_bytes0}, i64 {}, 1",
            name.len()
        ));
        let kind = match signature.kind.as_str() {
            "singleton" => 0,
            "instance" => 1,
            _ => {
                return Err(CodegenError::InvalidIr {
                    message: format!("表 {} 的种类无效", signature.name),
                });
            }
        };
        let descriptor = self.next_temp();
        self.emit(format!("  {descriptor} = alloca {TABLE_DESCRIPTOR_TYPE}"));
        let descriptor_value0 = self.next_temp();
        self.emit(format!(
            "  {descriptor_value0} = insertvalue {TABLE_DESCRIPTOR_TYPE} zeroinitializer, {BYTES_TYPE} {name_bytes1}, 0"
        ));
        let descriptor_value1 = self.next_temp();
        self.emit(format!(
            "  {descriptor_value1} = insertvalue {TABLE_DESCRIPTOR_TYPE} {descriptor_value0}, i32 {kind}, 1"
        ));
        let descriptor_value2 = self.next_temp();
        self.emit(format!(
            "  {descriptor_value2} = insertvalue {TABLE_DESCRIPTOR_TYPE} {descriptor_value1}, ptr {fields}, 2"
        ));
        let descriptor_value3 = self.next_temp();
        self.emit(format!(
            "  {descriptor_value3} = insertvalue {TABLE_DESCRIPTOR_TYPE} {descriptor_value2}, i64 {field_count}, 3"
        ));
        self.emit(format!(
            "  store {TABLE_DESCRIPTOR_TYPE} {descriptor_value3}, ptr {descriptor}"
        ));
        Ok(descriptor)
    }
}
