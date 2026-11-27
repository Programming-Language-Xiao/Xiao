; Xiao N0-B LLVM dynamic module
; target = x86_64-pc-windows-msvc
target triple = "x86_64-pc-windows-msvc"

%xiao.value = type { i32, i64 }
%xiao.bytes = type { ptr, i64 }

%xiao.table.field = type { %xiao.bytes, i32, i8 }
%xiao.table.descriptor = type { %xiao.bytes, i32, ptr, i64 }

%xiao.table.method = type { %xiao.bytes, i64, ptr, i64, i32, i8, ptr }
%xiao.table.descriptor.v2 = type { i32, i32, %xiao.table.descriptor, ptr, i64, ptr, ptr, ptr }

%xiao.error.location = type { i64, i64, i8, [7 x i8] }

; xiao.source-map statement 0..33
; xiao.source-map statement 34..55
@.xiao.bytes0 = private unnamed_addr constant [6 x i8] c"zh-CN\00"
@.xiao.bytes1 = private unnamed_addr constant [12 x i8] c"stdlib.args\00"
@.xiao.bytes2 = private unnamed_addr constant [5 x i8] c"argv\00"
@.xiao.bytes3 = private unnamed_addr constant [12 x i8] c"stdlib.args\00"
@.xiao.bytes4 = private unnamed_addr constant [13 x i8] c"program_name\00"

