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

; xiao.source-map statement 0..25
; xiao.source-map statement 26..60
; xiao.source-map statement 61..86
@.xiao.bytes0 = private unnamed_addr constant [6 x i8] c"zh-CN\00"
@.xiao.str1 = private unnamed_addr constant [17 x i8] c"XIAO_B3_TEST_ENV\00"
@.xiao.str2 = private unnamed_addr constant [17 x i8] c"XIAO_B3_TEST_ENV\00"
@.xiao.str3 = private unnamed_addr constant [6 x i8] c"probe\00"
@.xiao.str4 = private unnamed_addr constant [17 x i8] c"XIAO_B3_TEST_ENV\00"

declare i32 @xiao_runtime_abi_is_compatible(i32, i32)
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
declare i32 @xiao_runtime_env_get(ptr, ptr, i64)
declare i32 @xiao_runtime_env_run_native(ptr, i64, i32)
declare i32 @xiao_runtime_env_set(ptr, ptr, i64)
declare i32 @xiao_runtime_env_unset(ptr, ptr, i64)
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
  %xiao.statement.site0 = alloca %xiao.value
  %xiao.statement.site1 = alloca %xiao.value
  %xiao.statement.site2 = alloca %xiao.value
  %xiao.statement.site3 = alloca %xiao.value
  %xiao.statement.site4 = alloca %xiao.value
  %t3 = alloca %xiao.bytes
  %t8 = alloca %xiao.value
  %t12 = alloca %xiao.bytes
  %t13 = alloca ptr
  %t18 = alloca %xiao.value
  %t21 = alloca %xiao.value
  %t26 = alloca %xiao.value
  %t27 = alloca %xiao.value
  %t31 = alloca %xiao.bytes
  %t32 = alloca ptr
  %t37 = alloca %xiao.value
  %t43 = alloca %xiao.bytes
  %t44 = alloca ptr
  %t49 = alloca %xiao.value
  %t53 = alloca %xiao.value
  %t56 = alloca %xiao.value
  %t60 = alloca %xiao.bytes
  %t61 = alloca ptr
  %t66 = alloca %xiao.value
  %t69 = alloca %xiao.value
  %t72 = alloca %xiao.value
  store %xiao.value zeroinitializer, ptr %xiao.statement.site0
  store %xiao.value zeroinitializer, ptr %xiao.statement.site1
  store %xiao.value zeroinitializer, ptr %xiao.statement.site2
  store %xiao.value zeroinitializer, ptr %xiao.statement.site3
  store %xiao.value zeroinitializer, ptr %xiao.statement.site4
  call void @xiao_runtime_error_checkpoint_reset()
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_release_trace_begin()
  %t0 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes0, i64 0, i64 0
  %t1 = insertvalue %xiao.bytes zeroinitializer, ptr %t0, 0
  %t2 = insertvalue %xiao.bytes %t1, i64 5, 1
  store %xiao.bytes %t2, ptr %t3
  %t4 = call i32 @xiao_runtime_language_context_set(ptr %t3)
  %t5 = icmp eq i32 %t4, 0
  br i1 %t5, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 87)
  br label %xiao.error.terminal
abi.ok0:
  %t6 = call i32 @xiao_runtime_abi_is_compatible(i32 1, i32 20)
  %t7 = icmp eq i32 %t6, 1
  br i1 %t7, label %abi.entry.ok, label %abi.fail
abi.entry.ok:
  store %xiao.value zeroinitializer, ptr %t8
  %t9 = getelementptr inbounds [17 x i8], ptr @.xiao.str1, i64 0, i64 0
  %t10 = insertvalue %xiao.bytes zeroinitializer, ptr %t9, 0
  %t11 = insertvalue %xiao.bytes %t10, i64 16, 1
  store %xiao.bytes %t11, ptr %t12
  store ptr null, ptr %t13
  %t14 = call i32 @xiao_runtime_string_new(ptr %t12, ptr %t13)
  %t15 = icmp eq i32 %t14, 0
  br i1 %t15, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 87)
  br label %env.arguments.fail2
abi.ok4:
  %t16 = load ptr, ptr %t13
  store %xiao.value zeroinitializer, ptr %t18
  call void @xiao_runtime_value_str_owned(ptr sret(%xiao.value) %t18, ptr %t16)
  %t17 = load %xiao.value, ptr %t18
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site0)
  store %xiao.value %t17, ptr %xiao.statement.site0
  %t19 = call i32 @xiao_runtime_value_copy(ptr %xiao.statement.site0, ptr %t8)
  %t20 = icmp eq i32 %t19, 0
  br i1 %t20, label %abi.ok6, label %abi.status.fail7
abi.status.fail7:
  call i32 @xiao_runtime_error_attach_span(i64 6, i64 24)
  br label %env.arguments.fail2
abi.ok6:
  br label %env.arguments.ready3
env.arguments.fail2:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  br label %xiao.error.terminal
env.arguments.ready3:
  store %xiao.value zeroinitializer, ptr %t21
  %t22 = call i32 @xiao_runtime_env_get(ptr %t8, ptr %t21, i64 160)
  call void @xiao_runtime_value_release_strong(ptr %t8)
  switch i32 %t22, label %env.abi.failure10 [ i32 0, label %env.ok8 i32 6, label %env.error9 i32 8, label %env.error9 ]
