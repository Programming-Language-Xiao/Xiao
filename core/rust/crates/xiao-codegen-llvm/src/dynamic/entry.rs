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
        self.emit("  call void @xiao_runtime_error_clear()".to_owned());
        self.emit("  call void @xiao_runtime_release_trace_begin()".to_owned());
        let slots = self.slots.values().copied().collect::<Vec<_>>();
        for slot in slots {
            self.emit(format!("  %slot{} = alloca {VALUE_TYPE}", slot.index));
            self.emit(format!(
                "  store {VALUE_TYPE} zeroinitializer, ptr %slot{}",
                slot.index
            ));
        }
        let locale = self.emit_bytes_value(self.options.locale.as_bytes());
        let locale = self.emit_bytes_argument(&locale);
        let locale_status = self.next_temp();
        self.emit(format!(
            "  {locale_status} = call i32 @xiao_runtime_language_context_set({locale})"
        ));
        self.check_status_at(&locale_status, self.program.span);
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
            self.release_frame_temporaries();
            let status = self.next_temp();
            self.emit(format!("  {status} = call i32 @xiao_runtime_error_class()"));
            self.check_status(&status);
            self.emit("  call void @xiao_runtime_release_trace_flush()".to_owned());
            self.emit_observation_return();
            self.terminated = true;
        }
        self.emit(format!("{}:", self.error_terminal_label));
        let class = self.next_temp();
        self.emit(format!("  {class} = call i32 @xiao_runtime_error_class()"));
        let fatal = self.next_temp();
        self.emit(format!("  {fatal} = icmp eq i32 {class}, 2"));
        let report = self.next_label("xiao.error.report");
        let recoverable = self.next_label("xiao.error.unmatched");
        self.emit(format!(
            "  br i1 {fatal}, label %{report}, label %{recoverable}"
        ));
        self.terminated = true;
        self.emit_label(&recoverable);
        self.release_for_exit("unmatched_error")?;
        self.release_frame_temporaries();
        self.emit(format!("  br label %{report}"));
        self.terminated = true;
        self.emit_label(&report);
        self.emit("  call void @xiao_runtime_error_report()".to_owned());
        self.emit("  call void @xiao_runtime_release_trace_flush()".to_owned());
        let error_code = self.next_temp();
        self.emit(format!(
            "  {error_code} = call i32 @xiao_runtime_error_exit_code()"
        ));
        if observed {
            let error_code_i64 = self.next_temp();
            self.emit(format!("  {error_code_i64} = sext i32 {error_code} to i64"));
            self.emit(format!("  ret i64 {error_code_i64}"));
        } else {
            self.emit("  ret void".to_owned());
        }
        self.emit("abi.fail:".to_owned());
        self.emit("  call void @xiao_runtime_fatal_abi()".to_owned());
        self.emit("  unreachable".to_owned());
        self.emit("}".to_owned());
        self.emit(String::new());
        self.emit(self.main_adapter(observed));
        self.initialize_frame_temporaries();
        Ok(())
    }

    /// 发射带可选调试启动检查的 C `main` 适配器。
    fn main_adapter(&self, observed: bool) -> String {
        let entry = if observed {
            "  %xiao_exit = call i64 @xiao_entry()\n  %xiao_exit_code = trunc i64 %xiao_exit to i32\n"
        } else {
            "  call void @xiao_entry()\n  %xiao_exit_code = call i32 @xiao_runtime_error_exit_code()\n"
        };
        if self.options.debug_startup.is_none() {
            return format!(
                "define i32 @main() {{\nentry:\n{entry}  ret i32 %xiao_exit_code\n}}\n"
            );
        }
        format!(
            "define i32 @main() {{\nentry:\n  %xiao_debug_prepare = call i32 @xiao_runtime_diagnostic_prepare()\n  %xiao_debug_prepare_ok = icmp eq i32 %xiao_debug_prepare, 0\n  br i1 %xiao_debug_prepare_ok, label %xiao.debug.start, label %xiao.debug.prepare.fail\nxiao.debug.prepare.fail:\n  ret i32 %xiao_debug_prepare\nxiao.debug.start:\n  %xiao_debug_status = call i32 @xiao_native_debug_start()\n  %xiao_debug_ok = icmp eq i32 %xiao_debug_status, 0\n  br i1 %xiao_debug_ok, label %xiao.debug.ready, label %xiao.debug.fail\nxiao.debug.fail:\n  call void @xiao_runtime_diagnostic_finish()\n  ret i32 %xiao_debug_status\nxiao.debug.ready:\n  %xiao_debug_ready = call i32 @xiao_runtime_diagnostic_ready()\n  %xiao_debug_ready_ok = icmp eq i32 %xiao_debug_ready, 0\n  br i1 %xiao_debug_ready_ok, label %xiao.user, label %xiao.debug.ready.fail\nxiao.debug.ready.fail:\n  call void @xiao_runtime_diagnostic_finish()\n  ret i32 %xiao_debug_ready\nxiao.user:\n{entry}  call void @xiao_runtime_diagnostic_finish()\n  ret i32 %xiao_exit_code\n}}\n"
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