declare i32 @xiao_runtime_abi_is_compatible(i32, i32)
declare i32 @xiao_runtime_args_argv(ptr, i64)
declare i32 @xiao_runtime_args_program_name(ptr, i64)
declare i32 @xiao_runtime_args_run_native(ptr)
declare i32 @xiao_runtime_array_new(ptr, i64, ptr)
declare i32 @xiao_runtime_array_push(ptr, ptr)
declare i32 @xiao_runtime_concurrency_call(ptr, ptr, i64, ptr)
declare i32 @xiao_runtime_concurrency_call_with_entry(ptr, ptr, ptr, ptr, i64, ptr)
declare i32 @xiao_runtime_concurrency_cancellation_check(ptr)
declare i32 @xiao_runtime_concurrency_finish()
declare i32 @xiao_runtime_dict_get(ptr, ptr, ptr)
declare i32 @xiao_runtime_dict_new(i32, ptr, ptr, i64, ptr)
declare i32 @xiao_runtime_dynamic_check(ptr, ptr)
declare i32 @xiao_runtime_dynamic_member_get(ptr, ptr, ptr)
declare i32 @xiao_runtime_error_attach_span(i64, i64)
declare i32 @xiao_runtime_error_checkpoint_finish()
declare i32 @xiao_runtime_error_class()
declare i32 @xiao_runtime_error_code(ptr, ptr)
declare i32 @xiao_runtime_error_exit_code()
declare i32 @xiao_runtime_error_matches(ptr)
declare i32 @xiao_runtime_error_raise_value(ptr, ptr)
declare i32 @xiao_runtime_error_take(ptr)
declare i32 @xiao_runtime_language_context_set(ptr)
declare i32 @xiao_runtime_lfloat_divide(ptr, ptr, ptr, ptr, ptr)
declare i32 @xiao_runtime_print_values_with_capabilities(ptr, i64, i64)
declare i32 @xiao_runtime_process_close_with_capabilities(ptr, i64)
declare i32 @xiao_runtime_random_seed(ptr)
declare i32 @xiao_runtime_resource_close_with_capabilities(ptr, i64)
declare i32 @xiao_runtime_set_add(ptr, ptr)
declare i32 @xiao_runtime_set_new(ptr, i64, ptr)
declare i32 @xiao_runtime_set_remove(ptr, ptr, ptr)
declare i32 @xiao_runtime_string_new(ptr, ptr)
declare i32 @xiao_runtime_table_call(ptr, ptr, i64, ptr, i64, ptr)
declare i32 @xiao_runtime_table_get(ptr, ptr, ptr)
declare i32 @xiao_runtime_table_get_value(ptr, ptr, ptr)
declare i32 @xiao_runtime_table_new(ptr, ptr)
declare i32 @xiao_runtime_table_new_v2(ptr, ptr)
declare i32 @xiao_runtime_table_new_v2_with_args(ptr, ptr, i64, ptr)
declare i32 @xiao_runtime_table_set(ptr, ptr, ptr)
declare i32 @xiao_runtime_table_set_value(ptr, ptr, ptr)
declare i32 @xiao_runtime_tuple_new(ptr, i64, ptr)
declare i32 @xiao_runtime_value_binary(ptr, ptr, ptr, ptr)
declare i32 @xiao_runtime_value_cast(ptr, ptr, ptr)
declare i32 @xiao_runtime_value_chr(ptr, ptr)
declare i32 @xiao_runtime_value_copy(ptr, ptr)
declare i32 @xiao_runtime_value_dict_keys(ptr, ptr)
declare i32 @xiao_runtime_value_dict_remove_dynamic(ptr, ptr)
declare i32 @xiao_runtime_value_dict_values(ptr, ptr)
declare i32 @xiao_runtime_value_index_get_dynamic(ptr, ptr, ptr)
declare i32 @xiao_runtime_value_index_set_dynamic(ptr, ptr, ptr)
declare i32 @xiao_runtime_value_iter_get(ptr, i64, ptr)
declare i32 @xiao_runtime_value_iter_len(ptr, ptr)
declare i32 @xiao_runtime_value_iter_snapshot(ptr, ptr)
declare i32 @xiao_runtime_value_release_weak(ptr)
declare i32 @xiao_runtime_value_select(ptr, i64, ptr, ptr)
declare i32 @xiao_runtime_value_unary(ptr, ptr, ptr)
declare ptr @llvm.stacksave()
declare void @llvm.stackrestore(ptr)
declare void @llvm.trap()
declare void @xiao_runtime_concurrency_entry_begin()
declare void @xiao_runtime_error_checkpoint_begin()
declare void @xiao_runtime_error_checkpoint_discard()
declare void @xiao_runtime_error_checkpoint_reset()
declare void @xiao_runtime_error_clear()
declare void @xiao_runtime_error_new(ptr sret(%xiao.value) align 8, ptr, ptr, ptr, ptr)
declare void @xiao_runtime_error_new_values(ptr sret(%xiao.value) align 8, ptr, ptr, ptr, ptr)
declare void @xiao_runtime_error_push_native_frame(ptr, ptr, i64, i64)
declare void @xiao_runtime_error_report()
declare void @xiao_runtime_fatal_abi()
declare void @xiao_runtime_input_with_capabilities(ptr sret(%xiao.value) align 8, ptr, i8, i64)
declare void @xiao_runtime_process_spawn_with_capabilities(ptr sret(%xiao.value) align 8, ptr, i64)
declare void @xiao_runtime_process_wait_with_capabilities(ptr sret(%xiao.value) align 8, ptr, i64)
declare void @xiao_runtime_release(ptr)
declare void @xiao_runtime_release_trace_begin()
declare void @xiao_runtime_release_trace_flush()
declare void @xiao_runtime_resource_open_directory_with_capabilities(ptr sret(%xiao.value) align 8, ptr, i64)
declare void @xiao_runtime_resource_open_file_with_capabilities(ptr sret(%xiao.value) align 8, ptr, i64)
declare void @xiao_runtime_resource_read_text_with_capabilities(ptr sret(%xiao.value) align 8, ptr, i64)
declare void @xiao_runtime_value_array(ptr sret(%xiao.value) align 8, ptr)
declare void @xiao_runtime_value_array_owned(ptr sret(%xiao.value) align 8, ptr)
declare void @xiao_runtime_value_bool(ptr sret(%xiao.value) align 8, i8)
declare void @xiao_runtime_value_dict(ptr sret(%xiao.value) align 8, ptr, i32)
declare void @xiao_runtime_value_dict_owned(ptr sret(%xiao.value) align 8, ptr, i32)
declare void @xiao_runtime_value_float(ptr sret(%xiao.value) align 8, double)
declare void @xiao_runtime_value_int(ptr sret(%xiao.value) align 8, i64)
declare void @xiao_runtime_value_lfloat(ptr sret(%xiao.value) align 8, ptr)
declare void @xiao_runtime_value_lfloat_owned(ptr sret(%xiao.value) align 8, ptr)
declare void @xiao_runtime_value_lint(ptr sret(%xiao.value) align 8, ptr)
declare void @xiao_runtime_value_lint_owned(ptr sret(%xiao.value) align 8, ptr)
declare void @xiao_runtime_value_none(ptr sret(%xiao.value) align 8)
declare void @xiao_runtime_value_release_any(ptr)
declare void @xiao_runtime_value_release_strong(ptr)
declare void @xiao_runtime_value_set(ptr sret(%xiao.value) align 8, ptr)
declare void @xiao_runtime_value_set_owned(ptr sret(%xiao.value) align 8, ptr)
declare void @xiao_runtime_value_sfloat(ptr sret(%xiao.value) align 8, float)
declare void @xiao_runtime_value_sint(ptr sret(%xiao.value) align 8, i32)
declare void @xiao_runtime_value_str(ptr sret(%xiao.value) align 8, ptr)
declare void @xiao_runtime_value_str_owned(ptr sret(%xiao.value) align 8, ptr)
declare void @xiao_runtime_value_table(ptr sret(%xiao.value) align 8, ptr)
declare void @xiao_runtime_value_table_owned(ptr sret(%xiao.value) align 8, ptr)
declare void @xiao_runtime_value_tuple(ptr sret(%xiao.value) align 8, ptr)
declare void @xiao_runtime_value_tuple_owned(ptr sret(%xiao.value) align 8, ptr)