env.abi.failure10:
  call void @xiao_runtime_value_release_strong(ptr %t21)
  call void @xiao_runtime_fatal_abi()
  unreachable
env.error9:
  call void @xiao_runtime_value_release_strong(ptr %t21)
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 25)
  br label %xiao.error.terminal
env.ok8:
  %t23 = load %xiao.value, ptr %t21
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site1)
  store %xiao.value %t23, ptr %xiao.statement.site1
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site1)
  %t24 = call i32 @xiao_runtime_error_class()
  %t25 = icmp eq i32 %t24, 0
  br i1 %t25, label %abi.error.ok11, label %abi.error.fail12
abi.error.fail12:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 25)
  br label %xiao.error.terminal
abi.error.ok11:
  store %xiao.value zeroinitializer, ptr %t26
  store %xiao.value zeroinitializer, ptr %t27
  %t28 = getelementptr inbounds [17 x i8], ptr @.xiao.str2, i64 0, i64 0
  %t29 = insertvalue %xiao.bytes zeroinitializer, ptr %t28, 0
  %t30 = insertvalue %xiao.bytes %t29, i64 16, 1
  store %xiao.bytes %t30, ptr %t31
  store ptr null, ptr %t32
  %t33 = call i32 @xiao_runtime_string_new(ptr %t31, ptr %t32)
  %t34 = icmp eq i32 %t33, 0
  br i1 %t34, label %abi.ok15, label %abi.status.fail16
abi.status.fail16:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 87)
  br label %env.arguments.fail13
abi.ok15:
  %t35 = load ptr, ptr %t32
  store %xiao.value zeroinitializer, ptr %t37
  call void @xiao_runtime_value_str_owned(ptr sret(%xiao.value) %t37, ptr %t35)
  %t36 = load %xiao.value, ptr %t37
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site2)
  store %xiao.value %t36, ptr %xiao.statement.site2
  %t38 = call i32 @xiao_runtime_value_copy(ptr %xiao.statement.site2, ptr %t26)
  %t39 = icmp eq i32 %t38, 0
  br i1 %t39, label %abi.ok17, label %abi.status.fail18
abi.status.fail18:
  call i32 @xiao_runtime_error_attach_span(i64 32, i64 50)
  br label %env.arguments.fail13
abi.ok17:
  %t40 = getelementptr inbounds [6 x i8], ptr @.xiao.str3, i64 0, i64 0
  %t41 = insertvalue %xiao.bytes zeroinitializer, ptr %t40, 0
  %t42 = insertvalue %xiao.bytes %t41, i64 5, 1
  store %xiao.bytes %t42, ptr %t43
  store ptr null, ptr %t44
  %t45 = call i32 @xiao_runtime_string_new(ptr %t43, ptr %t44)
  %t46 = icmp eq i32 %t45, 0
  br i1 %t46, label %abi.ok19, label %abi.status.fail20
abi.status.fail20:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 87)
  br label %env.arguments.fail13
abi.ok19:
  %t47 = load ptr, ptr %t44
  store %xiao.value zeroinitializer, ptr %t49
  call void @xiao_runtime_value_str_owned(ptr sret(%xiao.value) %t49, ptr %t47)
  %t48 = load %xiao.value, ptr %t49
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site3)
  store %xiao.value %t48, ptr %xiao.statement.site3
  %t50 = call i32 @xiao_runtime_value_copy(ptr %xiao.statement.site3, ptr %t27)
  %t51 = icmp eq i32 %t50, 0
  br i1 %t51, label %abi.ok21, label %abi.status.fail22
abi.status.fail22:
  call i32 @xiao_runtime_error_attach_span(i64 52, i64 59)
  br label %env.arguments.fail13
abi.ok21:
  br label %env.arguments.ready14
env.arguments.fail13:
  call void @xiao_runtime_value_release_strong(ptr %t27)
  call void @xiao_runtime_value_release_strong(ptr %t26)
  br label %xiao.error.terminal
env.arguments.ready14:
  %t52 = call i32 @xiao_runtime_env_set(ptr %t26, ptr %t27, i64 160)
  call void @xiao_runtime_value_release_strong(ptr %t27)
  call void @xiao_runtime_value_release_strong(ptr %t26)
  switch i32 %t52, label %env.abi.failure25 [ i32 0, label %env.ok23 i32 6, label %env.error24 i32 8, label %env.error24 ]
env.abi.failure25:
  call void @xiao_runtime_fatal_abi()
  unreachable
env.error24:
  call i32 @xiao_runtime_error_attach_span(i64 26, i64 60)
  br label %xiao.error.terminal
env.ok23:
  store %xiao.value zeroinitializer, ptr %t53
  call void @xiao_runtime_value_release_strong(ptr %t53)
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site2)
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site3)
  %t54 = call i32 @xiao_runtime_error_class()
  %t55 = icmp eq i32 %t54, 0
  br i1 %t55, label %abi.error.ok26, label %abi.error.fail27
