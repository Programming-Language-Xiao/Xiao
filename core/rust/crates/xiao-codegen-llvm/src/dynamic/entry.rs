//! 动态降低器的程序入口、适配器与观察值发射。

use super::{DynamicGenerator, VALUE_TYPE};
use xiao_ir::IrType;
use xiao_runtime_abi::{ABI_MAJOR_VERSION, ABI_MINOR_VERSION};

impl<'a> DynamicGenerator<'a> {
    /// 发射入口函数和 C `main` 适配器。
    pub(super) fn emit_entry(&mut self) -> crate::error::Result<()> {
        let observed = self.observation_slot.is_some();
        let entry_return = if observed { "i64" } else { "void" };
        self.emit(format!("define {entry_return} @xiao_entry() {{"));
        self.emit("entry:".to_owned());
        let compatibility = self.next_temp();
        self.emit(format!(
            "  {compatibility} = call i32 @xiao_runtime_abi_is_compatible(i32 {ABI_MAJOR_VERSION}, i32 {ABI_MINOR_VERSION})"
        ));
        let compatibility_ok = self.next_temp();
        self.emit(format!(
            "  {compatibility_ok} = icmp eq i32 {compatibility}, 1"
        ));
        self.emit(format!(
            "  br i1 {compatibility_ok}, label %abi.entry.ok, label %abi.fail"
        ));
        self.emit("abi.entry.ok:".to_owned());
        let slots = self.slots.values().copied().collect::<Vec<_>>();
        for slot in slots {
            self.emit(format!("  %slot{} = alloca {VALUE_TYPE}", slot.index));
            self.emit(format!(
                "  store {VALUE_TYPE} zeroinitializer, ptr %slot{}",
                slot.index
            ));
        }
        if let Some(slot) = self.observation_slot {
            self.emit(format!("  %slot{slot} = alloca i64"));
            self.emit(format!("  store i64 0, ptr %slot{slot}"));
        }
        for statement in &self.program.body {
            self.emit_statements(std::slice::from_ref(statement))?;
            if self.terminated {
                break;
            }
        }
        if !self.terminated {
            self.release_for_exit("normal")?;
            self.emit_observation_return();
            self.terminated = true;
        }
        self.emit("abi.fail:".to_owned());
        self.emit("  call void @llvm.trap()".to_owned());
        self.emit("  unreachable".to_owned());
        self.emit("}".to_owned());
        self.emit(String::new());
        self.emit(self.main_adapter(observed));
        Ok(())
    }

    /// 发射带可选调试启动检查的 C `main` 适配器。
    fn main_adapter(&self, observed: bool) -> String {
        let entry = if observed {
            "  %xiao_exit = call i64 @xiao_entry()\n  %xiao_exit_code = trunc i64 %xiao_exit to i32\n  ret i32 %xiao_exit_code\n"
        } else {
            "  call void @xiao_entry()\n  ret i32 0\n"
        };
        if self.options.debug_startup.is_none() {
            return format!("define i32 @main() {{\nentry:\n{entry}}}\n");
        }
        format!(
            "define i32 @main() {{\nentry:\n  %xiao_debug_status = call i32 @xiao_native_debug_start()\n  %xiao_debug_ok = icmp eq i32 %xiao_debug_status, 0\n  br i1 %xiao_debug_ok, label %xiao.user, label %xiao.debug.fail\nxiao.user:\n{entry}xiao.debug.fail:\n  ret i32 %xiao_debug_status\n}}\n"
        )
    }

    /// 在入口观察模式下返回最近一次静态整数/布尔观察值。
    pub(super) fn emit_observation_return(&mut self) {
        if let Some(slot) = self.observation_slot {
            let value = self.next_temp();
            self.emit(format!("  {value} = load i64, ptr %slot{slot}"));
            self.emit(format!("  ret i64 {value}"));
        } else {
            self.emit("  ret void".to_owned());
        }
    }

    /// 记录动态模块中静态类型已经确定的整数或布尔值。
    pub(super) fn record_observation(&mut self, value: &str, ty: &IrType) {
        let Some(slot) = self.observation_slot else {
            return;
        };
        let IrType::Scalar { name } = ty else {
            return;
        };
        let (instruction, source_type) = match name.as_str() {
            "int" => (None, "i64"),
            "sint" => (Some("sext"), "i32"),
            "bool" => (Some("zext"), "i1"),
            _ => return,
        };
        let payload = self.next_temp();
        self.emit(format!(
            "  {payload} = extractvalue {VALUE_TYPE} {value}, 1"
        ));
        let observed = if let Some(instruction) = instruction {
            let narrowed = self.next_temp();
            self.emit(format!(
                "  {narrowed} = trunc i64 {payload} to {source_type}"
            ));
            let extended = self.next_temp();
            self.emit(format!(
                "  {extended} = {instruction} {source_type} {narrowed} to i64"
            ));
            extended
        } else {
            payload
        };
        self.emit(format!("  store i64 {observed}, ptr %slot{slot}"));
    }
}