define void @xiao_entry() {
entry:
  %slot0 = alloca %xiao.value
  %t3 = alloca %xiao.bytes
  %t8 = alloca %xiao.value
  %t12 = alloca %xiao.value, i64 1
  %t16 = alloca %xiao.value
  %t18 = alloca %xiao.value
  %t19 = alloca %xiao.value
  call void @xiao_runtime_error_checkpoint_reset()
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_release_trace_begin()
  store %xiao.value zeroinitializer, ptr %slot0
  %t0 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes0, i64 0, i64 0
  %t1 = insertvalue %xiao.bytes zeroinitializer, ptr %t0, 0
  %t2 = insertvalue %xiao.bytes %t1, i64 5, 1
  store %xiao.bytes %t2, ptr %t3
  %t4 = call i32 @xiao_runtime_language_context_set(ptr %t3)
  %t5 = icmp eq i32 %t4, 0
  br i1 %t5, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 56)
  br label %xiao.error.terminal
abi.ok0:
  %t6 = call i32 @xiao_runtime_abi_is_compatible(i32 1, i32 20)
  %t7 = icmp eq i32 %t6, 1
  br i1 %t7, label %abi.entry.ok, label %abi.fail
abi.entry.ok:
  br label %dynamic.call.arguments.ready3
dynamic.call.arguments.fail2:
  br label %xiao.error.terminal
dynamic.call.arguments.ready3:
  store %xiao.value zeroinitializer, ptr %t8
  call void @xiao.fn.274236ea8fa65b5b(ptr %t8)
  %t9 = call i32 @xiao_runtime_error_class()
  %t10 = icmp eq i32 %t9, 0
  br i1 %t10, label %dynamic.call.ok4, label %dynamic.call.fail5
dynamic.call.fail5:
  br label %xiao.error.terminal
dynamic.call.ok4:
  %t11 = load %xiao.value, ptr %t8
  %t13 = getelementptr inbounds %xiao.value, ptr %t12, i64 0
  store %xiao.value %t11, ptr %t13
  %t14 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t12, i64 1, i64 127)
  %t15 = icmp eq i32 %t14, 0
  br i1 %t15, label %abi.ok6, label %abi.status.fail7
abi.status.fail7:
  call i32 @xiao_runtime_error_attach_span(i64 34, i64 55)
  br label %xiao.error.terminal
abi.ok6:
  store %xiao.value %t11, ptr %t16
  call void @xiao_runtime_value_release_strong(ptr %t16)
  store %xiao.value zeroinitializer, ptr %t18
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t18)
  %t17 = load %xiao.value, ptr %t18
  store %xiao.value %t17, ptr %t19
  call void @xiao_runtime_value_release_strong(ptr %t19)
  %t20 = call i32 @xiao_runtime_error_class()
  %t21 = icmp eq i32 %t20, 0
  br i1 %t21, label %abi.ok8, label %abi.status.fail9