abi.error.fail27:
  call i32 @xiao_runtime_error_attach_span(i64 26, i64 60)
  br label %xiao.error.terminal
abi.error.ok26:
  store %xiao.value zeroinitializer, ptr %t56
  %t57 = getelementptr inbounds [17 x i8], ptr @.xiao.str4, i64 0, i64 0
  %t58 = insertvalue %xiao.bytes zeroinitializer, ptr %t57, 0
  %t59 = insertvalue %xiao.bytes %t58, i64 16, 1
  store %xiao.bytes %t59, ptr %t60
  store ptr null, ptr %t61
  %t62 = call i32 @xiao_runtime_string_new(ptr %t60, ptr %t61)
  %t63 = icmp eq i32 %t62, 0
  br i1 %t63, label %abi.ok30, label %abi.status.fail31
abi.status.fail31:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 87)
  br label %env.arguments.fail28
abi.ok30:
  %t64 = load ptr, ptr %t61
  store %xiao.value zeroinitializer, ptr %t66
  call void @xiao_runtime_value_str_owned(ptr sret(%xiao.value) %t66, ptr %t64)
  %t65 = load %xiao.value, ptr %t66
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site4)
  store %xiao.value %t65, ptr %xiao.statement.site4
  %t67 = call i32 @xiao_runtime_value_copy(ptr %xiao.statement.site4, ptr %t56)
  %t68 = icmp eq i32 %t67, 0
  br i1 %t68, label %abi.ok32, label %abi.status.fail33
abi.status.fail33:
  call i32 @xiao_runtime_error_attach_span(i64 67, i64 85)
  br label %env.arguments.fail28
abi.ok32:
  br label %env.arguments.ready29
env.arguments.fail28:
  call void @xiao_runtime_value_release_strong(ptr %t56)
  br label %xiao.error.terminal
env.arguments.ready29:
  store %xiao.value zeroinitializer, ptr %t69
  %t70 = call i32 @xiao_runtime_env_unset(ptr %t56, ptr %t69, i64 160)
  call void @xiao_runtime_value_release_strong(ptr %t56)
  switch i32 %t70, label %env.abi.failure36 [ i32 0, label %env.ok34 i32 6, label %env.error35 i32 8, label %env.error35 ]
env.abi.failure36:
  call void @xiao_runtime_value_release_strong(ptr %t69)
  call void @xiao_runtime_fatal_abi()
  unreachable
env.error35:
  call void @xiao_runtime_value_release_strong(ptr %t69)
  call i32 @xiao_runtime_error_attach_span(i64 61, i64 86)
  br label %xiao.error.terminal
env.ok34:
  %t71 = load %xiao.value, ptr %t69
  store %xiao.value %t71, ptr %t72
  call void @xiao_runtime_value_release_strong(ptr %t72)
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site4)
  %t73 = call i32 @xiao_runtime_error_class()
  %t74 = icmp eq i32 %t73, 0
  br i1 %t74, label %abi.error.ok37, label %abi.error.fail38
abi.error.fail38:
  call i32 @xiao_runtime_error_attach_span(i64 61, i64 86)
  br label %xiao.error.terminal
abi.error.ok37:
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site1)
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site2)
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site3)
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site4)
  %t75 = call i32 @xiao_runtime_error_class()
  %t76 = icmp eq i32 %t75, 0
  br i1 %t76, label %abi.ok39, label %abi.status.fail40
abi.status.fail40:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 87)
  br label %xiao.error.terminal
abi.ok39:
  call void @xiao_runtime_release_trace_flush()
  ret void
xiao.error.terminal:
  %t77 = call i32 @xiao_runtime_error_class()
  %t78 = icmp eq i32 %t77, 2
  br i1 %t78, label %xiao.error.report41, label %xiao.error.unmatched42
xiao.error.unmatched42:
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site1)
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site2)
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site3)
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site4)
  br label %xiao.error.report41
xiao.error.report41:
  call void @xiao_runtime_error_report()
  call void @xiao_runtime_release_trace_flush()
  %t79 = call i32 @xiao_runtime_error_exit_code()
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
  %status = call i32 @xiao_runtime_env_run_native(ptr @xiao_application_main, i64 160, i32 0)
  ret i32 %status
}


@xiao_native_metadata = constant [107 x i8] c"\58\49\41\4F\4D\45\54\41\01\00\00\00\6B\00\00\00\14\00\01\00\00\00\00\00\14\00\01\00\00\00\00\00\00\00\00\00\00\00\00\00\1B\00\00\00\40\00\00\00\01\00\00\00\14\00\01\00\00\00\00\00\16\00\00\00\78\38\36\5F\36\34\2D\70\63\2D\77\69\6E\64\6F\77\73\2D\6D\73\76\63\05\00\00\00\30\2E\31\2E\30\08\00\00\00\70\6F\72\74\61\62\6C\65", section ".xiaomta", align 4
@llvm.used = appending global [1 x ptr] [ptr @xiao_native_metadata], section "llvm.metadata"