abi.status.fail9:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 56)
  br label %xiao.error.terminal
abi.ok8:
  call void @xiao_runtime_release_trace_flush()
  ret void
xiao.error.terminal:
  %t22 = call i32 @xiao_runtime_error_class()
  %t23 = icmp eq i32 %t22, 2
  br i1 %t23, label %xiao.error.report10, label %xiao.error.unmatched11
xiao.error.unmatched11:
  br label %xiao.error.report10
xiao.error.report10:
  call void @xiao_runtime_error_report()
  call void @xiao_runtime_release_trace_flush()
  %t24 = call i32 @xiao_runtime_error_exit_code()
  ret void
abi.fail:
  call void @xiao_runtime_fatal_abi()
  unreachable
}

define internal i32 @xiao_application_main() {
entry:
  call void @xiao_entry()
  %xiao_exit_code = call i32 @xiao_runtime_error_exit_code()
  ret i32 %xiao_exit_code
}

define i32 @main() {
entry:
  %status = call i32 @xiao_runtime_args_run_native(ptr @xiao_application_main)
  ret i32 %status
}

define void @xiao.fn.40b8dba18b83e293(ptr %xiao.function.result) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %t0 = alloca %xiao.value
  %t18 = alloca %xiao.bytes
  %t22 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  br label %args.arguments.ready1
args.arguments.fail0:
  br label %xiao.fn.error
args.arguments.ready1:
  store %xiao.value zeroinitializer, ptr %t0
  %t1 = call i32 @xiao_runtime_args_argv(ptr %t0, i64 127)
  switch i32 %t1, label %args.abi.failure4 [ i32 0, label %args.ok2 i32 6, label %args.error3 ]
args.abi.failure4:
  call void @xiao_runtime_value_release_strong(ptr %t0)
  call void @xiao_runtime_fatal_abi()
  unreachable
args.error3:
  call void @xiao_runtime_value_release_strong(ptr %t0)
  call i32 @xiao_runtime_error_attach_span(i64 92, i64 106)
  br label %xiao.fn.error
args.ok2:
  %t2 = load %xiao.value, ptr %t0
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t2, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 true, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  %t3 = call i32 @xiao_runtime_error_class()
  %t4 = icmp eq i32 %t3, 0
  br i1 %t4, label %abi.ok5, label %abi.status.fail6
abi.status.fail6:
  call i32 @xiao_runtime_error_attach_span(i64 61, i64 107)
  br label %xiao.fn.error
abi.ok5:
  %t5 = load ptr, ptr %xiao.return.source
  %t6 = icmp ne ptr %t5, null
  br i1 %t6, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t7 = load i1, ptr %xiao.return.temporary
  br i1 %t7, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t8 = call i32 @xiao_runtime_value_copy(ptr %t5, ptr %xiao.function.result)
  %t9 = icmp eq i32 %t8, 0
  br i1 %t9, label %abi.ok7, label %abi.status.fail8
abi.status.fail8:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 56)
  br label %xiao.fn.error
abi.ok7:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t10 = load %xiao.value, ptr %t5
  store %xiao.value %t10, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t5
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t11 = call i32 @xiao_runtime_error_class()
  %t12 = icmp eq i32 %t11, 0
  br i1 %t12, label %abi.ok9, label %abi.status.fail10
abi.status.fail10:
  call i32 @xiao_runtime_error_attach_span(i64 61, i64 107)
  br label %xiao.fn.error
abi.ok9:
  ret void
xiao.fn.error:
  %t13 = call i32 @xiao_runtime_error_class()
  %t14 = icmp eq i32 %t13, 2
  br i1 %t14, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t15 = getelementptr inbounds [12 x i8], ptr @.xiao.bytes1, i64 0, i64 0
  %t16 = insertvalue %xiao.bytes zeroinitializer, ptr %t15, 0
  %t17 = insertvalue %xiao.bytes %t16, i64 11, 1
  store %xiao.bytes %t17, ptr %t18
  %t19 = getelementptr inbounds [5 x i8], ptr @.xiao.bytes2, i64 0, i64 0
  %t20 = insertvalue %xiao.bytes zeroinitializer, ptr %t19, 0
  %t21 = insertvalue %xiao.bytes %t20, i64 4, 1
  store %xiao.bytes %t21, ptr %t22
  call void @xiao_runtime_error_push_native_frame(ptr %t18, ptr %t22, i64 61, i64 107)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.274236ea8fa65b5b(ptr %xiao.function.result) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %t0 = alloca %xiao.value
  %t18 = alloca %xiao.bytes
  %t22 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  br label %args.arguments.ready1
args.arguments.fail0:
  br label %xiao.fn.error
args.arguments.ready1:
  store %xiao.value zeroinitializer, ptr %t0
  %t1 = call i32 @xiao_runtime_args_program_name(ptr %t0, i64 127)
  switch i32 %t1, label %args.abi.failure4 [ i32 0, label %args.ok2 i32 6, label %args.error3 ]
args.abi.failure4:
  call void @xiao_runtime_value_release_strong(ptr %t0)
  call void @xiao_runtime_fatal_abi()
  unreachable
args.error3:
  call void @xiao_runtime_value_release_strong(ptr %t0)
  call i32 @xiao_runtime_error_attach_span(i64 37, i64 59)
  br label %xiao.fn.error
args.ok2:
  %t2 = load %xiao.value, ptr %t0
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t2, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 true, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  %t3 = call i32 @xiao_runtime_error_class()
  %t4 = icmp eq i32 %t3, 0
  br i1 %t4, label %abi.ok5, label %abi.status.fail6
abi.status.fail6:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 61)
  br label %xiao.fn.error
abi.ok5:
  %t5 = load ptr, ptr %xiao.return.source
  %t6 = icmp ne ptr %t5, null
  br i1 %t6, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t7 = load i1, ptr %xiao.return.temporary
  br i1 %t7, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t8 = call i32 @xiao_runtime_value_copy(ptr %t5, ptr %xiao.function.result)
  %t9 = icmp eq i32 %t8, 0
  br i1 %t9, label %abi.ok7, label %abi.status.fail8
abi.status.fail8:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 56)
  br label %xiao.fn.error
abi.ok7:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t10 = load %xiao.value, ptr %t5
  store %xiao.value %t10, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t5
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t11 = call i32 @xiao_runtime_error_class()
  %t12 = icmp eq i32 %t11, 0
  br i1 %t12, label %abi.ok9, label %abi.status.fail10
abi.status.fail10:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 61)
  br label %xiao.fn.error
abi.ok9:
  ret void
xiao.fn.error:
  %t13 = call i32 @xiao_runtime_error_class()
  %t14 = icmp eq i32 %t13, 2
  br i1 %t14, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t15 = getelementptr inbounds [12 x i8], ptr @.xiao.bytes3, i64 0, i64 0
  %t16 = insertvalue %xiao.bytes zeroinitializer, ptr %t15, 0
  %t17 = insertvalue %xiao.bytes %t16, i64 11, 1
  store %xiao.bytes %t17, ptr %t18
  %t19 = getelementptr inbounds [13 x i8], ptr @.xiao.bytes4, i64 0, i64 0
  %t20 = insertvalue %xiao.bytes zeroinitializer, ptr %t19, 0
  %t21 = insertvalue %xiao.bytes %t20, i64 12, 1
  store %xiao.bytes %t21, ptr %t22
  call void @xiao_runtime_error_push_native_frame(ptr %t18, ptr %t22, i64 0, i64 61)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}


@xiao_native_metadata = constant [107 x i8] c"\58\49\41\4F\4D\45\54\41\01\00\00\00\6B\00\00\00\14\00\01\00\00\00\00\00\14\00\01\00\00\00\00\00\00\00\00\00\00\00\00\00\1A\00\00\00\40\00\00\00\01\00\00\00\14\00\01\00\00\00\00\00\16\00\00\00\78\38\36\5F\36\34\2D\70\63\2D\77\69\6E\64\6F\77\73\2D\6D\73\76\63\05\00\00\00\30\2E\31\2E\30\08\00\00\00\70\6F\72\74\61\62\6C\65", section ".xiaomta", align 4
@llvm.used = appending global [1 x ptr] [ptr @xiao_native_metadata], section "llvm.metadata"
