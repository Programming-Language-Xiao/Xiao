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

; xiao.source-map statement 0..35
; xiao.source-map statement 36..50
; xiao.source-map statement 51..73
; xiao.source-map statement 74..104
; xiao.source-map statement 105..141
; xiao.source-map statement 142..156
; xiao.source-map statement 157..179
; xiao.source-map statement 180..211
; xiao.source-map statement 212..247
; xiao.source-map statement 248..262
; xiao.source-map statement 263..285
; xiao.source-map statement 286..316
; xiao.source-map statement 317..353
; xiao.source-map statement 354..368
; xiao.source-map statement 369..391
; xiao.source-map statement 392..423
; xiao.source-map statement 424..460
; xiao.source-map statement 461..476
; xiao.source-map statement 477..500
; xiao.source-map statement 501..533
; xiao.source-map statement 534..568
; xiao.source-map statement 569..584
; xiao.source-map statement 585..608
; xiao.source-map statement 609..639
; xiao.source-map statement 640..674
; xiao.source-map statement 675..694
; xiao.source-map statement 695..729
; xiao.source-map statement 730..771
; xiao.source-map statement 772..806
; xiao.source-map statement 807..826
; xiao.source-map statement 827..861
; xiao.source-map statement 862..903
; xiao.source-map statement 904..938
; xiao.source-map statement 939..958
; xiao.source-map statement 959..996
; xiao.source-map statement 997..1041
; xiao.source-map statement 1042..1076
; xiao.source-map statement 1077..1091
; xiao.source-map statement 1092..1114
; xiao.source-map statement 1115..1144
; xiao.source-map statement 1145..1180
; xiao.source-map statement 1181..1196
; xiao.source-map statement 1197..1220
; xiao.source-map statement 1221..1250
; xiao.source-map statement 1251..1286
; xiao.source-map statement 1287..1302
; xiao.source-map statement 1303..1326
; xiao.source-map statement 1327..1356
; xiao.source-map statement 1357..1392
; xiao.source-map statement 1393..1408
; xiao.source-map statement 1409..1432
; xiao.source-map statement 1433..1462
; xiao.source-map statement 1463..1497
; xiao.source-map statement 1498..1513
; xiao.source-map statement 1514..1537
; xiao.source-map statement 1538..1566
; xiao.source-map statement 1567..1604
; xiao.source-map statement 1605..1620
; xiao.source-map statement 1621..1644
; xiao.source-map statement 1645..1676
; xiao.source-map statement 1677..1738
; xiao.source-map statement 1705..1722
; xiao.source-map return 1727..1737
; xiao.source-map statement 1738..1791
; xiao.source-map statement 1762..1775
; xiao.source-map return 1780..1790
; xiao.source-map statement 1791..1838
; xiao.source-map statement 1839..1878
; xiao.source-map statement 1879..1922
; xiao.source-map statement 1923..1947
; xiao.source-map statement 1948..1972
; xiao.source-map statement 1973..2014
; xiao.source-map statement 2015..2041
; xiao.source-map try 2042..2150
; xiao.source-map statement 2050..2058
; xiao.source-map catch 2059..2123
; xiao.source-map statement 2096..2122
; xiao.source-map statement 2135..2149
@.xiao.bytes0 = private unnamed_addr constant [6 x i8] c"zh-CN\00"
@.xiao.bytes1 = private unnamed_addr constant [6 x i8] c"minus\00"
@.xiao.bytes2 = private unnamed_addr constant [6 x i8] c"minus\00"
@.xiao.bytes3 = private unnamed_addr constant [6 x i8] c"minus\00"
@.xiao.bytes4 = private unnamed_addr constant [6 x i8] c"minus\00"
@.xiao.bytes5 = private unnamed_addr constant [6 x i8] c"minus\00"
@.xiao.bytes6 = private unnamed_addr constant [6 x i8] c"minus\00"
@.xiao.bytes7 = private unnamed_addr constant [6 x i8] c"minus\00"
@.xiao.str8 = private unnamed_addr constant [6 x i8] c"clean\00"
@.xiao.bytes9 = private unnamed_addr constant [16 x i8] c"ArithmeticError\00"
@.xiao.str10 = private unnamed_addr constant [6 x i8] c"clean\00"
@.xiao.str11 = private unnamed_addr constant [6 x i8] c"clean\00"
@.xiao.bytes12 = private unnamed_addr constant [12 x i8] c"error_value\00"
@.xiao.str13 = private unnamed_addr constant [16 x i8] c"X06-RUNTIME-004\00"
@.xiao.bytes14 = private unnamed_addr constant [12 x i8] c"MemoryError\00"
@.xiao.bytes15 = private unnamed_addr constant [13 x i8] c"stdlib.cmath\00"
@.xiao.bytes16 = private unnamed_addr constant [4 x i8] c"abs\00"
@.xiao.str17 = private unnamed_addr constant [5 x i8] c"base\00"
@.xiao.bytes18 = private unnamed_addr constant [6 x i8] c"entry\00"
@.xiao.bytes19 = private unnamed_addr constant [5 x i8] c"base\00"
@.xiao.bytes20 = private unnamed_addr constant [13 x i8] c"stdlib.cmath\00"
@.xiao.bytes21 = private unnamed_addr constant [5 x i8] c"ceil\00"
@.xiao.bytes22 = private unnamed_addr constant [3 x i8] c"gt\00"
@.xiao.str23 = private unnamed_addr constant [20 x i8] c"MATH_INVALID_BOUNDS\00"
@.xiao.bytes24 = private unnamed_addr constant [6 x i8] c"Error\00"
@.xiao.bytes25 = private unnamed_addr constant [3 x i8] c"lt\00"
@.xiao.bytes26 = private unnamed_addr constant [3 x i8] c"gt\00"
@.xiao.bytes27 = private unnamed_addr constant [12 x i8] c"stdlib.math\00"
@.xiao.bytes28 = private unnamed_addr constant [6 x i8] c"clamp\00"
@.xiao.bytes29 = private unnamed_addr constant [13 x i8] c"stdlib.cmath\00"
@.xiao.bytes30 = private unnamed_addr constant [4 x i8] c"cos\00"
@.xiao.bytes31 = private unnamed_addr constant [13 x i8] c"stdlib.cmath\00"
@.xiao.bytes32 = private unnamed_addr constant [4 x i8] c"exp\00"
@.xiao.str33 = private unnamed_addr constant [9 x i8] c"exponent\00"
@.xiao.bytes34 = private unnamed_addr constant [6 x i8] c"entry\00"
@.xiao.bytes35 = private unnamed_addr constant [9 x i8] c"exponent\00"
@.xiao.bytes36 = private unnamed_addr constant [13 x i8] c"stdlib.cmath\00"
@.xiao.bytes37 = private unnamed_addr constant [5 x i8] c"sqrt\00"
@.xiao.bytes38 = private unnamed_addr constant [13 x i8] c"stdlib.cmath\00"
@.xiao.bytes39 = private unnamed_addr constant [6 x i8] c"floor\00"
@.xiao.bytes40 = private unnamed_addr constant [13 x i8] c"stdlib.cmath\00"
@.xiao.bytes41 = private unnamed_addr constant [4 x i8] c"tan\00"
@.xiao.bytes42 = private unnamed_addr constant [13 x i8] c"stdlib.cmath\00"
@.xiao.bytes43 = private unnamed_addr constant [3 x i8] c"ln\00"
@.xiao.bytes44 = private unnamed_addr constant [13 x i8] c"stdlib.cmath\00"
@.xiao.bytes45 = private unnamed_addr constant [6 x i8] c"log10\00"
@.xiao.bytes46 = private unnamed_addr constant [13 x i8] c"stdlib.cmath\00"
@.xiao.bytes47 = private unnamed_addr constant [6 x i8] c"round\00"
@.xiao.bytes48 = private unnamed_addr constant [13 x i8] c"stdlib.cmath\00"
@.xiao.bytes49 = private unnamed_addr constant [6 x i8] c"trunc\00"
@.xiao.bytes50 = private unnamed_addr constant [13 x i8] c"stdlib.cmath\00"
@.xiao.bytes51 = private unnamed_addr constant [4 x i8] c"min\00"
@.xiao.bytes52 = private unnamed_addr constant [13 x i8] c"stdlib.cmath\00"
@.xiao.bytes53 = private unnamed_addr constant [4 x i8] c"max\00"
@.xiao.bytes54 = private unnamed_addr constant [13 x i8] c"stdlib.cmath\00"
@.xiao.bytes55 = private unnamed_addr constant [4 x i8] c"pow\00"
@.xiao.bytes56 = private unnamed_addr constant [13 x i8] c"stdlib.cmath\00"
@.xiao.bytes57 = private unnamed_addr constant [4 x i8] c"sin\00"
@.xiao.bytes58 = private unnamed_addr constant [3 x i8] c"gt\00"
@.xiao.bytes59 = private unnamed_addr constant [6 x i8] c"minus\00"
@.xiao.bytes60 = private unnamed_addr constant [3 x i8] c"gt\00"
@.xiao.bytes61 = private unnamed_addr constant [6 x i8] c"minus\00"
@.xiao.bytes62 = private unnamed_addr constant [3 x i8] c"ne\00"
@.xiao.bytes63 = private unnamed_addr constant [6 x i8] c"minus\00"
@.xiao.bytes64 = private unnamed_addr constant [3 x i8] c"eq\00"
@.xiao.bytes65 = private unnamed_addr constant [4 x i8] c"rem\00"
@.xiao.bytes66 = private unnamed_addr constant [6 x i8] c"minus\00"
@.xiao.bytes67 = private unnamed_addr constant [12 x i8] c"stdlib.math\00"
@.xiao.bytes68 = private unnamed_addr constant [4 x i8] c"gcd\00"
@.xiao.bytes69 = private unnamed_addr constant [3 x i8] c"lt\00"
@.xiao.bytes70 = private unnamed_addr constant [12 x i8] c"stdlib.math\00"
@.xiao.bytes71 = private unnamed_addr constant [4 x i8] c"min\00"
@.xiao.bytes72 = private unnamed_addr constant [9 x i8] c"iterable\00"
@.xiao.bytes73 = private unnamed_addr constant [4 x i8] c"add\00"
@.xiao.bytes74 = private unnamed_addr constant [3 x i8] c"gt\00"
@.xiao.str75 = private unnamed_addr constant [26 x i8] c"STATISTICS_LIMIT_EXCEEDED\00"
@.xiao.bytes76 = private unnamed_addr constant [14 x i8] c"ResourceError\00"
@.xiao.bytes77 = private unnamed_addr constant [3 x i8] c"eq\00"
@.xiao.str78 = private unnamed_addr constant [17 x i8] c"STATISTICS_EMPTY\00"
@.xiao.bytes79 = private unnamed_addr constant [6 x i8] c"Error\00"
@.xiao.bytes80 = private unnamed_addr constant [9 x i8] c"iterable\00"
@.xiao.bytes81 = private unnamed_addr constant [3 x i8] c"lt\00"
@.xiao.bytes82 = private unnamed_addr constant [18 x i8] c"stdlib.statistics\00"
@.xiao.bytes83 = private unnamed_addr constant [4 x i8] c"min\00"
@.xiao.bytes84 = private unnamed_addr constant [9 x i8] c"iterable\00"
@.xiao.bytes85 = private unnamed_addr constant [4 x i8] c"add\00"
@.xiao.bytes86 = private unnamed_addr constant [3 x i8] c"gt\00"
@.xiao.str87 = private unnamed_addr constant [26 x i8] c"STATISTICS_LIMIT_EXCEEDED\00"
@.xiao.bytes88 = private unnamed_addr constant [14 x i8] c"ResourceError\00"
@.xiao.bytes89 = private unnamed_addr constant [9 x i8] c"iterable\00"
@.xiao.bytes90 = private unnamed_addr constant [4 x i8] c"add\00"
@.xiao.bytes91 = private unnamed_addr constant [18 x i8] c"stdlib.statistics\00"
@.xiao.bytes92 = private unnamed_addr constant [4 x i8] c"sum\00"
@.xiao.bytes93 = private unnamed_addr constant [3 x i8] c"lt\00"
@.xiao.bytes94 = private unnamed_addr constant [6 x i8] c"minus\00"
@.xiao.bytes95 = private unnamed_addr constant [12 x i8] c"stdlib.math\00"
@.xiao.bytes96 = private unnamed_addr constant [4 x i8] c"abs\00"
@.xiao.bytes97 = private unnamed_addr constant [3 x i8] c"gt\00"
@.xiao.bytes98 = private unnamed_addr constant [12 x i8] c"stdlib.math\00"
@.xiao.bytes99 = private unnamed_addr constant [4 x i8] c"max\00"
@.xiao.bytes100 = private unnamed_addr constant [3 x i8] c"lt\00"
@.xiao.str101 = private unnamed_addr constant [23 x i8] c"MATH_NEGATIVE_EXPONENT\00"
@.xiao.bytes102 = private unnamed_addr constant [6 x i8] c"Error\00"
@.xiao.bytes103 = private unnamed_addr constant [3 x i8] c"gt\00"
@.xiao.bytes104 = private unnamed_addr constant [4 x i8] c"rem\00"
@.xiao.bytes105 = private unnamed_addr constant [3 x i8] c"eq\00"
@.xiao.bytes106 = private unnamed_addr constant [4 x i8] c"mul\00"
@.xiao.bytes107 = private unnamed_addr constant [10 x i8] c"floor_div\00"
@.xiao.bytes108 = private unnamed_addr constant [3 x i8] c"gt\00"
@.xiao.bytes109 = private unnamed_addr constant [4 x i8] c"mul\00"
@.xiao.bytes110 = private unnamed_addr constant [12 x i8] c"stdlib.math\00"
@.xiao.bytes111 = private unnamed_addr constant [4 x i8] c"pow\00"
@.xiao.bytes112 = private unnamed_addr constant [9 x i8] c"iterable\00"
@.xiao.bytes113 = private unnamed_addr constant [4 x i8] c"add\00"
@.xiao.bytes114 = private unnamed_addr constant [3 x i8] c"gt\00"
@.xiao.str115 = private unnamed_addr constant [26 x i8] c"STATISTICS_LIMIT_EXCEEDED\00"
@.xiao.bytes116 = private unnamed_addr constant [14 x i8] c"ResourceError\00"
@.xiao.bytes117 = private unnamed_addr constant [3 x i8] c"eq\00"
@.xiao.str118 = private unnamed_addr constant [17 x i8] c"STATISTICS_EMPTY\00"
@.xiao.bytes119 = private unnamed_addr constant [6 x i8] c"Error\00"
@.xiao.bytes120 = private unnamed_addr constant [9 x i8] c"iterable\00"
@.xiao.bytes121 = private unnamed_addr constant [3 x i8] c"gt\00"
@.xiao.bytes122 = private unnamed_addr constant [18 x i8] c"stdlib.statistics\00"
@.xiao.bytes123 = private unnamed_addr constant [4 x i8] c"max\00"

declare i32 @xiao_runtime_abi_is_compatible(i32, i32)
declare i32 @xiao_runtime_array_new(ptr, i64, ptr)
declare i32 @xiao_runtime_array_push(ptr, ptr)
declare i32 @xiao_runtime_cmath_abs(ptr, ptr)
declare i32 @xiao_runtime_cmath_ceil(ptr, ptr)
declare i32 @xiao_runtime_cmath_cos(ptr, ptr)
declare i32 @xiao_runtime_cmath_exp(ptr, ptr)
declare i32 @xiao_runtime_cmath_floor(ptr, ptr)
declare i32 @xiao_runtime_cmath_ln(ptr, ptr)
declare i32 @xiao_runtime_cmath_log10(ptr, ptr)
declare i32 @xiao_runtime_cmath_max(ptr, ptr, ptr)
declare i32 @xiao_runtime_cmath_min(ptr, ptr, ptr)
declare i32 @xiao_runtime_cmath_pow(ptr, ptr, ptr)
declare i32 @xiao_runtime_cmath_round(ptr, ptr)
declare i32 @xiao_runtime_cmath_sin(ptr, ptr)
declare i32 @xiao_runtime_cmath_sqrt(ptr, ptr)
declare i32 @xiao_runtime_cmath_tan(ptr, ptr)
declare i32 @xiao_runtime_cmath_trunc(ptr, ptr)
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
  %xiao.statement.site0 = alloca %xiao.value
  %slot0 = alloca %xiao.value
  %slot1 = alloca %xiao.value
  %slot10 = alloca %xiao.value
  %slot11 = alloca %xiao.value
  %slot12 = alloca %xiao.value
  %slot13 = alloca %xiao.value
  %slot14 = alloca %xiao.value
  %slot2 = alloca %xiao.value
  %slot3 = alloca %xiao.value
  %slot4 = alloca %xiao.value
  %slot5 = alloca %xiao.value
  %slot6 = alloca %xiao.value
  %slot7 = alloca %xiao.value
  %slot8 = alloca %xiao.value
  %slot9 = alloca %xiao.value
  %slot18 = alloca %xiao.value
  %slot17 = alloca %xiao.value
  %slot15 = alloca %xiao.value
  %slot16 = alloca %xiao.value
  %t3 = alloca %xiao.bytes
  %t8 = alloca %xiao.value
  %t10 = alloca %xiao.value
  %t11 = alloca %xiao.value
  %t15 = alloca %xiao.value, i64 1
  %t19 = alloca %xiao.value
  %t21 = alloca %xiao.value
  %t22 = alloca %xiao.value
  %t23 = alloca %xiao.value
  %t25 = alloca %xiao.value
  %t26 = alloca %xiao.value
  %t30 = alloca %xiao.value, i64 1
  %t34 = alloca %xiao.value
  %t36 = alloca %xiao.value
  %t37 = alloca %xiao.value
  %t38 = alloca %xiao.value
  %t40 = alloca %xiao.value
  %t41 = alloca %xiao.value
  %t44 = alloca %xiao.value, i64 1
  %t48 = alloca %xiao.value
  %t50 = alloca %xiao.value
  %t51 = alloca %xiao.value
  %t52 = alloca %xiao.value
  %t54 = alloca %xiao.value
  %t55 = alloca %xiao.value
  %t59 = alloca %xiao.value, i64 1
  %t63 = alloca %xiao.value
  %t65 = alloca %xiao.value
  %t66 = alloca %xiao.value
  %t67 = alloca %xiao.value
  %t69 = alloca %xiao.value
  %t70 = alloca %xiao.value
  %t74 = alloca %xiao.value, i64 1
  %t78 = alloca %xiao.value
  %t80 = alloca %xiao.value
  %t81 = alloca %xiao.value
  %t82 = alloca %xiao.value
  %t84 = alloca %xiao.value
  %t85 = alloca %xiao.value
  %t88 = alloca %xiao.value, i64 1
  %t92 = alloca %xiao.value
  %t94 = alloca %xiao.value
  %t95 = alloca %xiao.value
  %t96 = alloca %xiao.value
  %t98 = alloca %xiao.value
  %t99 = alloca %xiao.value
  %t103 = alloca %xiao.value, i64 1
  %t107 = alloca %xiao.value
  %t109 = alloca %xiao.value
  %t110 = alloca %xiao.value
  %t111 = alloca %xiao.value
  %t113 = alloca %xiao.value
  %t114 = alloca %xiao.value
  %t118 = alloca %xiao.value, i64 1
  %t122 = alloca %xiao.value
  %t124 = alloca %xiao.value
  %t125 = alloca %xiao.value
  %t126 = alloca %xiao.value
  %t128 = alloca %xiao.value
  %t129 = alloca %xiao.value
  %t132 = alloca %xiao.value, i64 1
  %t136 = alloca %xiao.value
  %t138 = alloca %xiao.value
  %t139 = alloca %xiao.value
  %t140 = alloca %xiao.value
  %t142 = alloca %xiao.value
  %t143 = alloca %xiao.value
  %t147 = alloca %xiao.value, i64 1
  %t151 = alloca %xiao.value
  %t153 = alloca %xiao.value
  %t154 = alloca %xiao.value
  %t155 = alloca %xiao.value
  %t157 = alloca %xiao.value
  %t158 = alloca %xiao.value
  %t162 = alloca %xiao.value, i64 1
  %t166 = alloca %xiao.value
  %t168 = alloca %xiao.value
  %t169 = alloca %xiao.value
  %t170 = alloca %xiao.value
  %t172 = alloca %xiao.value
  %t173 = alloca %xiao.value
  %t176 = alloca %xiao.value, i64 1
  %t180 = alloca %xiao.value
  %t182 = alloca %xiao.value
  %t183 = alloca %xiao.value
  %t184 = alloca %xiao.value
  %t186 = alloca %xiao.value
  %t190 = alloca %xiao.bytes
  %t191 = alloca %xiao.value
  %t192 = alloca %xiao.value
  %t196 = alloca %xiao.value
  %t197 = alloca %xiao.value
  %t198 = alloca %xiao.value
  %t202 = alloca %xiao.value, i64 1
  %t206 = alloca %xiao.value
  %t208 = alloca %xiao.value
  %t209 = alloca %xiao.value
  %t210 = alloca %xiao.value
  %t212 = alloca %xiao.value
  %t216 = alloca %xiao.bytes
  %t217 = alloca %xiao.value
  %t218 = alloca %xiao.value
  %t222 = alloca %xiao.value
  %t223 = alloca %xiao.value
  %t224 = alloca %xiao.value
  %t228 = alloca %xiao.value, i64 1
  %t232 = alloca %xiao.value
  %t234 = alloca %xiao.value
  %t235 = alloca %xiao.value
  %t236 = alloca %xiao.value
  %t238 = alloca %xiao.value
  %t242 = alloca %xiao.bytes
  %t243 = alloca %xiao.value
  %t244 = alloca %xiao.value
  %t248 = alloca %xiao.value
  %t249 = alloca %xiao.value
  %t250 = alloca %xiao.value
  %t253 = alloca %xiao.value, i64 1
  %t257 = alloca %xiao.value
  %t259 = alloca %xiao.value
  %t260 = alloca %xiao.value
  %t261 = alloca %xiao.value
  %t263 = alloca %xiao.value
  %t267 = alloca %xiao.bytes
  %t268 = alloca %xiao.value
  %t269 = alloca %xiao.value
  %t273 = alloca %xiao.value
  %t274 = alloca %xiao.value
  %t275 = alloca %xiao.value
  %t279 = alloca %xiao.value, i64 1
  %t283 = alloca %xiao.value
  %t285 = alloca %xiao.value
  %t286 = alloca %xiao.value
  %t287 = alloca %xiao.value
  %t289 = alloca %xiao.value
  %t293 = alloca %xiao.bytes
  %t294 = alloca %xiao.value
  %t295 = alloca %xiao.value
  %t299 = alloca %xiao.value
  %t300 = alloca %xiao.value
  %t301 = alloca %xiao.value
  %t305 = alloca %xiao.value, i64 1
  %t309 = alloca %xiao.value
  %t311 = alloca %xiao.value
  %t312 = alloca %xiao.value
  %t313 = alloca %xiao.value
  %t315 = alloca %xiao.value
  %t319 = alloca %xiao.bytes
  %t320 = alloca %xiao.value
  %t321 = alloca %xiao.value
  %t325 = alloca %xiao.value
  %t326 = alloca %xiao.value
  %t327 = alloca %xiao.value
  %t330 = alloca %xiao.value, i64 1
  %t334 = alloca %xiao.value
  %t336 = alloca %xiao.value
  %t337 = alloca %xiao.value
  %t338 = alloca %xiao.value
  %t339 = alloca %xiao.value
  %t341 = alloca %xiao.value
  %t343 = alloca %xiao.value
  %t344 = alloca %xiao.value
  %t348 = alloca %xiao.value, i64 1
  %t352 = alloca %xiao.value
  %t354 = alloca %xiao.value
  %t355 = alloca %xiao.value
  %t356 = alloca %xiao.value
  %t357 = alloca %xiao.value
  %t359 = alloca %xiao.value
  %t361 = alloca %xiao.value
  %t362 = alloca %xiao.value
  %t366 = alloca %xiao.value, i64 1
  %t370 = alloca %xiao.value
  %t372 = alloca %xiao.value
  %t373 = alloca %xiao.value
  %t374 = alloca %xiao.value
  %t375 = alloca %xiao.value
  %t377 = alloca %xiao.value
  %t379 = alloca %xiao.value
  %t380 = alloca %xiao.value
  %t383 = alloca %xiao.value, i64 1
  %t387 = alloca %xiao.value
  %t389 = alloca %xiao.value
  %t390 = alloca %xiao.value
  %t391 = alloca %xiao.value
  %t392 = alloca %xiao.value
  %t394 = alloca %xiao.value
  %t396 = alloca %xiao.value
  %t397 = alloca %xiao.value
  %t401 = alloca %xiao.value, i64 1
  %t405 = alloca %xiao.value
  %t407 = alloca %xiao.value
  %t408 = alloca %xiao.value
  %t409 = alloca %xiao.value
  %t410 = alloca %xiao.value
  %t412 = alloca %xiao.value
  %t414 = alloca %xiao.value
  %t415 = alloca %xiao.value
  %t419 = alloca %xiao.value, i64 1
  %t423 = alloca %xiao.value
  %t425 = alloca %xiao.value
  %t426 = alloca %xiao.value
  %t427 = alloca %xiao.value
  %t428 = alloca %xiao.value
  %t430 = alloca %xiao.value
  %t432 = alloca %xiao.value
  %t433 = alloca %xiao.value
  %t436 = alloca %xiao.value, i64 1
  %t440 = alloca %xiao.value
  %t442 = alloca %xiao.value
  %t443 = alloca %xiao.value
  %t444 = alloca %xiao.value
  %t445 = alloca %xiao.value
  %t447 = alloca %xiao.value
  %t449 = alloca %xiao.value
  %t450 = alloca %xiao.value
  %t454 = alloca %xiao.value, i64 1
  %t458 = alloca %xiao.value
  %t460 = alloca %xiao.value
  %t461 = alloca %xiao.value
  %t462 = alloca %xiao.value
  %t463 = alloca %xiao.value
  %t465 = alloca %xiao.value
  %t467 = alloca %xiao.value
  %t468 = alloca %xiao.value
  %t472 = alloca %xiao.value, i64 1
  %t476 = alloca %xiao.value
  %t478 = alloca %xiao.value
  %t479 = alloca %xiao.value
  %t480 = alloca %xiao.value
  %t481 = alloca %xiao.value
  %t483 = alloca %xiao.value
  %t485 = alloca %xiao.value
  %t486 = alloca %xiao.value
  %t489 = alloca %xiao.value, i64 1
  %t493 = alloca %xiao.value
  %t495 = alloca %xiao.value
  %t496 = alloca %xiao.value
  %t497 = alloca %xiao.value
  %t499 = alloca %xiao.value
  %t500 = alloca %xiao.value
  %t504 = alloca %xiao.value, i64 1
  %t508 = alloca %xiao.value
  %t510 = alloca %xiao.value
  %t511 = alloca %xiao.value
  %t512 = alloca %xiao.value
  %t514 = alloca %xiao.value
  %t515 = alloca %xiao.value
  %t519 = alloca %xiao.value, i64 1
  %t523 = alloca %xiao.value
  %t525 = alloca %xiao.value
  %t526 = alloca %xiao.value
  %t527 = alloca %xiao.value
  %t529 = alloca %xiao.value
  %t530 = alloca %xiao.value
  %t533 = alloca %xiao.value, i64 1
  %t537 = alloca %xiao.value
  %t539 = alloca %xiao.value
  %t540 = alloca %xiao.value
  %t541 = alloca %xiao.value
  %t543 = alloca %xiao.value
  %t544 = alloca %xiao.value
  %t548 = alloca %xiao.value, i64 1
  %t552 = alloca %xiao.value
  %t554 = alloca %xiao.value
  %t555 = alloca %xiao.value
  %t556 = alloca %xiao.value
  %t558 = alloca %xiao.value
  %t559 = alloca %xiao.value
  %t563 = alloca %xiao.value, i64 1
  %t567 = alloca %xiao.value
  %t569 = alloca %xiao.value
  %t570 = alloca %xiao.value
  %t571 = alloca %xiao.value
  %t573 = alloca %xiao.value
  %t574 = alloca %xiao.value
  %t577 = alloca %xiao.value, i64 1
  %t581 = alloca %xiao.value
  %t583 = alloca %xiao.value
  %t584 = alloca %xiao.value
  %t585 = alloca %xiao.value
  %t587 = alloca %xiao.value
  %t588 = alloca %xiao.value
  %t592 = alloca %xiao.value, i64 1
  %t596 = alloca %xiao.value
  %t598 = alloca %xiao.value
  %t599 = alloca %xiao.value
  %t600 = alloca %xiao.value
  %t602 = alloca %xiao.value
  %t603 = alloca %xiao.value
  %t607 = alloca %xiao.value, i64 1
  %t611 = alloca %xiao.value
  %t613 = alloca %xiao.value
  %t614 = alloca %xiao.value
  %t615 = alloca %xiao.value
  %t617 = alloca %xiao.value
  %t618 = alloca %xiao.value
  %t621 = alloca %xiao.value, i64 1
  %t625 = alloca %xiao.value
  %t627 = alloca %xiao.value
  %t628 = alloca %xiao.value
  %t629 = alloca %xiao.value
  %t631 = alloca %xiao.value
  %t632 = alloca %xiao.value
  %t636 = alloca %xiao.value, i64 1
  %t640 = alloca %xiao.value
  %t642 = alloca %xiao.value
  %t643 = alloca %xiao.value
  %t644 = alloca %xiao.value
  %t646 = alloca %xiao.value
  %t647 = alloca %xiao.value
  %t651 = alloca %xiao.value, i64 1
  %t655 = alloca %xiao.value
  %t657 = alloca %xiao.value
  %t658 = alloca %xiao.value
  %t659 = alloca %xiao.value
  %t661 = alloca %xiao.value
  %t662 = alloca %xiao.value
  %t665 = alloca %xiao.value, i64 1
  %t669 = alloca %xiao.value
  %t671 = alloca %xiao.value
  %t672 = alloca %xiao.value
  %t673 = alloca %xiao.value
  %t675 = alloca %xiao.value
  %t676 = alloca %xiao.value
  %t680 = alloca %xiao.value, i64 1
  %t684 = alloca %xiao.value
  %t686 = alloca %xiao.value
  %t687 = alloca %xiao.value
  %t688 = alloca %xiao.value
  %t690 = alloca %xiao.value
  %t691 = alloca %xiao.value
  %t695 = alloca %xiao.value, i64 1
  %t699 = alloca %xiao.value
  %t701 = alloca %xiao.value
  %t702 = alloca %xiao.value
  %t703 = alloca %xiao.value
  %t705 = alloca %xiao.value
  %t706 = alloca %xiao.value
  %t709 = alloca %xiao.value, i64 1
  %t713 = alloca %xiao.value
  %t715 = alloca %xiao.value
  %t716 = alloca %xiao.value
  %t717 = alloca %xiao.value
  %t719 = alloca %xiao.value
  %t720 = alloca %xiao.value
  %t724 = alloca %xiao.value, i64 1
  %t728 = alloca %xiao.value
  %t730 = alloca %xiao.value
  %t731 = alloca %xiao.value
  %t732 = alloca %xiao.value
  %t734 = alloca %xiao.value
  %t735 = alloca %xiao.value
  %t739 = alloca %xiao.value, i64 1
  %t743 = alloca %xiao.value
  %t745 = alloca %xiao.value
  %t746 = alloca %xiao.value
  %t747 = alloca %xiao.value
  %t749 = alloca %xiao.value
  %t750 = alloca %xiao.value
  %t753 = alloca %xiao.value, i64 1
  %t757 = alloca %xiao.value
  %t759 = alloca %xiao.value
  %t760 = alloca %xiao.value
  %t761 = alloca %xiao.value
  %t762 = alloca %xiao.value
  %t763 = alloca %xiao.value
  %t767 = alloca %xiao.value
  %t771 = alloca %xiao.value
  %t775 = alloca %xiao.value, i64 1
  %t779 = alloca %xiao.value
  %t781 = alloca %xiao.value
  %t782 = alloca %xiao.value
  %t783 = alloca %xiao.value
  %t784 = alloca %xiao.value
  %t786 = alloca %xiao.value
  %t788 = alloca %xiao.value
  %t789 = alloca %xiao.value
  %t793 = alloca %xiao.value, i64 1
  %t797 = alloca %xiao.value
  %t799 = alloca %xiao.value
  %t800 = alloca %xiao.value
  %t801 = alloca %xiao.value
  %t802 = alloca %xiao.value, i64 2
  %t804 = alloca %xiao.value
  %t807 = alloca %xiao.value
  %t809 = alloca ptr
  %t812 = alloca %xiao.value
  %t813 = alloca %xiao.value
  %t816 = alloca %xiao.value
  %t819 = alloca %xiao.value
  %t823 = alloca %xiao.value, i64 1
  %t827 = alloca %xiao.value
  %t829 = alloca %xiao.value
  %t830 = alloca %xiao.value
  %t833 = alloca %xiao.value
  %t834 = alloca %xiao.value
  %t836 = alloca %xiao.value
  %t838 = alloca %xiao.value
  %t839 = alloca %xiao.value
  %t843 = alloca %xiao.value, i64 1
  %t847 = alloca %xiao.value
  %t849 = alloca %xiao.value
  %t850 = alloca %xiao.value
  %t851 = alloca %xiao.value
  %t853 = alloca %xiao.value
  %t857 = alloca %xiao.bytes
  %t858 = alloca %xiao.value
  %t859 = alloca %xiao.value
  %t863 = alloca %xiao.value
  %t864 = alloca %xiao.value
  %t865 = alloca %xiao.value
  %t869 = alloca %xiao.value
  %t874 = alloca %xiao.bytes
  %t875 = alloca ptr
  %t880 = alloca %xiao.value
  %t881 = alloca %xiao.value, i64 1
  %t885 = alloca %xiao.value
  %t887 = alloca %xiao.value
  %t888 = alloca %xiao.value
  %t896 = alloca %xiao.bytes
  %t903 = alloca %xiao.bytes
  %t904 = alloca ptr
  %t909 = alloca %xiao.value
  %t910 = alloca %xiao.value, i64 1
  %t914 = alloca %xiao.value
  %t916 = alloca %xiao.value
  %t917 = alloca %xiao.value
  %t924 = alloca %xiao.bytes
  %t925 = alloca ptr
  %t930 = alloca %xiao.value
  %t931 = alloca %xiao.value, i64 1
  %t935 = alloca %xiao.value
  %t937 = alloca %xiao.value
  %t938 = alloca %xiao.value
  %t941 = alloca %xiao.value
  %t945 = alloca %xiao.value
  %t949 = alloca %xiao.value
  %t953 = alloca %xiao.bytes
  %t956 = alloca %xiao.value
  %t957 = alloca %xiao.bytes
  %t960 = alloca %xiao.value
  %t965 = alloca %xiao.bytes
  %t966 = alloca ptr
  %t971 = alloca %xiao.value
  %t972 = alloca %xiao.value
  %t976 = alloca %xiao.bytes
  %t977 = alloca %xiao.error.location
  %t982 = alloca %xiao.value
  %t983 = alloca %xiao.value
  %t986 = alloca %xiao.value
  %t987 = alloca %xiao.error.location
  %t991 = alloca %xiao.value
  %t993 = alloca %xiao.bytes
  %t994 = alloca ptr
  %t997 = alloca %xiao.value
  %t1000 = alloca %xiao.value
  %t1001 = alloca %xiao.value
  %t1002 = alloca %xiao.value, i64 1
  %t1006 = alloca %xiao.value
  %t1008 = alloca %xiao.value
  %t1009 = alloca %xiao.value
  store %xiao.value zeroinitializer, ptr %xiao.statement.site0
  call void @xiao_runtime_error_checkpoint_reset()
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_release_trace_begin()
  store %xiao.value zeroinitializer, ptr %slot0
  store %xiao.value zeroinitializer, ptr %slot1
  store %xiao.value zeroinitializer, ptr %slot10
  store %xiao.value zeroinitializer, ptr %slot11
  store %xiao.value zeroinitializer, ptr %slot12
  store %xiao.value zeroinitializer, ptr %slot13
  store %xiao.value zeroinitializer, ptr %slot14
  store %xiao.value zeroinitializer, ptr %slot2
  store %xiao.value zeroinitializer, ptr %slot3
  store %xiao.value zeroinitializer, ptr %slot4
  store %xiao.value zeroinitializer, ptr %slot5
  store %xiao.value zeroinitializer, ptr %slot6
  store %xiao.value zeroinitializer, ptr %slot7
  store %xiao.value zeroinitializer, ptr %slot8
  store %xiao.value zeroinitializer, ptr %slot9
  store %xiao.value zeroinitializer, ptr %slot18
  store %xiao.value zeroinitializer, ptr %slot17
  store %xiao.value zeroinitializer, ptr %slot15
  store %xiao.value zeroinitializer, ptr %slot16
  %t0 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes0, i64 0, i64 0
  %t1 = insertvalue %xiao.bytes zeroinitializer, ptr %t0, 0
  %t2 = insertvalue %xiao.bytes %t1, i64 5, 1
  store %xiao.bytes %t2, ptr %t3
  %t4 = call i32 @xiao_runtime_language_context_set(ptr %t3)
  %t5 = icmp eq i32 %t4, 0
  br i1 %t5, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.error.terminal
abi.ok0:
  %t6 = call i32 @xiao_runtime_abi_is_compatible(i32 1, i32 18)
  %t7 = icmp eq i32 %t6, 1
  br i1 %t7, label %abi.entry.ok, label %abi.fail
abi.entry.ok:
  store %xiao.value zeroinitializer, ptr %t8
  store %xiao.value zeroinitializer, ptr %t10
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t10, double 4.00000000000000000e0)
  %t9 = load %xiao.value, ptr %t10
  store %xiao.value %t9, ptr %t8
  br label %dynamic.call.arguments.ready3
dynamic.call.arguments.fail2:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  br label %xiao.error.terminal
dynamic.call.arguments.ready3:
  store %xiao.value zeroinitializer, ptr %t11
  call void @xiao.fn.cb1ffaaf3c3d15fa(ptr %t11, ptr %t8)
  call void @xiao_runtime_value_release_strong(ptr %t8)
  %t12 = call i32 @xiao_runtime_error_class()
  %t13 = icmp eq i32 %t12, 0
  br i1 %t13, label %dynamic.call.ok4, label %dynamic.call.fail5
dynamic.call.fail5:
  br label %xiao.error.terminal
dynamic.call.ok4:
  %t14 = load %xiao.value, ptr %t11
  %t16 = getelementptr inbounds %xiao.value, ptr %t15, i64 0
  store %xiao.value %t14, ptr %t16
  %t17 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t15, i64 1, i64 127)
  %t18 = icmp eq i32 %t17, 0
  br i1 %t18, label %abi.ok6, label %abi.status.fail7
abi.status.fail7:
  call i32 @xiao_runtime_error_attach_span(i64 36, i64 50)
  br label %xiao.error.terminal
abi.ok6:
  store %xiao.value %t14, ptr %t19
  call void @xiao_runtime_value_release_strong(ptr %t19)
  store %xiao.value zeroinitializer, ptr %t21
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t21)
  %t20 = load %xiao.value, ptr %t21
  store %xiao.value %t20, ptr %t22
  call void @xiao_runtime_value_release_strong(ptr %t22)
  store %xiao.value zeroinitializer, ptr %t23
  store %xiao.value zeroinitializer, ptr %t25
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t25, double 4.00000000000000000e0)
  %t24 = load %xiao.value, ptr %t25
  store %xiao.value %t24, ptr %t23
  br label %dynamic.call.arguments.ready9
dynamic.call.arguments.fail8:
  call void @xiao_runtime_value_release_strong(ptr %t23)
  br label %xiao.error.terminal
dynamic.call.arguments.ready9:
  store %xiao.value zeroinitializer, ptr %t26
  call void @xiao.fn.cb1ffaaf3c3d15fa(ptr %t26, ptr %t23)
  call void @xiao_runtime_value_release_strong(ptr %t23)
  %t27 = call i32 @xiao_runtime_error_class()
  %t28 = icmp eq i32 %t27, 0
  br i1 %t28, label %dynamic.call.ok10, label %dynamic.call.fail11
dynamic.call.fail11:
  br label %xiao.error.terminal
dynamic.call.ok10:
  %t29 = load %xiao.value, ptr %t26
  %t31 = getelementptr inbounds %xiao.value, ptr %t30, i64 0
  store %xiao.value %t29, ptr %t31
  %t32 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t30, i64 1, i64 127)
  %t33 = icmp eq i32 %t32, 0
  br i1 %t33, label %abi.ok12, label %abi.status.fail13
abi.status.fail13:
  call i32 @xiao_runtime_error_attach_span(i64 51, i64 73)
  br label %xiao.error.terminal
abi.ok12:
  store %xiao.value %t29, ptr %t34
  call void @xiao_runtime_value_release_strong(ptr %t34)
  store %xiao.value zeroinitializer, ptr %t36
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t36)
  %t35 = load %xiao.value, ptr %t36
  store %xiao.value %t35, ptr %t37
  call void @xiao_runtime_value_release_strong(ptr %t37)
  store %xiao.value zeroinitializer, ptr %t38
  store %xiao.value zeroinitializer, ptr %t40
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t40, double 4.00000000000000000e0)
  %t39 = load %xiao.value, ptr %t40
  store %xiao.value %t39, ptr %t38
  br label %cmath.arguments.ready15
cmath.arguments.fail14:
  call void @xiao_runtime_value_release_strong(ptr %t38)
  br label %xiao.error.terminal
cmath.arguments.ready15:
  store %xiao.value zeroinitializer, ptr %t41
  %t42 = call i32 @xiao_runtime_cmath_sqrt(ptr %t38, ptr %t41)
  call void @xiao_runtime_value_release_strong(ptr %t38)
  switch i32 %t42, label %cmath.abi.failure18 [ i32 0, label %cmath.ok16 i32 6, label %cmath.error17 ]
cmath.abi.failure18:
  call void @xiao_runtime_value_release_strong(ptr %t41)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error17:
  call void @xiao_runtime_value_release_strong(ptr %t41)
  call i32 @xiao_runtime_error_attach_span(i64 80, i64 103)
  br label %xiao.error.terminal
cmath.ok16:
  %t43 = load %xiao.value, ptr %t41
  %t45 = getelementptr inbounds %xiao.value, ptr %t44, i64 0
  store %xiao.value %t43, ptr %t45
  %t46 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t44, i64 1, i64 127)
  %t47 = icmp eq i32 %t46, 0
  br i1 %t47, label %abi.ok19, label %abi.status.fail20
abi.status.fail20:
  call i32 @xiao_runtime_error_attach_span(i64 74, i64 104)
  br label %xiao.error.terminal
abi.ok19:
  store %xiao.value %t43, ptr %t48
  call void @xiao_runtime_value_release_strong(ptr %t48)
  store %xiao.value zeroinitializer, ptr %t50
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t50)
  %t49 = load %xiao.value, ptr %t50
  store %xiao.value %t49, ptr %t51
  call void @xiao_runtime_value_release_strong(ptr %t51)
  store %xiao.value zeroinitializer, ptr %t52
  store %xiao.value zeroinitializer, ptr %t54
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t54, double 1.50000000000000000e0)
  %t53 = load %xiao.value, ptr %t54
  store %xiao.value %t53, ptr %t52
  br label %dynamic.call.arguments.ready22
dynamic.call.arguments.fail21:
  call void @xiao_runtime_value_release_strong(ptr %t52)
  br label %xiao.error.terminal
dynamic.call.arguments.ready22:
  store %xiao.value zeroinitializer, ptr %t55
  call void @xiao.fn.860bf3c2a9f5cdfc(ptr %t55, ptr %t52)
  call void @xiao_runtime_value_release_strong(ptr %t52)
  %t56 = call i32 @xiao_runtime_error_class()
  %t57 = icmp eq i32 %t56, 0
  br i1 %t57, label %dynamic.call.ok23, label %dynamic.call.fail24
dynamic.call.fail24:
  br label %xiao.error.terminal
dynamic.call.ok23:
  %t58 = load %xiao.value, ptr %t55
  %t60 = getelementptr inbounds %xiao.value, ptr %t59, i64 0
  store %xiao.value %t58, ptr %t60
  %t61 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t59, i64 1, i64 127)
  %t62 = icmp eq i32 %t61, 0
  br i1 %t62, label %abi.ok25, label %abi.status.fail26
abi.status.fail26:
  call i32 @xiao_runtime_error_attach_span(i64 142, i64 156)
  br label %xiao.error.terminal
abi.ok25:
  store %xiao.value %t58, ptr %t63
  call void @xiao_runtime_value_release_strong(ptr %t63)
  store %xiao.value zeroinitializer, ptr %t65
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t65)
  %t64 = load %xiao.value, ptr %t65
  store %xiao.value %t64, ptr %t66
  call void @xiao_runtime_value_release_strong(ptr %t66)
  store %xiao.value zeroinitializer, ptr %t67
  store %xiao.value zeroinitializer, ptr %t69
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t69, double 1.50000000000000000e0)
  %t68 = load %xiao.value, ptr %t69
  store %xiao.value %t68, ptr %t67
  br label %dynamic.call.arguments.ready28
dynamic.call.arguments.fail27:
  call void @xiao_runtime_value_release_strong(ptr %t67)
  br label %xiao.error.terminal
dynamic.call.arguments.ready28:
  store %xiao.value zeroinitializer, ptr %t70
  call void @xiao.fn.860bf3c2a9f5cdfc(ptr %t70, ptr %t67)
  call void @xiao_runtime_value_release_strong(ptr %t67)
  %t71 = call i32 @xiao_runtime_error_class()
  %t72 = icmp eq i32 %t71, 0
  br i1 %t72, label %dynamic.call.ok29, label %dynamic.call.fail30
dynamic.call.fail30:
  br label %xiao.error.terminal
dynamic.call.ok29:
  %t73 = load %xiao.value, ptr %t70
  %t75 = getelementptr inbounds %xiao.value, ptr %t74, i64 0
  store %xiao.value %t73, ptr %t75
  %t76 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t74, i64 1, i64 127)
  %t77 = icmp eq i32 %t76, 0
  br i1 %t77, label %abi.ok31, label %abi.status.fail32
abi.status.fail32:
  call i32 @xiao_runtime_error_attach_span(i64 157, i64 179)
  br label %xiao.error.terminal
abi.ok31:
  store %xiao.value %t73, ptr %t78
  call void @xiao_runtime_value_release_strong(ptr %t78)
  store %xiao.value zeroinitializer, ptr %t80
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t80)
  %t79 = load %xiao.value, ptr %t80
  store %xiao.value %t79, ptr %t81
  call void @xiao_runtime_value_release_strong(ptr %t81)
  store %xiao.value zeroinitializer, ptr %t82
  store %xiao.value zeroinitializer, ptr %t84
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t84, double 1.50000000000000000e0)
  %t83 = load %xiao.value, ptr %t84
  store %xiao.value %t83, ptr %t82
  br label %cmath.arguments.ready34
cmath.arguments.fail33:
  call void @xiao_runtime_value_release_strong(ptr %t82)
  br label %xiao.error.terminal
cmath.arguments.ready34:
  store %xiao.value zeroinitializer, ptr %t85
  %t86 = call i32 @xiao_runtime_cmath_floor(ptr %t82, ptr %t85)
  call void @xiao_runtime_value_release_strong(ptr %t82)
  switch i32 %t86, label %cmath.abi.failure37 [ i32 0, label %cmath.ok35 i32 6, label %cmath.error36 ]
cmath.abi.failure37:
  call void @xiao_runtime_value_release_strong(ptr %t85)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error36:
  call void @xiao_runtime_value_release_strong(ptr %t85)
  call i32 @xiao_runtime_error_attach_span(i64 186, i64 210)
  br label %xiao.error.terminal
cmath.ok35:
  %t87 = load %xiao.value, ptr %t85
  %t89 = getelementptr inbounds %xiao.value, ptr %t88, i64 0
  store %xiao.value %t87, ptr %t89
  %t90 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t88, i64 1, i64 127)
  %t91 = icmp eq i32 %t90, 0
  br i1 %t91, label %abi.ok38, label %abi.status.fail39
abi.status.fail39:
  call i32 @xiao_runtime_error_attach_span(i64 180, i64 211)
  br label %xiao.error.terminal
abi.ok38:
  store %xiao.value %t87, ptr %t92
  call void @xiao_runtime_value_release_strong(ptr %t92)
  store %xiao.value zeroinitializer, ptr %t94
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t94)
  %t93 = load %xiao.value, ptr %t94
  store %xiao.value %t93, ptr %t95
  call void @xiao_runtime_value_release_strong(ptr %t95)
  store %xiao.value zeroinitializer, ptr %t96
  store %xiao.value zeroinitializer, ptr %t98
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t98, double 1.50000000000000000e0)
  %t97 = load %xiao.value, ptr %t98
  store %xiao.value %t97, ptr %t96
  br label %dynamic.call.arguments.ready41
dynamic.call.arguments.fail40:
  call void @xiao_runtime_value_release_strong(ptr %t96)
  br label %xiao.error.terminal
dynamic.call.arguments.ready41:
  store %xiao.value zeroinitializer, ptr %t99
  call void @xiao.fn.b34d23c8bbb4bb95(ptr %t99, ptr %t96)
  call void @xiao_runtime_value_release_strong(ptr %t96)
  %t100 = call i32 @xiao_runtime_error_class()
  %t101 = icmp eq i32 %t100, 0
  br i1 %t101, label %dynamic.call.ok42, label %dynamic.call.fail43
dynamic.call.fail43:
  br label %xiao.error.terminal
dynamic.call.ok42:
  %t102 = load %xiao.value, ptr %t99
  %t104 = getelementptr inbounds %xiao.value, ptr %t103, i64 0
  store %xiao.value %t102, ptr %t104
  %t105 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t103, i64 1, i64 127)
  %t106 = icmp eq i32 %t105, 0
  br i1 %t106, label %abi.ok44, label %abi.status.fail45
abi.status.fail45:
  call i32 @xiao_runtime_error_attach_span(i64 248, i64 262)
  br label %xiao.error.terminal
abi.ok44:
  store %xiao.value %t102, ptr %t107
  call void @xiao_runtime_value_release_strong(ptr %t107)
  store %xiao.value zeroinitializer, ptr %t109
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t109)
  %t108 = load %xiao.value, ptr %t109
  store %xiao.value %t108, ptr %t110
  call void @xiao_runtime_value_release_strong(ptr %t110)
  store %xiao.value zeroinitializer, ptr %t111
  store %xiao.value zeroinitializer, ptr %t113
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t113, double 1.50000000000000000e0)
  %t112 = load %xiao.value, ptr %t113
  store %xiao.value %t112, ptr %t111
  br label %dynamic.call.arguments.ready47
dynamic.call.arguments.fail46:
  call void @xiao_runtime_value_release_strong(ptr %t111)
  br label %xiao.error.terminal
dynamic.call.arguments.ready47:
  store %xiao.value zeroinitializer, ptr %t114
  call void @xiao.fn.b34d23c8bbb4bb95(ptr %t114, ptr %t111)
  call void @xiao_runtime_value_release_strong(ptr %t111)
  %t115 = call i32 @xiao_runtime_error_class()
  %t116 = icmp eq i32 %t115, 0
  br i1 %t116, label %dynamic.call.ok48, label %dynamic.call.fail49
dynamic.call.fail49:
  br label %xiao.error.terminal
dynamic.call.ok48:
  %t117 = load %xiao.value, ptr %t114
  %t119 = getelementptr inbounds %xiao.value, ptr %t118, i64 0
  store %xiao.value %t117, ptr %t119
  %t120 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t118, i64 1, i64 127)
  %t121 = icmp eq i32 %t120, 0
  br i1 %t121, label %abi.ok50, label %abi.status.fail51
abi.status.fail51:
  call i32 @xiao_runtime_error_attach_span(i64 263, i64 285)
  br label %xiao.error.terminal
abi.ok50:
  store %xiao.value %t117, ptr %t122
  call void @xiao_runtime_value_release_strong(ptr %t122)
  store %xiao.value zeroinitializer, ptr %t124
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t124)
  %t123 = load %xiao.value, ptr %t124
  store %xiao.value %t123, ptr %t125
  call void @xiao_runtime_value_release_strong(ptr %t125)
  store %xiao.value zeroinitializer, ptr %t126
  store %xiao.value zeroinitializer, ptr %t128
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t128, double 1.50000000000000000e0)
  %t127 = load %xiao.value, ptr %t128
  store %xiao.value %t127, ptr %t126
  br label %cmath.arguments.ready53
cmath.arguments.fail52:
  call void @xiao_runtime_value_release_strong(ptr %t126)
  br label %xiao.error.terminal
cmath.arguments.ready53:
  store %xiao.value zeroinitializer, ptr %t129
  %t130 = call i32 @xiao_runtime_cmath_ceil(ptr %t126, ptr %t129)
  call void @xiao_runtime_value_release_strong(ptr %t126)
  switch i32 %t130, label %cmath.abi.failure56 [ i32 0, label %cmath.ok54 i32 6, label %cmath.error55 ]
cmath.abi.failure56:
  call void @xiao_runtime_value_release_strong(ptr %t129)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error55:
  call void @xiao_runtime_value_release_strong(ptr %t129)
  call i32 @xiao_runtime_error_attach_span(i64 292, i64 315)
  br label %xiao.error.terminal
cmath.ok54:
  %t131 = load %xiao.value, ptr %t129
  %t133 = getelementptr inbounds %xiao.value, ptr %t132, i64 0
  store %xiao.value %t131, ptr %t133
  %t134 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t132, i64 1, i64 127)
  %t135 = icmp eq i32 %t134, 0
  br i1 %t135, label %abi.ok57, label %abi.status.fail58
abi.status.fail58:
  call i32 @xiao_runtime_error_attach_span(i64 286, i64 316)
  br label %xiao.error.terminal
abi.ok57:
  store %xiao.value %t131, ptr %t136
  call void @xiao_runtime_value_release_strong(ptr %t136)
  store %xiao.value zeroinitializer, ptr %t138
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t138)
  %t137 = load %xiao.value, ptr %t138
  store %xiao.value %t137, ptr %t139
  call void @xiao_runtime_value_release_strong(ptr %t139)
  store %xiao.value zeroinitializer, ptr %t140
  store %xiao.value zeroinitializer, ptr %t142
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t142, double 2.50000000000000000e0)
  %t141 = load %xiao.value, ptr %t142
  store %xiao.value %t141, ptr %t140
  br label %dynamic.call.arguments.ready60
dynamic.call.arguments.fail59:
  call void @xiao_runtime_value_release_strong(ptr %t140)
  br label %xiao.error.terminal
dynamic.call.arguments.ready60:
  store %xiao.value zeroinitializer, ptr %t143
  call void @xiao.fn.c13a89556a9f20ac(ptr %t143, ptr %t140)
  call void @xiao_runtime_value_release_strong(ptr %t140)
  %t144 = call i32 @xiao_runtime_error_class()
  %t145 = icmp eq i32 %t144, 0
  br i1 %t145, label %dynamic.call.ok61, label %dynamic.call.fail62
dynamic.call.fail62:
  br label %xiao.error.terminal
dynamic.call.ok61:
  %t146 = load %xiao.value, ptr %t143
  %t148 = getelementptr inbounds %xiao.value, ptr %t147, i64 0
  store %xiao.value %t146, ptr %t148
  %t149 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t147, i64 1, i64 127)
  %t150 = icmp eq i32 %t149, 0
  br i1 %t150, label %abi.ok63, label %abi.status.fail64
abi.status.fail64:
  call i32 @xiao_runtime_error_attach_span(i64 354, i64 368)
  br label %xiao.error.terminal
abi.ok63:
  store %xiao.value %t146, ptr %t151
  call void @xiao_runtime_value_release_strong(ptr %t151)
  store %xiao.value zeroinitializer, ptr %t153
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t153)
  %t152 = load %xiao.value, ptr %t153
  store %xiao.value %t152, ptr %t154
  call void @xiao_runtime_value_release_strong(ptr %t154)
  store %xiao.value zeroinitializer, ptr %t155
  store %xiao.value zeroinitializer, ptr %t157
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t157, double 2.50000000000000000e0)
  %t156 = load %xiao.value, ptr %t157
  store %xiao.value %t156, ptr %t155
  br label %dynamic.call.arguments.ready66
dynamic.call.arguments.fail65:
  call void @xiao_runtime_value_release_strong(ptr %t155)
  br label %xiao.error.terminal
dynamic.call.arguments.ready66:
  store %xiao.value zeroinitializer, ptr %t158
  call void @xiao.fn.c13a89556a9f20ac(ptr %t158, ptr %t155)
  call void @xiao_runtime_value_release_strong(ptr %t155)
  %t159 = call i32 @xiao_runtime_error_class()
  %t160 = icmp eq i32 %t159, 0
  br i1 %t160, label %dynamic.call.ok67, label %dynamic.call.fail68
dynamic.call.fail68:
  br label %xiao.error.terminal
dynamic.call.ok67:
  %t161 = load %xiao.value, ptr %t158
  %t163 = getelementptr inbounds %xiao.value, ptr %t162, i64 0
  store %xiao.value %t161, ptr %t163
  %t164 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t162, i64 1, i64 127)
  %t165 = icmp eq i32 %t164, 0
  br i1 %t165, label %abi.ok69, label %abi.status.fail70
abi.status.fail70:
  call i32 @xiao_runtime_error_attach_span(i64 369, i64 391)
  br label %xiao.error.terminal
abi.ok69:
  store %xiao.value %t161, ptr %t166
  call void @xiao_runtime_value_release_strong(ptr %t166)
  store %xiao.value zeroinitializer, ptr %t168
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t168)
  %t167 = load %xiao.value, ptr %t168
  store %xiao.value %t167, ptr %t169
  call void @xiao_runtime_value_release_strong(ptr %t169)
  store %xiao.value zeroinitializer, ptr %t170
  store %xiao.value zeroinitializer, ptr %t172
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t172, double 2.50000000000000000e0)
  %t171 = load %xiao.value, ptr %t172
  store %xiao.value %t171, ptr %t170
  br label %cmath.arguments.ready72
cmath.arguments.fail71:
  call void @xiao_runtime_value_release_strong(ptr %t170)
  br label %xiao.error.terminal
cmath.arguments.ready72:
  store %xiao.value zeroinitializer, ptr %t173
  %t174 = call i32 @xiao_runtime_cmath_round(ptr %t170, ptr %t173)
  call void @xiao_runtime_value_release_strong(ptr %t170)
  switch i32 %t174, label %cmath.abi.failure75 [ i32 0, label %cmath.ok73 i32 6, label %cmath.error74 ]
cmath.abi.failure75:
  call void @xiao_runtime_value_release_strong(ptr %t173)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error74:
  call void @xiao_runtime_value_release_strong(ptr %t173)
  call i32 @xiao_runtime_error_attach_span(i64 398, i64 422)
  br label %xiao.error.terminal
cmath.ok73:
  %t175 = load %xiao.value, ptr %t173
  %t177 = getelementptr inbounds %xiao.value, ptr %t176, i64 0
  store %xiao.value %t175, ptr %t177
  %t178 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t176, i64 1, i64 127)
  %t179 = icmp eq i32 %t178, 0
  br i1 %t179, label %abi.ok76, label %abi.status.fail77
abi.status.fail77:
  call i32 @xiao_runtime_error_attach_span(i64 392, i64 423)
  br label %xiao.error.terminal
abi.ok76:
  store %xiao.value %t175, ptr %t180
  call void @xiao_runtime_value_release_strong(ptr %t180)
  store %xiao.value zeroinitializer, ptr %t182
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t182)
  %t181 = load %xiao.value, ptr %t182
  store %xiao.value %t181, ptr %t183
  call void @xiao_runtime_value_release_strong(ptr %t183)
  store %xiao.value zeroinitializer, ptr %t184
  store %xiao.value zeroinitializer, ptr %t186
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t186, double 1.50000000000000000e0)
  %t185 = load %xiao.value, ptr %t186
  %t187 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes1, i64 0, i64 0
  %t188 = insertvalue %xiao.bytes zeroinitializer, ptr %t187, 0
  %t189 = insertvalue %xiao.bytes %t188, i64 5, 1
  store %xiao.bytes %t189, ptr %t190
  store %xiao.value %t185, ptr %t191
  store %xiao.value zeroinitializer, ptr %t192
  %t193 = call i32 @xiao_runtime_value_unary(ptr %t190, ptr %t191, ptr %t192)
  %t194 = icmp eq i32 %t193, 0
  br i1 %t194, label %abi.ok82, label %abi.status.fail83
abi.status.fail83:
  call i32 @xiao_runtime_error_attach_span(i64 470, i64 474)
  br label %dynamic.unary.fail80
abi.ok82:
  %t195 = load %xiao.value, ptr %t192
  store %xiao.value %t185, ptr %t196
  call void @xiao_runtime_value_release_strong(ptr %t196)
  br label %dynamic.unary.continue81
dynamic.unary.fail80:
  store %xiao.value %t185, ptr %t197
  call void @xiao_runtime_value_release_strong(ptr %t197)
  br label %dynamic.call.arguments.fail78
dynamic.unary.continue81:
  store %xiao.value %t195, ptr %t184
  br label %dynamic.call.arguments.ready79
dynamic.call.arguments.fail78:
  call void @xiao_runtime_value_release_strong(ptr %t184)
  br label %xiao.error.terminal
dynamic.call.arguments.ready79:
  store %xiao.value zeroinitializer, ptr %t198
  call void @xiao.fn.cf09f9314f602e51(ptr %t198, ptr %t184)
  call void @xiao_runtime_value_release_strong(ptr %t184)
  %t199 = call i32 @xiao_runtime_error_class()
  %t200 = icmp eq i32 %t199, 0
  br i1 %t200, label %dynamic.call.ok84, label %dynamic.call.fail85
dynamic.call.fail85:
  br label %xiao.error.terminal
dynamic.call.ok84:
  %t201 = load %xiao.value, ptr %t198
  %t203 = getelementptr inbounds %xiao.value, ptr %t202, i64 0
  store %xiao.value %t201, ptr %t203
  %t204 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t202, i64 1, i64 127)
  %t205 = icmp eq i32 %t204, 0
  br i1 %t205, label %abi.ok86, label %abi.status.fail87
abi.status.fail87:
  call i32 @xiao_runtime_error_attach_span(i64 461, i64 476)
  br label %xiao.error.terminal
abi.ok86:
  store %xiao.value %t201, ptr %t206
  call void @xiao_runtime_value_release_strong(ptr %t206)
  store %xiao.value zeroinitializer, ptr %t208
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t208)
  %t207 = load %xiao.value, ptr %t208
  store %xiao.value %t207, ptr %t209
  call void @xiao_runtime_value_release_strong(ptr %t209)
  store %xiao.value zeroinitializer, ptr %t210
  store %xiao.value zeroinitializer, ptr %t212
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t212, double 1.50000000000000000e0)
  %t211 = load %xiao.value, ptr %t212
  %t213 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes2, i64 0, i64 0
  %t214 = insertvalue %xiao.bytes zeroinitializer, ptr %t213, 0
  %t215 = insertvalue %xiao.bytes %t214, i64 5, 1
  store %xiao.bytes %t215, ptr %t216
  store %xiao.value %t211, ptr %t217
  store %xiao.value zeroinitializer, ptr %t218
  %t219 = call i32 @xiao_runtime_value_unary(ptr %t216, ptr %t217, ptr %t218)
  %t220 = icmp eq i32 %t219, 0
  br i1 %t220, label %abi.ok92, label %abi.status.fail93
abi.status.fail93:
  call i32 @xiao_runtime_error_attach_span(i64 494, i64 498)
  br label %dynamic.unary.fail90
abi.ok92:
  %t221 = load %xiao.value, ptr %t218
  store %xiao.value %t211, ptr %t222
  call void @xiao_runtime_value_release_strong(ptr %t222)
  br label %dynamic.unary.continue91
dynamic.unary.fail90:
  store %xiao.value %t211, ptr %t223
  call void @xiao_runtime_value_release_strong(ptr %t223)
  br label %dynamic.call.arguments.fail88
dynamic.unary.continue91:
  store %xiao.value %t221, ptr %t210
  br label %dynamic.call.arguments.ready89
dynamic.call.arguments.fail88:
  call void @xiao_runtime_value_release_strong(ptr %t210)
  br label %xiao.error.terminal
dynamic.call.arguments.ready89:
  store %xiao.value zeroinitializer, ptr %t224
  call void @xiao.fn.cf09f9314f602e51(ptr %t224, ptr %t210)
  call void @xiao_runtime_value_release_strong(ptr %t210)
  %t225 = call i32 @xiao_runtime_error_class()
  %t226 = icmp eq i32 %t225, 0
  br i1 %t226, label %dynamic.call.ok94, label %dynamic.call.fail95
dynamic.call.fail95:
  br label %xiao.error.terminal
dynamic.call.ok94:
  %t227 = load %xiao.value, ptr %t224
  %t229 = getelementptr inbounds %xiao.value, ptr %t228, i64 0
  store %xiao.value %t227, ptr %t229
  %t230 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t228, i64 1, i64 127)
  %t231 = icmp eq i32 %t230, 0
  br i1 %t231, label %abi.ok96, label %abi.status.fail97
abi.status.fail97:
  call i32 @xiao_runtime_error_attach_span(i64 477, i64 500)
  br label %xiao.error.terminal
abi.ok96:
  store %xiao.value %t227, ptr %t232
  call void @xiao_runtime_value_release_strong(ptr %t232)
  store %xiao.value zeroinitializer, ptr %t234
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t234)
  %t233 = load %xiao.value, ptr %t234
  store %xiao.value %t233, ptr %t235
  call void @xiao_runtime_value_release_strong(ptr %t235)
  store %xiao.value zeroinitializer, ptr %t236
  store %xiao.value zeroinitializer, ptr %t238
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t238, double 1.50000000000000000e0)
  %t237 = load %xiao.value, ptr %t238
  %t239 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes3, i64 0, i64 0
  %t240 = insertvalue %xiao.bytes zeroinitializer, ptr %t239, 0
  %t241 = insertvalue %xiao.bytes %t240, i64 5, 1
  store %xiao.bytes %t241, ptr %t242
  store %xiao.value %t237, ptr %t243
  store %xiao.value zeroinitializer, ptr %t244
  %t245 = call i32 @xiao_runtime_value_unary(ptr %t242, ptr %t243, ptr %t244)
  %t246 = icmp eq i32 %t245, 0
  br i1 %t246, label %abi.ok102, label %abi.status.fail103
abi.status.fail103:
  call i32 @xiao_runtime_error_attach_span(i64 527, i64 531)
  br label %dynamic.unary.fail100
abi.ok102:
  %t247 = load %xiao.value, ptr %t244
  store %xiao.value %t237, ptr %t248
  call void @xiao_runtime_value_release_strong(ptr %t248)
  br label %dynamic.unary.continue101
dynamic.unary.fail100:
  store %xiao.value %t237, ptr %t249
  call void @xiao_runtime_value_release_strong(ptr %t249)
  br label %cmath.arguments.fail98
dynamic.unary.continue101:
  store %xiao.value %t247, ptr %t236
  br label %cmath.arguments.ready99
cmath.arguments.fail98:
  call void @xiao_runtime_value_release_strong(ptr %t236)
  br label %xiao.error.terminal
cmath.arguments.ready99:
  store %xiao.value zeroinitializer, ptr %t250
  %t251 = call i32 @xiao_runtime_cmath_trunc(ptr %t236, ptr %t250)
  call void @xiao_runtime_value_release_strong(ptr %t236)
  switch i32 %t251, label %cmath.abi.failure106 [ i32 0, label %cmath.ok104 i32 6, label %cmath.error105 ]
cmath.abi.failure106:
  call void @xiao_runtime_value_release_strong(ptr %t250)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error105:
  call void @xiao_runtime_value_release_strong(ptr %t250)
  call i32 @xiao_runtime_error_attach_span(i64 507, i64 532)
  br label %xiao.error.terminal
cmath.ok104:
  %t252 = load %xiao.value, ptr %t250
  %t254 = getelementptr inbounds %xiao.value, ptr %t253, i64 0
  store %xiao.value %t252, ptr %t254
  %t255 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t253, i64 1, i64 127)
  %t256 = icmp eq i32 %t255, 0
  br i1 %t256, label %abi.ok107, label %abi.status.fail108
abi.status.fail108:
  call i32 @xiao_runtime_error_attach_span(i64 501, i64 533)
  br label %xiao.error.terminal
abi.ok107:
  store %xiao.value %t252, ptr %t257
  call void @xiao_runtime_value_release_strong(ptr %t257)
  store %xiao.value zeroinitializer, ptr %t259
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t259)
  %t258 = load %xiao.value, ptr %t259
  store %xiao.value %t258, ptr %t260
  call void @xiao_runtime_value_release_strong(ptr %t260)
  store %xiao.value zeroinitializer, ptr %t261
  store %xiao.value zeroinitializer, ptr %t263
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t263, double 3.00000000000000000e0)
  %t262 = load %xiao.value, ptr %t263
  %t264 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes4, i64 0, i64 0
  %t265 = insertvalue %xiao.bytes zeroinitializer, ptr %t264, 0
  %t266 = insertvalue %xiao.bytes %t265, i64 5, 1
  store %xiao.bytes %t266, ptr %t267
  store %xiao.value %t262, ptr %t268
  store %xiao.value zeroinitializer, ptr %t269
  %t270 = call i32 @xiao_runtime_value_unary(ptr %t267, ptr %t268, ptr %t269)
  %t271 = icmp eq i32 %t270, 0
  br i1 %t271, label %abi.ok113, label %abi.status.fail114
abi.status.fail114:
  call i32 @xiao_runtime_error_attach_span(i64 578, i64 582)
  br label %dynamic.unary.fail111
abi.ok113:
  %t272 = load %xiao.value, ptr %t269
  store %xiao.value %t262, ptr %t273
  call void @xiao_runtime_value_release_strong(ptr %t273)
  br label %dynamic.unary.continue112
dynamic.unary.fail111:
  store %xiao.value %t262, ptr %t274
  call void @xiao_runtime_value_release_strong(ptr %t274)
  br label %dynamic.call.arguments.fail109
dynamic.unary.continue112:
  store %xiao.value %t272, ptr %t261
  br label %dynamic.call.arguments.ready110
dynamic.call.arguments.fail109:
  call void @xiao_runtime_value_release_strong(ptr %t261)
  br label %xiao.error.terminal
dynamic.call.arguments.ready110:
  store %xiao.value zeroinitializer, ptr %t275
  call void @xiao.fn.1ab66cb157484850(ptr %t275, ptr %t261)
  call void @xiao_runtime_value_release_strong(ptr %t261)
  %t276 = call i32 @xiao_runtime_error_class()
  %t277 = icmp eq i32 %t276, 0
  br i1 %t277, label %dynamic.call.ok115, label %dynamic.call.fail116
dynamic.call.fail116:
  br label %xiao.error.terminal
dynamic.call.ok115:
  %t278 = load %xiao.value, ptr %t275
  %t280 = getelementptr inbounds %xiao.value, ptr %t279, i64 0
  store %xiao.value %t278, ptr %t280
  %t281 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t279, i64 1, i64 127)
  %t282 = icmp eq i32 %t281, 0
  br i1 %t282, label %abi.ok117, label %abi.status.fail118
abi.status.fail118:
  call i32 @xiao_runtime_error_attach_span(i64 569, i64 584)
  br label %xiao.error.terminal
abi.ok117:
  store %xiao.value %t278, ptr %t283
  call void @xiao_runtime_value_release_strong(ptr %t283)
  store %xiao.value zeroinitializer, ptr %t285
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t285)
  %t284 = load %xiao.value, ptr %t285
  store %xiao.value %t284, ptr %t286
  call void @xiao_runtime_value_release_strong(ptr %t286)
  store %xiao.value zeroinitializer, ptr %t287
  store %xiao.value zeroinitializer, ptr %t289
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t289, double 3.00000000000000000e0)
  %t288 = load %xiao.value, ptr %t289
  %t290 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes5, i64 0, i64 0
  %t291 = insertvalue %xiao.bytes zeroinitializer, ptr %t290, 0
  %t292 = insertvalue %xiao.bytes %t291, i64 5, 1
  store %xiao.bytes %t292, ptr %t293
  store %xiao.value %t288, ptr %t294
  store %xiao.value zeroinitializer, ptr %t295
  %t296 = call i32 @xiao_runtime_value_unary(ptr %t293, ptr %t294, ptr %t295)
  %t297 = icmp eq i32 %t296, 0
  br i1 %t297, label %abi.ok123, label %abi.status.fail124
abi.status.fail124:
  call i32 @xiao_runtime_error_attach_span(i64 602, i64 606)
  br label %dynamic.unary.fail121
abi.ok123:
  %t298 = load %xiao.value, ptr %t295
  store %xiao.value %t288, ptr %t299
  call void @xiao_runtime_value_release_strong(ptr %t299)
  br label %dynamic.unary.continue122
dynamic.unary.fail121:
  store %xiao.value %t288, ptr %t300
  call void @xiao_runtime_value_release_strong(ptr %t300)
  br label %dynamic.call.arguments.fail119
dynamic.unary.continue122:
  store %xiao.value %t298, ptr %t287
  br label %dynamic.call.arguments.ready120
dynamic.call.arguments.fail119:
  call void @xiao_runtime_value_release_strong(ptr %t287)
  br label %xiao.error.terminal
dynamic.call.arguments.ready120:
  store %xiao.value zeroinitializer, ptr %t301
  call void @xiao.fn.1ab66cb157484850(ptr %t301, ptr %t287)
  call void @xiao_runtime_value_release_strong(ptr %t287)
  %t302 = call i32 @xiao_runtime_error_class()
  %t303 = icmp eq i32 %t302, 0
  br i1 %t303, label %dynamic.call.ok125, label %dynamic.call.fail126
dynamic.call.fail126:
  br label %xiao.error.terminal
dynamic.call.ok125:
  %t304 = load %xiao.value, ptr %t301
  %t306 = getelementptr inbounds %xiao.value, ptr %t305, i64 0
  store %xiao.value %t304, ptr %t306
  %t307 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t305, i64 1, i64 127)
  %t308 = icmp eq i32 %t307, 0
  br i1 %t308, label %abi.ok127, label %abi.status.fail128
abi.status.fail128:
  call i32 @xiao_runtime_error_attach_span(i64 585, i64 608)
  br label %xiao.error.terminal
abi.ok127:
  store %xiao.value %t304, ptr %t309
  call void @xiao_runtime_value_release_strong(ptr %t309)
  store %xiao.value zeroinitializer, ptr %t311
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t311)
  %t310 = load %xiao.value, ptr %t311
  store %xiao.value %t310, ptr %t312
  call void @xiao_runtime_value_release_strong(ptr %t312)
  store %xiao.value zeroinitializer, ptr %t313
  store %xiao.value zeroinitializer, ptr %t315
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t315, double 3.00000000000000000e0)
  %t314 = load %xiao.value, ptr %t315
  %t316 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes6, i64 0, i64 0
  %t317 = insertvalue %xiao.bytes zeroinitializer, ptr %t316, 0
  %t318 = insertvalue %xiao.bytes %t317, i64 5, 1
  store %xiao.bytes %t318, ptr %t319
  store %xiao.value %t314, ptr %t320
  store %xiao.value zeroinitializer, ptr %t321
  %t322 = call i32 @xiao_runtime_value_unary(ptr %t319, ptr %t320, ptr %t321)
  %t323 = icmp eq i32 %t322, 0
  br i1 %t323, label %abi.ok133, label %abi.status.fail134
abi.status.fail134:
  call i32 @xiao_runtime_error_attach_span(i64 633, i64 637)
  br label %dynamic.unary.fail131
abi.ok133:
  %t324 = load %xiao.value, ptr %t321
  store %xiao.value %t314, ptr %t325
  call void @xiao_runtime_value_release_strong(ptr %t325)
  br label %dynamic.unary.continue132
dynamic.unary.fail131:
  store %xiao.value %t314, ptr %t326
  call void @xiao_runtime_value_release_strong(ptr %t326)
  br label %cmath.arguments.fail129
dynamic.unary.continue132:
  store %xiao.value %t324, ptr %t313
  br label %cmath.arguments.ready130
cmath.arguments.fail129:
  call void @xiao_runtime_value_release_strong(ptr %t313)
  br label %xiao.error.terminal
cmath.arguments.ready130:
  store %xiao.value zeroinitializer, ptr %t327
  %t328 = call i32 @xiao_runtime_cmath_abs(ptr %t313, ptr %t327)
  call void @xiao_runtime_value_release_strong(ptr %t313)
  switch i32 %t328, label %cmath.abi.failure137 [ i32 0, label %cmath.ok135 i32 6, label %cmath.error136 ]
cmath.abi.failure137:
  call void @xiao_runtime_value_release_strong(ptr %t327)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error136:
  call void @xiao_runtime_value_release_strong(ptr %t327)
  call i32 @xiao_runtime_error_attach_span(i64 615, i64 638)
  br label %xiao.error.terminal
cmath.ok135:
  %t329 = load %xiao.value, ptr %t327
  %t331 = getelementptr inbounds %xiao.value, ptr %t330, i64 0
  store %xiao.value %t329, ptr %t331
  %t332 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t330, i64 1, i64 127)
  %t333 = icmp eq i32 %t332, 0
  br i1 %t333, label %abi.ok138, label %abi.status.fail139
abi.status.fail139:
  call i32 @xiao_runtime_error_attach_span(i64 609, i64 639)
  br label %xiao.error.terminal
abi.ok138:
  store %xiao.value %t329, ptr %t334
  call void @xiao_runtime_value_release_strong(ptr %t334)
  store %xiao.value zeroinitializer, ptr %t336
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t336)
  %t335 = load %xiao.value, ptr %t336
  store %xiao.value %t335, ptr %t337
  call void @xiao_runtime_value_release_strong(ptr %t337)
  store %xiao.value zeroinitializer, ptr %t338
  store %xiao.value zeroinitializer, ptr %t339
  store %xiao.value zeroinitializer, ptr %t341
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t341, double 7.00000000000000000e0)
  %t340 = load %xiao.value, ptr %t341
  store %xiao.value %t340, ptr %t338
  store %xiao.value zeroinitializer, ptr %t343
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t343, double 2.00000000000000000e0)
  %t342 = load %xiao.value, ptr %t343
  store %xiao.value %t342, ptr %t339
  br label %dynamic.call.arguments.ready141
dynamic.call.arguments.fail140:
  call void @xiao_runtime_value_release_strong(ptr %t338)
  call void @xiao_runtime_value_release_strong(ptr %t339)
  br label %xiao.error.terminal
dynamic.call.arguments.ready141:
  store %xiao.value zeroinitializer, ptr %t344
  call void @xiao.fn.b66bfb82035c3390(ptr %t344, ptr %t338, ptr %t339)
  call void @xiao_runtime_value_release_strong(ptr %t338)
  call void @xiao_runtime_value_release_strong(ptr %t339)
  %t345 = call i32 @xiao_runtime_error_class()
  %t346 = icmp eq i32 %t345, 0
  br i1 %t346, label %dynamic.call.ok142, label %dynamic.call.fail143
dynamic.call.fail143:
  br label %xiao.error.terminal
dynamic.call.ok142:
  %t347 = load %xiao.value, ptr %t344
  %t349 = getelementptr inbounds %xiao.value, ptr %t348, i64 0
  store %xiao.value %t347, ptr %t349
  %t350 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t348, i64 1, i64 127)
  %t351 = icmp eq i32 %t350, 0
  br i1 %t351, label %abi.ok144, label %abi.status.fail145
abi.status.fail145:
  call i32 @xiao_runtime_error_attach_span(i64 675, i64 694)
  br label %xiao.error.terminal
abi.ok144:
  store %xiao.value %t347, ptr %t352
  call void @xiao_runtime_value_release_strong(ptr %t352)
  store %xiao.value zeroinitializer, ptr %t354
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t354)
  %t353 = load %xiao.value, ptr %t354
  store %xiao.value %t353, ptr %t355
  call void @xiao_runtime_value_release_strong(ptr %t355)
  store %xiao.value zeroinitializer, ptr %t356
  store %xiao.value zeroinitializer, ptr %t357
  store %xiao.value zeroinitializer, ptr %t359
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t359, double 2.00000000000000000e0)
  %t358 = load %xiao.value, ptr %t359
  store %xiao.value %t358, ptr %t356
  store %xiao.value zeroinitializer, ptr %t361
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t361, double 7.00000000000000000e0)
  %t360 = load %xiao.value, ptr %t361
  store %xiao.value %t360, ptr %t357
  br label %dynamic.call.arguments.ready147
dynamic.call.arguments.fail146:
  call void @xiao_runtime_value_release_strong(ptr %t356)
  call void @xiao_runtime_value_release_strong(ptr %t357)
  br label %xiao.error.terminal
dynamic.call.arguments.ready147:
  store %xiao.value zeroinitializer, ptr %t362
  call void @xiao.fn.b66bfb82035c3390(ptr %t362, ptr %t357, ptr %t356)
  call void @xiao_runtime_value_release_strong(ptr %t356)
  call void @xiao_runtime_value_release_strong(ptr %t357)
  %t363 = call i32 @xiao_runtime_error_class()
  %t364 = icmp eq i32 %t363, 0
  br i1 %t364, label %dynamic.call.ok148, label %dynamic.call.fail149
dynamic.call.fail149:
  br label %xiao.error.terminal
dynamic.call.ok148:
  %t365 = load %xiao.value, ptr %t362
  %t367 = getelementptr inbounds %xiao.value, ptr %t366, i64 0
  store %xiao.value %t365, ptr %t367
  %t368 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t366, i64 1, i64 127)
  %t369 = icmp eq i32 %t368, 0
  br i1 %t369, label %abi.ok150, label %abi.status.fail151
abi.status.fail151:
  call i32 @xiao_runtime_error_attach_span(i64 695, i64 729)
  br label %xiao.error.terminal
abi.ok150:
  store %xiao.value %t365, ptr %t370
  call void @xiao_runtime_value_release_strong(ptr %t370)
  store %xiao.value zeroinitializer, ptr %t372
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t372)
  %t371 = load %xiao.value, ptr %t372
  store %xiao.value %t371, ptr %t373
  call void @xiao_runtime_value_release_strong(ptr %t373)
  store %xiao.value zeroinitializer, ptr %t374
  store %xiao.value zeroinitializer, ptr %t375
  store %xiao.value zeroinitializer, ptr %t377
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t377, double 2.00000000000000000e0)
  %t376 = load %xiao.value, ptr %t377
  store %xiao.value %t376, ptr %t374
  store %xiao.value zeroinitializer, ptr %t379
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t379, double 7.00000000000000000e0)
  %t378 = load %xiao.value, ptr %t379
  store %xiao.value %t378, ptr %t375
  br label %cmath.arguments.ready153
cmath.arguments.fail152:
  call void @xiao_runtime_value_release_strong(ptr %t375)
  call void @xiao_runtime_value_release_strong(ptr %t374)
  br label %xiao.error.terminal
cmath.arguments.ready153:
  store %xiao.value zeroinitializer, ptr %t380
  %t381 = call i32 @xiao_runtime_cmath_min(ptr %t375, ptr %t374, ptr %t380)
  call void @xiao_runtime_value_release_strong(ptr %t375)
  call void @xiao_runtime_value_release_strong(ptr %t374)
  switch i32 %t381, label %cmath.abi.failure156 [ i32 0, label %cmath.ok154 i32 6, label %cmath.error155 ]
cmath.abi.failure156:
  call void @xiao_runtime_value_release_strong(ptr %t380)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error155:
  call void @xiao_runtime_value_release_strong(ptr %t380)
  call i32 @xiao_runtime_error_attach_span(i64 736, i64 770)
  br label %xiao.error.terminal
cmath.ok154:
  %t382 = load %xiao.value, ptr %t380
  %t384 = getelementptr inbounds %xiao.value, ptr %t383, i64 0
  store %xiao.value %t382, ptr %t384
  %t385 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t383, i64 1, i64 127)
  %t386 = icmp eq i32 %t385, 0
  br i1 %t386, label %abi.ok157, label %abi.status.fail158
abi.status.fail158:
  call i32 @xiao_runtime_error_attach_span(i64 730, i64 771)
  br label %xiao.error.terminal
abi.ok157:
  store %xiao.value %t382, ptr %t387
  call void @xiao_runtime_value_release_strong(ptr %t387)
  store %xiao.value zeroinitializer, ptr %t389
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t389)
  %t388 = load %xiao.value, ptr %t389
  store %xiao.value %t388, ptr %t390
  call void @xiao_runtime_value_release_strong(ptr %t390)
  store %xiao.value zeroinitializer, ptr %t391
  store %xiao.value zeroinitializer, ptr %t392
  store %xiao.value zeroinitializer, ptr %t394
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t394, double 7.00000000000000000e0)
  %t393 = load %xiao.value, ptr %t394
  store %xiao.value %t393, ptr %t391
  store %xiao.value zeroinitializer, ptr %t396
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t396, double 2.00000000000000000e0)
  %t395 = load %xiao.value, ptr %t396
  store %xiao.value %t395, ptr %t392
  br label %dynamic.call.arguments.ready160
dynamic.call.arguments.fail159:
  call void @xiao_runtime_value_release_strong(ptr %t391)
  call void @xiao_runtime_value_release_strong(ptr %t392)
  br label %xiao.error.terminal
dynamic.call.arguments.ready160:
  store %xiao.value zeroinitializer, ptr %t397
  call void @xiao.fn.91742dfdb14adccd(ptr %t397, ptr %t391, ptr %t392)
  call void @xiao_runtime_value_release_strong(ptr %t391)
  call void @xiao_runtime_value_release_strong(ptr %t392)
  %t398 = call i32 @xiao_runtime_error_class()
  %t399 = icmp eq i32 %t398, 0
  br i1 %t399, label %dynamic.call.ok161, label %dynamic.call.fail162
dynamic.call.fail162:
  br label %xiao.error.terminal
dynamic.call.ok161:
  %t400 = load %xiao.value, ptr %t397
  %t402 = getelementptr inbounds %xiao.value, ptr %t401, i64 0
  store %xiao.value %t400, ptr %t402
  %t403 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t401, i64 1, i64 127)
  %t404 = icmp eq i32 %t403, 0
  br i1 %t404, label %abi.ok163, label %abi.status.fail164
abi.status.fail164:
  call i32 @xiao_runtime_error_attach_span(i64 807, i64 826)
  br label %xiao.error.terminal
abi.ok163:
  store %xiao.value %t400, ptr %t405
  call void @xiao_runtime_value_release_strong(ptr %t405)
  store %xiao.value zeroinitializer, ptr %t407
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t407)
  %t406 = load %xiao.value, ptr %t407
  store %xiao.value %t406, ptr %t408
  call void @xiao_runtime_value_release_strong(ptr %t408)
  store %xiao.value zeroinitializer, ptr %t409
  store %xiao.value zeroinitializer, ptr %t410
  store %xiao.value zeroinitializer, ptr %t412
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t412, double 2.00000000000000000e0)
  %t411 = load %xiao.value, ptr %t412
  store %xiao.value %t411, ptr %t409
  store %xiao.value zeroinitializer, ptr %t414
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t414, double 7.00000000000000000e0)
  %t413 = load %xiao.value, ptr %t414
  store %xiao.value %t413, ptr %t410
  br label %dynamic.call.arguments.ready166
dynamic.call.arguments.fail165:
  call void @xiao_runtime_value_release_strong(ptr %t409)
  call void @xiao_runtime_value_release_strong(ptr %t410)
  br label %xiao.error.terminal
dynamic.call.arguments.ready166:
  store %xiao.value zeroinitializer, ptr %t415
  call void @xiao.fn.91742dfdb14adccd(ptr %t415, ptr %t410, ptr %t409)
  call void @xiao_runtime_value_release_strong(ptr %t409)
  call void @xiao_runtime_value_release_strong(ptr %t410)
  %t416 = call i32 @xiao_runtime_error_class()
  %t417 = icmp eq i32 %t416, 0
  br i1 %t417, label %dynamic.call.ok167, label %dynamic.call.fail168
dynamic.call.fail168:
  br label %xiao.error.terminal
dynamic.call.ok167:
  %t418 = load %xiao.value, ptr %t415
  %t420 = getelementptr inbounds %xiao.value, ptr %t419, i64 0
  store %xiao.value %t418, ptr %t420
  %t421 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t419, i64 1, i64 127)
  %t422 = icmp eq i32 %t421, 0
  br i1 %t422, label %abi.ok169, label %abi.status.fail170
abi.status.fail170:
  call i32 @xiao_runtime_error_attach_span(i64 827, i64 861)
  br label %xiao.error.terminal
abi.ok169:
  store %xiao.value %t418, ptr %t423
  call void @xiao_runtime_value_release_strong(ptr %t423)
  store %xiao.value zeroinitializer, ptr %t425
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t425)
  %t424 = load %xiao.value, ptr %t425
  store %xiao.value %t424, ptr %t426
  call void @xiao_runtime_value_release_strong(ptr %t426)
  store %xiao.value zeroinitializer, ptr %t427
  store %xiao.value zeroinitializer, ptr %t428
  store %xiao.value zeroinitializer, ptr %t430
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t430, double 2.00000000000000000e0)
  %t429 = load %xiao.value, ptr %t430
  store %xiao.value %t429, ptr %t427
  store %xiao.value zeroinitializer, ptr %t432
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t432, double 7.00000000000000000e0)
  %t431 = load %xiao.value, ptr %t432
  store %xiao.value %t431, ptr %t428
  br label %cmath.arguments.ready172
cmath.arguments.fail171:
  call void @xiao_runtime_value_release_strong(ptr %t428)
  call void @xiao_runtime_value_release_strong(ptr %t427)
  br label %xiao.error.terminal
cmath.arguments.ready172:
  store %xiao.value zeroinitializer, ptr %t433
  %t434 = call i32 @xiao_runtime_cmath_max(ptr %t428, ptr %t427, ptr %t433)
  call void @xiao_runtime_value_release_strong(ptr %t428)
  call void @xiao_runtime_value_release_strong(ptr %t427)
  switch i32 %t434, label %cmath.abi.failure175 [ i32 0, label %cmath.ok173 i32 6, label %cmath.error174 ]
cmath.abi.failure175:
  call void @xiao_runtime_value_release_strong(ptr %t433)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error174:
  call void @xiao_runtime_value_release_strong(ptr %t433)
  call i32 @xiao_runtime_error_attach_span(i64 868, i64 902)
  br label %xiao.error.terminal
cmath.ok173:
  %t435 = load %xiao.value, ptr %t433
  %t437 = getelementptr inbounds %xiao.value, ptr %t436, i64 0
  store %xiao.value %t435, ptr %t437
  %t438 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t436, i64 1, i64 127)
  %t439 = icmp eq i32 %t438, 0
  br i1 %t439, label %abi.ok176, label %abi.status.fail177
abi.status.fail177:
  call i32 @xiao_runtime_error_attach_span(i64 862, i64 903)
  br label %xiao.error.terminal
abi.ok176:
  store %xiao.value %t435, ptr %t440
  call void @xiao_runtime_value_release_strong(ptr %t440)
  store %xiao.value zeroinitializer, ptr %t442
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t442)
  %t441 = load %xiao.value, ptr %t442
  store %xiao.value %t441, ptr %t443
  call void @xiao_runtime_value_release_strong(ptr %t443)
  store %xiao.value zeroinitializer, ptr %t444
  store %xiao.value zeroinitializer, ptr %t445
  store %xiao.value zeroinitializer, ptr %t447
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t447, double 2.00000000000000000e0)
  %t446 = load %xiao.value, ptr %t447
  store %xiao.value %t446, ptr %t444
  store %xiao.value zeroinitializer, ptr %t449
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t449, double 3.00000000000000000e0)
  %t448 = load %xiao.value, ptr %t449
  store %xiao.value %t448, ptr %t445
  br label %dynamic.call.arguments.ready179
dynamic.call.arguments.fail178:
  call void @xiao_runtime_value_release_strong(ptr %t444)
  call void @xiao_runtime_value_release_strong(ptr %t445)
  br label %xiao.error.terminal
dynamic.call.arguments.ready179:
  store %xiao.value zeroinitializer, ptr %t450
  call void @xiao.fn.4e6166bed798273c(ptr %t450, ptr %t444, ptr %t445)
  call void @xiao_runtime_value_release_strong(ptr %t444)
  call void @xiao_runtime_value_release_strong(ptr %t445)
  %t451 = call i32 @xiao_runtime_error_class()
  %t452 = icmp eq i32 %t451, 0
  br i1 %t452, label %dynamic.call.ok180, label %dynamic.call.fail181
dynamic.call.fail181:
  br label %xiao.error.terminal
dynamic.call.ok180:
  %t453 = load %xiao.value, ptr %t450
  %t455 = getelementptr inbounds %xiao.value, ptr %t454, i64 0
  store %xiao.value %t453, ptr %t455
  %t456 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t454, i64 1, i64 127)
  %t457 = icmp eq i32 %t456, 0
  br i1 %t457, label %abi.ok182, label %abi.status.fail183
abi.status.fail183:
  call i32 @xiao_runtime_error_attach_span(i64 939, i64 958)
  br label %xiao.error.terminal
abi.ok182:
  store %xiao.value %t453, ptr %t458
  call void @xiao_runtime_value_release_strong(ptr %t458)
  store %xiao.value zeroinitializer, ptr %t460
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t460)
  %t459 = load %xiao.value, ptr %t460
  store %xiao.value %t459, ptr %t461
  call void @xiao_runtime_value_release_strong(ptr %t461)
  store %xiao.value zeroinitializer, ptr %t462
  store %xiao.value zeroinitializer, ptr %t463
  store %xiao.value zeroinitializer, ptr %t465
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t465, double 3.00000000000000000e0)
  %t464 = load %xiao.value, ptr %t465
  store %xiao.value %t464, ptr %t462
  store %xiao.value zeroinitializer, ptr %t467
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t467, double 2.00000000000000000e0)
  %t466 = load %xiao.value, ptr %t467
  store %xiao.value %t466, ptr %t463
  br label %dynamic.call.arguments.ready185
dynamic.call.arguments.fail184:
  call void @xiao_runtime_value_release_strong(ptr %t462)
  call void @xiao_runtime_value_release_strong(ptr %t463)
  br label %xiao.error.terminal
dynamic.call.arguments.ready185:
  store %xiao.value zeroinitializer, ptr %t468
  call void @xiao.fn.4e6166bed798273c(ptr %t468, ptr %t463, ptr %t462)
  call void @xiao_runtime_value_release_strong(ptr %t462)
  call void @xiao_runtime_value_release_strong(ptr %t463)
  %t469 = call i32 @xiao_runtime_error_class()
  %t470 = icmp eq i32 %t469, 0
  br i1 %t470, label %dynamic.call.ok186, label %dynamic.call.fail187
dynamic.call.fail187:
  br label %xiao.error.terminal
dynamic.call.ok186:
  %t471 = load %xiao.value, ptr %t468
  %t473 = getelementptr inbounds %xiao.value, ptr %t472, i64 0
  store %xiao.value %t471, ptr %t473
  %t474 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t472, i64 1, i64 127)
  %t475 = icmp eq i32 %t474, 0
  br i1 %t475, label %abi.ok188, label %abi.status.fail189
abi.status.fail189:
  call i32 @xiao_runtime_error_attach_span(i64 959, i64 996)
  br label %xiao.error.terminal
abi.ok188:
  store %xiao.value %t471, ptr %t476
  call void @xiao_runtime_value_release_strong(ptr %t476)
  store %xiao.value zeroinitializer, ptr %t478
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t478)
  %t477 = load %xiao.value, ptr %t478
  store %xiao.value %t477, ptr %t479
  call void @xiao_runtime_value_release_strong(ptr %t479)
  store %xiao.value zeroinitializer, ptr %t480
  store %xiao.value zeroinitializer, ptr %t481
  store %xiao.value zeroinitializer, ptr %t483
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t483, double 3.00000000000000000e0)
  %t482 = load %xiao.value, ptr %t483
  store %xiao.value %t482, ptr %t480
  store %xiao.value zeroinitializer, ptr %t485
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t485, double 2.00000000000000000e0)
  %t484 = load %xiao.value, ptr %t485
  store %xiao.value %t484, ptr %t481
  br label %cmath.arguments.ready191
cmath.arguments.fail190:
  call void @xiao_runtime_value_release_strong(ptr %t481)
  call void @xiao_runtime_value_release_strong(ptr %t480)
  br label %xiao.error.terminal
cmath.arguments.ready191:
  store %xiao.value zeroinitializer, ptr %t486
  %t487 = call i32 @xiao_runtime_cmath_pow(ptr %t481, ptr %t480, ptr %t486)
  call void @xiao_runtime_value_release_strong(ptr %t481)
  call void @xiao_runtime_value_release_strong(ptr %t480)
  switch i32 %t487, label %cmath.abi.failure194 [ i32 0, label %cmath.ok192 i32 6, label %cmath.error193 ]
cmath.abi.failure194:
  call void @xiao_runtime_value_release_strong(ptr %t486)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error193:
  call void @xiao_runtime_value_release_strong(ptr %t486)
  call i32 @xiao_runtime_error_attach_span(i64 1003, i64 1040)
  br label %xiao.error.terminal
cmath.ok192:
  %t488 = load %xiao.value, ptr %t486
  %t490 = getelementptr inbounds %xiao.value, ptr %t489, i64 0
  store %xiao.value %t488, ptr %t490
  %t491 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t489, i64 1, i64 127)
  %t492 = icmp eq i32 %t491, 0
  br i1 %t492, label %abi.ok195, label %abi.status.fail196
abi.status.fail196:
  call i32 @xiao_runtime_error_attach_span(i64 997, i64 1041)
  br label %xiao.error.terminal
abi.ok195:
  store %xiao.value %t488, ptr %t493
  call void @xiao_runtime_value_release_strong(ptr %t493)
  store %xiao.value zeroinitializer, ptr %t495
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t495)
  %t494 = load %xiao.value, ptr %t495
  store %xiao.value %t494, ptr %t496
  call void @xiao_runtime_value_release_strong(ptr %t496)
  store %xiao.value zeroinitializer, ptr %t497
  store %xiao.value zeroinitializer, ptr %t499
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t499, double 0.00000000000000000e0)
  %t498 = load %xiao.value, ptr %t499
  store %xiao.value %t498, ptr %t497
  br label %dynamic.call.arguments.ready198
dynamic.call.arguments.fail197:
  call void @xiao_runtime_value_release_strong(ptr %t497)
  br label %xiao.error.terminal
dynamic.call.arguments.ready198:
  store %xiao.value zeroinitializer, ptr %t500
  call void @xiao.fn.e5d6d1fbcd3af2ba(ptr %t500, ptr %t497)
  call void @xiao_runtime_value_release_strong(ptr %t497)
  %t501 = call i32 @xiao_runtime_error_class()
  %t502 = icmp eq i32 %t501, 0
  br i1 %t502, label %dynamic.call.ok199, label %dynamic.call.fail200
dynamic.call.fail200:
  br label %xiao.error.terminal
dynamic.call.ok199:
  %t503 = load %xiao.value, ptr %t500
  %t505 = getelementptr inbounds %xiao.value, ptr %t504, i64 0
  store %xiao.value %t503, ptr %t505
  %t506 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t504, i64 1, i64 127)
  %t507 = icmp eq i32 %t506, 0
  br i1 %t507, label %abi.ok201, label %abi.status.fail202
abi.status.fail202:
  call i32 @xiao_runtime_error_attach_span(i64 1077, i64 1091)
  br label %xiao.error.terminal
abi.ok201:
  store %xiao.value %t503, ptr %t508
  call void @xiao_runtime_value_release_strong(ptr %t508)
  store %xiao.value zeroinitializer, ptr %t510
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t510)
  %t509 = load %xiao.value, ptr %t510
  store %xiao.value %t509, ptr %t511
  call void @xiao_runtime_value_release_strong(ptr %t511)
  store %xiao.value zeroinitializer, ptr %t512
  store %xiao.value zeroinitializer, ptr %t514
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t514, double 0.00000000000000000e0)
  %t513 = load %xiao.value, ptr %t514
  store %xiao.value %t513, ptr %t512
  br label %dynamic.call.arguments.ready204
dynamic.call.arguments.fail203:
  call void @xiao_runtime_value_release_strong(ptr %t512)
  br label %xiao.error.terminal
dynamic.call.arguments.ready204:
  store %xiao.value zeroinitializer, ptr %t515
  call void @xiao.fn.e5d6d1fbcd3af2ba(ptr %t515, ptr %t512)
  call void @xiao_runtime_value_release_strong(ptr %t512)
  %t516 = call i32 @xiao_runtime_error_class()
  %t517 = icmp eq i32 %t516, 0
  br i1 %t517, label %dynamic.call.ok205, label %dynamic.call.fail206
dynamic.call.fail206:
  br label %xiao.error.terminal
dynamic.call.ok205:
  %t518 = load %xiao.value, ptr %t515
  %t520 = getelementptr inbounds %xiao.value, ptr %t519, i64 0
  store %xiao.value %t518, ptr %t520
  %t521 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t519, i64 1, i64 127)
  %t522 = icmp eq i32 %t521, 0
  br i1 %t522, label %abi.ok207, label %abi.status.fail208
abi.status.fail208:
  call i32 @xiao_runtime_error_attach_span(i64 1092, i64 1114)
  br label %xiao.error.terminal
abi.ok207:
  store %xiao.value %t518, ptr %t523
  call void @xiao_runtime_value_release_strong(ptr %t523)
  store %xiao.value zeroinitializer, ptr %t525
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t525)
  %t524 = load %xiao.value, ptr %t525
  store %xiao.value %t524, ptr %t526
  call void @xiao_runtime_value_release_strong(ptr %t526)
  store %xiao.value zeroinitializer, ptr %t527
  store %xiao.value zeroinitializer, ptr %t529
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t529, double 0.00000000000000000e0)
  %t528 = load %xiao.value, ptr %t529
  store %xiao.value %t528, ptr %t527
  br label %cmath.arguments.ready210
cmath.arguments.fail209:
  call void @xiao_runtime_value_release_strong(ptr %t527)
  br label %xiao.error.terminal
cmath.arguments.ready210:
  store %xiao.value zeroinitializer, ptr %t530
  %t531 = call i32 @xiao_runtime_cmath_sin(ptr %t527, ptr %t530)
  call void @xiao_runtime_value_release_strong(ptr %t527)
  switch i32 %t531, label %cmath.abi.failure213 [ i32 0, label %cmath.ok211 i32 6, label %cmath.error212 ]
cmath.abi.failure213:
  call void @xiao_runtime_value_release_strong(ptr %t530)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error212:
  call void @xiao_runtime_value_release_strong(ptr %t530)
  call i32 @xiao_runtime_error_attach_span(i64 1121, i64 1143)
  br label %xiao.error.terminal
cmath.ok211:
  %t532 = load %xiao.value, ptr %t530
  %t534 = getelementptr inbounds %xiao.value, ptr %t533, i64 0
  store %xiao.value %t532, ptr %t534
  %t535 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t533, i64 1, i64 127)
  %t536 = icmp eq i32 %t535, 0
  br i1 %t536, label %abi.ok214, label %abi.status.fail215
abi.status.fail215:
  call i32 @xiao_runtime_error_attach_span(i64 1115, i64 1144)
  br label %xiao.error.terminal
abi.ok214:
  store %xiao.value %t532, ptr %t537
  call void @xiao_runtime_value_release_strong(ptr %t537)
  store %xiao.value zeroinitializer, ptr %t539
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t539)
  %t538 = load %xiao.value, ptr %t539
  store %xiao.value %t538, ptr %t540
  call void @xiao_runtime_value_release_strong(ptr %t540)
  store %xiao.value zeroinitializer, ptr %t541
  store %xiao.value zeroinitializer, ptr %t543
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t543, double 0.00000000000000000e0)
  %t542 = load %xiao.value, ptr %t543
  store %xiao.value %t542, ptr %t541
  br label %dynamic.call.arguments.ready217
dynamic.call.arguments.fail216:
  call void @xiao_runtime_value_release_strong(ptr %t541)
  br label %xiao.error.terminal
dynamic.call.arguments.ready217:
  store %xiao.value zeroinitializer, ptr %t544
  call void @xiao.fn.e4403f97ead6ef36(ptr %t544, ptr %t541)
  call void @xiao_runtime_value_release_strong(ptr %t541)
  %t545 = call i32 @xiao_runtime_error_class()
  %t546 = icmp eq i32 %t545, 0
  br i1 %t546, label %dynamic.call.ok218, label %dynamic.call.fail219
dynamic.call.fail219:
  br label %xiao.error.terminal
dynamic.call.ok218:
  %t547 = load %xiao.value, ptr %t544
  %t549 = getelementptr inbounds %xiao.value, ptr %t548, i64 0
  store %xiao.value %t547, ptr %t549
  %t550 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t548, i64 1, i64 127)
  %t551 = icmp eq i32 %t550, 0
  br i1 %t551, label %abi.ok220, label %abi.status.fail221
abi.status.fail221:
  call i32 @xiao_runtime_error_attach_span(i64 1181, i64 1196)
  br label %xiao.error.terminal
abi.ok220:
  store %xiao.value %t547, ptr %t552
  call void @xiao_runtime_value_release_strong(ptr %t552)
  store %xiao.value zeroinitializer, ptr %t554
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t554)
  %t553 = load %xiao.value, ptr %t554
  store %xiao.value %t553, ptr %t555
  call void @xiao_runtime_value_release_strong(ptr %t555)
  store %xiao.value zeroinitializer, ptr %t556
  store %xiao.value zeroinitializer, ptr %t558
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t558, double 0.00000000000000000e0)
  %t557 = load %xiao.value, ptr %t558
  store %xiao.value %t557, ptr %t556
  br label %dynamic.call.arguments.ready223
dynamic.call.arguments.fail222:
  call void @xiao_runtime_value_release_strong(ptr %t556)
  br label %xiao.error.terminal
dynamic.call.arguments.ready223:
  store %xiao.value zeroinitializer, ptr %t559
  call void @xiao.fn.e4403f97ead6ef36(ptr %t559, ptr %t556)
  call void @xiao_runtime_value_release_strong(ptr %t556)
  %t560 = call i32 @xiao_runtime_error_class()
  %t561 = icmp eq i32 %t560, 0
  br i1 %t561, label %dynamic.call.ok224, label %dynamic.call.fail225
dynamic.call.fail225:
  br label %xiao.error.terminal
dynamic.call.ok224:
  %t562 = load %xiao.value, ptr %t559
  %t564 = getelementptr inbounds %xiao.value, ptr %t563, i64 0
  store %xiao.value %t562, ptr %t564
  %t565 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t563, i64 1, i64 127)
  %t566 = icmp eq i32 %t565, 0
  br i1 %t566, label %abi.ok226, label %abi.status.fail227
abi.status.fail227:
  call i32 @xiao_runtime_error_attach_span(i64 1197, i64 1220)
  br label %xiao.error.terminal
abi.ok226:
  store %xiao.value %t562, ptr %t567
  call void @xiao_runtime_value_release_strong(ptr %t567)
  store %xiao.value zeroinitializer, ptr %t569
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t569)
  %t568 = load %xiao.value, ptr %t569
  store %xiao.value %t568, ptr %t570
  call void @xiao_runtime_value_release_strong(ptr %t570)
  store %xiao.value zeroinitializer, ptr %t571
  store %xiao.value zeroinitializer, ptr %t573
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t573, double 0.00000000000000000e0)
  %t572 = load %xiao.value, ptr %t573
  store %xiao.value %t572, ptr %t571
  br label %cmath.arguments.ready229
cmath.arguments.fail228:
  call void @xiao_runtime_value_release_strong(ptr %t571)
  br label %xiao.error.terminal
cmath.arguments.ready229:
  store %xiao.value zeroinitializer, ptr %t574
  %t575 = call i32 @xiao_runtime_cmath_cos(ptr %t571, ptr %t574)
  call void @xiao_runtime_value_release_strong(ptr %t571)
  switch i32 %t575, label %cmath.abi.failure232 [ i32 0, label %cmath.ok230 i32 6, label %cmath.error231 ]
cmath.abi.failure232:
  call void @xiao_runtime_value_release_strong(ptr %t574)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error231:
  call void @xiao_runtime_value_release_strong(ptr %t574)
  call i32 @xiao_runtime_error_attach_span(i64 1227, i64 1249)
  br label %xiao.error.terminal
cmath.ok230:
  %t576 = load %xiao.value, ptr %t574
  %t578 = getelementptr inbounds %xiao.value, ptr %t577, i64 0
  store %xiao.value %t576, ptr %t578
  %t579 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t577, i64 1, i64 127)
  %t580 = icmp eq i32 %t579, 0
  br i1 %t580, label %abi.ok233, label %abi.status.fail234
abi.status.fail234:
  call i32 @xiao_runtime_error_attach_span(i64 1221, i64 1250)
  br label %xiao.error.terminal
abi.ok233:
  store %xiao.value %t576, ptr %t581
  call void @xiao_runtime_value_release_strong(ptr %t581)
  store %xiao.value zeroinitializer, ptr %t583
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t583)
  %t582 = load %xiao.value, ptr %t583
  store %xiao.value %t582, ptr %t584
  call void @xiao_runtime_value_release_strong(ptr %t584)
  store %xiao.value zeroinitializer, ptr %t585
  store %xiao.value zeroinitializer, ptr %t587
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t587, double 0.00000000000000000e0)
  %t586 = load %xiao.value, ptr %t587
  store %xiao.value %t586, ptr %t585
  br label %dynamic.call.arguments.ready236
dynamic.call.arguments.fail235:
  call void @xiao_runtime_value_release_strong(ptr %t585)
  br label %xiao.error.terminal
dynamic.call.arguments.ready236:
  store %xiao.value zeroinitializer, ptr %t588
  call void @xiao.fn.538f14385bfc00b5(ptr %t588, ptr %t585)
  call void @xiao_runtime_value_release_strong(ptr %t585)
  %t589 = call i32 @xiao_runtime_error_class()
  %t590 = icmp eq i32 %t589, 0
  br i1 %t590, label %dynamic.call.ok237, label %dynamic.call.fail238
dynamic.call.fail238:
  br label %xiao.error.terminal
dynamic.call.ok237:
  %t591 = load %xiao.value, ptr %t588
  %t593 = getelementptr inbounds %xiao.value, ptr %t592, i64 0
  store %xiao.value %t591, ptr %t593
  %t594 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t592, i64 1, i64 127)
  %t595 = icmp eq i32 %t594, 0
  br i1 %t595, label %abi.ok239, label %abi.status.fail240
abi.status.fail240:
  call i32 @xiao_runtime_error_attach_span(i64 1287, i64 1302)
  br label %xiao.error.terminal
abi.ok239:
  store %xiao.value %t591, ptr %t596
  call void @xiao_runtime_value_release_strong(ptr %t596)
  store %xiao.value zeroinitializer, ptr %t598
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t598)
  %t597 = load %xiao.value, ptr %t598
  store %xiao.value %t597, ptr %t599
  call void @xiao_runtime_value_release_strong(ptr %t599)
  store %xiao.value zeroinitializer, ptr %t600
  store %xiao.value zeroinitializer, ptr %t602
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t602, double 0.00000000000000000e0)
  %t601 = load %xiao.value, ptr %t602
  store %xiao.value %t601, ptr %t600
  br label %dynamic.call.arguments.ready242
dynamic.call.arguments.fail241:
  call void @xiao_runtime_value_release_strong(ptr %t600)
  br label %xiao.error.terminal
dynamic.call.arguments.ready242:
  store %xiao.value zeroinitializer, ptr %t603
  call void @xiao.fn.538f14385bfc00b5(ptr %t603, ptr %t600)
  call void @xiao_runtime_value_release_strong(ptr %t600)
  %t604 = call i32 @xiao_runtime_error_class()
  %t605 = icmp eq i32 %t604, 0
  br i1 %t605, label %dynamic.call.ok243, label %dynamic.call.fail244
dynamic.call.fail244:
  br label %xiao.error.terminal
dynamic.call.ok243:
  %t606 = load %xiao.value, ptr %t603
  %t608 = getelementptr inbounds %xiao.value, ptr %t607, i64 0
  store %xiao.value %t606, ptr %t608
  %t609 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t607, i64 1, i64 127)
  %t610 = icmp eq i32 %t609, 0
  br i1 %t610, label %abi.ok245, label %abi.status.fail246
abi.status.fail246:
  call i32 @xiao_runtime_error_attach_span(i64 1303, i64 1326)
  br label %xiao.error.terminal
abi.ok245:
  store %xiao.value %t606, ptr %t611
  call void @xiao_runtime_value_release_strong(ptr %t611)
  store %xiao.value zeroinitializer, ptr %t613
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t613)
  %t612 = load %xiao.value, ptr %t613
  store %xiao.value %t612, ptr %t614
  call void @xiao_runtime_value_release_strong(ptr %t614)
  store %xiao.value zeroinitializer, ptr %t615
  store %xiao.value zeroinitializer, ptr %t617
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t617, double 0.00000000000000000e0)
  %t616 = load %xiao.value, ptr %t617
  store %xiao.value %t616, ptr %t615
  br label %cmath.arguments.ready248
cmath.arguments.fail247:
  call void @xiao_runtime_value_release_strong(ptr %t615)
  br label %xiao.error.terminal
cmath.arguments.ready248:
  store %xiao.value zeroinitializer, ptr %t618
  %t619 = call i32 @xiao_runtime_cmath_tan(ptr %t615, ptr %t618)
  call void @xiao_runtime_value_release_strong(ptr %t615)
  switch i32 %t619, label %cmath.abi.failure251 [ i32 0, label %cmath.ok249 i32 6, label %cmath.error250 ]
cmath.abi.failure251:
  call void @xiao_runtime_value_release_strong(ptr %t618)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error250:
  call void @xiao_runtime_value_release_strong(ptr %t618)
  call i32 @xiao_runtime_error_attach_span(i64 1333, i64 1355)
  br label %xiao.error.terminal
cmath.ok249:
  %t620 = load %xiao.value, ptr %t618
  %t622 = getelementptr inbounds %xiao.value, ptr %t621, i64 0
  store %xiao.value %t620, ptr %t622
  %t623 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t621, i64 1, i64 127)
  %t624 = icmp eq i32 %t623, 0
  br i1 %t624, label %abi.ok252, label %abi.status.fail253
abi.status.fail253:
  call i32 @xiao_runtime_error_attach_span(i64 1327, i64 1356)
  br label %xiao.error.terminal
abi.ok252:
  store %xiao.value %t620, ptr %t625
  call void @xiao_runtime_value_release_strong(ptr %t625)
  store %xiao.value zeroinitializer, ptr %t627
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t627)
  %t626 = load %xiao.value, ptr %t627
  store %xiao.value %t626, ptr %t628
  call void @xiao_runtime_value_release_strong(ptr %t628)
  store %xiao.value zeroinitializer, ptr %t629
  store %xiao.value zeroinitializer, ptr %t631
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t631, double 0.00000000000000000e0)
  %t630 = load %xiao.value, ptr %t631
  store %xiao.value %t630, ptr %t629
  br label %dynamic.call.arguments.ready255
dynamic.call.arguments.fail254:
  call void @xiao_runtime_value_release_strong(ptr %t629)
  br label %xiao.error.terminal
dynamic.call.arguments.ready255:
  store %xiao.value zeroinitializer, ptr %t632
  call void @xiao.fn.d0cbe39c8d270929(ptr %t632, ptr %t629)
  call void @xiao_runtime_value_release_strong(ptr %t629)
  %t633 = call i32 @xiao_runtime_error_class()
  %t634 = icmp eq i32 %t633, 0
  br i1 %t634, label %dynamic.call.ok256, label %dynamic.call.fail257
dynamic.call.fail257:
  br label %xiao.error.terminal
dynamic.call.ok256:
  %t635 = load %xiao.value, ptr %t632
  %t637 = getelementptr inbounds %xiao.value, ptr %t636, i64 0
  store %xiao.value %t635, ptr %t637
  %t638 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t636, i64 1, i64 127)
  %t639 = icmp eq i32 %t638, 0
  br i1 %t639, label %abi.ok258, label %abi.status.fail259
abi.status.fail259:
  call i32 @xiao_runtime_error_attach_span(i64 1393, i64 1408)
  br label %xiao.error.terminal
abi.ok258:
  store %xiao.value %t635, ptr %t640
  call void @xiao_runtime_value_release_strong(ptr %t640)
  store %xiao.value zeroinitializer, ptr %t642
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t642)
  %t641 = load %xiao.value, ptr %t642
  store %xiao.value %t641, ptr %t643
  call void @xiao_runtime_value_release_strong(ptr %t643)
  store %xiao.value zeroinitializer, ptr %t644
  store %xiao.value zeroinitializer, ptr %t646
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t646, double 0.00000000000000000e0)
  %t645 = load %xiao.value, ptr %t646
  store %xiao.value %t645, ptr %t644
  br label %dynamic.call.arguments.ready261
dynamic.call.arguments.fail260:
  call void @xiao_runtime_value_release_strong(ptr %t644)
  br label %xiao.error.terminal
dynamic.call.arguments.ready261:
  store %xiao.value zeroinitializer, ptr %t647
  call void @xiao.fn.d0cbe39c8d270929(ptr %t647, ptr %t644)
  call void @xiao_runtime_value_release_strong(ptr %t644)
  %t648 = call i32 @xiao_runtime_error_class()
  %t649 = icmp eq i32 %t648, 0
  br i1 %t649, label %dynamic.call.ok262, label %dynamic.call.fail263
dynamic.call.fail263:
  br label %xiao.error.terminal
dynamic.call.ok262:
  %t650 = load %xiao.value, ptr %t647
  %t652 = getelementptr inbounds %xiao.value, ptr %t651, i64 0
  store %xiao.value %t650, ptr %t652
  %t653 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t651, i64 1, i64 127)
  %t654 = icmp eq i32 %t653, 0
  br i1 %t654, label %abi.ok264, label %abi.status.fail265
abi.status.fail265:
  call i32 @xiao_runtime_error_attach_span(i64 1409, i64 1432)
  br label %xiao.error.terminal
abi.ok264:
  store %xiao.value %t650, ptr %t655
  call void @xiao_runtime_value_release_strong(ptr %t655)
  store %xiao.value zeroinitializer, ptr %t657
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t657)
  %t656 = load %xiao.value, ptr %t657
  store %xiao.value %t656, ptr %t658
  call void @xiao_runtime_value_release_strong(ptr %t658)
  store %xiao.value zeroinitializer, ptr %t659
  store %xiao.value zeroinitializer, ptr %t661
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t661, double 0.00000000000000000e0)
  %t660 = load %xiao.value, ptr %t661
  store %xiao.value %t660, ptr %t659
  br label %cmath.arguments.ready267
cmath.arguments.fail266:
  call void @xiao_runtime_value_release_strong(ptr %t659)
  br label %xiao.error.terminal
cmath.arguments.ready267:
  store %xiao.value zeroinitializer, ptr %t662
  %t663 = call i32 @xiao_runtime_cmath_exp(ptr %t659, ptr %t662)
  call void @xiao_runtime_value_release_strong(ptr %t659)
  switch i32 %t663, label %cmath.abi.failure270 [ i32 0, label %cmath.ok268 i32 6, label %cmath.error269 ]
cmath.abi.failure270:
  call void @xiao_runtime_value_release_strong(ptr %t662)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error269:
  call void @xiao_runtime_value_release_strong(ptr %t662)
  call i32 @xiao_runtime_error_attach_span(i64 1439, i64 1461)
  br label %xiao.error.terminal
cmath.ok268:
  %t664 = load %xiao.value, ptr %t662
  %t666 = getelementptr inbounds %xiao.value, ptr %t665, i64 0
  store %xiao.value %t664, ptr %t666
  %t667 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t665, i64 1, i64 127)
  %t668 = icmp eq i32 %t667, 0
  br i1 %t668, label %abi.ok271, label %abi.status.fail272
abi.status.fail272:
  call i32 @xiao_runtime_error_attach_span(i64 1433, i64 1462)
  br label %xiao.error.terminal
abi.ok271:
  store %xiao.value %t664, ptr %t669
  call void @xiao_runtime_value_release_strong(ptr %t669)
  store %xiao.value zeroinitializer, ptr %t671
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t671)
  %t670 = load %xiao.value, ptr %t671
  store %xiao.value %t670, ptr %t672
  call void @xiao_runtime_value_release_strong(ptr %t672)
  store %xiao.value zeroinitializer, ptr %t673
  store %xiao.value zeroinitializer, ptr %t675
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t675, double 1.00000000000000000e0)
  %t674 = load %xiao.value, ptr %t675
  store %xiao.value %t674, ptr %t673
  br label %dynamic.call.arguments.ready274
dynamic.call.arguments.fail273:
  call void @xiao_runtime_value_release_strong(ptr %t673)
  br label %xiao.error.terminal
dynamic.call.arguments.ready274:
  store %xiao.value zeroinitializer, ptr %t676
  call void @xiao.fn.1c7fbe93e376e39c(ptr %t676, ptr %t673)
  call void @xiao_runtime_value_release_strong(ptr %t673)
  %t677 = call i32 @xiao_runtime_error_class()
  %t678 = icmp eq i32 %t677, 0
  br i1 %t678, label %dynamic.call.ok275, label %dynamic.call.fail276
dynamic.call.fail276:
  br label %xiao.error.terminal
dynamic.call.ok275:
  %t679 = load %xiao.value, ptr %t676
  %t681 = getelementptr inbounds %xiao.value, ptr %t680, i64 0
  store %xiao.value %t679, ptr %t681
  %t682 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t680, i64 1, i64 127)
  %t683 = icmp eq i32 %t682, 0
  br i1 %t683, label %abi.ok277, label %abi.status.fail278
abi.status.fail278:
  call i32 @xiao_runtime_error_attach_span(i64 1498, i64 1513)
  br label %xiao.error.terminal
abi.ok277:
  store %xiao.value %t679, ptr %t684
  call void @xiao_runtime_value_release_strong(ptr %t684)
  store %xiao.value zeroinitializer, ptr %t686
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t686)
  %t685 = load %xiao.value, ptr %t686
  store %xiao.value %t685, ptr %t687
  call void @xiao_runtime_value_release_strong(ptr %t687)
  store %xiao.value zeroinitializer, ptr %t688
  store %xiao.value zeroinitializer, ptr %t690
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t690, double 1.00000000000000000e0)
  %t689 = load %xiao.value, ptr %t690
  store %xiao.value %t689, ptr %t688
  br label %dynamic.call.arguments.ready280
dynamic.call.arguments.fail279:
  call void @xiao_runtime_value_release_strong(ptr %t688)
  br label %xiao.error.terminal
dynamic.call.arguments.ready280:
  store %xiao.value zeroinitializer, ptr %t691
  call void @xiao.fn.1c7fbe93e376e39c(ptr %t691, ptr %t688)
  call void @xiao_runtime_value_release_strong(ptr %t688)
  %t692 = call i32 @xiao_runtime_error_class()
  %t693 = icmp eq i32 %t692, 0
  br i1 %t693, label %dynamic.call.ok281, label %dynamic.call.fail282
dynamic.call.fail282:
  br label %xiao.error.terminal
dynamic.call.ok281:
  %t694 = load %xiao.value, ptr %t691
  %t696 = getelementptr inbounds %xiao.value, ptr %t695, i64 0
  store %xiao.value %t694, ptr %t696
  %t697 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t695, i64 1, i64 127)
  %t698 = icmp eq i32 %t697, 0
  br i1 %t698, label %abi.ok283, label %abi.status.fail284
abi.status.fail284:
  call i32 @xiao_runtime_error_attach_span(i64 1514, i64 1537)
  br label %xiao.error.terminal
abi.ok283:
  store %xiao.value %t694, ptr %t699
  call void @xiao_runtime_value_release_strong(ptr %t699)
  store %xiao.value zeroinitializer, ptr %t701
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t701)
  %t700 = load %xiao.value, ptr %t701
  store %xiao.value %t700, ptr %t702
  call void @xiao_runtime_value_release_strong(ptr %t702)
  store %xiao.value zeroinitializer, ptr %t703
  store %xiao.value zeroinitializer, ptr %t705
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t705, double 1.00000000000000000e0)
  %t704 = load %xiao.value, ptr %t705
  store %xiao.value %t704, ptr %t703
  br label %cmath.arguments.ready286
cmath.arguments.fail285:
  call void @xiao_runtime_value_release_strong(ptr %t703)
  br label %xiao.error.terminal
cmath.arguments.ready286:
  store %xiao.value zeroinitializer, ptr %t706
  %t707 = call i32 @xiao_runtime_cmath_ln(ptr %t703, ptr %t706)
  call void @xiao_runtime_value_release_strong(ptr %t703)
  switch i32 %t707, label %cmath.abi.failure289 [ i32 0, label %cmath.ok287 i32 6, label %cmath.error288 ]
cmath.abi.failure289:
  call void @xiao_runtime_value_release_strong(ptr %t706)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error288:
  call void @xiao_runtime_value_release_strong(ptr %t706)
  call i32 @xiao_runtime_error_attach_span(i64 1544, i64 1565)
  br label %xiao.error.terminal
cmath.ok287:
  %t708 = load %xiao.value, ptr %t706
  %t710 = getelementptr inbounds %xiao.value, ptr %t709, i64 0
  store %xiao.value %t708, ptr %t710
  %t711 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t709, i64 1, i64 127)
  %t712 = icmp eq i32 %t711, 0
  br i1 %t712, label %abi.ok290, label %abi.status.fail291
abi.status.fail291:
  call i32 @xiao_runtime_error_attach_span(i64 1538, i64 1566)
  br label %xiao.error.terminal
abi.ok290:
  store %xiao.value %t708, ptr %t713
  call void @xiao_runtime_value_release_strong(ptr %t713)
  store %xiao.value zeroinitializer, ptr %t715
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t715)
  %t714 = load %xiao.value, ptr %t715
  store %xiao.value %t714, ptr %t716
  call void @xiao_runtime_value_release_strong(ptr %t716)
  store %xiao.value zeroinitializer, ptr %t717
  store %xiao.value zeroinitializer, ptr %t719
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t719, double 1.00000000000000000e0)
  %t718 = load %xiao.value, ptr %t719
  store %xiao.value %t718, ptr %t717
  br label %dynamic.call.arguments.ready293
dynamic.call.arguments.fail292:
  call void @xiao_runtime_value_release_strong(ptr %t717)
  br label %xiao.error.terminal
dynamic.call.arguments.ready293:
  store %xiao.value zeroinitializer, ptr %t720
  call void @xiao.fn.4e7be7efa34b4cfc(ptr %t720, ptr %t717)
  call void @xiao_runtime_value_release_strong(ptr %t717)
  %t721 = call i32 @xiao_runtime_error_class()
  %t722 = icmp eq i32 %t721, 0
  br i1 %t722, label %dynamic.call.ok294, label %dynamic.call.fail295
dynamic.call.fail295:
  br label %xiao.error.terminal
dynamic.call.ok294:
  %t723 = load %xiao.value, ptr %t720
  %t725 = getelementptr inbounds %xiao.value, ptr %t724, i64 0
  store %xiao.value %t723, ptr %t725
  %t726 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t724, i64 1, i64 127)
  %t727 = icmp eq i32 %t726, 0
  br i1 %t727, label %abi.ok296, label %abi.status.fail297
abi.status.fail297:
  call i32 @xiao_runtime_error_attach_span(i64 1605, i64 1620)
  br label %xiao.error.terminal
abi.ok296:
  store %xiao.value %t723, ptr %t728
  call void @xiao_runtime_value_release_strong(ptr %t728)
  store %xiao.value zeroinitializer, ptr %t730
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t730)
  %t729 = load %xiao.value, ptr %t730
  store %xiao.value %t729, ptr %t731
  call void @xiao_runtime_value_release_strong(ptr %t731)
  store %xiao.value zeroinitializer, ptr %t732
  store %xiao.value zeroinitializer, ptr %t734
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t734, double 1.00000000000000000e0)
  %t733 = load %xiao.value, ptr %t734
  store %xiao.value %t733, ptr %t732
  br label %dynamic.call.arguments.ready299
dynamic.call.arguments.fail298:
  call void @xiao_runtime_value_release_strong(ptr %t732)
  br label %xiao.error.terminal
dynamic.call.arguments.ready299:
  store %xiao.value zeroinitializer, ptr %t735
  call void @xiao.fn.4e7be7efa34b4cfc(ptr %t735, ptr %t732)
  call void @xiao_runtime_value_release_strong(ptr %t732)
  %t736 = call i32 @xiao_runtime_error_class()
  %t737 = icmp eq i32 %t736, 0
  br i1 %t737, label %dynamic.call.ok300, label %dynamic.call.fail301
dynamic.call.fail301:
  br label %xiao.error.terminal
dynamic.call.ok300:
  %t738 = load %xiao.value, ptr %t735
  %t740 = getelementptr inbounds %xiao.value, ptr %t739, i64 0
  store %xiao.value %t738, ptr %t740
  %t741 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t739, i64 1, i64 127)
  %t742 = icmp eq i32 %t741, 0
  br i1 %t742, label %abi.ok302, label %abi.status.fail303
abi.status.fail303:
  call i32 @xiao_runtime_error_attach_span(i64 1621, i64 1644)
  br label %xiao.error.terminal
abi.ok302:
  store %xiao.value %t738, ptr %t743
  call void @xiao_runtime_value_release_strong(ptr %t743)
  store %xiao.value zeroinitializer, ptr %t745
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t745)
  %t744 = load %xiao.value, ptr %t745
  store %xiao.value %t744, ptr %t746
  call void @xiao_runtime_value_release_strong(ptr %t746)
  store %xiao.value zeroinitializer, ptr %t747
  store %xiao.value zeroinitializer, ptr %t749
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t749, double 1.00000000000000000e0)
  %t748 = load %xiao.value, ptr %t749
  store %xiao.value %t748, ptr %t747
  br label %cmath.arguments.ready305
cmath.arguments.fail304:
  call void @xiao_runtime_value_release_strong(ptr %t747)
  br label %xiao.error.terminal
cmath.arguments.ready305:
  store %xiao.value zeroinitializer, ptr %t750
  %t751 = call i32 @xiao_runtime_cmath_log10(ptr %t747, ptr %t750)
  call void @xiao_runtime_value_release_strong(ptr %t747)
  switch i32 %t751, label %cmath.abi.failure308 [ i32 0, label %cmath.ok306 i32 6, label %cmath.error307 ]
cmath.abi.failure308:
  call void @xiao_runtime_value_release_strong(ptr %t750)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error307:
  call void @xiao_runtime_value_release_strong(ptr %t750)
  call i32 @xiao_runtime_error_attach_span(i64 1651, i64 1675)
  br label %xiao.error.terminal
cmath.ok306:
  %t752 = load %xiao.value, ptr %t750
  %t754 = getelementptr inbounds %xiao.value, ptr %t753, i64 0
  store %xiao.value %t752, ptr %t754
  %t755 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t753, i64 1, i64 127)
  %t756 = icmp eq i32 %t755, 0
  br i1 %t756, label %abi.ok309, label %abi.status.fail310
abi.status.fail310:
  call i32 @xiao_runtime_error_attach_span(i64 1645, i64 1676)
  br label %xiao.error.terminal
abi.ok309:
  store %xiao.value %t752, ptr %t757
  call void @xiao_runtime_value_release_strong(ptr %t757)
  store %xiao.value zeroinitializer, ptr %t759
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t759)
  %t758 = load %xiao.value, ptr %t759
  store %xiao.value %t758, ptr %t760
  call void @xiao_runtime_value_release_strong(ptr %t760)
  store %xiao.value zeroinitializer, ptr %t761
  store %xiao.value zeroinitializer, ptr %t762
  br label %dynamic.call.arguments.ready314
dynamic.call.arguments.fail313:
  br label %dynamic.call.arguments.fail311
dynamic.call.arguments.ready314:
  store %xiao.value zeroinitializer, ptr %t763
  call void @xiao.fn.d41bba4dda4a06ca(ptr %t763)
  %t764 = call i32 @xiao_runtime_error_class()
  %t765 = icmp eq i32 %t764, 0
  br i1 %t765, label %dynamic.call.ok315, label %dynamic.call.fail316
dynamic.call.fail316:
  br label %dynamic.call.arguments.fail311
dynamic.call.ok315:
  %t766 = load %xiao.value, ptr %t763
  store %xiao.value %t766, ptr %t761
  br label %dynamic.call.arguments.ready318
dynamic.call.arguments.fail317:
  br label %dynamic.call.arguments.fail311
dynamic.call.arguments.ready318:
  store %xiao.value zeroinitializer, ptr %t767
  call void @xiao.fn.e86580f5191d83b7(ptr %t767)
  %t768 = call i32 @xiao_runtime_error_class()
  %t769 = icmp eq i32 %t768, 0
  br i1 %t769, label %dynamic.call.ok319, label %dynamic.call.fail320
dynamic.call.fail320:
  br label %dynamic.call.arguments.fail311
dynamic.call.ok319:
  %t770 = load %xiao.value, ptr %t767
  store %xiao.value %t770, ptr %t762
  br label %dynamic.call.arguments.ready312
dynamic.call.arguments.fail311:
  call void @xiao_runtime_value_release_strong(ptr %t761)
  call void @xiao_runtime_value_release_strong(ptr %t762)
  br label %xiao.error.terminal
dynamic.call.arguments.ready312:
  store %xiao.value zeroinitializer, ptr %t771
  call void @xiao.fn.4e6166bed798273c(ptr %t771, ptr %t762, ptr %t761)
  call void @xiao_runtime_value_release_strong(ptr %t761)
  call void @xiao_runtime_value_release_strong(ptr %t762)
  %t772 = call i32 @xiao_runtime_error_class()
  %t773 = icmp eq i32 %t772, 0
  br i1 %t773, label %dynamic.call.ok321, label %dynamic.call.fail322
dynamic.call.fail322:
  br label %xiao.error.terminal
dynamic.call.ok321:
  %t774 = load %xiao.value, ptr %t771
  %t776 = getelementptr inbounds %xiao.value, ptr %t775, i64 0
  store %xiao.value %t774, ptr %t776
  %t777 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t775, i64 1, i64 127)
  %t778 = icmp eq i32 %t777, 0
  br i1 %t778, label %abi.ok323, label %abi.status.fail324
abi.status.fail324:
  call i32 @xiao_runtime_error_attach_span(i64 1791, i64 1838)
  br label %xiao.error.terminal
abi.ok323:
  store %xiao.value %t774, ptr %t779
  call void @xiao_runtime_value_release_strong(ptr %t779)
  store %xiao.value zeroinitializer, ptr %t781
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t781)
  %t780 = load %xiao.value, ptr %t781
  store %xiao.value %t780, ptr %t782
  call void @xiao_runtime_value_release_strong(ptr %t782)
  store %xiao.value zeroinitializer, ptr %t783
  store %xiao.value zeroinitializer, ptr %t784
  store %xiao.value zeroinitializer, ptr %t786
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t786, i64 7)
  %t785 = load %xiao.value, ptr %t786
  store %xiao.value %t785, ptr %t783
  store %xiao.value zeroinitializer, ptr %t788
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t788, i64 2)
  %t787 = load %xiao.value, ptr %t788
  store %xiao.value %t787, ptr %t784
  br label %dynamic.call.arguments.ready326
dynamic.call.arguments.fail325:
  call void @xiao_runtime_value_release_strong(ptr %t783)
  call void @xiao_runtime_value_release_strong(ptr %t784)
  br label %xiao.error.terminal
dynamic.call.arguments.ready326:
  store %xiao.value zeroinitializer, ptr %t789
  call void @xiao.fn.476bad2c9b53486a(ptr %t789, ptr %t783, ptr %t784)
  call void @xiao_runtime_value_release_strong(ptr %t783)
  call void @xiao_runtime_value_release_strong(ptr %t784)
  %t790 = call i32 @xiao_runtime_error_class()
  %t791 = icmp eq i32 %t790, 0
  br i1 %t791, label %dynamic.call.ok327, label %dynamic.call.fail328
dynamic.call.fail328:
  br label %xiao.error.terminal
dynamic.call.ok327:
  %t792 = load %xiao.value, ptr %t789
  %t794 = getelementptr inbounds %xiao.value, ptr %t793, i64 0
  store %xiao.value %t792, ptr %t794
  %t795 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t793, i64 1, i64 127)
  %t796 = icmp eq i32 %t795, 0
  br i1 %t796, label %abi.ok329, label %abi.status.fail330
abi.status.fail330:
  call i32 @xiao_runtime_error_attach_span(i64 1923, i64 1947)
  br label %xiao.error.terminal
abi.ok329:
  store %xiao.value %t792, ptr %t797
  call void @xiao_runtime_value_release_strong(ptr %t797)
  store %xiao.value zeroinitializer, ptr %t799
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t799)
  %t798 = load %xiao.value, ptr %t799
  store %xiao.value %t798, ptr %t800
  call void @xiao_runtime_value_release_strong(ptr %t800)
  store %xiao.value zeroinitializer, ptr %t801
  store %xiao.value zeroinitializer, ptr %t804
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t804, i64 7)
  %t803 = load %xiao.value, ptr %t804
  %t805 = getelementptr %xiao.value, ptr %t802, i64 0
  store %xiao.value %t803, ptr %t805
  store %xiao.value zeroinitializer, ptr %t807
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t807, i64 2)
  %t806 = load %xiao.value, ptr %t807
  %t808 = getelementptr %xiao.value, ptr %t802, i64 1
  store %xiao.value %t806, ptr %t808
  store ptr null, ptr %t809
  %t810 = call i32 @xiao_runtime_array_new(ptr %t802, i64 2, ptr %t809)
  %t811 = icmp eq i32 %t810, 0
  br i1 %t811, label %abi.ok333, label %abi.status.fail334
abi.status.fail334:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.call.arguments.fail331
abi.ok333:
  store %xiao.value %t803, ptr %t812
  call void @xiao_runtime_value_release_strong(ptr %t812)
  store %xiao.value %t806, ptr %t813
  call void @xiao_runtime_value_release_strong(ptr %t813)
  %t814 = load ptr, ptr %t809
  store %xiao.value zeroinitializer, ptr %t816
  call void @xiao_runtime_value_array_owned(ptr sret(%xiao.value) %t816, ptr %t814)
  %t815 = load %xiao.value, ptr %t816
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site0)
  store %xiao.value %t815, ptr %xiao.statement.site0
  %t817 = call i32 @xiao_runtime_value_copy(ptr %xiao.statement.site0, ptr %t801)
  %t818 = icmp eq i32 %t817, 0
  br i1 %t818, label %abi.ok335, label %abi.status.fail336
abi.status.fail336:
  call i32 @xiao_runtime_error_attach_span(i64 1964, i64 1970)
  br label %dynamic.call.arguments.fail331
abi.ok335:
  br label %dynamic.call.arguments.ready332
dynamic.call.arguments.fail331:
  call void @xiao_runtime_value_release_strong(ptr %t801)
  br label %xiao.error.terminal
dynamic.call.arguments.ready332:
  store %xiao.value zeroinitializer, ptr %t819
  call void @xiao.fn.b1efe0225cc1dafd(ptr %t819, ptr %t801)
  call void @xiao_runtime_value_release_strong(ptr %t801)
  %t820 = call i32 @xiao_runtime_error_class()
  %t821 = icmp eq i32 %t820, 0
  br i1 %t821, label %dynamic.call.ok337, label %dynamic.call.fail338
dynamic.call.fail338:
  br label %xiao.error.terminal
dynamic.call.ok337:
  %t822 = load %xiao.value, ptr %t819
  %t824 = getelementptr inbounds %xiao.value, ptr %t823, i64 0
  store %xiao.value %t822, ptr %t824
  %t825 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t823, i64 1, i64 127)
  %t826 = icmp eq i32 %t825, 0
  br i1 %t826, label %abi.ok339, label %abi.status.fail340
abi.status.fail340:
  call i32 @xiao_runtime_error_attach_span(i64 1948, i64 1972)
  br label %xiao.error.terminal
abi.ok339:
  store %xiao.value %t822, ptr %t827
  call void @xiao_runtime_value_release_strong(ptr %t827)
  store %xiao.value zeroinitializer, ptr %t829
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t829)
  %t828 = load %xiao.value, ptr %t829
  store %xiao.value %t828, ptr %t830
  call void @xiao_runtime_value_release_strong(ptr %t830)
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site0)
  %t831 = call i32 @xiao_runtime_error_class()
  %t832 = icmp eq i32 %t831, 0
  br i1 %t832, label %abi.error.ok341, label %abi.error.fail342
abi.error.fail342:
  call i32 @xiao_runtime_error_attach_span(i64 1948, i64 1972)
  br label %xiao.error.terminal
abi.error.ok341:
  store %xiao.value zeroinitializer, ptr %t833
  store %xiao.value zeroinitializer, ptr %t834
  store %xiao.value zeroinitializer, ptr %t836
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t836, double 7.00000000000000000e0)
  %t835 = load %xiao.value, ptr %t836
  store %xiao.value %t835, ptr %t833
  store %xiao.value zeroinitializer, ptr %t838
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t838, double 2.00000000000000000e0)
  %t837 = load %xiao.value, ptr %t838
  store %xiao.value %t837, ptr %t834
  br label %dynamic.call.arguments.ready344
dynamic.call.arguments.fail343:
  call void @xiao_runtime_value_release_strong(ptr %t833)
  call void @xiao_runtime_value_release_strong(ptr %t834)
  br label %xiao.error.terminal
dynamic.call.arguments.ready344:
  store %xiao.value zeroinitializer, ptr %t839
  call void @xiao.fn.b66bfb82035c3390(ptr %t839, ptr %t833, ptr %t834)
  call void @xiao_runtime_value_release_strong(ptr %t833)
  call void @xiao_runtime_value_release_strong(ptr %t834)
  %t840 = call i32 @xiao_runtime_error_class()
  %t841 = icmp eq i32 %t840, 0
  br i1 %t841, label %dynamic.call.ok345, label %dynamic.call.fail346
dynamic.call.fail346:
  br label %xiao.error.terminal
dynamic.call.ok345:
  %t842 = load %xiao.value, ptr %t839
  %t844 = getelementptr inbounds %xiao.value, ptr %t843, i64 0
  store %xiao.value %t842, ptr %t844
  %t845 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t843, i64 1, i64 127)
  %t846 = icmp eq i32 %t845, 0
  br i1 %t846, label %abi.ok347, label %abi.status.fail348
abi.status.fail348:
  call i32 @xiao_runtime_error_attach_span(i64 2015, i64 2041)
  br label %xiao.error.terminal
abi.ok347:
  store %xiao.value %t842, ptr %t847
  call void @xiao_runtime_value_release_strong(ptr %t847)
  store %xiao.value zeroinitializer, ptr %t849
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t849)
  %t848 = load %xiao.value, ptr %t849
  store %xiao.value %t848, ptr %t850
  call void @xiao_runtime_value_release_strong(ptr %t850)
  store %xiao.value zeroinitializer, ptr %t851
  store %xiao.value zeroinitializer, ptr %t853
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t853, double 1.00000000000000000e0)
  %t852 = load %xiao.value, ptr %t853
  %t854 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes7, i64 0, i64 0
  %t855 = insertvalue %xiao.bytes zeroinitializer, ptr %t854, 0
  %t856 = insertvalue %xiao.bytes %t855, i64 5, 1
  store %xiao.bytes %t856, ptr %t857
  store %xiao.value %t852, ptr %t858
  store %xiao.value zeroinitializer, ptr %t859
  %t860 = call i32 @xiao_runtime_value_unary(ptr %t857, ptr %t858, ptr %t859)
  %t861 = icmp eq i32 %t860, 0
  br i1 %t861, label %abi.ok356, label %abi.status.fail357
abi.status.fail357:
  call i32 @xiao_runtime_error_attach_span(i64 2053, i64 2057)
  br label %dynamic.unary.fail354
abi.ok356:
  %t862 = load %xiao.value, ptr %t859
  store %xiao.value %t852, ptr %t863
  call void @xiao_runtime_value_release_strong(ptr %t863)
  br label %dynamic.unary.continue355
dynamic.unary.fail354:
  store %xiao.value %t852, ptr %t864
  call void @xiao_runtime_value_release_strong(ptr %t864)
  br label %dynamic.call.arguments.fail352
dynamic.unary.continue355:
  store %xiao.value %t862, ptr %t851
  br label %dynamic.call.arguments.ready353
dynamic.call.arguments.fail352:
  call void @xiao_runtime_value_release_strong(ptr %t851)
  br label %dynamic.try.dispatch349
dynamic.call.arguments.ready353:
  store %xiao.value zeroinitializer, ptr %t865
  call void @xiao.fn.cb1ffaaf3c3d15fa(ptr %t865, ptr %t851)
  call void @xiao_runtime_value_release_strong(ptr %t851)
  %t866 = call i32 @xiao_runtime_error_class()
  %t867 = icmp eq i32 %t866, 0
  br i1 %t867, label %dynamic.call.ok358, label %dynamic.call.fail359
dynamic.call.fail359:
  br label %dynamic.try.dispatch349
dynamic.call.ok358:
  %t868 = load %xiao.value, ptr %t865
  store %xiao.value %t868, ptr %t869
  call void @xiao_runtime_value_release_strong(ptr %t869)
  br label %dynamic.try.normal350
dynamic.try.normal350:
  call void @xiao_runtime_error_checkpoint_begin()
  %t870 = call ptr @llvm.stacksave()
  %t871 = getelementptr inbounds [6 x i8], ptr @.xiao.str8, i64 0, i64 0
  %t872 = insertvalue %xiao.bytes zeroinitializer, ptr %t871, 0
  %t873 = insertvalue %xiao.bytes %t872, i64 5, 1
  store %xiao.bytes %t873, ptr %t874
  store ptr null, ptr %t875
  %t876 = call i32 @xiao_runtime_string_new(ptr %t874, ptr %t875)
  %t877 = icmp eq i32 %t876, 0
  br i1 %t877, label %abi.ok362, label %abi.status.fail363
abi.status.fail363:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.try.normal.finally.fail360
abi.ok362:
  %t878 = load ptr, ptr %t875
  store %xiao.value zeroinitializer, ptr %t880
  call void @xiao_runtime_value_str_owned(ptr sret(%xiao.value) %t880, ptr %t878)
  %t879 = load %xiao.value, ptr %t880
  %t882 = getelementptr inbounds %xiao.value, ptr %t881, i64 0
  store %xiao.value %t879, ptr %t882
  %t883 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t881, i64 1, i64 127)
  %t884 = icmp eq i32 %t883, 0
  br i1 %t884, label %abi.ok364, label %abi.status.fail365
abi.status.fail365:
  call i32 @xiao_runtime_error_attach_span(i64 2135, i64 2149)
  br label %dynamic.try.normal.finally.fail360
abi.ok364:
  store %xiao.value %t879, ptr %t885
  call void @xiao_runtime_value_release_strong(ptr %t885)
  store %xiao.value zeroinitializer, ptr %t887
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t887)
  %t886 = load %xiao.value, ptr %t887
  store %xiao.value %t886, ptr %t888
  call void @xiao_runtime_value_release_strong(ptr %t888)
  call i32 @xiao_runtime_error_checkpoint_finish()
  call void @llvm.stackrestore(ptr %t870)
  br label %dynamic.try.normal.finally.success361
dynamic.try.normal.finally.fail360:
  call i32 @xiao_runtime_error_checkpoint_finish()
  call void @llvm.stackrestore(ptr %t870)
  %t889 = call i32 @xiao_runtime_error_class()
  %t890 = icmp eq i32 %t889, 2
  br i1 %t890, label %xiao.error.terminal, label %dynamic.error-cleanup.recoverable366
dynamic.error-cleanup.recoverable366:
  br label %xiao.error.terminal
dynamic.try.normal.finally.success361:
  br label %dynamic.try.continue351
dynamic.try.dispatch349:
  %t891 = call i32 @xiao_runtime_error_class()
  %t892 = icmp eq i32 %t891, 2
  br i1 %t892, label %xiao.error.terminal, label %dynamic.try.recoverable367
dynamic.try.recoverable367:
  %t893 = getelementptr inbounds [16 x i8], ptr @.xiao.bytes9, i64 0, i64 0
  %t894 = insertvalue %xiao.bytes zeroinitializer, ptr %t893, 0
  %t895 = insertvalue %xiao.bytes %t894, i64 15, 1
  store %xiao.bytes %t895, ptr %t896
  %t897 = call i32 @xiao_runtime_error_matches(ptr %t896)
  %t898 = icmp eq i32 %t897, 1
  br i1 %t898, label %dynamic.try.catch0369, label %dynamic.try.unmatched368
dynamic.try.unmatched368:
  call void @xiao_runtime_error_checkpoint_begin()
  %t899 = call ptr @llvm.stacksave()
  %t900 = getelementptr inbounds [6 x i8], ptr @.xiao.str10, i64 0, i64 0
  %t901 = insertvalue %xiao.bytes zeroinitializer, ptr %t900, 0
  %t902 = insertvalue %xiao.bytes %t901, i64 5, 1
  store %xiao.bytes %t902, ptr %t903
  store ptr null, ptr %t904
  %t905 = call i32 @xiao_runtime_string_new(ptr %t903, ptr %t904)
  %t906 = icmp eq i32 %t905, 0
  br i1 %t906, label %abi.ok372, label %abi.status.fail373
abi.status.fail373:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.try.unmatched.finally.fail370
abi.ok372:
  %t907 = load ptr, ptr %t904
  store %xiao.value zeroinitializer, ptr %t909
  call void @xiao_runtime_value_str_owned(ptr sret(%xiao.value) %t909, ptr %t907)
  %t908 = load %xiao.value, ptr %t909
  %t911 = getelementptr inbounds %xiao.value, ptr %t910, i64 0
  store %xiao.value %t908, ptr %t911
  %t912 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t910, i64 1, i64 127)
  %t913 = icmp eq i32 %t912, 0
  br i1 %t913, label %abi.ok374, label %abi.status.fail375
abi.status.fail375:
  call i32 @xiao_runtime_error_attach_span(i64 2135, i64 2149)
  br label %dynamic.try.unmatched.finally.fail370
abi.ok374:
  store %xiao.value %t908, ptr %t914
  call void @xiao_runtime_value_release_strong(ptr %t914)
  store %xiao.value zeroinitializer, ptr %t916
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t916)
  %t915 = load %xiao.value, ptr %t916
  store %xiao.value %t915, ptr %t917
  call void @xiao_runtime_value_release_strong(ptr %t917)
  call i32 @xiao_runtime_error_checkpoint_finish()
  call void @llvm.stackrestore(ptr %t899)
  br label %dynamic.try.unmatched.finally.success371
dynamic.try.unmatched.finally.fail370:
  call i32 @xiao_runtime_error_checkpoint_finish()
  call void @llvm.stackrestore(ptr %t899)
  %t918 = call i32 @xiao_runtime_error_class()
  %t919 = icmp eq i32 %t918, 2
  br i1 %t919, label %xiao.error.terminal, label %dynamic.error-cleanup.recoverable376
dynamic.error-cleanup.recoverable376:
  br label %xiao.error.terminal
dynamic.try.unmatched.finally.success371:
  br label %xiao.error.terminal
dynamic.try.catch0369:
  call void @xiao_runtime_error_checkpoint_begin()
  %t920 = call ptr @llvm.stacksave()
  %t921 = getelementptr inbounds [6 x i8], ptr @.xiao.str11, i64 0, i64 0
  %t922 = insertvalue %xiao.bytes zeroinitializer, ptr %t921, 0
  %t923 = insertvalue %xiao.bytes %t922, i64 5, 1
  store %xiao.bytes %t923, ptr %t924
  store ptr null, ptr %t925
  %t926 = call i32 @xiao_runtime_string_new(ptr %t924, ptr %t925)
  %t927 = icmp eq i32 %t926, 0
  br i1 %t927, label %abi.ok380, label %abi.status.fail381
abi.status.fail381:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.try.catch0.finally.fail378
abi.ok380:
  %t928 = load ptr, ptr %t925
  store %xiao.value zeroinitializer, ptr %t930
  call void @xiao_runtime_value_str_owned(ptr sret(%xiao.value) %t930, ptr %t928)
  %t929 = load %xiao.value, ptr %t930
  %t932 = getelementptr inbounds %xiao.value, ptr %t931, i64 0
  store %xiao.value %t929, ptr %t932
  %t933 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t931, i64 1, i64 127)
  %t934 = icmp eq i32 %t933, 0
  br i1 %t934, label %abi.ok382, label %abi.status.fail383
abi.status.fail383:
  call i32 @xiao_runtime_error_attach_span(i64 2135, i64 2149)
  br label %dynamic.try.catch0.finally.fail378
abi.ok382:
  store %xiao.value %t929, ptr %t935
  call void @xiao_runtime_value_release_strong(ptr %t935)
  store %xiao.value zeroinitializer, ptr %t937
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t937)
  %t936 = load %xiao.value, ptr %t937
  store %xiao.value %t936, ptr %t938
  call void @xiao_runtime_value_release_strong(ptr %t938)
  call i32 @xiao_runtime_error_checkpoint_finish()
  call void @llvm.stackrestore(ptr %t920)
  br label %dynamic.try.catch0.finally.success379
dynamic.try.catch0.finally.fail378:
  call i32 @xiao_runtime_error_checkpoint_finish()
  call void @llvm.stackrestore(ptr %t920)
  %t939 = call i32 @xiao_runtime_error_class()
  %t940 = icmp eq i32 %t939, 2
  br i1 %t940, label %xiao.error.terminal, label %dynamic.error-cleanup.recoverable384
dynamic.error-cleanup.recoverable384:
  br label %xiao.error.terminal
dynamic.try.catch0.finally.success379:
  br label %dynamic.try.catch0.body377
dynamic.try.catch0.body377:
  store %xiao.value zeroinitializer, ptr %t941
  %t942 = call i32 @xiao_runtime_error_take(ptr %t941)
  %t943 = icmp eq i32 %t942, 0
  br i1 %t943, label %abi.ok385, label %abi.status.fail386
abi.status.fail386:
  call i32 @xiao_runtime_error_attach_span(i64 2059, i64 2123)
  br label %xiao.error.terminal
abi.ok385:
  %t944 = load %xiao.value, ptr %t941
  call void @xiao_runtime_value_release_strong(ptr %slot18)
  store %xiao.value %t944, ptr %slot18
  store %xiao.value zeroinitializer, ptr %t945
  %t946 = call i32 @xiao_runtime_value_copy(ptr %slot18, ptr %t945)
  %t947 = icmp eq i32 %t946, 0
  br i1 %t947, label %abi.ok388, label %abi.status.fail389
abi.status.fail389:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.try.catch0.fail387
abi.ok388:
  %t948 = load %xiao.value, ptr %t945
  store %xiao.value %t948, ptr %t949
  %t950 = getelementptr inbounds [12 x i8], ptr @.xiao.bytes12, i64 0, i64 0
  %t951 = insertvalue %xiao.bytes zeroinitializer, ptr %t950, 0
  %t952 = insertvalue %xiao.bytes %t951, i64 11, 1
  store %xiao.bytes %t952, ptr %t953
  %t954 = call i32 @xiao_runtime_dynamic_check(ptr %t953, ptr %t949)
  %t955 = icmp eq i32 %t954, 0
  br i1 %t955, label %error.code.operation.ok390, label %error.code.operation.fail391
error.code.operation.fail391:
  store %xiao.value %t948, ptr %t956
  call void @xiao_runtime_value_release_strong(ptr %t956)
  call i32 @xiao_runtime_error_attach_span(i64 2102, i64 2121)
  br label %dynamic.try.catch0.fail387
error.code.operation.ok390:
  store %xiao.bytes zeroinitializer, ptr %t957
  %t958 = call i32 @xiao_runtime_error_code(ptr %t949, ptr %t957)
  %t959 = icmp eq i32 %t958, 0
  br i1 %t959, label %error.code.view.ok392, label %error.code.view.fail393
error.code.view.fail393:
  store %xiao.value %t948, ptr %t960
  call void @xiao_runtime_value_release_strong(ptr %t960)
  %t961 = icmp eq i32 %t958, 3
  br i1 %t961, label %error.code.invalid.handle394, label %error.code.invalid.abi395
error.code.invalid.abi395:
  call void @xiao_runtime_fatal_abi()
  unreachable
error.code.invalid.handle394:
  %t962 = getelementptr inbounds [16 x i8], ptr @.xiao.str13, i64 0, i64 0
  %t963 = insertvalue %xiao.bytes zeroinitializer, ptr %t962, 0
  %t964 = insertvalue %xiao.bytes %t963, i64 15, 1
  store %xiao.bytes %t964, ptr %t965
  store ptr null, ptr %t966
  %t967 = call i32 @xiao_runtime_string_new(ptr %t965, ptr %t966)
  %t968 = icmp eq i32 %t967, 0
  br i1 %t968, label %abi.ok396, label %abi.status.fail397
abi.status.fail397:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.try.catch0.fail387
abi.ok396:
  %t969 = load ptr, ptr %t966
  store %xiao.value zeroinitializer, ptr %t971
  call void @xiao_runtime_value_str_owned(ptr sret(%xiao.value) %t971, ptr %t969)
  %t970 = load %xiao.value, ptr %t971
  store %xiao.value %t970, ptr %t972
  %t973 = getelementptr inbounds [12 x i8], ptr @.xiao.bytes14, i64 0, i64 0
  %t974 = insertvalue %xiao.bytes zeroinitializer, ptr %t973, 0
  %t975 = insertvalue %xiao.bytes %t974, i64 11, 1
  store %xiao.bytes %t975, ptr %t976
  %t978 = insertvalue %xiao.error.location zeroinitializer, i64 2102, 0
  %t979 = insertvalue %xiao.error.location %t978, i64 2121, 1
  %t980 = insertvalue %xiao.error.location %t979, i8 1, 2
  store %xiao.error.location %t980, ptr %t977
  store %xiao.value zeroinitializer, ptr %t982
  call void @xiao_runtime_error_new_values(ptr sret(%xiao.value) %t982, ptr %t976, ptr %t972, ptr null, ptr %t977)
  %t981 = load %xiao.value, ptr %t982
  store %xiao.value %t970, ptr %t983
  call void @xiao_runtime_value_release_strong(ptr %t983)
  %t984 = call i32 @xiao_runtime_error_class()
  %t985 = icmp eq i32 %t984, 0
  br i1 %t985, label %abi.error.ok398, label %abi.error.fail399
abi.error.fail399:
  call i32 @xiao_runtime_error_attach_span(i64 2102, i64 2121)
  br label %dynamic.try.catch0.fail387
abi.error.ok398:
  store %xiao.value %t981, ptr %t986
  %t988 = insertvalue %xiao.error.location zeroinitializer, i64 2102, 0
  %t989 = insertvalue %xiao.error.location %t988, i64 2121, 1
  %t990 = insertvalue %xiao.error.location %t989, i8 1, 2
  store %xiao.error.location %t990, ptr %t987
  call i32 @xiao_runtime_error_raise_value(ptr %t986, ptr %t987)
  store %xiao.value %t981, ptr %t991
  call void @xiao_runtime_value_release_strong(ptr %t991)
  br label %dynamic.try.catch0.fail387
error.code.view.ok392:
  %t992 = load %xiao.bytes, ptr %t957
  store %xiao.bytes %t992, ptr %t993
  store ptr null, ptr %t994
  %t995 = call i32 @xiao_runtime_string_new(ptr %t993, ptr %t994)
  %t996 = icmp eq i32 %t995, 0
  br i1 %t996, label %error.code.operation.ok400, label %error.code.operation.fail401
error.code.operation.fail401:
  store %xiao.value %t948, ptr %t997
  call void @xiao_runtime_value_release_strong(ptr %t997)
  call i32 @xiao_runtime_error_attach_span(i64 2102, i64 2121)
  br label %dynamic.try.catch0.fail387
error.code.operation.ok400:
  %t998 = load ptr, ptr %t994
  store %xiao.value zeroinitializer, ptr %t1000
  call void @xiao_runtime_value_str_owned(ptr sret(%xiao.value) %t1000, ptr %t998)
  %t999 = load %xiao.value, ptr %t1000
  store %xiao.value %t948, ptr %t1001
  call void @xiao_runtime_value_release_strong(ptr %t1001)
  %t1003 = getelementptr inbounds %xiao.value, ptr %t1002, i64 0
  store %xiao.value %t999, ptr %t1003
  %t1004 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t1002, i64 1, i64 127)
  %t1005 = icmp eq i32 %t1004, 0
  br i1 %t1005, label %abi.ok402, label %abi.status.fail403
abi.status.fail403:
  call i32 @xiao_runtime_error_attach_span(i64 2096, i64 2122)
  br label %dynamic.try.catch0.fail387
abi.ok402:
  store %xiao.value %t999, ptr %t1006
  call void @xiao_runtime_value_release_strong(ptr %t1006)
  store %xiao.value zeroinitializer, ptr %t1008
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t1008)
  %t1007 = load %xiao.value, ptr %t1008
  store %xiao.value %t1007, ptr %t1009
  call void @xiao_runtime_value_release_strong(ptr %t1009)
  call void @xiao_runtime_value_release_strong(ptr %slot18)
  br label %dynamic.try.continue351
dynamic.try.catch0.fail387:
  %t1010 = call i32 @xiao_runtime_error_class()
  %t1011 = icmp eq i32 %t1010, 2
  br i1 %t1011, label %xiao.error.terminal, label %dynamic.error-cleanup.recoverable404
dynamic.error-cleanup.recoverable404:
  call void @xiao_runtime_value_release_strong(ptr %slot18)
  br label %xiao.error.terminal
dynamic.try.continue351:
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site0)
  %t1012 = call i32 @xiao_runtime_error_class()
  %t1013 = icmp eq i32 %t1012, 0
  br i1 %t1013, label %abi.ok405, label %abi.status.fail406
abi.status.fail406:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.error.terminal
abi.ok405:
  call void @xiao_runtime_release_trace_flush()
  ret void
xiao.error.terminal:
  %t1014 = call i32 @xiao_runtime_error_class()
  %t1015 = icmp eq i32 %t1014, 2
  br i1 %t1015, label %xiao.error.report407, label %xiao.error.unmatched408
xiao.error.unmatched408:
  call void @xiao_runtime_value_release_strong(ptr %xiao.statement.site0)
  br label %xiao.error.report407
xiao.error.report407:
  call void @xiao_runtime_error_report()
  call void @xiao_runtime_release_trace_flush()
  %t1016 = call i32 @xiao_runtime_error_exit_code()
  ret void
abi.fail:
  call void @xiao_runtime_fatal_abi()
  unreachable
}

define i32 @main() {
entry:
  call void @xiao_entry()
  %xiao_exit_code = call i32 @xiao_runtime_error_exit_code()
  ret i32 %xiao_exit_code
}

define void @xiao.fn.1ab66cb157484850(ptr %xiao.function.result, ptr %xiao.arg0) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot0 = alloca %xiao.value
  %t3 = alloca %xiao.value
  %t4 = alloca %xiao.value
  %t8 = alloca %xiao.value
  %t26 = alloca %xiao.bytes
  %t30 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot0
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot0)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = load %xiao.value, ptr %slot0
  store %xiao.value zeroinitializer, ptr %t3
  store %xiao.value zeroinitializer, ptr %t4
  %t5 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t4)
  %t6 = icmp eq i32 %t5, 0
  br i1 %t6, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %cmath.arguments.fail2
abi.ok4:
  %t7 = load %xiao.value, ptr %t4
  store %xiao.value %t7, ptr %t3
  br label %cmath.arguments.ready3
cmath.arguments.fail2:
  call void @xiao_runtime_value_release_strong(ptr %t3)
  br label %xiao.fn.error
cmath.arguments.ready3:
  store %xiao.value zeroinitializer, ptr %t8
  %t9 = call i32 @xiao_runtime_cmath_abs(ptr %t3, ptr %t8)
  call void @xiao_runtime_value_release_strong(ptr %t3)
  switch i32 %t9, label %cmath.abi.failure8 [ i32 0, label %cmath.ok6 i32 6, label %cmath.error7 ]
cmath.abi.failure8:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error7:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call i32 @xiao_runtime_error_attach_span(i64 420, i64 436)
  br label %xiao.fn.error
cmath.ok6:
  %t10 = load %xiao.value, ptr %t8
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t10, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 true, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t11 = call i32 @xiao_runtime_error_class()
  %t12 = icmp eq i32 %t11, 0
  br i1 %t12, label %abi.ok9, label %abi.status.fail10
abi.status.fail10:
  call i32 @xiao_runtime_error_attach_span(i64 379, i64 438)
  br label %xiao.fn.error
abi.ok9:
  %t13 = load ptr, ptr %xiao.return.source
  %t14 = icmp ne ptr %t13, null
  br i1 %t14, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t15 = load i1, ptr %xiao.return.temporary
  br i1 %t15, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t16 = call i32 @xiao_runtime_value_copy(ptr %t13, ptr %xiao.function.result)
  %t17 = icmp eq i32 %t16, 0
  br i1 %t17, label %abi.ok11, label %abi.status.fail12
abi.status.fail12:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok11:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t18 = load %xiao.value, ptr %t13
  store %xiao.value %t18, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t13
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t19 = call i32 @xiao_runtime_error_class()
  %t20 = icmp eq i32 %t19, 0
  br i1 %t20, label %abi.ok13, label %abi.status.fail14
abi.status.fail14:
  call i32 @xiao_runtime_error_attach_span(i64 379, i64 438)
  br label %xiao.fn.error
abi.ok13:
  ret void
xiao.fn.error:
  %t21 = call i32 @xiao_runtime_error_class()
  %t22 = icmp eq i32 %t21, 2
  br i1 %t22, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t23 = getelementptr inbounds [13 x i8], ptr @.xiao.bytes15, i64 0, i64 0
  %t24 = insertvalue %xiao.bytes zeroinitializer, ptr %t23, 0
  %t25 = insertvalue %xiao.bytes %t24, i64 12, 1
  store %xiao.bytes %t25, ptr %t26
  %t27 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes16, i64 0, i64 0
  %t28 = insertvalue %xiao.bytes zeroinitializer, ptr %t27, 0
  %t29 = insertvalue %xiao.bytes %t28, i64 3, 1
  store %xiao.bytes %t29, ptr %t30
  call void @xiao_runtime_error_push_native_frame(ptr %t26, ptr %t30, i64 379, i64 438)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.e86580f5191d83b7(ptr %xiao.function.result) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %t3 = alloca %xiao.bytes
  %t4 = alloca ptr
  %t9 = alloca %xiao.value
  %t10 = alloca %xiao.value, i64 1
  %t14 = alloca %xiao.value
  %t16 = alloca %xiao.value
  %t17 = alloca %xiao.value
  %t19 = alloca %xiao.value
  %t35 = alloca %xiao.bytes
  %t39 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  %t0 = getelementptr inbounds [5 x i8], ptr @.xiao.str17, i64 0, i64 0
  %t1 = insertvalue %xiao.bytes zeroinitializer, ptr %t0, 0
  %t2 = insertvalue %xiao.bytes %t1, i64 4, 1
  store %xiao.bytes %t2, ptr %t3
  store ptr null, ptr %t4
  %t5 = call i32 @xiao_runtime_string_new(ptr %t3, ptr %t4)
  %t6 = icmp eq i32 %t5, 0
  br i1 %t6, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t7 = load ptr, ptr %t4
  store %xiao.value zeroinitializer, ptr %t9
  call void @xiao_runtime_value_str_owned(ptr sret(%xiao.value) %t9, ptr %t7)
  %t8 = load %xiao.value, ptr %t9
  %t11 = getelementptr inbounds %xiao.value, ptr %t10, i64 0
  store %xiao.value %t8, ptr %t11
  %t12 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t10, i64 1, i64 127)
  %t13 = icmp eq i32 %t12, 0
  br i1 %t13, label %abi.ok2, label %abi.status.fail3
abi.status.fail3:
  call i32 @xiao_runtime_error_attach_span(i64 1762, i64 1775)
  br label %xiao.fn.error
abi.ok2:
  store %xiao.value %t8, ptr %t14
  call void @xiao_runtime_value_release_strong(ptr %t14)
  store %xiao.value zeroinitializer, ptr %t16
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t16)
  %t15 = load %xiao.value, ptr %t16
  store %xiao.value %t15, ptr %t17
  call void @xiao_runtime_value_release_strong(ptr %t17)
  store %xiao.value zeroinitializer, ptr %t19
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t19, double 2.00000000000000000e0)
  %t18 = load %xiao.value, ptr %t19
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t18, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 true, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  %t20 = call i32 @xiao_runtime_error_class()
  %t21 = icmp eq i32 %t20, 0
  br i1 %t21, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 1738, i64 1791)
  br label %xiao.fn.error
abi.ok4:
  %t22 = load ptr, ptr %xiao.return.source
  %t23 = icmp ne ptr %t22, null
  br i1 %t23, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t24 = load i1, ptr %xiao.return.temporary
  br i1 %t24, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t25 = call i32 @xiao_runtime_value_copy(ptr %t22, ptr %xiao.function.result)
  %t26 = icmp eq i32 %t25, 0
  br i1 %t26, label %abi.ok6, label %abi.status.fail7
abi.status.fail7:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok6:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t27 = load %xiao.value, ptr %t22
  store %xiao.value %t27, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t22
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t28 = call i32 @xiao_runtime_error_class()
  %t29 = icmp eq i32 %t28, 0
  br i1 %t29, label %abi.ok8, label %abi.status.fail9
abi.status.fail9:
  call i32 @xiao_runtime_error_attach_span(i64 1738, i64 1791)
  br label %xiao.fn.error
abi.ok8:
  ret void
xiao.fn.error:
  %t30 = call i32 @xiao_runtime_error_class()
  %t31 = icmp eq i32 %t30, 2
  br i1 %t31, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t32 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes18, i64 0, i64 0
  %t33 = insertvalue %xiao.bytes zeroinitializer, ptr %t32, 0
  %t34 = insertvalue %xiao.bytes %t33, i64 5, 1
  store %xiao.bytes %t34, ptr %t35
  %t36 = getelementptr inbounds [5 x i8], ptr @.xiao.bytes19, i64 0, i64 0
  %t37 = insertvalue %xiao.bytes zeroinitializer, ptr %t36, 0
  %t38 = insertvalue %xiao.bytes %t37, i64 4, 1
  store %xiao.bytes %t38, ptr %t39
  call void @xiao_runtime_error_push_native_frame(ptr %t35, ptr %t39, i64 1738, i64 1791)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.b34d23c8bbb4bb95(ptr %xiao.function.result, ptr %xiao.arg0) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot0 = alloca %xiao.value
  %t3 = alloca %xiao.value
  %t4 = alloca %xiao.value
  %t8 = alloca %xiao.value
  %t26 = alloca %xiao.bytes
  %t30 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot0
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot0)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = load %xiao.value, ptr %slot0
  store %xiao.value zeroinitializer, ptr %t3
  store %xiao.value zeroinitializer, ptr %t4
  %t5 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t4)
  %t6 = icmp eq i32 %t5, 0
  br i1 %t6, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %cmath.arguments.fail2
abi.ok4:
  %t7 = load %xiao.value, ptr %t4
  store %xiao.value %t7, ptr %t3
  br label %cmath.arguments.ready3
cmath.arguments.fail2:
  call void @xiao_runtime_value_release_strong(ptr %t3)
  br label %xiao.fn.error
cmath.arguments.ready3:
  store %xiao.value zeroinitializer, ptr %t8
  %t9 = call i32 @xiao_runtime_cmath_ceil(ptr %t3, ptr %t8)
  call void @xiao_runtime_value_release_strong(ptr %t3)
  switch i32 %t9, label %cmath.abi.failure8 [ i32 0, label %cmath.ok6 i32 6, label %cmath.error7 ]
cmath.abi.failure8:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error7:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call i32 @xiao_runtime_error_attach_span(i64 234, i64 251)
  br label %xiao.fn.error
cmath.ok6:
  %t10 = load %xiao.value, ptr %t8
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t10, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 true, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t11 = call i32 @xiao_runtime_error_class()
  %t12 = icmp eq i32 %t11, 0
  br i1 %t12, label %abi.ok9, label %abi.status.fail10
abi.status.fail10:
  call i32 @xiao_runtime_error_attach_span(i64 192, i64 253)
  br label %xiao.fn.error
abi.ok9:
  %t13 = load ptr, ptr %xiao.return.source
  %t14 = icmp ne ptr %t13, null
  br i1 %t14, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t15 = load i1, ptr %xiao.return.temporary
  br i1 %t15, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t16 = call i32 @xiao_runtime_value_copy(ptr %t13, ptr %xiao.function.result)
  %t17 = icmp eq i32 %t16, 0
  br i1 %t17, label %abi.ok11, label %abi.status.fail12
abi.status.fail12:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok11:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t18 = load %xiao.value, ptr %t13
  store %xiao.value %t18, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t13
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t19 = call i32 @xiao_runtime_error_class()
  %t20 = icmp eq i32 %t19, 0
  br i1 %t20, label %abi.ok13, label %abi.status.fail14
abi.status.fail14:
  call i32 @xiao_runtime_error_attach_span(i64 192, i64 253)
  br label %xiao.fn.error
abi.ok13:
  ret void
xiao.fn.error:
  %t21 = call i32 @xiao_runtime_error_class()
  %t22 = icmp eq i32 %t21, 2
  br i1 %t22, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t23 = getelementptr inbounds [13 x i8], ptr @.xiao.bytes20, i64 0, i64 0
  %t24 = insertvalue %xiao.bytes zeroinitializer, ptr %t23, 0
  %t25 = insertvalue %xiao.bytes %t24, i64 12, 1
  store %xiao.bytes %t25, ptr %t26
  %t27 = getelementptr inbounds [5 x i8], ptr @.xiao.bytes21, i64 0, i64 0
  %t28 = insertvalue %xiao.bytes zeroinitializer, ptr %t27, 0
  %t29 = insertvalue %xiao.bytes %t28, i64 4, 1
  store %xiao.bytes %t29, ptr %t30
  call void @xiao_runtime_error_push_native_frame(ptr %t26, ptr %t30, i64 192, i64 253)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.a6a14f7ec3217a8a(ptr %xiao.function.result, ptr %xiao.arg0, ptr %xiao.arg1, ptr %xiao.arg2) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.site1 = alloca %xiao.value
  %xiao.return.site2 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot1 = alloca %xiao.value
  %slot2 = alloca %xiao.value
  %slot0 = alloca %xiao.value
  %t9 = alloca %xiao.value
  %t13 = alloca %xiao.value
  %t20 = alloca %xiao.bytes
  %t21 = alloca %xiao.value
  %t22 = alloca %xiao.value
  %t23 = alloca %xiao.value
  %t27 = alloca %xiao.value
  %t28 = alloca %xiao.value
  %t29 = alloca %xiao.value
  %t30 = alloca %xiao.value
  %t35 = alloca %xiao.value
  %t39 = alloca %xiao.bytes
  %t40 = alloca ptr
  %t45 = alloca %xiao.value
  %t46 = alloca %xiao.value
  %t50 = alloca %xiao.bytes
  %t51 = alloca %xiao.error.location
  %t56 = alloca %xiao.value
  %t57 = alloca %xiao.value
  %t60 = alloca %xiao.value
  %t61 = alloca %xiao.error.location
  %t67 = alloca %xiao.value
  %t71 = alloca %xiao.value
  %t78 = alloca %xiao.bytes
  %t79 = alloca %xiao.value
  %t80 = alloca %xiao.value
  %t81 = alloca %xiao.value
  %t85 = alloca %xiao.value
  %t86 = alloca %xiao.value
  %t87 = alloca %xiao.value
  %t88 = alloca %xiao.value
  %t93 = alloca %xiao.value
  %t94 = alloca %xiao.value
  %t98 = alloca %xiao.value
  %t102 = alloca %xiao.value
  %t109 = alloca %xiao.bytes
  %t110 = alloca %xiao.value
  %t111 = alloca %xiao.value
  %t112 = alloca %xiao.value
  %t116 = alloca %xiao.value
  %t117 = alloca %xiao.value
  %t118 = alloca %xiao.value
  %t119 = alloca %xiao.value
  %t124 = alloca %xiao.value
  %t125 = alloca %xiao.value
  %t129 = alloca %xiao.value
  %t148 = alloca %xiao.bytes
  %t152 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store %xiao.value zeroinitializer, ptr %xiao.return.site1
  store %xiao.value zeroinitializer, ptr %xiao.return.site2
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot1
  store %xiao.value zeroinitializer, ptr %slot2
  store %xiao.value zeroinitializer, ptr %slot0
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot0)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg1, ptr %slot1)
  %t3 = icmp eq i32 %t2, 0
  br i1 %t3, label %abi.ok2, label %abi.status.fail3
abi.status.fail3:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok2:
  %t4 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg2, ptr %slot2)
  %t5 = icmp eq i32 %t4, 0
  br i1 %t5, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok4:
  %t6 = load %xiao.value, ptr %slot0
  %t7 = load %xiao.value, ptr %slot1
  %t8 = load %xiao.value, ptr %slot2
  store %xiao.value zeroinitializer, ptr %t9
  %t10 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t9)
  %t11 = icmp eq i32 %t10, 0
  br i1 %t11, label %abi.ok6, label %abi.status.fail7
abi.status.fail7:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok6:
  %t12 = load %xiao.value, ptr %t9
  store %xiao.value zeroinitializer, ptr %t13
  %t14 = call i32 @xiao_runtime_value_copy(ptr %slot2, ptr %t13)
  %t15 = icmp eq i32 %t14, 0
  br i1 %t15, label %abi.ok9, label %abi.status.fail10
abi.status.fail10:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.binary.left.fail8
abi.ok9:
  %t16 = load %xiao.value, ptr %t13
  %t17 = getelementptr inbounds [3 x i8], ptr @.xiao.bytes22, i64 0, i64 0
  %t18 = insertvalue %xiao.bytes zeroinitializer, ptr %t17, 0
  %t19 = insertvalue %xiao.bytes %t18, i64 2, 1
  store %xiao.bytes %t19, ptr %t20
  store %xiao.value %t12, ptr %t21
  store %xiao.value %t16, ptr %t22
  store %xiao.value zeroinitializer, ptr %t23
  %t24 = call i32 @xiao_runtime_value_binary(ptr %t20, ptr %t21, ptr %t22, ptr %t23)
  %t25 = icmp eq i32 %t24, 0
  br i1 %t25, label %abi.ok13, label %abi.status.fail14
abi.status.fail14:
  call i32 @xiao_runtime_error_attach_span(i64 404, i64 417)
  br label %dynamic.binary.fail11
abi.ok13:
  %t26 = load %xiao.value, ptr %t23
  store %xiao.value %t12, ptr %t27
  call void @xiao_runtime_value_release_strong(ptr %t27)
  store %xiao.value %t16, ptr %t28
  call void @xiao_runtime_value_release_strong(ptr %t28)
  br label %dynamic.binary.continue12
dynamic.binary.fail11:
  store %xiao.value %t16, ptr %t29
  call void @xiao_runtime_value_release_strong(ptr %t29)
  br label %dynamic.binary.left.fail8
dynamic.binary.left.fail8:
  store %xiao.value %t12, ptr %t30
  call void @xiao_runtime_value_release_strong(ptr %t30)
  br label %xiao.fn.error
dynamic.binary.continue12:
  %t31 = extractvalue %xiao.value %t26, 0
  %t32 = icmp eq i32 %t31, 1
  br i1 %t32, label %dynamic.bool.ok15, label %xiao.fn.error
dynamic.bool.ok15:
  %t33 = extractvalue %xiao.value %t26, 1
  %t34 = trunc i64 %t33 to i1
  store %xiao.value %t26, ptr %t35
  call void @xiao_runtime_value_release_strong(ptr %t35)
  br i1 %t34, label %dynamic.if.then16, label %dynamic.if.next17
dynamic.if.then16:
  %t36 = getelementptr inbounds [20 x i8], ptr @.xiao.str23, i64 0, i64 0
  %t37 = insertvalue %xiao.bytes zeroinitializer, ptr %t36, 0
  %t38 = insertvalue %xiao.bytes %t37, i64 19, 1
  store %xiao.bytes %t38, ptr %t39
  store ptr null, ptr %t40
  %t41 = call i32 @xiao_runtime_string_new(ptr %t39, ptr %t40)
  %t42 = icmp eq i32 %t41, 0
  br i1 %t42, label %abi.ok19, label %abi.status.fail20
abi.status.fail20:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok19:
  %t43 = load ptr, ptr %t40
  store %xiao.value zeroinitializer, ptr %t45
  call void @xiao_runtime_value_str_owned(ptr sret(%xiao.value) %t45, ptr %t43)
  %t44 = load %xiao.value, ptr %t45
  store %xiao.value %t44, ptr %t46
  %t47 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes24, i64 0, i64 0
  %t48 = insertvalue %xiao.bytes zeroinitializer, ptr %t47, 0
  %t49 = insertvalue %xiao.bytes %t48, i64 5, 1
  store %xiao.bytes %t49, ptr %t50
  %t52 = insertvalue %xiao.error.location zeroinitializer, i64 432, 0
  %t53 = insertvalue %xiao.error.location %t52, i64 467, 1
  %t54 = insertvalue %xiao.error.location %t53, i8 1, 2
  store %xiao.error.location %t54, ptr %t51
  store %xiao.value zeroinitializer, ptr %t56
  call void @xiao_runtime_error_new_values(ptr sret(%xiao.value) %t56, ptr %t50, ptr %t46, ptr null, ptr %t51)
  %t55 = load %xiao.value, ptr %t56
  store %xiao.value %t44, ptr %t57
  call void @xiao_runtime_value_release_strong(ptr %t57)
  %t58 = call i32 @xiao_runtime_error_class()
  %t59 = icmp eq i32 %t58, 0
  br i1 %t59, label %abi.error.ok21, label %abi.error.fail22
abi.error.fail22:
  call i32 @xiao_runtime_error_attach_span(i64 432, i64 467)
  br label %xiao.fn.error
abi.error.ok21:
  store %xiao.value %t55, ptr %t60
  %t62 = insertvalue %xiao.error.location zeroinitializer, i64 426, 0
  %t63 = insertvalue %xiao.error.location %t62, i64 467, 1
  %t64 = insertvalue %xiao.error.location %t63, i8 1, 2
  store %xiao.error.location %t64, ptr %t61
  %t65 = call i32 @xiao_runtime_error_raise_value(ptr %t60, ptr %t61)
  call void @xiao_runtime_value_release_strong(ptr %t60)
  %t66 = icmp eq i32 %t65, 0
  br i1 %t66, label %abi.ok23, label %abi.status.fail24
abi.status.fail24:
  call i32 @xiao_runtime_error_attach_span(i64 426, i64 467)
  br label %xiao.fn.error
abi.ok23:
  br label %xiao.fn.error
dynamic.if.next17:
  br label %dynamic.if.merge18
dynamic.if.merge18:
  store %xiao.value zeroinitializer, ptr %t67
  %t68 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t67)
  %t69 = icmp eq i32 %t68, 0
  br i1 %t69, label %abi.ok25, label %abi.status.fail26
abi.status.fail26:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok25:
  %t70 = load %xiao.value, ptr %t67
  store %xiao.value zeroinitializer, ptr %t71
  %t72 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t71)
  %t73 = icmp eq i32 %t72, 0
  br i1 %t73, label %abi.ok28, label %abi.status.fail29
abi.status.fail29:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.binary.left.fail27
abi.ok28:
  %t74 = load %xiao.value, ptr %t71
  %t75 = getelementptr inbounds [3 x i8], ptr @.xiao.bytes25, i64 0, i64 0
  %t76 = insertvalue %xiao.bytes zeroinitializer, ptr %t75, 0
  %t77 = insertvalue %xiao.bytes %t76, i64 2, 1
  store %xiao.bytes %t77, ptr %t78
  store %xiao.value %t70, ptr %t79
  store %xiao.value %t74, ptr %t80
  store %xiao.value zeroinitializer, ptr %t81
  %t82 = call i32 @xiao_runtime_value_binary(ptr %t78, ptr %t79, ptr %t80, ptr %t81)
  %t83 = icmp eq i32 %t82, 0
  br i1 %t83, label %abi.ok32, label %abi.status.fail33
abi.status.fail33:
  call i32 @xiao_runtime_error_attach_span(i64 475, i64 488)
  br label %dynamic.binary.fail30
abi.ok32:
  %t84 = load %xiao.value, ptr %t81
  store %xiao.value %t70, ptr %t85
  call void @xiao_runtime_value_release_strong(ptr %t85)
  store %xiao.value %t74, ptr %t86
  call void @xiao_runtime_value_release_strong(ptr %t86)
  br label %dynamic.binary.continue31
dynamic.binary.fail30:
  store %xiao.value %t74, ptr %t87
  call void @xiao_runtime_value_release_strong(ptr %t87)
  br label %dynamic.binary.left.fail27
dynamic.binary.left.fail27:
  store %xiao.value %t70, ptr %t88
  call void @xiao_runtime_value_release_strong(ptr %t88)
  br label %xiao.fn.error
dynamic.binary.continue31:
  %t89 = extractvalue %xiao.value %t84, 0
  %t90 = icmp eq i32 %t89, 1
  br i1 %t90, label %dynamic.bool.ok34, label %xiao.fn.error
dynamic.bool.ok34:
  %t91 = extractvalue %xiao.value %t84, 1
  %t92 = trunc i64 %t91 to i1
  store %xiao.value %t84, ptr %t93
  call void @xiao_runtime_value_release_strong(ptr %t93)
  br i1 %t92, label %dynamic.if.then35, label %dynamic.if.next36
dynamic.if.then35:
  store %xiao.value zeroinitializer, ptr %t94
  %t95 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t94)
  %t96 = icmp eq i32 %t95, 0
  br i1 %t96, label %abi.ok38, label %abi.status.fail39
abi.status.fail39:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok38:
  %t97 = load %xiao.value, ptr %t94
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t97, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  br label %xiao.fn.return
dynamic.if.next36:
  br label %dynamic.if.merge37
dynamic.if.merge37:
  store %xiao.value zeroinitializer, ptr %t98
  %t99 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t98)
  %t100 = icmp eq i32 %t99, 0
  br i1 %t100, label %abi.ok40, label %abi.status.fail41
abi.status.fail41:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok40:
  %t101 = load %xiao.value, ptr %t98
  store %xiao.value zeroinitializer, ptr %t102
  %t103 = call i32 @xiao_runtime_value_copy(ptr %slot2, ptr %t102)
  %t104 = icmp eq i32 %t103, 0
  br i1 %t104, label %abi.ok43, label %abi.status.fail44
abi.status.fail44:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.binary.left.fail42
abi.ok43:
  %t105 = load %xiao.value, ptr %t102
  %t106 = getelementptr inbounds [3 x i8], ptr @.xiao.bytes26, i64 0, i64 0
  %t107 = insertvalue %xiao.bytes zeroinitializer, ptr %t106, 0
  %t108 = insertvalue %xiao.bytes %t107, i64 2, 1
  store %xiao.bytes %t108, ptr %t109
  store %xiao.value %t101, ptr %t110
  store %xiao.value %t105, ptr %t111
  store %xiao.value zeroinitializer, ptr %t112
  %t113 = call i32 @xiao_runtime_value_binary(ptr %t109, ptr %t110, ptr %t111, ptr %t112)
  %t114 = icmp eq i32 %t113, 0
  br i1 %t114, label %abi.ok47, label %abi.status.fail48
abi.status.fail48:
  call i32 @xiao_runtime_error_attach_span(i64 517, i64 530)
  br label %dynamic.binary.fail45
abi.ok47:
  %t115 = load %xiao.value, ptr %t112
  store %xiao.value %t101, ptr %t116
  call void @xiao_runtime_value_release_strong(ptr %t116)
  store %xiao.value %t105, ptr %t117
  call void @xiao_runtime_value_release_strong(ptr %t117)
  br label %dynamic.binary.continue46
dynamic.binary.fail45:
  store %xiao.value %t105, ptr %t118
  call void @xiao_runtime_value_release_strong(ptr %t118)
  br label %dynamic.binary.left.fail42
dynamic.binary.left.fail42:
  store %xiao.value %t101, ptr %t119
  call void @xiao_runtime_value_release_strong(ptr %t119)
  br label %xiao.fn.error
dynamic.binary.continue46:
  %t120 = extractvalue %xiao.value %t115, 0
  %t121 = icmp eq i32 %t120, 1
  br i1 %t121, label %dynamic.bool.ok49, label %xiao.fn.error
dynamic.bool.ok49:
  %t122 = extractvalue %xiao.value %t115, 1
  %t123 = trunc i64 %t122 to i1
  store %xiao.value %t115, ptr %t124
  call void @xiao_runtime_value_release_strong(ptr %t124)
  br i1 %t123, label %dynamic.if.then50, label %dynamic.if.next51
dynamic.if.then50:
  store %xiao.value zeroinitializer, ptr %t125
  %t126 = call i32 @xiao_runtime_value_copy(ptr %slot2, ptr %t125)
  %t127 = icmp eq i32 %t126, 0
  br i1 %t127, label %abi.ok53, label %abi.status.fail54
abi.status.fail54:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok53:
  %t128 = load %xiao.value, ptr %t125
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site1)
  store %xiao.value %t128, ptr %xiao.return.site1
  store ptr %xiao.return.site1, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  br label %xiao.fn.return
dynamic.if.next51:
  br label %dynamic.if.merge52
dynamic.if.merge52:
  store %xiao.value zeroinitializer, ptr %t129
  %t130 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t129)
  %t131 = icmp eq i32 %t130, 0
  br i1 %t131, label %abi.ok55, label %abi.status.fail56
abi.status.fail56:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok55:
  %t132 = load %xiao.value, ptr %t129
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site2)
  store %xiao.value %t132, ptr %xiao.return.site2
  store ptr %xiao.return.site2, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot2)
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t133 = call i32 @xiao_runtime_error_class()
  %t134 = icmp eq i32 %t133, 0
  br i1 %t134, label %abi.ok57, label %abi.status.fail58
abi.status.fail58:
  call i32 @xiao_runtime_error_attach_span(i64 347, i64 570)
  br label %xiao.fn.error
abi.ok57:
  %t135 = load ptr, ptr %xiao.return.source
  %t136 = icmp ne ptr %t135, null
  br i1 %t136, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t137 = load i1, ptr %xiao.return.temporary
  br i1 %t137, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t138 = call i32 @xiao_runtime_value_copy(ptr %t135, ptr %xiao.function.result)
  %t139 = icmp eq i32 %t138, 0
  br i1 %t139, label %abi.ok59, label %abi.status.fail60
abi.status.fail60:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok59:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t140 = load %xiao.value, ptr %t135
  store %xiao.value %t140, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t135
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site1)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site2)
  %t141 = call i32 @xiao_runtime_error_class()
  %t142 = icmp eq i32 %t141, 0
  br i1 %t142, label %abi.ok61, label %abi.status.fail62
abi.status.fail62:
  call i32 @xiao_runtime_error_attach_span(i64 347, i64 570)
  br label %xiao.fn.error
abi.ok61:
  ret void
xiao.fn.error:
  %t143 = call i32 @xiao_runtime_error_class()
  %t144 = icmp eq i32 %t143, 2
  br i1 %t144, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot2)
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site1)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site2)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t145 = getelementptr inbounds [12 x i8], ptr @.xiao.bytes27, i64 0, i64 0
  %t146 = insertvalue %xiao.bytes zeroinitializer, ptr %t145, 0
  %t147 = insertvalue %xiao.bytes %t146, i64 11, 1
  store %xiao.bytes %t147, ptr %t148
  %t149 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes28, i64 0, i64 0
  %t150 = insertvalue %xiao.bytes zeroinitializer, ptr %t149, 0
  %t151 = insertvalue %xiao.bytes %t150, i64 5, 1
  store %xiao.bytes %t151, ptr %t152
  call void @xiao_runtime_error_push_native_frame(ptr %t148, ptr %t152, i64 347, i64 570)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.e4403f97ead6ef36(ptr %xiao.function.result, ptr %xiao.arg0) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot0 = alloca %xiao.value
  %t3 = alloca %xiao.value
  %t4 = alloca %xiao.value
  %t8 = alloca %xiao.value
  %t26 = alloca %xiao.bytes
  %t30 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot0
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot0)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = load %xiao.value, ptr %slot0
  store %xiao.value zeroinitializer, ptr %t3
  store %xiao.value zeroinitializer, ptr %t4
  %t5 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t4)
  %t6 = icmp eq i32 %t5, 0
  br i1 %t6, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %cmath.arguments.fail2
abi.ok4:
  %t7 = load %xiao.value, ptr %t4
  store %xiao.value %t7, ptr %t3
  br label %cmath.arguments.ready3
cmath.arguments.fail2:
  call void @xiao_runtime_value_release_strong(ptr %t3)
  br label %xiao.fn.error
cmath.arguments.ready3:
  store %xiao.value zeroinitializer, ptr %t8
  %t9 = call i32 @xiao_runtime_cmath_cos(ptr %t3, ptr %t8)
  call void @xiao_runtime_value_release_strong(ptr %t3)
  switch i32 %t9, label %cmath.abi.failure8 [ i32 0, label %cmath.ok6 i32 6, label %cmath.error7 ]
cmath.abi.failure8:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error7:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call i32 @xiao_runtime_error_attach_span(i64 775, i64 791)
  br label %xiao.fn.error
cmath.ok6:
  %t10 = load %xiao.value, ptr %t8
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t10, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 true, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t11 = call i32 @xiao_runtime_error_class()
  %t12 = icmp eq i32 %t11, 0
  br i1 %t12, label %abi.ok9, label %abi.status.fail10
abi.status.fail10:
  call i32 @xiao_runtime_error_attach_span(i64 734, i64 793)
  br label %xiao.fn.error
abi.ok9:
  %t13 = load ptr, ptr %xiao.return.source
  %t14 = icmp ne ptr %t13, null
  br i1 %t14, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t15 = load i1, ptr %xiao.return.temporary
  br i1 %t15, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t16 = call i32 @xiao_runtime_value_copy(ptr %t13, ptr %xiao.function.result)
  %t17 = icmp eq i32 %t16, 0
  br i1 %t17, label %abi.ok11, label %abi.status.fail12
abi.status.fail12:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok11:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t18 = load %xiao.value, ptr %t13
  store %xiao.value %t18, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t13
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t19 = call i32 @xiao_runtime_error_class()
  %t20 = icmp eq i32 %t19, 0
  br i1 %t20, label %abi.ok13, label %abi.status.fail14
abi.status.fail14:
  call i32 @xiao_runtime_error_attach_span(i64 734, i64 793)
  br label %xiao.fn.error
abi.ok13:
  ret void
xiao.fn.error:
  %t21 = call i32 @xiao_runtime_error_class()
  %t22 = icmp eq i32 %t21, 2
  br i1 %t22, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t23 = getelementptr inbounds [13 x i8], ptr @.xiao.bytes29, i64 0, i64 0
  %t24 = insertvalue %xiao.bytes zeroinitializer, ptr %t23, 0
  %t25 = insertvalue %xiao.bytes %t24, i64 12, 1
  store %xiao.bytes %t25, ptr %t26
  %t27 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes30, i64 0, i64 0
  %t28 = insertvalue %xiao.bytes zeroinitializer, ptr %t27, 0
  %t29 = insertvalue %xiao.bytes %t28, i64 3, 1
  store %xiao.bytes %t29, ptr %t30
  call void @xiao_runtime_error_push_native_frame(ptr %t26, ptr %t30, i64 734, i64 793)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.d0cbe39c8d270929(ptr %xiao.function.result, ptr %xiao.arg0) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot0 = alloca %xiao.value
  %t3 = alloca %xiao.value
  %t4 = alloca %xiao.value
  %t8 = alloca %xiao.value
  %t26 = alloca %xiao.bytes
  %t30 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot0
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot0)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = load %xiao.value, ptr %slot0
  store %xiao.value zeroinitializer, ptr %t3
  store %xiao.value zeroinitializer, ptr %t4
  %t5 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t4)
  %t6 = icmp eq i32 %t5, 0
  br i1 %t6, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %cmath.arguments.fail2
abi.ok4:
  %t7 = load %xiao.value, ptr %t4
  store %xiao.value %t7, ptr %t3
  br label %cmath.arguments.ready3
cmath.arguments.fail2:
  call void @xiao_runtime_value_release_strong(ptr %t3)
  br label %xiao.fn.error
cmath.arguments.ready3:
  store %xiao.value zeroinitializer, ptr %t8
  %t9 = call i32 @xiao_runtime_cmath_exp(ptr %t3, ptr %t8)
  call void @xiao_runtime_value_release_strong(ptr %t3)
  switch i32 %t9, label %cmath.abi.failure8 [ i32 0, label %cmath.ok6 i32 6, label %cmath.error7 ]
cmath.abi.failure8:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error7:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call i32 @xiao_runtime_error_attach_span(i64 893, i64 909)
  br label %xiao.fn.error
cmath.ok6:
  %t10 = load %xiao.value, ptr %t8
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t10, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 true, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t11 = call i32 @xiao_runtime_error_class()
  %t12 = icmp eq i32 %t11, 0
  br i1 %t12, label %abi.ok9, label %abi.status.fail10
abi.status.fail10:
  call i32 @xiao_runtime_error_attach_span(i64 852, i64 911)
  br label %xiao.fn.error
abi.ok9:
  %t13 = load ptr, ptr %xiao.return.source
  %t14 = icmp ne ptr %t13, null
  br i1 %t14, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t15 = load i1, ptr %xiao.return.temporary
  br i1 %t15, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t16 = call i32 @xiao_runtime_value_copy(ptr %t13, ptr %xiao.function.result)
  %t17 = icmp eq i32 %t16, 0
  br i1 %t17, label %abi.ok11, label %abi.status.fail12
abi.status.fail12:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok11:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t18 = load %xiao.value, ptr %t13
  store %xiao.value %t18, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t13
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t19 = call i32 @xiao_runtime_error_class()
  %t20 = icmp eq i32 %t19, 0
  br i1 %t20, label %abi.ok13, label %abi.status.fail14
abi.status.fail14:
  call i32 @xiao_runtime_error_attach_span(i64 852, i64 911)
  br label %xiao.fn.error
abi.ok13:
  ret void
xiao.fn.error:
  %t21 = call i32 @xiao_runtime_error_class()
  %t22 = icmp eq i32 %t21, 2
  br i1 %t22, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t23 = getelementptr inbounds [13 x i8], ptr @.xiao.bytes31, i64 0, i64 0
  %t24 = insertvalue %xiao.bytes zeroinitializer, ptr %t23, 0
  %t25 = insertvalue %xiao.bytes %t24, i64 12, 1
  store %xiao.bytes %t25, ptr %t26
  %t27 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes32, i64 0, i64 0
  %t28 = insertvalue %xiao.bytes zeroinitializer, ptr %t27, 0
  %t29 = insertvalue %xiao.bytes %t28, i64 3, 1
  store %xiao.bytes %t29, ptr %t30
  call void @xiao_runtime_error_push_native_frame(ptr %t26, ptr %t30, i64 852, i64 911)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.d41bba4dda4a06ca(ptr %xiao.function.result) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %t3 = alloca %xiao.bytes
  %t4 = alloca ptr
  %t9 = alloca %xiao.value
  %t10 = alloca %xiao.value, i64 1
  %t14 = alloca %xiao.value
  %t16 = alloca %xiao.value
  %t17 = alloca %xiao.value
  %t19 = alloca %xiao.value
  %t35 = alloca %xiao.bytes
  %t39 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  %t0 = getelementptr inbounds [9 x i8], ptr @.xiao.str33, i64 0, i64 0
  %t1 = insertvalue %xiao.bytes zeroinitializer, ptr %t0, 0
  %t2 = insertvalue %xiao.bytes %t1, i64 8, 1
  store %xiao.bytes %t2, ptr %t3
  store ptr null, ptr %t4
  %t5 = call i32 @xiao_runtime_string_new(ptr %t3, ptr %t4)
  %t6 = icmp eq i32 %t5, 0
  br i1 %t6, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t7 = load ptr, ptr %t4
  store %xiao.value zeroinitializer, ptr %t9
  call void @xiao_runtime_value_str_owned(ptr sret(%xiao.value) %t9, ptr %t7)
  %t8 = load %xiao.value, ptr %t9
  %t11 = getelementptr inbounds %xiao.value, ptr %t10, i64 0
  store %xiao.value %t8, ptr %t11
  %t12 = call i32 @xiao_runtime_print_values_with_capabilities(ptr %t10, i64 1, i64 127)
  %t13 = icmp eq i32 %t12, 0
  br i1 %t13, label %abi.ok2, label %abi.status.fail3
abi.status.fail3:
  call i32 @xiao_runtime_error_attach_span(i64 1705, i64 1722)
  br label %xiao.fn.error
abi.ok2:
  store %xiao.value %t8, ptr %t14
  call void @xiao_runtime_value_release_strong(ptr %t14)
  store %xiao.value zeroinitializer, ptr %t16
  call void @xiao_runtime_value_none(ptr sret(%xiao.value) %t16)
  %t15 = load %xiao.value, ptr %t16
  store %xiao.value %t15, ptr %t17
  call void @xiao_runtime_value_release_strong(ptr %t17)
  store %xiao.value zeroinitializer, ptr %t19
  call void @xiao_runtime_value_float(ptr sret(%xiao.value) %t19, double 3.00000000000000000e0)
  %t18 = load %xiao.value, ptr %t19
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t18, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 true, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  %t20 = call i32 @xiao_runtime_error_class()
  %t21 = icmp eq i32 %t20, 0
  br i1 %t21, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 1677, i64 1738)
  br label %xiao.fn.error
abi.ok4:
  %t22 = load ptr, ptr %xiao.return.source
  %t23 = icmp ne ptr %t22, null
  br i1 %t23, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t24 = load i1, ptr %xiao.return.temporary
  br i1 %t24, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t25 = call i32 @xiao_runtime_value_copy(ptr %t22, ptr %xiao.function.result)
  %t26 = icmp eq i32 %t25, 0
  br i1 %t26, label %abi.ok6, label %abi.status.fail7
abi.status.fail7:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok6:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t27 = load %xiao.value, ptr %t22
  store %xiao.value %t27, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t22
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t28 = call i32 @xiao_runtime_error_class()
  %t29 = icmp eq i32 %t28, 0
  br i1 %t29, label %abi.ok8, label %abi.status.fail9
abi.status.fail9:
  call i32 @xiao_runtime_error_attach_span(i64 1677, i64 1738)
  br label %xiao.fn.error
abi.ok8:
  ret void
xiao.fn.error:
  %t30 = call i32 @xiao_runtime_error_class()
  %t31 = icmp eq i32 %t30, 2
  br i1 %t31, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t32 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes34, i64 0, i64 0
  %t33 = insertvalue %xiao.bytes zeroinitializer, ptr %t32, 0
  %t34 = insertvalue %xiao.bytes %t33, i64 5, 1
  store %xiao.bytes %t34, ptr %t35
  %t36 = getelementptr inbounds [9 x i8], ptr @.xiao.bytes35, i64 0, i64 0
  %t37 = insertvalue %xiao.bytes zeroinitializer, ptr %t36, 0
  %t38 = insertvalue %xiao.bytes %t37, i64 8, 1
  store %xiao.bytes %t38, ptr %t39
  call void @xiao_runtime_error_push_native_frame(ptr %t35, ptr %t39, i64 1677, i64 1738)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.cb1ffaaf3c3d15fa(ptr %xiao.function.result, ptr %xiao.arg0) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot0 = alloca %xiao.value
  %t3 = alloca %xiao.value
  %t4 = alloca %xiao.value
  %t8 = alloca %xiao.value
  %t26 = alloca %xiao.bytes
  %t30 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot0
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot0)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = load %xiao.value, ptr %slot0
  store %xiao.value zeroinitializer, ptr %t3
  store %xiao.value zeroinitializer, ptr %t4
  %t5 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t4)
  %t6 = icmp eq i32 %t5, 0
  br i1 %t6, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %cmath.arguments.fail2
abi.ok4:
  %t7 = load %xiao.value, ptr %t4
  store %xiao.value %t7, ptr %t3
  br label %cmath.arguments.ready3
cmath.arguments.fail2:
  call void @xiao_runtime_value_release_strong(ptr %t3)
  br label %xiao.fn.error
cmath.arguments.ready3:
  store %xiao.value zeroinitializer, ptr %t8
  %t9 = call i32 @xiao_runtime_cmath_sqrt(ptr %t3, ptr %t8)
  call void @xiao_runtime_value_release_strong(ptr %t3)
  switch i32 %t9, label %cmath.abi.failure8 [ i32 0, label %cmath.ok6 i32 6, label %cmath.error7 ]
cmath.abi.failure8:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error7:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call i32 @xiao_runtime_error_attach_span(i64 110, i64 127)
  br label %xiao.fn.error
cmath.ok6:
  %t10 = load %xiao.value, ptr %t8
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t10, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 true, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t11 = call i32 @xiao_runtime_error_class()
  %t12 = icmp eq i32 %t11, 0
  br i1 %t12, label %abi.ok9, label %abi.status.fail10
abi.status.fail10:
  call i32 @xiao_runtime_error_attach_span(i64 68, i64 129)
  br label %xiao.fn.error
abi.ok9:
  %t13 = load ptr, ptr %xiao.return.source
  %t14 = icmp ne ptr %t13, null
  br i1 %t14, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t15 = load i1, ptr %xiao.return.temporary
  br i1 %t15, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t16 = call i32 @xiao_runtime_value_copy(ptr %t13, ptr %xiao.function.result)
  %t17 = icmp eq i32 %t16, 0
  br i1 %t17, label %abi.ok11, label %abi.status.fail12
abi.status.fail12:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok11:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t18 = load %xiao.value, ptr %t13
  store %xiao.value %t18, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t13
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t19 = call i32 @xiao_runtime_error_class()
  %t20 = icmp eq i32 %t19, 0
  br i1 %t20, label %abi.ok13, label %abi.status.fail14
abi.status.fail14:
  call i32 @xiao_runtime_error_attach_span(i64 68, i64 129)
  br label %xiao.fn.error
abi.ok13:
  ret void
xiao.fn.error:
  %t21 = call i32 @xiao_runtime_error_class()
  %t22 = icmp eq i32 %t21, 2
  br i1 %t22, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t23 = getelementptr inbounds [13 x i8], ptr @.xiao.bytes36, i64 0, i64 0
  %t24 = insertvalue %xiao.bytes zeroinitializer, ptr %t23, 0
  %t25 = insertvalue %xiao.bytes %t24, i64 12, 1
  store %xiao.bytes %t25, ptr %t26
  %t27 = getelementptr inbounds [5 x i8], ptr @.xiao.bytes37, i64 0, i64 0
  %t28 = insertvalue %xiao.bytes zeroinitializer, ptr %t27, 0
  %t29 = insertvalue %xiao.bytes %t28, i64 4, 1
  store %xiao.bytes %t29, ptr %t30
  call void @xiao_runtime_error_push_native_frame(ptr %t26, ptr %t30, i64 68, i64 129)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.860bf3c2a9f5cdfc(ptr %xiao.function.result, ptr %xiao.arg0) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot0 = alloca %xiao.value
  %t3 = alloca %xiao.value
  %t4 = alloca %xiao.value
  %t8 = alloca %xiao.value
  %t26 = alloca %xiao.bytes
  %t30 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot0
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot0)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = load %xiao.value, ptr %slot0
  store %xiao.value zeroinitializer, ptr %t3
  store %xiao.value zeroinitializer, ptr %t4
  %t5 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t4)
  %t6 = icmp eq i32 %t5, 0
  br i1 %t6, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %cmath.arguments.fail2
abi.ok4:
  %t7 = load %xiao.value, ptr %t4
  store %xiao.value %t7, ptr %t3
  br label %cmath.arguments.ready3
cmath.arguments.fail2:
  call void @xiao_runtime_value_release_strong(ptr %t3)
  br label %xiao.fn.error
cmath.arguments.ready3:
  store %xiao.value zeroinitializer, ptr %t8
  %t9 = call i32 @xiao_runtime_cmath_floor(ptr %t3, ptr %t8)
  call void @xiao_runtime_value_release_strong(ptr %t3)
  switch i32 %t9, label %cmath.abi.failure8 [ i32 0, label %cmath.ok6 i32 6, label %cmath.error7 ]
cmath.abi.failure8:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error7:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call i32 @xiao_runtime_error_attach_span(i64 172, i64 190)
  br label %xiao.fn.error
cmath.ok6:
  %t10 = load %xiao.value, ptr %t8
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t10, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 true, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t11 = call i32 @xiao_runtime_error_class()
  %t12 = icmp eq i32 %t11, 0
  br i1 %t12, label %abi.ok9, label %abi.status.fail10
abi.status.fail10:
  call i32 @xiao_runtime_error_attach_span(i64 129, i64 192)
  br label %xiao.fn.error
abi.ok9:
  %t13 = load ptr, ptr %xiao.return.source
  %t14 = icmp ne ptr %t13, null
  br i1 %t14, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t15 = load i1, ptr %xiao.return.temporary
  br i1 %t15, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t16 = call i32 @xiao_runtime_value_copy(ptr %t13, ptr %xiao.function.result)
  %t17 = icmp eq i32 %t16, 0
  br i1 %t17, label %abi.ok11, label %abi.status.fail12
abi.status.fail12:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok11:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t18 = load %xiao.value, ptr %t13
  store %xiao.value %t18, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t13
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t19 = call i32 @xiao_runtime_error_class()
  %t20 = icmp eq i32 %t19, 0
  br i1 %t20, label %abi.ok13, label %abi.status.fail14
abi.status.fail14:
  call i32 @xiao_runtime_error_attach_span(i64 129, i64 192)
  br label %xiao.fn.error
abi.ok13:
  ret void
xiao.fn.error:
  %t21 = call i32 @xiao_runtime_error_class()
  %t22 = icmp eq i32 %t21, 2
  br i1 %t22, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t23 = getelementptr inbounds [13 x i8], ptr @.xiao.bytes38, i64 0, i64 0
  %t24 = insertvalue %xiao.bytes zeroinitializer, ptr %t23, 0
  %t25 = insertvalue %xiao.bytes %t24, i64 12, 1
  store %xiao.bytes %t25, ptr %t26
  %t27 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes39, i64 0, i64 0
  %t28 = insertvalue %xiao.bytes zeroinitializer, ptr %t27, 0
  %t29 = insertvalue %xiao.bytes %t28, i64 5, 1
  store %xiao.bytes %t29, ptr %t30
  call void @xiao_runtime_error_push_native_frame(ptr %t26, ptr %t30, i64 129, i64 192)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.538f14385bfc00b5(ptr %xiao.function.result, ptr %xiao.arg0) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot0 = alloca %xiao.value
  %t3 = alloca %xiao.value
  %t4 = alloca %xiao.value
  %t8 = alloca %xiao.value
  %t26 = alloca %xiao.bytes
  %t30 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot0
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot0)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = load %xiao.value, ptr %slot0
  store %xiao.value zeroinitializer, ptr %t3
  store %xiao.value zeroinitializer, ptr %t4
  %t5 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t4)
  %t6 = icmp eq i32 %t5, 0
  br i1 %t6, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %cmath.arguments.fail2
abi.ok4:
  %t7 = load %xiao.value, ptr %t4
  store %xiao.value %t7, ptr %t3
  br label %cmath.arguments.ready3
cmath.arguments.fail2:
  call void @xiao_runtime_value_release_strong(ptr %t3)
  br label %xiao.fn.error
cmath.arguments.ready3:
  store %xiao.value zeroinitializer, ptr %t8
  %t9 = call i32 @xiao_runtime_cmath_tan(ptr %t3, ptr %t8)
  call void @xiao_runtime_value_release_strong(ptr %t3)
  switch i32 %t9, label %cmath.abi.failure8 [ i32 0, label %cmath.ok6 i32 6, label %cmath.error7 ]
cmath.abi.failure8:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error7:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call i32 @xiao_runtime_error_attach_span(i64 834, i64 850)
  br label %xiao.fn.error
cmath.ok6:
  %t10 = load %xiao.value, ptr %t8
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t10, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 true, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t11 = call i32 @xiao_runtime_error_class()
  %t12 = icmp eq i32 %t11, 0
  br i1 %t12, label %abi.ok9, label %abi.status.fail10
abi.status.fail10:
  call i32 @xiao_runtime_error_attach_span(i64 793, i64 852)
  br label %xiao.fn.error
abi.ok9:
  %t13 = load ptr, ptr %xiao.return.source
  %t14 = icmp ne ptr %t13, null
  br i1 %t14, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t15 = load i1, ptr %xiao.return.temporary
  br i1 %t15, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t16 = call i32 @xiao_runtime_value_copy(ptr %t13, ptr %xiao.function.result)
  %t17 = icmp eq i32 %t16, 0
  br i1 %t17, label %abi.ok11, label %abi.status.fail12
abi.status.fail12:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok11:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t18 = load %xiao.value, ptr %t13
  store %xiao.value %t18, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t13
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t19 = call i32 @xiao_runtime_error_class()
  %t20 = icmp eq i32 %t19, 0
  br i1 %t20, label %abi.ok13, label %abi.status.fail14
abi.status.fail14:
  call i32 @xiao_runtime_error_attach_span(i64 793, i64 852)
  br label %xiao.fn.error
abi.ok13:
  ret void
xiao.fn.error:
  %t21 = call i32 @xiao_runtime_error_class()
  %t22 = icmp eq i32 %t21, 2
  br i1 %t22, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t23 = getelementptr inbounds [13 x i8], ptr @.xiao.bytes40, i64 0, i64 0
  %t24 = insertvalue %xiao.bytes zeroinitializer, ptr %t23, 0
  %t25 = insertvalue %xiao.bytes %t24, i64 12, 1
  store %xiao.bytes %t25, ptr %t26
  %t27 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes41, i64 0, i64 0
  %t28 = insertvalue %xiao.bytes zeroinitializer, ptr %t27, 0
  %t29 = insertvalue %xiao.bytes %t28, i64 3, 1
  store %xiao.bytes %t29, ptr %t30
  call void @xiao_runtime_error_push_native_frame(ptr %t26, ptr %t30, i64 793, i64 852)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.1c7fbe93e376e39c(ptr %xiao.function.result, ptr %xiao.arg0) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot0 = alloca %xiao.value
  %t3 = alloca %xiao.value
  %t4 = alloca %xiao.value
  %t8 = alloca %xiao.value
  %t26 = alloca %xiao.bytes
  %t30 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot0
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot0)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = load %xiao.value, ptr %slot0
  store %xiao.value zeroinitializer, ptr %t3
  store %xiao.value zeroinitializer, ptr %t4
  %t5 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t4)
  %t6 = icmp eq i32 %t5, 0
  br i1 %t6, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %cmath.arguments.fail2
abi.ok4:
  %t7 = load %xiao.value, ptr %t4
  store %xiao.value %t7, ptr %t3
  br label %cmath.arguments.ready3
cmath.arguments.fail2:
  call void @xiao_runtime_value_release_strong(ptr %t3)
  br label %xiao.fn.error
cmath.arguments.ready3:
  store %xiao.value zeroinitializer, ptr %t8
  %t9 = call i32 @xiao_runtime_cmath_ln(ptr %t3, ptr %t8)
  call void @xiao_runtime_value_release_strong(ptr %t3)
  switch i32 %t9, label %cmath.abi.failure8 [ i32 0, label %cmath.ok6 i32 6, label %cmath.error7 ]
cmath.abi.failure8:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error7:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call i32 @xiao_runtime_error_attach_span(i64 951, i64 966)
  br label %xiao.fn.error
cmath.ok6:
  %t10 = load %xiao.value, ptr %t8
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t10, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 true, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t11 = call i32 @xiao_runtime_error_class()
  %t12 = icmp eq i32 %t11, 0
  br i1 %t12, label %abi.ok9, label %abi.status.fail10
abi.status.fail10:
  call i32 @xiao_runtime_error_attach_span(i64 911, i64 968)
  br label %xiao.fn.error
abi.ok9:
  %t13 = load ptr, ptr %xiao.return.source
  %t14 = icmp ne ptr %t13, null
  br i1 %t14, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t15 = load i1, ptr %xiao.return.temporary
  br i1 %t15, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t16 = call i32 @xiao_runtime_value_copy(ptr %t13, ptr %xiao.function.result)
  %t17 = icmp eq i32 %t16, 0
  br i1 %t17, label %abi.ok11, label %abi.status.fail12
abi.status.fail12:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok11:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t18 = load %xiao.value, ptr %t13
  store %xiao.value %t18, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t13
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t19 = call i32 @xiao_runtime_error_class()
  %t20 = icmp eq i32 %t19, 0
  br i1 %t20, label %abi.ok13, label %abi.status.fail14
abi.status.fail14:
  call i32 @xiao_runtime_error_attach_span(i64 911, i64 968)
  br label %xiao.fn.error
abi.ok13:
  ret void
xiao.fn.error:
  %t21 = call i32 @xiao_runtime_error_class()
  %t22 = icmp eq i32 %t21, 2
  br i1 %t22, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t23 = getelementptr inbounds [13 x i8], ptr @.xiao.bytes42, i64 0, i64 0
  %t24 = insertvalue %xiao.bytes zeroinitializer, ptr %t23, 0
  %t25 = insertvalue %xiao.bytes %t24, i64 12, 1
  store %xiao.bytes %t25, ptr %t26
  %t27 = getelementptr inbounds [3 x i8], ptr @.xiao.bytes43, i64 0, i64 0
  %t28 = insertvalue %xiao.bytes zeroinitializer, ptr %t27, 0
  %t29 = insertvalue %xiao.bytes %t28, i64 2, 1
  store %xiao.bytes %t29, ptr %t30
  call void @xiao_runtime_error_push_native_frame(ptr %t26, ptr %t30, i64 911, i64 968)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.4e7be7efa34b4cfc(ptr %xiao.function.result, ptr %xiao.arg0) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot0 = alloca %xiao.value
  %t3 = alloca %xiao.value
  %t4 = alloca %xiao.value
  %t8 = alloca %xiao.value
  %t26 = alloca %xiao.bytes
  %t30 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot0
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot0)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = load %xiao.value, ptr %slot0
  store %xiao.value zeroinitializer, ptr %t3
  store %xiao.value zeroinitializer, ptr %t4
  %t5 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t4)
  %t6 = icmp eq i32 %t5, 0
  br i1 %t6, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %cmath.arguments.fail2
abi.ok4:
  %t7 = load %xiao.value, ptr %t4
  store %xiao.value %t7, ptr %t3
  br label %cmath.arguments.ready3
cmath.arguments.fail2:
  call void @xiao_runtime_value_release_strong(ptr %t3)
  br label %xiao.fn.error
cmath.arguments.ready3:
  store %xiao.value zeroinitializer, ptr %t8
  %t9 = call i32 @xiao_runtime_cmath_log10(ptr %t3, ptr %t8)
  call void @xiao_runtime_value_release_strong(ptr %t3)
  switch i32 %t9, label %cmath.abi.failure8 [ i32 0, label %cmath.ok6 i32 6, label %cmath.error7 ]
cmath.abi.failure8:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error7:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call i32 @xiao_runtime_error_attach_span(i64 1011, i64 1029)
  br label %xiao.fn.error
cmath.ok6:
  %t10 = load %xiao.value, ptr %t8
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t10, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 true, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t11 = call i32 @xiao_runtime_error_class()
  %t12 = icmp eq i32 %t11, 0
  br i1 %t12, label %abi.ok9, label %abi.status.fail10
abi.status.fail10:
  call i32 @xiao_runtime_error_attach_span(i64 968, i64 1030)
  br label %xiao.fn.error
abi.ok9:
  %t13 = load ptr, ptr %xiao.return.source
  %t14 = icmp ne ptr %t13, null
  br i1 %t14, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t15 = load i1, ptr %xiao.return.temporary
  br i1 %t15, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t16 = call i32 @xiao_runtime_value_copy(ptr %t13, ptr %xiao.function.result)
  %t17 = icmp eq i32 %t16, 0
  br i1 %t17, label %abi.ok11, label %abi.status.fail12
abi.status.fail12:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok11:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t18 = load %xiao.value, ptr %t13
  store %xiao.value %t18, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t13
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t19 = call i32 @xiao_runtime_error_class()
  %t20 = icmp eq i32 %t19, 0
  br i1 %t20, label %abi.ok13, label %abi.status.fail14
abi.status.fail14:
  call i32 @xiao_runtime_error_attach_span(i64 968, i64 1030)
  br label %xiao.fn.error
abi.ok13:
  ret void
xiao.fn.error:
  %t21 = call i32 @xiao_runtime_error_class()
  %t22 = icmp eq i32 %t21, 2
  br i1 %t22, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t23 = getelementptr inbounds [13 x i8], ptr @.xiao.bytes44, i64 0, i64 0
  %t24 = insertvalue %xiao.bytes zeroinitializer, ptr %t23, 0
  %t25 = insertvalue %xiao.bytes %t24, i64 12, 1
  store %xiao.bytes %t25, ptr %t26
  %t27 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes45, i64 0, i64 0
  %t28 = insertvalue %xiao.bytes zeroinitializer, ptr %t27, 0
  %t29 = insertvalue %xiao.bytes %t28, i64 5, 1
  store %xiao.bytes %t29, ptr %t30
  call void @xiao_runtime_error_push_native_frame(ptr %t26, ptr %t30, i64 968, i64 1030)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.c13a89556a9f20ac(ptr %xiao.function.result, ptr %xiao.arg0) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot0 = alloca %xiao.value
  %t3 = alloca %xiao.value
  %t4 = alloca %xiao.value
  %t8 = alloca %xiao.value
  %t26 = alloca %xiao.bytes
  %t30 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot0
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot0)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = load %xiao.value, ptr %slot0
  store %xiao.value zeroinitializer, ptr %t3
  store %xiao.value zeroinitializer, ptr %t4
  %t5 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t4)
  %t6 = icmp eq i32 %t5, 0
  br i1 %t6, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %cmath.arguments.fail2
abi.ok4:
  %t7 = load %xiao.value, ptr %t4
  store %xiao.value %t7, ptr %t3
  br label %cmath.arguments.ready3
cmath.arguments.fail2:
  call void @xiao_runtime_value_release_strong(ptr %t3)
  br label %xiao.fn.error
cmath.arguments.ready3:
  store %xiao.value zeroinitializer, ptr %t8
  %t9 = call i32 @xiao_runtime_cmath_round(ptr %t3, ptr %t8)
  call void @xiao_runtime_value_release_strong(ptr %t3)
  switch i32 %t9, label %cmath.abi.failure8 [ i32 0, label %cmath.ok6 i32 6, label %cmath.error7 ]
cmath.abi.failure8:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error7:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call i32 @xiao_runtime_error_attach_span(i64 296, i64 314)
  br label %xiao.fn.error
cmath.ok6:
  %t10 = load %xiao.value, ptr %t8
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t10, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 true, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t11 = call i32 @xiao_runtime_error_class()
  %t12 = icmp eq i32 %t11, 0
  br i1 %t12, label %abi.ok9, label %abi.status.fail10
abi.status.fail10:
  call i32 @xiao_runtime_error_attach_span(i64 253, i64 316)
  br label %xiao.fn.error
abi.ok9:
  %t13 = load ptr, ptr %xiao.return.source
  %t14 = icmp ne ptr %t13, null
  br i1 %t14, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t15 = load i1, ptr %xiao.return.temporary
  br i1 %t15, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t16 = call i32 @xiao_runtime_value_copy(ptr %t13, ptr %xiao.function.result)
  %t17 = icmp eq i32 %t16, 0
  br i1 %t17, label %abi.ok11, label %abi.status.fail12
abi.status.fail12:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok11:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t18 = load %xiao.value, ptr %t13
  store %xiao.value %t18, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t13
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t19 = call i32 @xiao_runtime_error_class()
  %t20 = icmp eq i32 %t19, 0
  br i1 %t20, label %abi.ok13, label %abi.status.fail14
abi.status.fail14:
  call i32 @xiao_runtime_error_attach_span(i64 253, i64 316)
  br label %xiao.fn.error
abi.ok13:
  ret void
xiao.fn.error:
  %t21 = call i32 @xiao_runtime_error_class()
  %t22 = icmp eq i32 %t21, 2
  br i1 %t22, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t23 = getelementptr inbounds [13 x i8], ptr @.xiao.bytes46, i64 0, i64 0
  %t24 = insertvalue %xiao.bytes zeroinitializer, ptr %t23, 0
  %t25 = insertvalue %xiao.bytes %t24, i64 12, 1
  store %xiao.bytes %t25, ptr %t26
  %t27 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes47, i64 0, i64 0
  %t28 = insertvalue %xiao.bytes zeroinitializer, ptr %t27, 0
  %t29 = insertvalue %xiao.bytes %t28, i64 5, 1
  store %xiao.bytes %t29, ptr %t30
  call void @xiao_runtime_error_push_native_frame(ptr %t26, ptr %t30, i64 253, i64 316)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.cf09f9314f602e51(ptr %xiao.function.result, ptr %xiao.arg0) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot0 = alloca %xiao.value
  %t3 = alloca %xiao.value
  %t4 = alloca %xiao.value
  %t8 = alloca %xiao.value
  %t26 = alloca %xiao.bytes
  %t30 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot0
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot0)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = load %xiao.value, ptr %slot0
  store %xiao.value zeroinitializer, ptr %t3
  store %xiao.value zeroinitializer, ptr %t4
  %t5 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t4)
  %t6 = icmp eq i32 %t5, 0
  br i1 %t6, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %cmath.arguments.fail2
abi.ok4:
  %t7 = load %xiao.value, ptr %t4
  store %xiao.value %t7, ptr %t3
  br label %cmath.arguments.ready3
cmath.arguments.fail2:
  call void @xiao_runtime_value_release_strong(ptr %t3)
  br label %xiao.fn.error
cmath.arguments.ready3:
  store %xiao.value zeroinitializer, ptr %t8
  %t9 = call i32 @xiao_runtime_cmath_trunc(ptr %t3, ptr %t8)
  call void @xiao_runtime_value_release_strong(ptr %t3)
  switch i32 %t9, label %cmath.abi.failure8 [ i32 0, label %cmath.ok6 i32 6, label %cmath.error7 ]
cmath.abi.failure8:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error7:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call i32 @xiao_runtime_error_attach_span(i64 359, i64 377)
  br label %xiao.fn.error
cmath.ok6:
  %t10 = load %xiao.value, ptr %t8
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t10, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 true, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t11 = call i32 @xiao_runtime_error_class()
  %t12 = icmp eq i32 %t11, 0
  br i1 %t12, label %abi.ok9, label %abi.status.fail10
abi.status.fail10:
  call i32 @xiao_runtime_error_attach_span(i64 316, i64 379)
  br label %xiao.fn.error
abi.ok9:
  %t13 = load ptr, ptr %xiao.return.source
  %t14 = icmp ne ptr %t13, null
  br i1 %t14, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t15 = load i1, ptr %xiao.return.temporary
  br i1 %t15, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t16 = call i32 @xiao_runtime_value_copy(ptr %t13, ptr %xiao.function.result)
  %t17 = icmp eq i32 %t16, 0
  br i1 %t17, label %abi.ok11, label %abi.status.fail12
abi.status.fail12:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok11:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t18 = load %xiao.value, ptr %t13
  store %xiao.value %t18, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t13
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t19 = call i32 @xiao_runtime_error_class()
  %t20 = icmp eq i32 %t19, 0
  br i1 %t20, label %abi.ok13, label %abi.status.fail14
abi.status.fail14:
  call i32 @xiao_runtime_error_attach_span(i64 316, i64 379)
  br label %xiao.fn.error
abi.ok13:
  ret void
xiao.fn.error:
  %t21 = call i32 @xiao_runtime_error_class()
  %t22 = icmp eq i32 %t21, 2
  br i1 %t22, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t23 = getelementptr inbounds [13 x i8], ptr @.xiao.bytes48, i64 0, i64 0
  %t24 = insertvalue %xiao.bytes zeroinitializer, ptr %t23, 0
  %t25 = insertvalue %xiao.bytes %t24, i64 12, 1
  store %xiao.bytes %t25, ptr %t26
  %t27 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes49, i64 0, i64 0
  %t28 = insertvalue %xiao.bytes zeroinitializer, ptr %t27, 0
  %t29 = insertvalue %xiao.bytes %t28, i64 5, 1
  store %xiao.bytes %t29, ptr %t30
  call void @xiao_runtime_error_push_native_frame(ptr %t26, ptr %t30, i64 316, i64 379)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.b66bfb82035c3390(ptr %xiao.function.result, ptr %xiao.arg0, ptr %xiao.arg1) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot0 = alloca %xiao.value
  %slot1 = alloca %xiao.value
  %t6 = alloca %xiao.value
  %t7 = alloca %xiao.value
  %t8 = alloca %xiao.value
  %t12 = alloca %xiao.value
  %t16 = alloca %xiao.value
  %t34 = alloca %xiao.bytes
  %t38 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot0
  store %xiao.value zeroinitializer, ptr %slot1
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot0)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg1, ptr %slot1)
  %t3 = icmp eq i32 %t2, 0
  br i1 %t3, label %abi.ok2, label %abi.status.fail3
abi.status.fail3:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok2:
  %t4 = load %xiao.value, ptr %slot0
  %t5 = load %xiao.value, ptr %slot1
  store %xiao.value zeroinitializer, ptr %t6
  store %xiao.value zeroinitializer, ptr %t7
  store %xiao.value zeroinitializer, ptr %t8
  %t9 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t8)
  %t10 = icmp eq i32 %t9, 0
  br i1 %t10, label %abi.ok6, label %abi.status.fail7
abi.status.fail7:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %cmath.arguments.fail4
abi.ok6:
  %t11 = load %xiao.value, ptr %t8
  store %xiao.value %t11, ptr %t6
  store %xiao.value zeroinitializer, ptr %t12
  %t13 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t12)
  %t14 = icmp eq i32 %t13, 0
  br i1 %t14, label %abi.ok8, label %abi.status.fail9
abi.status.fail9:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %cmath.arguments.fail4
abi.ok8:
  %t15 = load %xiao.value, ptr %t12
  store %xiao.value %t15, ptr %t7
  br label %cmath.arguments.ready5
cmath.arguments.fail4:
  call void @xiao_runtime_value_release_strong(ptr %t7)
  call void @xiao_runtime_value_release_strong(ptr %t6)
  br label %xiao.fn.error
cmath.arguments.ready5:
  store %xiao.value zeroinitializer, ptr %t16
  %t17 = call i32 @xiao_runtime_cmath_min(ptr %t6, ptr %t7, ptr %t16)
  call void @xiao_runtime_value_release_strong(ptr %t7)
  call void @xiao_runtime_value_release_strong(ptr %t6)
  switch i32 %t17, label %cmath.abi.failure12 [ i32 0, label %cmath.ok10 i32 6, label %cmath.error11 ]
cmath.abi.failure12:
  call void @xiao_runtime_value_release_strong(ptr %t16)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error11:
  call void @xiao_runtime_value_release_strong(ptr %t16)
  call i32 @xiao_runtime_error_attach_span(i64 491, i64 513)
  br label %xiao.fn.error
cmath.ok10:
  %t18 = load %xiao.value, ptr %t16
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t18, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 true, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t19 = call i32 @xiao_runtime_error_class()
  %t20 = icmp eq i32 %t19, 0
  br i1 %t20, label %abi.ok13, label %abi.status.fail14
abi.status.fail14:
  call i32 @xiao_runtime_error_attach_span(i64 438, i64 515)
  br label %xiao.fn.error
abi.ok13:
  %t21 = load ptr, ptr %xiao.return.source
  %t22 = icmp ne ptr %t21, null
  br i1 %t22, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t23 = load i1, ptr %xiao.return.temporary
  br i1 %t23, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t24 = call i32 @xiao_runtime_value_copy(ptr %t21, ptr %xiao.function.result)
  %t25 = icmp eq i32 %t24, 0
  br i1 %t25, label %abi.ok15, label %abi.status.fail16
abi.status.fail16:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok15:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t26 = load %xiao.value, ptr %t21
  store %xiao.value %t26, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t21
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t27 = call i32 @xiao_runtime_error_class()
  %t28 = icmp eq i32 %t27, 0
  br i1 %t28, label %abi.ok17, label %abi.status.fail18
abi.status.fail18:
  call i32 @xiao_runtime_error_attach_span(i64 438, i64 515)
  br label %xiao.fn.error
abi.ok17:
  ret void
xiao.fn.error:
  %t29 = call i32 @xiao_runtime_error_class()
  %t30 = icmp eq i32 %t29, 2
  br i1 %t30, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t31 = getelementptr inbounds [13 x i8], ptr @.xiao.bytes50, i64 0, i64 0
  %t32 = insertvalue %xiao.bytes zeroinitializer, ptr %t31, 0
  %t33 = insertvalue %xiao.bytes %t32, i64 12, 1
  store %xiao.bytes %t33, ptr %t34
  %t35 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes51, i64 0, i64 0
  %t36 = insertvalue %xiao.bytes zeroinitializer, ptr %t35, 0
  %t37 = insertvalue %xiao.bytes %t36, i64 3, 1
  store %xiao.bytes %t37, ptr %t38
  call void @xiao_runtime_error_push_native_frame(ptr %t34, ptr %t38, i64 438, i64 515)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.91742dfdb14adccd(ptr %xiao.function.result, ptr %xiao.arg0, ptr %xiao.arg1) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot0 = alloca %xiao.value
  %slot1 = alloca %xiao.value
  %t6 = alloca %xiao.value
  %t7 = alloca %xiao.value
  %t8 = alloca %xiao.value
  %t12 = alloca %xiao.value
  %t16 = alloca %xiao.value
  %t34 = alloca %xiao.bytes
  %t38 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot0
  store %xiao.value zeroinitializer, ptr %slot1
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot0)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg1, ptr %slot1)
  %t3 = icmp eq i32 %t2, 0
  br i1 %t3, label %abi.ok2, label %abi.status.fail3
abi.status.fail3:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok2:
  %t4 = load %xiao.value, ptr %slot0
  %t5 = load %xiao.value, ptr %slot1
  store %xiao.value zeroinitializer, ptr %t6
  store %xiao.value zeroinitializer, ptr %t7
  store %xiao.value zeroinitializer, ptr %t8
  %t9 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t8)
  %t10 = icmp eq i32 %t9, 0
  br i1 %t10, label %abi.ok6, label %abi.status.fail7
abi.status.fail7:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %cmath.arguments.fail4
abi.ok6:
  %t11 = load %xiao.value, ptr %t8
  store %xiao.value %t11, ptr %t6
  store %xiao.value zeroinitializer, ptr %t12
  %t13 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t12)
  %t14 = icmp eq i32 %t13, 0
  br i1 %t14, label %abi.ok8, label %abi.status.fail9
abi.status.fail9:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %cmath.arguments.fail4
abi.ok8:
  %t15 = load %xiao.value, ptr %t12
  store %xiao.value %t15, ptr %t7
  br label %cmath.arguments.ready5
cmath.arguments.fail4:
  call void @xiao_runtime_value_release_strong(ptr %t7)
  call void @xiao_runtime_value_release_strong(ptr %t6)
  br label %xiao.fn.error
cmath.arguments.ready5:
  store %xiao.value zeroinitializer, ptr %t16
  %t17 = call i32 @xiao_runtime_cmath_max(ptr %t6, ptr %t7, ptr %t16)
  call void @xiao_runtime_value_release_strong(ptr %t7)
  call void @xiao_runtime_value_release_strong(ptr %t6)
  switch i32 %t17, label %cmath.abi.failure12 [ i32 0, label %cmath.ok10 i32 6, label %cmath.error11 ]
cmath.abi.failure12:
  call void @xiao_runtime_value_release_strong(ptr %t16)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error11:
  call void @xiao_runtime_value_release_strong(ptr %t16)
  call i32 @xiao_runtime_error_attach_span(i64 568, i64 590)
  br label %xiao.fn.error
cmath.ok10:
  %t18 = load %xiao.value, ptr %t16
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t18, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 true, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t19 = call i32 @xiao_runtime_error_class()
  %t20 = icmp eq i32 %t19, 0
  br i1 %t20, label %abi.ok13, label %abi.status.fail14
abi.status.fail14:
  call i32 @xiao_runtime_error_attach_span(i64 515, i64 592)
  br label %xiao.fn.error
abi.ok13:
  %t21 = load ptr, ptr %xiao.return.source
  %t22 = icmp ne ptr %t21, null
  br i1 %t22, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t23 = load i1, ptr %xiao.return.temporary
  br i1 %t23, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t24 = call i32 @xiao_runtime_value_copy(ptr %t21, ptr %xiao.function.result)
  %t25 = icmp eq i32 %t24, 0
  br i1 %t25, label %abi.ok15, label %abi.status.fail16
abi.status.fail16:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok15:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t26 = load %xiao.value, ptr %t21
  store %xiao.value %t26, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t21
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t27 = call i32 @xiao_runtime_error_class()
  %t28 = icmp eq i32 %t27, 0
  br i1 %t28, label %abi.ok17, label %abi.status.fail18
abi.status.fail18:
  call i32 @xiao_runtime_error_attach_span(i64 515, i64 592)
  br label %xiao.fn.error
abi.ok17:
  ret void
xiao.fn.error:
  %t29 = call i32 @xiao_runtime_error_class()
  %t30 = icmp eq i32 %t29, 2
  br i1 %t30, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t31 = getelementptr inbounds [13 x i8], ptr @.xiao.bytes52, i64 0, i64 0
  %t32 = insertvalue %xiao.bytes zeroinitializer, ptr %t31, 0
  %t33 = insertvalue %xiao.bytes %t32, i64 12, 1
  store %xiao.bytes %t33, ptr %t34
  %t35 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes53, i64 0, i64 0
  %t36 = insertvalue %xiao.bytes zeroinitializer, ptr %t35, 0
  %t37 = insertvalue %xiao.bytes %t36, i64 3, 1
  store %xiao.bytes %t37, ptr %t38
  call void @xiao_runtime_error_push_native_frame(ptr %t34, ptr %t38, i64 515, i64 592)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.4e6166bed798273c(ptr %xiao.function.result, ptr %xiao.arg0, ptr %xiao.arg1) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot0 = alloca %xiao.value
  %slot1 = alloca %xiao.value
  %t6 = alloca %xiao.value
  %t7 = alloca %xiao.value
  %t8 = alloca %xiao.value
  %t12 = alloca %xiao.value
  %t16 = alloca %xiao.value
  %t34 = alloca %xiao.bytes
  %t38 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot0
  store %xiao.value zeroinitializer, ptr %slot1
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot0)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg1, ptr %slot1)
  %t3 = icmp eq i32 %t2, 0
  br i1 %t3, label %abi.ok2, label %abi.status.fail3
abi.status.fail3:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok2:
  %t4 = load %xiao.value, ptr %slot0
  %t5 = load %xiao.value, ptr %slot1
  store %xiao.value zeroinitializer, ptr %t6
  store %xiao.value zeroinitializer, ptr %t7
  store %xiao.value zeroinitializer, ptr %t8
  %t9 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t8)
  %t10 = icmp eq i32 %t9, 0
  br i1 %t10, label %abi.ok6, label %abi.status.fail7
abi.status.fail7:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %cmath.arguments.fail4
abi.ok6:
  %t11 = load %xiao.value, ptr %t8
  store %xiao.value %t11, ptr %t6
  store %xiao.value zeroinitializer, ptr %t12
  %t13 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t12)
  %t14 = icmp eq i32 %t13, 0
  br i1 %t14, label %abi.ok8, label %abi.status.fail9
abi.status.fail9:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %cmath.arguments.fail4
abi.ok8:
  %t15 = load %xiao.value, ptr %t12
  store %xiao.value %t15, ptr %t7
  br label %cmath.arguments.ready5
cmath.arguments.fail4:
  call void @xiao_runtime_value_release_strong(ptr %t7)
  call void @xiao_runtime_value_release_strong(ptr %t6)
  br label %xiao.fn.error
cmath.arguments.ready5:
  store %xiao.value zeroinitializer, ptr %t16
  %t17 = call i32 @xiao_runtime_cmath_pow(ptr %t6, ptr %t7, ptr %t16)
  call void @xiao_runtime_value_release_strong(ptr %t7)
  call void @xiao_runtime_value_release_strong(ptr %t6)
  switch i32 %t17, label %cmath.abi.failure12 [ i32 0, label %cmath.ok10 i32 6, label %cmath.error11 ]
cmath.abi.failure12:
  call void @xiao_runtime_value_release_strong(ptr %t16)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error11:
  call void @xiao_runtime_value_release_strong(ptr %t16)
  call i32 @xiao_runtime_error_attach_span(i64 648, i64 673)
  br label %xiao.fn.error
cmath.ok10:
  %t18 = load %xiao.value, ptr %t16
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t18, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 true, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t19 = call i32 @xiao_runtime_error_class()
  %t20 = icmp eq i32 %t19, 0
  br i1 %t20, label %abi.ok13, label %abi.status.fail14
abi.status.fail14:
  call i32 @xiao_runtime_error_attach_span(i64 592, i64 675)
  br label %xiao.fn.error
abi.ok13:
  %t21 = load ptr, ptr %xiao.return.source
  %t22 = icmp ne ptr %t21, null
  br i1 %t22, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t23 = load i1, ptr %xiao.return.temporary
  br i1 %t23, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t24 = call i32 @xiao_runtime_value_copy(ptr %t21, ptr %xiao.function.result)
  %t25 = icmp eq i32 %t24, 0
  br i1 %t25, label %abi.ok15, label %abi.status.fail16
abi.status.fail16:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok15:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t26 = load %xiao.value, ptr %t21
  store %xiao.value %t26, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t21
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t27 = call i32 @xiao_runtime_error_class()
  %t28 = icmp eq i32 %t27, 0
  br i1 %t28, label %abi.ok17, label %abi.status.fail18
abi.status.fail18:
  call i32 @xiao_runtime_error_attach_span(i64 592, i64 675)
  br label %xiao.fn.error
abi.ok17:
  ret void
xiao.fn.error:
  %t29 = call i32 @xiao_runtime_error_class()
  %t30 = icmp eq i32 %t29, 2
  br i1 %t30, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t31 = getelementptr inbounds [13 x i8], ptr @.xiao.bytes54, i64 0, i64 0
  %t32 = insertvalue %xiao.bytes zeroinitializer, ptr %t31, 0
  %t33 = insertvalue %xiao.bytes %t32, i64 12, 1
  store %xiao.bytes %t33, ptr %t34
  %t35 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes55, i64 0, i64 0
  %t36 = insertvalue %xiao.bytes zeroinitializer, ptr %t35, 0
  %t37 = insertvalue %xiao.bytes %t36, i64 3, 1
  store %xiao.bytes %t37, ptr %t38
  call void @xiao_runtime_error_push_native_frame(ptr %t34, ptr %t38, i64 592, i64 675)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.e5d6d1fbcd3af2ba(ptr %xiao.function.result, ptr %xiao.arg0) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot0 = alloca %xiao.value
  %t3 = alloca %xiao.value
  %t4 = alloca %xiao.value
  %t8 = alloca %xiao.value
  %t26 = alloca %xiao.bytes
  %t30 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot0
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot0)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = load %xiao.value, ptr %slot0
  store %xiao.value zeroinitializer, ptr %t3
  store %xiao.value zeroinitializer, ptr %t4
  %t5 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t4)
  %t6 = icmp eq i32 %t5, 0
  br i1 %t6, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %cmath.arguments.fail2
abi.ok4:
  %t7 = load %xiao.value, ptr %t4
  store %xiao.value %t7, ptr %t3
  br label %cmath.arguments.ready3
cmath.arguments.fail2:
  call void @xiao_runtime_value_release_strong(ptr %t3)
  br label %xiao.fn.error
cmath.arguments.ready3:
  store %xiao.value zeroinitializer, ptr %t8
  %t9 = call i32 @xiao_runtime_cmath_sin(ptr %t3, ptr %t8)
  call void @xiao_runtime_value_release_strong(ptr %t3)
  switch i32 %t9, label %cmath.abi.failure8 [ i32 0, label %cmath.ok6 i32 6, label %cmath.error7 ]
cmath.abi.failure8:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call void @xiao_runtime_fatal_abi()
  unreachable
cmath.error7:
  call void @xiao_runtime_value_release_strong(ptr %t8)
  call i32 @xiao_runtime_error_attach_span(i64 716, i64 732)
  br label %xiao.fn.error
cmath.ok6:
  %t10 = load %xiao.value, ptr %t8
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t10, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 true, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t11 = call i32 @xiao_runtime_error_class()
  %t12 = icmp eq i32 %t11, 0
  br i1 %t12, label %abi.ok9, label %abi.status.fail10
abi.status.fail10:
  call i32 @xiao_runtime_error_attach_span(i64 675, i64 734)
  br label %xiao.fn.error
abi.ok9:
  %t13 = load ptr, ptr %xiao.return.source
  %t14 = icmp ne ptr %t13, null
  br i1 %t14, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t15 = load i1, ptr %xiao.return.temporary
  br i1 %t15, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t16 = call i32 @xiao_runtime_value_copy(ptr %t13, ptr %xiao.function.result)
  %t17 = icmp eq i32 %t16, 0
  br i1 %t17, label %abi.ok11, label %abi.status.fail12
abi.status.fail12:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok11:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t18 = load %xiao.value, ptr %t13
  store %xiao.value %t18, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t13
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t19 = call i32 @xiao_runtime_error_class()
  %t20 = icmp eq i32 %t19, 0
  br i1 %t20, label %abi.ok13, label %abi.status.fail14
abi.status.fail14:
  call i32 @xiao_runtime_error_attach_span(i64 675, i64 734)
  br label %xiao.fn.error
abi.ok13:
  ret void
xiao.fn.error:
  %t21 = call i32 @xiao_runtime_error_class()
  %t22 = icmp eq i32 %t21, 2
  br i1 %t22, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t23 = getelementptr inbounds [13 x i8], ptr @.xiao.bytes56, i64 0, i64 0
  %t24 = insertvalue %xiao.bytes zeroinitializer, ptr %t23, 0
  %t25 = insertvalue %xiao.bytes %t24, i64 12, 1
  store %xiao.bytes %t25, ptr %t26
  %t27 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes57, i64 0, i64 0
  %t28 = insertvalue %xiao.bytes zeroinitializer, ptr %t27, 0
  %t29 = insertvalue %xiao.bytes %t28, i64 3, 1
  store %xiao.bytes %t29, ptr %t30
  call void @xiao_runtime_error_push_native_frame(ptr %t26, ptr %t30, i64 675, i64 734)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.674bab6222f36ddf(ptr %xiao.function.result, ptr %xiao.arg0, ptr %xiao.arg1) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.site1 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot0 = alloca %xiao.value
  %slot2 = alloca %xiao.value
  %slot1 = alloca %xiao.value
  %t6 = alloca %xiao.value
  %t11 = alloca %xiao.value
  %t15 = alloca %xiao.bytes
  %t16 = alloca %xiao.value
  %t17 = alloca %xiao.value
  %t18 = alloca %xiao.value
  %t22 = alloca %xiao.value
  %t23 = alloca %xiao.value
  %t24 = alloca %xiao.value
  %t25 = alloca %xiao.value
  %t30 = alloca %xiao.value
  %t31 = alloca %xiao.value
  %t38 = alloca %xiao.bytes
  %t39 = alloca %xiao.value
  %t40 = alloca %xiao.value
  %t44 = alloca %xiao.value
  %t45 = alloca %xiao.value
  %t46 = alloca %xiao.value
  %t51 = alloca %xiao.value
  %t55 = alloca %xiao.bytes
  %t56 = alloca %xiao.value
  %t57 = alloca %xiao.value
  %t58 = alloca %xiao.value
  %t62 = alloca %xiao.value
  %t63 = alloca %xiao.value
  %t64 = alloca %xiao.value
  %t65 = alloca %xiao.value
  %t70 = alloca %xiao.value
  %t71 = alloca %xiao.value
  %t78 = alloca %xiao.bytes
  %t79 = alloca %xiao.value
  %t80 = alloca %xiao.value
  %t84 = alloca %xiao.value
  %t85 = alloca %xiao.value
  %t86 = alloca %xiao.value
  %t91 = alloca %xiao.value
  %t95 = alloca %xiao.bytes
  %t96 = alloca %xiao.value
  %t97 = alloca %xiao.value
  %t98 = alloca %xiao.value
  %t102 = alloca %xiao.value
  %t103 = alloca %xiao.value
  %t104 = alloca %xiao.value
  %t105 = alloca %xiao.value
  %t110 = alloca %xiao.value
  %t111 = alloca %xiao.value
  %t116 = alloca %xiao.value
  %t120 = alloca %xiao.bytes
  %t121 = alloca %xiao.value
  %t122 = alloca %xiao.value
  %t126 = alloca %xiao.value
  %t127 = alloca %xiao.value
  %t131 = alloca %xiao.bytes
  %t132 = alloca %xiao.value
  %t133 = alloca %xiao.value
  %t134 = alloca %xiao.value
  %t138 = alloca %xiao.value
  %t139 = alloca %xiao.value
  %t140 = alloca %xiao.value
  %t141 = alloca %xiao.value
  %t146 = alloca %xiao.value
  %t148 = alloca %xiao.value
  %t149 = alloca %xiao.value
  %t153 = alloca %xiao.value
  %t160 = alloca %xiao.bytes
  %t161 = alloca %xiao.value
  %t162 = alloca %xiao.value
  %t163 = alloca %xiao.value
  %t167 = alloca %xiao.value
  %t168 = alloca %xiao.value
  %t169 = alloca %xiao.value
  %t170 = alloca %xiao.value
  %t171 = alloca %xiao.value
  %t175 = alloca %xiao.value
  %t179 = alloca %xiao.value
  %t186 = alloca %xiao.bytes
  %t187 = alloca %xiao.value
  %t188 = alloca %xiao.value
  %t192 = alloca %xiao.value
  %t193 = alloca %xiao.value
  %t209 = alloca %xiao.bytes
  %t213 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store %xiao.value zeroinitializer, ptr %xiao.return.site1
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot0
  store %xiao.value zeroinitializer, ptr %slot2
  store %xiao.value zeroinitializer, ptr %slot1
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot0)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg1, ptr %slot1)
  %t3 = icmp eq i32 %t2, 0
  br i1 %t3, label %abi.ok2, label %abi.status.fail3
abi.status.fail3:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok2:
  %t4 = load %xiao.value, ptr %slot0
  %t5 = load %xiao.value, ptr %slot1
  store %xiao.value zeroinitializer, ptr %t6
  %t7 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t6)
  %t8 = icmp eq i32 %t7, 0
  br i1 %t8, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok4:
  %t9 = load %xiao.value, ptr %t6
  store %xiao.value zeroinitializer, ptr %t11
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t11, i64 0)
  %t10 = load %xiao.value, ptr %t11
  %t12 = getelementptr inbounds [3 x i8], ptr @.xiao.bytes58, i64 0, i64 0
  %t13 = insertvalue %xiao.bytes zeroinitializer, ptr %t12, 0
  %t14 = insertvalue %xiao.bytes %t13, i64 2, 1
  store %xiao.bytes %t14, ptr %t15
  store %xiao.value %t9, ptr %t16
  store %xiao.value %t10, ptr %t17
  store %xiao.value zeroinitializer, ptr %t18
  %t19 = call i32 @xiao_runtime_value_binary(ptr %t15, ptr %t16, ptr %t17, ptr %t18)
  %t20 = icmp eq i32 %t19, 0
  br i1 %t20, label %abi.ok9, label %abi.status.fail10
abi.status.fail10:
  call i32 @xiao_runtime_error_attach_span(i64 708, i64 716)
  br label %dynamic.binary.fail7
abi.ok9:
  %t21 = load %xiao.value, ptr %t18
  store %xiao.value %t9, ptr %t22
  call void @xiao_runtime_value_release_strong(ptr %t22)
  store %xiao.value %t10, ptr %t23
  call void @xiao_runtime_value_release_strong(ptr %t23)
  br label %dynamic.binary.continue8
dynamic.binary.fail7:
  store %xiao.value %t10, ptr %t24
  call void @xiao_runtime_value_release_strong(ptr %t24)
  br label %dynamic.binary.left.fail6
dynamic.binary.left.fail6:
  store %xiao.value %t9, ptr %t25
  call void @xiao_runtime_value_release_strong(ptr %t25)
  br label %xiao.fn.error
dynamic.binary.continue8:
  %t26 = extractvalue %xiao.value %t21, 0
  %t27 = icmp eq i32 %t26, 1
  br i1 %t27, label %dynamic.bool.ok11, label %xiao.fn.error
dynamic.bool.ok11:
  %t28 = extractvalue %xiao.value %t21, 1
  %t29 = trunc i64 %t28 to i1
  store %xiao.value %t21, ptr %t30
  call void @xiao_runtime_value_release_strong(ptr %t30)
  br i1 %t29, label %dynamic.if.then12, label %dynamic.if.next13
dynamic.if.then12:
  store %xiao.value zeroinitializer, ptr %t31
  %t32 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t31)
  %t33 = icmp eq i32 %t32, 0
  br i1 %t33, label %abi.ok15, label %abi.status.fail16
abi.status.fail16:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok15:
  %t34 = load %xiao.value, ptr %t31
  %t35 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes59, i64 0, i64 0
  %t36 = insertvalue %xiao.bytes zeroinitializer, ptr %t35, 0
  %t37 = insertvalue %xiao.bytes %t36, i64 5, 1
  store %xiao.bytes %t37, ptr %t38
  store %xiao.value %t34, ptr %t39
  store %xiao.value zeroinitializer, ptr %t40
  %t41 = call i32 @xiao_runtime_value_unary(ptr %t38, ptr %t39, ptr %t40)
  %t42 = icmp eq i32 %t41, 0
  br i1 %t42, label %abi.ok19, label %abi.status.fail20
abi.status.fail20:
  call i32 @xiao_runtime_error_attach_span(i64 732, i64 737)
  br label %dynamic.unary.fail17
abi.ok19:
  %t43 = load %xiao.value, ptr %t40
  store %xiao.value %t34, ptr %t44
  call void @xiao_runtime_value_release_strong(ptr %t44)
  br label %dynamic.unary.continue18
dynamic.unary.fail17:
  store %xiao.value %t34, ptr %t45
  call void @xiao_runtime_value_release_strong(ptr %t45)
  br label %xiao.fn.error
dynamic.unary.continue18:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  store %xiao.value %t43, ptr %slot0
  br label %dynamic.if.merge14
dynamic.if.next13:
  br label %dynamic.if.merge14
dynamic.if.merge14:
  store %xiao.value zeroinitializer, ptr %t46
  %t47 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t46)
  %t48 = icmp eq i32 %t47, 0
  br i1 %t48, label %abi.ok21, label %abi.status.fail22
abi.status.fail22:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok21:
  %t49 = load %xiao.value, ptr %t46
  store %xiao.value zeroinitializer, ptr %t51
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t51, i64 0)
  %t50 = load %xiao.value, ptr %t51
  %t52 = getelementptr inbounds [3 x i8], ptr @.xiao.bytes60, i64 0, i64 0
  %t53 = insertvalue %xiao.bytes zeroinitializer, ptr %t52, 0
  %t54 = insertvalue %xiao.bytes %t53, i64 2, 1
  store %xiao.bytes %t54, ptr %t55
  store %xiao.value %t49, ptr %t56
  store %xiao.value %t50, ptr %t57
  store %xiao.value zeroinitializer, ptr %t58
  %t59 = call i32 @xiao_runtime_value_binary(ptr %t55, ptr %t56, ptr %t57, ptr %t58)
  %t60 = icmp eq i32 %t59, 0
  br i1 %t60, label %abi.ok26, label %abi.status.fail27
abi.status.fail27:
  call i32 @xiao_runtime_error_attach_span(i64 745, i64 754)
  br label %dynamic.binary.fail24
abi.ok26:
  %t61 = load %xiao.value, ptr %t58
  store %xiao.value %t49, ptr %t62
  call void @xiao_runtime_value_release_strong(ptr %t62)
  store %xiao.value %t50, ptr %t63
  call void @xiao_runtime_value_release_strong(ptr %t63)
  br label %dynamic.binary.continue25
dynamic.binary.fail24:
  store %xiao.value %t50, ptr %t64
  call void @xiao_runtime_value_release_strong(ptr %t64)
  br label %dynamic.binary.left.fail23
dynamic.binary.left.fail23:
  store %xiao.value %t49, ptr %t65
  call void @xiao_runtime_value_release_strong(ptr %t65)
  br label %xiao.fn.error
dynamic.binary.continue25:
  %t66 = extractvalue %xiao.value %t61, 0
  %t67 = icmp eq i32 %t66, 1
  br i1 %t67, label %dynamic.bool.ok28, label %xiao.fn.error
dynamic.bool.ok28:
  %t68 = extractvalue %xiao.value %t61, 1
  %t69 = trunc i64 %t68 to i1
  store %xiao.value %t61, ptr %t70
  call void @xiao_runtime_value_release_strong(ptr %t70)
  br i1 %t69, label %dynamic.if.then29, label %dynamic.if.next30
dynamic.if.then29:
  store %xiao.value zeroinitializer, ptr %t71
  %t72 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t71)
  %t73 = icmp eq i32 %t72, 0
  br i1 %t73, label %abi.ok32, label %abi.status.fail33
abi.status.fail33:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok32:
  %t74 = load %xiao.value, ptr %t71
  %t75 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes61, i64 0, i64 0
  %t76 = insertvalue %xiao.bytes zeroinitializer, ptr %t75, 0
  %t77 = insertvalue %xiao.bytes %t76, i64 5, 1
  store %xiao.bytes %t77, ptr %t78
  store %xiao.value %t74, ptr %t79
  store %xiao.value zeroinitializer, ptr %t80
  %t81 = call i32 @xiao_runtime_value_unary(ptr %t78, ptr %t79, ptr %t80)
  %t82 = icmp eq i32 %t81, 0
  br i1 %t82, label %abi.ok36, label %abi.status.fail37
abi.status.fail37:
  call i32 @xiao_runtime_error_attach_span(i64 771, i64 777)
  br label %dynamic.unary.fail34
abi.ok36:
  %t83 = load %xiao.value, ptr %t80
  store %xiao.value %t74, ptr %t84
  call void @xiao_runtime_value_release_strong(ptr %t84)
  br label %dynamic.unary.continue35
dynamic.unary.fail34:
  store %xiao.value %t74, ptr %t85
  call void @xiao_runtime_value_release_strong(ptr %t85)
  br label %xiao.fn.error
dynamic.unary.continue35:
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  store %xiao.value %t83, ptr %slot1
  br label %dynamic.if.merge31
dynamic.if.next30:
  br label %dynamic.if.merge31
dynamic.if.merge31:
  br label %dynamic.while.cond38
dynamic.while.cond38:
  store %xiao.value zeroinitializer, ptr %t86
  %t87 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t86)
  %t88 = icmp eq i32 %t87, 0
  br i1 %t88, label %abi.ok41, label %abi.status.fail42
abi.status.fail42:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok41:
  %t89 = load %xiao.value, ptr %t86
  store %xiao.value zeroinitializer, ptr %t91
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t91, i64 0)
  %t90 = load %xiao.value, ptr %t91
  %t92 = getelementptr inbounds [3 x i8], ptr @.xiao.bytes62, i64 0, i64 0
  %t93 = insertvalue %xiao.bytes zeroinitializer, ptr %t92, 0
  %t94 = insertvalue %xiao.bytes %t93, i64 2, 1
  store %xiao.bytes %t94, ptr %t95
  store %xiao.value %t89, ptr %t96
  store %xiao.value %t90, ptr %t97
  store %xiao.value zeroinitializer, ptr %t98
  %t99 = call i32 @xiao_runtime_value_binary(ptr %t95, ptr %t96, ptr %t97, ptr %t98)
  %t100 = icmp eq i32 %t99, 0
  br i1 %t100, label %abi.ok46, label %abi.status.fail47
abi.status.fail47:
  call i32 @xiao_runtime_error_attach_span(i64 788, i64 798)
  br label %dynamic.binary.fail44
abi.ok46:
  %t101 = load %xiao.value, ptr %t98
  store %xiao.value %t89, ptr %t102
  call void @xiao_runtime_value_release_strong(ptr %t102)
  store %xiao.value %t90, ptr %t103
  call void @xiao_runtime_value_release_strong(ptr %t103)
  br label %dynamic.binary.continue45
dynamic.binary.fail44:
  store %xiao.value %t90, ptr %t104
  call void @xiao_runtime_value_release_strong(ptr %t104)
  br label %dynamic.binary.left.fail43
dynamic.binary.left.fail43:
  store %xiao.value %t89, ptr %t105
  call void @xiao_runtime_value_release_strong(ptr %t105)
  br label %xiao.fn.error
dynamic.binary.continue45:
  %t106 = extractvalue %xiao.value %t101, 0
  %t107 = icmp eq i32 %t106, 1
  br i1 %t107, label %dynamic.bool.ok48, label %xiao.fn.error
dynamic.bool.ok48:
  %t108 = extractvalue %xiao.value %t101, 1
  %t109 = trunc i64 %t108 to i1
  store %xiao.value %t101, ptr %t110
  call void @xiao_runtime_value_release_strong(ptr %t110)
  br i1 %t109, label %dynamic.while.body39, label %dynamic.while.end40
dynamic.while.body39:
  store %xiao.value zeroinitializer, ptr %t111
  %t112 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t111)
  %t113 = icmp eq i32 %t112, 0
  br i1 %t113, label %abi.ok49, label %abi.status.fail50
abi.status.fail50:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok49:
  %t114 = load %xiao.value, ptr %t111
  store %xiao.value zeroinitializer, ptr %t116
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t116, i64 1)
  %t115 = load %xiao.value, ptr %t116
  %t117 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes63, i64 0, i64 0
  %t118 = insertvalue %xiao.bytes zeroinitializer, ptr %t117, 0
  %t119 = insertvalue %xiao.bytes %t118, i64 5, 1
  store %xiao.bytes %t119, ptr %t120
  store %xiao.value %t115, ptr %t121
  store %xiao.value zeroinitializer, ptr %t122
  %t123 = call i32 @xiao_runtime_value_unary(ptr %t120, ptr %t121, ptr %t122)
  %t124 = icmp eq i32 %t123, 0
  br i1 %t124, label %abi.ok54, label %abi.status.fail55
abi.status.fail55:
  call i32 @xiao_runtime_error_attach_span(i64 888, i64 890)
  br label %dynamic.unary.fail52
abi.ok54:
  %t125 = load %xiao.value, ptr %t122
  store %xiao.value %t115, ptr %t126
  call void @xiao_runtime_value_release_strong(ptr %t126)
  br label %dynamic.unary.continue53
dynamic.unary.fail52:
  store %xiao.value %t115, ptr %t127
  call void @xiao_runtime_value_release_strong(ptr %t127)
  br label %dynamic.binary.left.fail51
dynamic.unary.continue53:
  %t128 = getelementptr inbounds [3 x i8], ptr @.xiao.bytes64, i64 0, i64 0
  %t129 = insertvalue %xiao.bytes zeroinitializer, ptr %t128, 0
  %t130 = insertvalue %xiao.bytes %t129, i64 2, 1
  store %xiao.bytes %t130, ptr %t131
  store %xiao.value %t114, ptr %t132
  store %xiao.value %t125, ptr %t133
  store %xiao.value zeroinitializer, ptr %t134
  %t135 = call i32 @xiao_runtime_value_binary(ptr %t131, ptr %t132, ptr %t133, ptr %t134)
  %t136 = icmp eq i32 %t135, 0
  br i1 %t136, label %abi.ok58, label %abi.status.fail59
abi.status.fail59:
  call i32 @xiao_runtime_error_attach_span(i64 879, i64 890)
  br label %dynamic.binary.fail56
abi.ok58:
  %t137 = load %xiao.value, ptr %t134
  store %xiao.value %t114, ptr %t138
  call void @xiao_runtime_value_release_strong(ptr %t138)
  store %xiao.value %t125, ptr %t139
  call void @xiao_runtime_value_release_strong(ptr %t139)
  br label %dynamic.binary.continue57
dynamic.binary.fail56:
  store %xiao.value %t125, ptr %t140
  call void @xiao_runtime_value_release_strong(ptr %t140)
  br label %dynamic.binary.left.fail51
dynamic.binary.left.fail51:
  store %xiao.value %t114, ptr %t141
  call void @xiao_runtime_value_release_strong(ptr %t141)
  br label %xiao.fn.error
dynamic.binary.continue57:
  %t142 = extractvalue %xiao.value %t137, 0
  %t143 = icmp eq i32 %t142, 1
  br i1 %t143, label %dynamic.bool.ok60, label %xiao.fn.error
dynamic.bool.ok60:
  %t144 = extractvalue %xiao.value %t137, 1
  %t145 = trunc i64 %t144 to i1
  store %xiao.value %t137, ptr %t146
  call void @xiao_runtime_value_release_strong(ptr %t146)
  br i1 %t145, label %dynamic.if.then61, label %dynamic.if.next62
dynamic.if.then61:
  store %xiao.value zeroinitializer, ptr %t148
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t148, i64 1)
  %t147 = load %xiao.value, ptr %t148
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t147, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 true, ptr %xiao.return.temporary
  br label %xiao.fn.return
dynamic.if.next62:
  br label %dynamic.if.merge63
dynamic.if.merge63:
  store %xiao.value zeroinitializer, ptr %t149
  %t150 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t149)
  %t151 = icmp eq i32 %t150, 0
  br i1 %t151, label %abi.ok64, label %abi.status.fail65
abi.status.fail65:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok64:
  %t152 = load %xiao.value, ptr %t149
  store %xiao.value zeroinitializer, ptr %t153
  %t154 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t153)
  %t155 = icmp eq i32 %t154, 0
  br i1 %t155, label %abi.ok67, label %abi.status.fail68
abi.status.fail68:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.binary.left.fail66
abi.ok67:
  %t156 = load %xiao.value, ptr %t153
  %t157 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes65, i64 0, i64 0
  %t158 = insertvalue %xiao.bytes zeroinitializer, ptr %t157, 0
  %t159 = insertvalue %xiao.bytes %t158, i64 3, 1
  store %xiao.bytes %t159, ptr %t160
  store %xiao.value %t152, ptr %t161
  store %xiao.value %t156, ptr %t162
  store %xiao.value zeroinitializer, ptr %t163
  %t164 = call i32 @xiao_runtime_value_binary(ptr %t160, ptr %t161, ptr %t162, ptr %t163)
  %t165 = icmp eq i32 %t164, 0
  br i1 %t165, label %abi.ok71, label %abi.status.fail72
abi.status.fail72:
  call i32 @xiao_runtime_error_attach_span(i64 932, i64 944)
  br label %dynamic.binary.fail69
abi.ok71:
  %t166 = load %xiao.value, ptr %t163
  store %xiao.value %t152, ptr %t167
  call void @xiao_runtime_value_release_strong(ptr %t167)
  store %xiao.value %t156, ptr %t168
  call void @xiao_runtime_value_release_strong(ptr %t168)
  br label %dynamic.binary.continue70
dynamic.binary.fail69:
  store %xiao.value %t156, ptr %t169
  call void @xiao_runtime_value_release_strong(ptr %t169)
  br label %dynamic.binary.left.fail66
dynamic.binary.left.fail66:
  store %xiao.value %t152, ptr %t170
  call void @xiao_runtime_value_release_strong(ptr %t170)
  br label %xiao.fn.error
dynamic.binary.continue70:
  call void @xiao_runtime_value_release_strong(ptr %slot2)
  store %xiao.value %t166, ptr %slot2
  store %xiao.value zeroinitializer, ptr %t171
  %t172 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t171)
  %t173 = icmp eq i32 %t172, 0
  br i1 %t173, label %abi.ok73, label %abi.status.fail74
abi.status.fail74:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok73:
  %t174 = load %xiao.value, ptr %t171
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  store %xiao.value %t174, ptr %slot0
  store %xiao.value zeroinitializer, ptr %t175
  %t176 = call i32 @xiao_runtime_value_copy(ptr %slot2, ptr %t175)
  %t177 = icmp eq i32 %t176, 0
  br i1 %t177, label %abi.ok75, label %abi.status.fail76
abi.status.fail76:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok75:
  %t178 = load %xiao.value, ptr %t175
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  store %xiao.value %t178, ptr %slot1
  br label %dynamic.while.cond38
dynamic.while.end40:
  store %xiao.value zeroinitializer, ptr %t179
  %t180 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t179)
  %t181 = icmp eq i32 %t180, 0
  br i1 %t181, label %abi.ok77, label %abi.status.fail78
abi.status.fail78:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok77:
  %t182 = load %xiao.value, ptr %t179
  %t183 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes66, i64 0, i64 0
  %t184 = insertvalue %xiao.bytes zeroinitializer, ptr %t183, 0
  %t185 = insertvalue %xiao.bytes %t184, i64 5, 1
  store %xiao.bytes %t185, ptr %t186
  store %xiao.value %t182, ptr %t187
  store %xiao.value zeroinitializer, ptr %t188
  %t189 = call i32 @xiao_runtime_value_unary(ptr %t186, ptr %t187, ptr %t188)
  %t190 = icmp eq i32 %t189, 0
  br i1 %t190, label %abi.ok81, label %abi.status.fail82
abi.status.fail82:
  call i32 @xiao_runtime_error_attach_span(i64 1003, i64 1008)
  br label %dynamic.unary.fail79
abi.ok81:
  %t191 = load %xiao.value, ptr %t188
  store %xiao.value %t182, ptr %t192
  call void @xiao_runtime_value_release_strong(ptr %t192)
  br label %dynamic.unary.continue80
dynamic.unary.fail79:
  store %xiao.value %t182, ptr %t193
  call void @xiao_runtime_value_release_strong(ptr %t193)
  br label %xiao.fn.error
dynamic.unary.continue80:
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site1)
  store %xiao.value %t191, ptr %xiao.return.site1
  store ptr %xiao.return.site1, ptr %xiao.return.source
  store i1 true, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot2)
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t194 = call i32 @xiao_runtime_error_class()
  %t195 = icmp eq i32 %t194, 0
  br i1 %t195, label %abi.ok83, label %abi.status.fail84
abi.status.fail84:
  call i32 @xiao_runtime_error_attach_span(i64 570, i64 1010)
  br label %xiao.fn.error
abi.ok83:
  %t196 = load ptr, ptr %xiao.return.source
  %t197 = icmp ne ptr %t196, null
  br i1 %t197, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t198 = load i1, ptr %xiao.return.temporary
  br i1 %t198, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t199 = call i32 @xiao_runtime_value_copy(ptr %t196, ptr %xiao.function.result)
  %t200 = icmp eq i32 %t199, 0
  br i1 %t200, label %abi.ok85, label %abi.status.fail86
abi.status.fail86:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok85:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t201 = load %xiao.value, ptr %t196
  store %xiao.value %t201, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t196
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site1)
  %t202 = call i32 @xiao_runtime_error_class()
  %t203 = icmp eq i32 %t202, 0
  br i1 %t203, label %abi.ok87, label %abi.status.fail88
abi.status.fail88:
  call i32 @xiao_runtime_error_attach_span(i64 570, i64 1010)
  br label %xiao.fn.error
abi.ok87:
  ret void
xiao.fn.error:
  %t204 = call i32 @xiao_runtime_error_class()
  %t205 = icmp eq i32 %t204, 2
  br i1 %t205, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot2)
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site1)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t206 = getelementptr inbounds [12 x i8], ptr @.xiao.bytes67, i64 0, i64 0
  %t207 = insertvalue %xiao.bytes zeroinitializer, ptr %t206, 0
  %t208 = insertvalue %xiao.bytes %t207, i64 11, 1
  store %xiao.bytes %t208, ptr %t209
  %t210 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes68, i64 0, i64 0
  %t211 = insertvalue %xiao.bytes zeroinitializer, ptr %t210, 0
  %t212 = insertvalue %xiao.bytes %t211, i64 3, 1
  store %xiao.bytes %t212, ptr %t213
  call void @xiao_runtime_error_push_native_frame(ptr %t209, ptr %t213, i64 570, i64 1010)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.476bad2c9b53486a(ptr %xiao.function.result, ptr %xiao.arg0, ptr %xiao.arg1) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.site1 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot0 = alloca %xiao.value
  %slot1 = alloca %xiao.value
  %t6 = alloca %xiao.value
  %t10 = alloca %xiao.value
  %t17 = alloca %xiao.bytes
  %t18 = alloca %xiao.value
  %t19 = alloca %xiao.value
  %t20 = alloca %xiao.value
  %t24 = alloca %xiao.value
  %t25 = alloca %xiao.value
  %t26 = alloca %xiao.value
  %t27 = alloca %xiao.value
  %t32 = alloca %xiao.value
  %t33 = alloca %xiao.value
  %t37 = alloca %xiao.value
  %t56 = alloca %xiao.bytes
  %t60 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store %xiao.value zeroinitializer, ptr %xiao.return.site1
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot0
  store %xiao.value zeroinitializer, ptr %slot1
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot0)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg1, ptr %slot1)
  %t3 = icmp eq i32 %t2, 0
  br i1 %t3, label %abi.ok2, label %abi.status.fail3
abi.status.fail3:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok2:
  %t4 = load %xiao.value, ptr %slot0
  %t5 = load %xiao.value, ptr %slot1
  store %xiao.value zeroinitializer, ptr %t6
  %t7 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t6)
  %t8 = icmp eq i32 %t7, 0
  br i1 %t8, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok4:
  %t9 = load %xiao.value, ptr %t6
  store %xiao.value zeroinitializer, ptr %t10
  %t11 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t10)
  %t12 = icmp eq i32 %t11, 0
  br i1 %t12, label %abi.ok7, label %abi.status.fail8
abi.status.fail8:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.binary.left.fail6
abi.ok7:
  %t13 = load %xiao.value, ptr %t10
  %t14 = getelementptr inbounds [3 x i8], ptr @.xiao.bytes69, i64 0, i64 0
  %t15 = insertvalue %xiao.bytes zeroinitializer, ptr %t14, 0
  %t16 = insertvalue %xiao.bytes %t15, i64 2, 1
  store %xiao.bytes %t16, ptr %t17
  store %xiao.value %t9, ptr %t18
  store %xiao.value %t13, ptr %t19
  store %xiao.value zeroinitializer, ptr %t20
  %t21 = call i32 @xiao_runtime_value_binary(ptr %t17, ptr %t18, ptr %t19, ptr %t20)
  %t22 = icmp eq i32 %t21, 0
  br i1 %t22, label %abi.ok11, label %abi.status.fail12
abi.status.fail12:
  call i32 @xiao_runtime_error_attach_span(i64 202, i64 214)
  br label %dynamic.binary.fail9
abi.ok11:
  %t23 = load %xiao.value, ptr %t20
  store %xiao.value %t9, ptr %t24
  call void @xiao_runtime_value_release_strong(ptr %t24)
  store %xiao.value %t13, ptr %t25
  call void @xiao_runtime_value_release_strong(ptr %t25)
  br label %dynamic.binary.continue10
dynamic.binary.fail9:
  store %xiao.value %t13, ptr %t26
  call void @xiao_runtime_value_release_strong(ptr %t26)
  br label %dynamic.binary.left.fail6
dynamic.binary.left.fail6:
  store %xiao.value %t9, ptr %t27
  call void @xiao_runtime_value_release_strong(ptr %t27)
  br label %xiao.fn.error
dynamic.binary.continue10:
  %t28 = extractvalue %xiao.value %t23, 0
  %t29 = icmp eq i32 %t28, 1
  br i1 %t29, label %dynamic.bool.ok13, label %xiao.fn.error
dynamic.bool.ok13:
  %t30 = extractvalue %xiao.value %t23, 1
  %t31 = trunc i64 %t30 to i1
  store %xiao.value %t23, ptr %t32
  call void @xiao_runtime_value_release_strong(ptr %t32)
  br i1 %t31, label %dynamic.if.then14, label %dynamic.if.next15
dynamic.if.then14:
  store %xiao.value zeroinitializer, ptr %t33
  %t34 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t33)
  %t35 = icmp eq i32 %t34, 0
  br i1 %t35, label %abi.ok17, label %abi.status.fail18
abi.status.fail18:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok17:
  %t36 = load %xiao.value, ptr %t33
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t36, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  br label %xiao.fn.return
dynamic.if.next15:
  br label %dynamic.if.merge16
dynamic.if.merge16:
  store %xiao.value zeroinitializer, ptr %t37
  %t38 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t37)
  %t39 = icmp eq i32 %t38, 0
  br i1 %t39, label %abi.ok19, label %abi.status.fail20
abi.status.fail20:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok19:
  %t40 = load %xiao.value, ptr %t37
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site1)
  store %xiao.value %t40, ptr %xiao.return.site1
  store ptr %xiao.return.site1, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t41 = call i32 @xiao_runtime_error_class()
  %t42 = icmp eq i32 %t41, 0
  br i1 %t42, label %abi.ok21, label %abi.status.fail22
abi.status.fail22:
  call i32 @xiao_runtime_error_attach_span(i64 159, i64 253)
  br label %xiao.fn.error
abi.ok21:
  %t43 = load ptr, ptr %xiao.return.source
  %t44 = icmp ne ptr %t43, null
  br i1 %t44, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t45 = load i1, ptr %xiao.return.temporary
  br i1 %t45, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t46 = call i32 @xiao_runtime_value_copy(ptr %t43, ptr %xiao.function.result)
  %t47 = icmp eq i32 %t46, 0
  br i1 %t47, label %abi.ok23, label %abi.status.fail24
abi.status.fail24:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok23:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t48 = load %xiao.value, ptr %t43
  store %xiao.value %t48, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t43
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site1)
  %t49 = call i32 @xiao_runtime_error_class()
  %t50 = icmp eq i32 %t49, 0
  br i1 %t50, label %abi.ok25, label %abi.status.fail26
abi.status.fail26:
  call i32 @xiao_runtime_error_attach_span(i64 159, i64 253)
  br label %xiao.fn.error
abi.ok25:
  ret void
xiao.fn.error:
  %t51 = call i32 @xiao_runtime_error_class()
  %t52 = icmp eq i32 %t51, 2
  br i1 %t52, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site1)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t53 = getelementptr inbounds [12 x i8], ptr @.xiao.bytes70, i64 0, i64 0
  %t54 = insertvalue %xiao.bytes zeroinitializer, ptr %t53, 0
  %t55 = insertvalue %xiao.bytes %t54, i64 11, 1
  store %xiao.bytes %t55, ptr %t56
  %t57 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes71, i64 0, i64 0
  %t58 = insertvalue %xiao.bytes zeroinitializer, ptr %t57, 0
  %t59 = insertvalue %xiao.bytes %t58, i64 3, 1
  store %xiao.bytes %t59, ptr %t60
  call void @xiao_runtime_error_push_native_frame(ptr %t56, ptr %t60, i64 159, i64 253)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.b1efe0225cc1dafd(ptr %xiao.function.result, ptr %xiao.arg0) {
entry:
  %xiao.generated.site0 = alloca %xiao.value
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot0 = alloca %xiao.value
  %slot2 = alloca %xiao.value
  %slot1 = alloca %xiao.value
  %slot3 = alloca %xiao.value
  %t4 = alloca %xiao.value
  %t9 = alloca %xiao.bytes
  %t10 = alloca %xiao.value
  %t13 = alloca %xiao.value
  %t14 = alloca i64
  %t18 = alloca i64
  %t22 = alloca %xiao.value
  %t23 = alloca %xiao.value
  %t27 = alloca %xiao.value
  %t32 = alloca %xiao.value
  %t36 = alloca %xiao.bytes
  %t37 = alloca %xiao.value
  %t38 = alloca %xiao.value
  %t39 = alloca %xiao.value
  %t43 = alloca %xiao.value
  %t44 = alloca %xiao.value
  %t45 = alloca %xiao.value
  %t46 = alloca %xiao.value
  %t47 = alloca %xiao.value
  %t52 = alloca %xiao.value
  %t56 = alloca %xiao.bytes
  %t57 = alloca %xiao.value
  %t58 = alloca %xiao.value
  %t59 = alloca %xiao.value
  %t63 = alloca %xiao.value
  %t64 = alloca %xiao.value
  %t65 = alloca %xiao.value
  %t66 = alloca %xiao.value
  %t71 = alloca %xiao.value
  %t75 = alloca %xiao.bytes
  %t76 = alloca ptr
  %t81 = alloca %xiao.value
  %t82 = alloca %xiao.value
  %t86 = alloca %xiao.bytes
  %t87 = alloca %xiao.error.location
  %t92 = alloca %xiao.value
  %t93 = alloca %xiao.value
  %t96 = alloca %xiao.value
  %t97 = alloca %xiao.error.location
  %t106 = alloca %xiao.value
  %t111 = alloca %xiao.value
  %t115 = alloca %xiao.bytes
  %t116 = alloca %xiao.value
  %t117 = alloca %xiao.value
  %t118 = alloca %xiao.value
  %t122 = alloca %xiao.value
  %t123 = alloca %xiao.value
  %t124 = alloca %xiao.value
  %t125 = alloca %xiao.value
  %t130 = alloca %xiao.value
  %t134 = alloca %xiao.bytes
  %t135 = alloca ptr
  %t140 = alloca %xiao.value
  %t141 = alloca %xiao.value
  %t145 = alloca %xiao.bytes
  %t146 = alloca %xiao.error.location
  %t151 = alloca %xiao.value
  %t152 = alloca %xiao.value
  %t155 = alloca %xiao.value
  %t156 = alloca %xiao.error.location
  %t162 = alloca %xiao.value
  %t166 = alloca %xiao.value
  %t167 = alloca %xiao.value
  %t177 = alloca %xiao.bytes
  %t178 = alloca %xiao.value
  %t181 = alloca %xiao.value
  %t182 = alloca i64
  %t186 = alloca i64
  %t190 = alloca %xiao.value
  %t191 = alloca %xiao.value
  %t195 = alloca %xiao.value
  %t199 = alloca %xiao.value
  %t206 = alloca %xiao.bytes
  %t207 = alloca %xiao.value
  %t208 = alloca %xiao.value
  %t209 = alloca %xiao.value
  %t213 = alloca %xiao.value
  %t214 = alloca %xiao.value
  %t215 = alloca %xiao.value
  %t216 = alloca %xiao.value
  %t221 = alloca %xiao.value
  %t222 = alloca %xiao.value
  %t229 = alloca %xiao.value
  %t248 = alloca %xiao.bytes
  %t252 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.generated.site0
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot0
  store %xiao.value zeroinitializer, ptr %slot2
  store %xiao.value zeroinitializer, ptr %slot1
  store %xiao.value zeroinitializer, ptr %slot3
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot3)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = load %xiao.value, ptr %slot3
  store %xiao.value zeroinitializer, ptr %t4
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t4, i64 0)
  %t3 = load %xiao.value, ptr %t4
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  store %xiao.value %t3, ptr %slot0
  %t5 = load %xiao.value, ptr %slot3
  %t6 = getelementptr inbounds [9 x i8], ptr @.xiao.bytes72, i64 0, i64 0
  %t7 = insertvalue %xiao.bytes zeroinitializer, ptr %t6, 0
  %t8 = insertvalue %xiao.bytes %t7, i64 8, 1
  store %xiao.bytes %t8, ptr %t9
  store %xiao.value %t5, ptr %t10
  %t11 = call i32 @xiao_runtime_dynamic_check(ptr %t9, ptr %t10)
  %t12 = icmp eq i32 %t11, 0
  br i1 %t12, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 400, i64 406)
  br label %dynamic.for.fail2
abi.ok4:
  store %xiao.value %t5, ptr %t13
  store i64 0, ptr %t14
  %t15 = call i32 @xiao_runtime_value_iter_len(ptr %t13, ptr %t14)
  %t16 = icmp eq i32 %t15, 0
  br i1 %t16, label %abi.ok6, label %abi.status.fail7
abi.status.fail7:
  call i32 @xiao_runtime_error_attach_span(i64 400, i64 406)
  br label %dynamic.for.fail2
abi.ok6:
  %t17 = load i64, ptr %t14
  store i64 0, ptr %t18
  br label %dynamic.for.cond8
dynamic.for.cond8:
  %t19 = load i64, ptr %t18
  %t20 = icmp ult i64 %t19, %t17
  br i1 %t20, label %dynamic.for.body9, label %dynamic.for.end11
dynamic.for.body9:
  %t21 = load %xiao.value, ptr %slot3
  store %xiao.value %t21, ptr %t22
  store %xiao.value zeroinitializer, ptr %t23
  %t24 = call i32 @xiao_runtime_value_iter_get(ptr %t22, i64 %t19, ptr %t23)
  %t25 = icmp eq i32 %t24, 0
  br i1 %t25, label %abi.ok12, label %abi.status.fail13
abi.status.fail13:
  call i32 @xiao_runtime_error_attach_span(i64 391, i64 396)
  br label %dynamic.for.fail2
abi.ok12:
  %t26 = load %xiao.value, ptr %t23
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  store %xiao.value %t26, ptr %slot1
  store %xiao.value zeroinitializer, ptr %t27
  %t28 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t27)
  %t29 = icmp eq i32 %t28, 0
  br i1 %t29, label %abi.ok14, label %abi.status.fail15
abi.status.fail15:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.for.fail2
abi.ok14:
  %t30 = load %xiao.value, ptr %t27
  store %xiao.value zeroinitializer, ptr %t32
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t32, i64 1)
  %t31 = load %xiao.value, ptr %t32
  %t33 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes73, i64 0, i64 0
  %t34 = insertvalue %xiao.bytes zeroinitializer, ptr %t33, 0
  %t35 = insertvalue %xiao.bytes %t34, i64 3, 1
  store %xiao.bytes %t35, ptr %t36
  store %xiao.value %t30, ptr %t37
  store %xiao.value %t31, ptr %t38
  store %xiao.value zeroinitializer, ptr %t39
  %t40 = call i32 @xiao_runtime_value_binary(ptr %t36, ptr %t37, ptr %t38, ptr %t39)
  %t41 = icmp eq i32 %t40, 0
  br i1 %t41, label %abi.ok19, label %abi.status.fail20
abi.status.fail20:
  call i32 @xiao_runtime_error_attach_span(i64 423, i64 432)
  br label %dynamic.binary.fail17
abi.ok19:
  %t42 = load %xiao.value, ptr %t39
  store %xiao.value %t30, ptr %t43
  call void @xiao_runtime_value_release_strong(ptr %t43)
  store %xiao.value %t31, ptr %t44
  call void @xiao_runtime_value_release_strong(ptr %t44)
  br label %dynamic.binary.continue18
dynamic.binary.fail17:
  store %xiao.value %t31, ptr %t45
  call void @xiao_runtime_value_release_strong(ptr %t45)
  br label %dynamic.binary.left.fail16
dynamic.binary.left.fail16:
  store %xiao.value %t30, ptr %t46
  call void @xiao_runtime_value_release_strong(ptr %t46)
  br label %dynamic.for.fail2
dynamic.binary.continue18:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  store %xiao.value %t42, ptr %slot0
  store %xiao.value zeroinitializer, ptr %t47
  %t48 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t47)
  %t49 = icmp eq i32 %t48, 0
  br i1 %t49, label %abi.ok21, label %abi.status.fail22
abi.status.fail22:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.for.fail2
abi.ok21:
  %t50 = load %xiao.value, ptr %t47
  store %xiao.value zeroinitializer, ptr %t52
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t52, i64 65536)
  %t51 = load %xiao.value, ptr %t52
  %t53 = getelementptr inbounds [3 x i8], ptr @.xiao.bytes74, i64 0, i64 0
  %t54 = insertvalue %xiao.bytes zeroinitializer, ptr %t53, 0
  %t55 = insertvalue %xiao.bytes %t54, i64 2, 1
  store %xiao.bytes %t55, ptr %t56
  store %xiao.value %t50, ptr %t57
  store %xiao.value %t51, ptr %t58
  store %xiao.value zeroinitializer, ptr %t59
  %t60 = call i32 @xiao_runtime_value_binary(ptr %t56, ptr %t57, ptr %t58, ptr %t59)
  %t61 = icmp eq i32 %t60, 0
  br i1 %t61, label %abi.ok26, label %abi.status.fail27
abi.status.fail27:
  call i32 @xiao_runtime_error_attach_span(i64 444, i64 457)
  br label %dynamic.binary.fail24
abi.ok26:
  %t62 = load %xiao.value, ptr %t59
  store %xiao.value %t50, ptr %t63
  call void @xiao_runtime_value_release_strong(ptr %t63)
  store %xiao.value %t51, ptr %t64
  call void @xiao_runtime_value_release_strong(ptr %t64)
  br label %dynamic.binary.continue25
dynamic.binary.fail24:
  store %xiao.value %t51, ptr %t65
  call void @xiao_runtime_value_release_strong(ptr %t65)
  br label %dynamic.binary.left.fail23
dynamic.binary.left.fail23:
  store %xiao.value %t50, ptr %t66
  call void @xiao_runtime_value_release_strong(ptr %t66)
  br label %dynamic.for.fail2
dynamic.binary.continue25:
  %t67 = extractvalue %xiao.value %t62, 0
  %t68 = icmp eq i32 %t67, 1
  br i1 %t68, label %dynamic.bool.ok28, label %dynamic.for.fail2
dynamic.bool.ok28:
  %t69 = extractvalue %xiao.value %t62, 1
  %t70 = trunc i64 %t69 to i1
  store %xiao.value %t62, ptr %t71
  call void @xiao_runtime_value_release_strong(ptr %t71)
  br i1 %t70, label %dynamic.if.then29, label %dynamic.if.next30
dynamic.if.then29:
  %t72 = getelementptr inbounds [26 x i8], ptr @.xiao.str75, i64 0, i64 0
  %t73 = insertvalue %xiao.bytes zeroinitializer, ptr %t72, 0
  %t74 = insertvalue %xiao.bytes %t73, i64 25, 1
  store %xiao.bytes %t74, ptr %t75
  store ptr null, ptr %t76
  %t77 = call i32 @xiao_runtime_string_new(ptr %t75, ptr %t76)
  %t78 = icmp eq i32 %t77, 0
  br i1 %t78, label %abi.ok32, label %abi.status.fail33
abi.status.fail33:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.for.fail2
abi.ok32:
  %t79 = load ptr, ptr %t76
  store %xiao.value zeroinitializer, ptr %t81
  call void @xiao_runtime_value_str_owned(ptr sret(%xiao.value) %t81, ptr %t79)
  %t80 = load %xiao.value, ptr %t81
  store %xiao.value %t80, ptr %t82
  %t83 = getelementptr inbounds [14 x i8], ptr @.xiao.bytes76, i64 0, i64 0
  %t84 = insertvalue %xiao.bytes zeroinitializer, ptr %t83, 0
  %t85 = insertvalue %xiao.bytes %t84, i64 13, 1
  store %xiao.bytes %t85, ptr %t86
  %t88 = insertvalue %xiao.error.location zeroinitializer, i64 476, 0
  %t89 = insertvalue %xiao.error.location %t88, i64 525, 1
  %t90 = insertvalue %xiao.error.location %t89, i8 1, 2
  store %xiao.error.location %t90, ptr %t87
  store %xiao.value zeroinitializer, ptr %t92
  call void @xiao_runtime_error_new_values(ptr sret(%xiao.value) %t92, ptr %t86, ptr %t82, ptr null, ptr %t87)
  %t91 = load %xiao.value, ptr %t92
  store %xiao.value %t80, ptr %t93
  call void @xiao_runtime_value_release_strong(ptr %t93)
  %t94 = call i32 @xiao_runtime_error_class()
  %t95 = icmp eq i32 %t94, 0
  br i1 %t95, label %abi.error.ok34, label %abi.error.fail35
abi.error.fail35:
  call i32 @xiao_runtime_error_attach_span(i64 476, i64 525)
  br label %dynamic.for.fail2
abi.error.ok34:
  store %xiao.value %t91, ptr %t96
  %t98 = insertvalue %xiao.error.location zeroinitializer, i64 470, 0
  %t99 = insertvalue %xiao.error.location %t98, i64 525, 1
  %t100 = insertvalue %xiao.error.location %t99, i8 1, 2
  store %xiao.error.location %t100, ptr %t97
  %t101 = call i32 @xiao_runtime_error_raise_value(ptr %t96, ptr %t97)
  call void @xiao_runtime_value_release_strong(ptr %t96)
  %t102 = icmp eq i32 %t101, 0
  br i1 %t102, label %abi.ok36, label %abi.status.fail37
abi.status.fail37:
  call i32 @xiao_runtime_error_attach_span(i64 470, i64 525)
  br label %dynamic.for.fail2
abi.ok36:
  br label %dynamic.for.fail2
dynamic.if.next30:
  br label %dynamic.if.merge31
dynamic.if.merge31:
  br label %dynamic.for.advance10
dynamic.for.advance10:
  %t103 = add i64 %t19, 1
  store i64 %t103, ptr %t18
  br label %dynamic.for.cond8
dynamic.for.end11:
  br label %dynamic.for.continue3
dynamic.for.fail2:
  %t104 = call i32 @xiao_runtime_error_class()
  %t105 = icmp eq i32 %t104, 2
  br i1 %t105, label %xiao.fn.error, label %dynamic.error-cleanup.recoverable39
dynamic.error-cleanup.recoverable39:
  br label %dynamic.for.source-cleanup38
dynamic.for.source-cleanup38:
  br label %xiao.fn.error
dynamic.for.continue3:
  store %xiao.value zeroinitializer, ptr %t106
  %t107 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t106)
  %t108 = icmp eq i32 %t107, 0
  br i1 %t108, label %abi.ok40, label %abi.status.fail41
abi.status.fail41:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok40:
  %t109 = load %xiao.value, ptr %t106
  store %xiao.value zeroinitializer, ptr %t111
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t111, i64 0)
  %t110 = load %xiao.value, ptr %t111
  %t112 = getelementptr inbounds [3 x i8], ptr @.xiao.bytes77, i64 0, i64 0
  %t113 = insertvalue %xiao.bytes zeroinitializer, ptr %t112, 0
  %t114 = insertvalue %xiao.bytes %t113, i64 2, 1
  store %xiao.bytes %t114, ptr %t115
  store %xiao.value %t109, ptr %t116
  store %xiao.value %t110, ptr %t117
  store %xiao.value zeroinitializer, ptr %t118
  %t119 = call i32 @xiao_runtime_value_binary(ptr %t115, ptr %t116, ptr %t117, ptr %t118)
  %t120 = icmp eq i32 %t119, 0
  br i1 %t120, label %abi.ok45, label %abi.status.fail46
abi.status.fail46:
  call i32 @xiao_runtime_error_attach_span(i64 533, i64 543)
  br label %dynamic.binary.fail43
abi.ok45:
  %t121 = load %xiao.value, ptr %t118
  store %xiao.value %t109, ptr %t122
  call void @xiao_runtime_value_release_strong(ptr %t122)
  store %xiao.value %t110, ptr %t123
  call void @xiao_runtime_value_release_strong(ptr %t123)
  br label %dynamic.binary.continue44
dynamic.binary.fail43:
  store %xiao.value %t110, ptr %t124
  call void @xiao_runtime_value_release_strong(ptr %t124)
  br label %dynamic.binary.left.fail42
dynamic.binary.left.fail42:
  store %xiao.value %t109, ptr %t125
  call void @xiao_runtime_value_release_strong(ptr %t125)
  br label %xiao.fn.error
dynamic.binary.continue44:
  %t126 = extractvalue %xiao.value %t121, 0
  %t127 = icmp eq i32 %t126, 1
  br i1 %t127, label %dynamic.bool.ok47, label %xiao.fn.error
dynamic.bool.ok47:
  %t128 = extractvalue %xiao.value %t121, 1
  %t129 = trunc i64 %t128 to i1
  store %xiao.value %t121, ptr %t130
  call void @xiao_runtime_value_release_strong(ptr %t130)
  br i1 %t129, label %dynamic.if.then48, label %dynamic.if.next49
dynamic.if.then48:
  %t131 = getelementptr inbounds [17 x i8], ptr @.xiao.str78, i64 0, i64 0
  %t132 = insertvalue %xiao.bytes zeroinitializer, ptr %t131, 0
  %t133 = insertvalue %xiao.bytes %t132, i64 16, 1
  store %xiao.bytes %t133, ptr %t134
  store ptr null, ptr %t135
  %t136 = call i32 @xiao_runtime_string_new(ptr %t134, ptr %t135)
  %t137 = icmp eq i32 %t136, 0
  br i1 %t137, label %abi.ok51, label %abi.status.fail52
abi.status.fail52:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok51:
  %t138 = load ptr, ptr %t135
  store %xiao.value zeroinitializer, ptr %t140
  call void @xiao_runtime_value_str_owned(ptr sret(%xiao.value) %t140, ptr %t138)
  %t139 = load %xiao.value, ptr %t140
  store %xiao.value %t139, ptr %t141
  %t142 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes79, i64 0, i64 0
  %t143 = insertvalue %xiao.bytes zeroinitializer, ptr %t142, 0
  %t144 = insertvalue %xiao.bytes %t143, i64 5, 1
  store %xiao.bytes %t144, ptr %t145
  %t147 = insertvalue %xiao.error.location zeroinitializer, i64 558, 0
  %t148 = insertvalue %xiao.error.location %t147, i64 590, 1
  %t149 = insertvalue %xiao.error.location %t148, i8 1, 2
  store %xiao.error.location %t149, ptr %t146
  store %xiao.value zeroinitializer, ptr %t151
  call void @xiao_runtime_error_new_values(ptr sret(%xiao.value) %t151, ptr %t145, ptr %t141, ptr null, ptr %t146)
  %t150 = load %xiao.value, ptr %t151
  store %xiao.value %t139, ptr %t152
  call void @xiao_runtime_value_release_strong(ptr %t152)
  %t153 = call i32 @xiao_runtime_error_class()
  %t154 = icmp eq i32 %t153, 0
  br i1 %t154, label %abi.error.ok53, label %abi.error.fail54
abi.error.fail54:
  call i32 @xiao_runtime_error_attach_span(i64 558, i64 590)
  br label %xiao.fn.error
abi.error.ok53:
  store %xiao.value %t150, ptr %t155
  %t157 = insertvalue %xiao.error.location zeroinitializer, i64 552, 0
  %t158 = insertvalue %xiao.error.location %t157, i64 590, 1
  %t159 = insertvalue %xiao.error.location %t158, i8 1, 2
  store %xiao.error.location %t159, ptr %t156
  %t160 = call i32 @xiao_runtime_error_raise_value(ptr %t155, ptr %t156)
  call void @xiao_runtime_value_release_strong(ptr %t155)
  %t161 = icmp eq i32 %t160, 0
  br i1 %t161, label %abi.ok55, label %abi.status.fail56
abi.status.fail56:
  call i32 @xiao_runtime_error_attach_span(i64 552, i64 590)
  br label %xiao.fn.error
abi.ok55:
  br label %xiao.fn.error
dynamic.if.next49:
  br label %dynamic.if.merge50
dynamic.if.merge50:
  store %xiao.value zeroinitializer, ptr %t162
  %t163 = call i32 @xiao_runtime_value_copy(ptr %slot3, ptr %t162)
  %t164 = icmp eq i32 %t163, 0
  br i1 %t164, label %abi.ok59, label %abi.status.fail60
abi.status.fail60:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.selector.fail57
abi.ok59:
  %t165 = load %xiao.value, ptr %t162
  call void @xiao_runtime_value_release_strong(ptr %xiao.generated.site0)
  store %xiao.value %t165, ptr %xiao.generated.site0
  store %xiao.value %t165, ptr %t166
  store %xiao.value zeroinitializer, ptr %t167
  %t168 = call i32 @xiao_runtime_value_iter_get(ptr %t166, i64 0, ptr %t167)
  %t169 = icmp eq i32 %t168, 0
  br i1 %t169, label %abi.ok61, label %abi.status.fail62
abi.status.fail62:
  call i32 @xiao_runtime_error_attach_span(i64 604, i64 613)
  br label %dynamic.selector.fail57
abi.ok61:
  %t170 = load %xiao.value, ptr %t167
  call void @xiao_runtime_value_release_strong(ptr %xiao.generated.site0)
  br label %dynamic.selector.continue58
dynamic.selector.fail57:
  call void @xiao_runtime_value_release_strong(ptr %xiao.generated.site0)
  br label %xiao.fn.error
dynamic.selector.continue58:
  call void @xiao_runtime_value_release_strong(ptr %slot2)
  store %xiao.value %t170, ptr %slot2
  call void @xiao_runtime_value_release_strong(ptr %xiao.generated.site0)
  %t171 = call i32 @xiao_runtime_error_class()
  %t172 = icmp eq i32 %t171, 0
  br i1 %t172, label %abi.error.ok63, label %abi.error.fail64
abi.error.fail64:
  call i32 @xiao_runtime_error_attach_span(i64 595, i64 613)
  br label %xiao.fn.error
abi.error.ok63:
  %t173 = load %xiao.value, ptr %slot3
  %t174 = getelementptr inbounds [9 x i8], ptr @.xiao.bytes80, i64 0, i64 0
  %t175 = insertvalue %xiao.bytes zeroinitializer, ptr %t174, 0
  %t176 = insertvalue %xiao.bytes %t175, i64 8, 1
  store %xiao.bytes %t176, ptr %t177
  store %xiao.value %t173, ptr %t178
  %t179 = call i32 @xiao_runtime_dynamic_check(ptr %t177, ptr %t178)
  %t180 = icmp eq i32 %t179, 0
  br i1 %t180, label %abi.ok67, label %abi.status.fail68
abi.status.fail68:
  call i32 @xiao_runtime_error_attach_span(i64 631, i64 637)
  br label %dynamic.for.fail65
abi.ok67:
  store %xiao.value %t173, ptr %t181
  store i64 0, ptr %t182
  %t183 = call i32 @xiao_runtime_value_iter_len(ptr %t181, ptr %t182)
  %t184 = icmp eq i32 %t183, 0
  br i1 %t184, label %abi.ok69, label %abi.status.fail70
abi.status.fail70:
  call i32 @xiao_runtime_error_attach_span(i64 631, i64 637)
  br label %dynamic.for.fail65
abi.ok69:
  %t185 = load i64, ptr %t182
  store i64 0, ptr %t186
  br label %dynamic.for.cond71
dynamic.for.cond71:
  %t187 = load i64, ptr %t186
  %t188 = icmp ult i64 %t187, %t185
  br i1 %t188, label %dynamic.for.body72, label %dynamic.for.end74
dynamic.for.body72:
  %t189 = load %xiao.value, ptr %slot3
  store %xiao.value %t189, ptr %t190
  store %xiao.value zeroinitializer, ptr %t191
  %t192 = call i32 @xiao_runtime_value_iter_get(ptr %t190, i64 %t187, ptr %t191)
  %t193 = icmp eq i32 %t192, 0
  br i1 %t193, label %abi.ok75, label %abi.status.fail76
abi.status.fail76:
  call i32 @xiao_runtime_error_attach_span(i64 622, i64 627)
  br label %dynamic.for.fail65
abi.ok75:
  %t194 = load %xiao.value, ptr %t191
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  store %xiao.value %t194, ptr %slot1
  store %xiao.value zeroinitializer, ptr %t195
  %t196 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t195)
  %t197 = icmp eq i32 %t196, 0
  br i1 %t197, label %abi.ok77, label %abi.status.fail78
abi.status.fail78:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.for.fail65
abi.ok77:
  %t198 = load %xiao.value, ptr %t195
  store %xiao.value zeroinitializer, ptr %t199
  %t200 = call i32 @xiao_runtime_value_copy(ptr %slot2, ptr %t199)
  %t201 = icmp eq i32 %t200, 0
  br i1 %t201, label %abi.ok80, label %abi.status.fail81
abi.status.fail81:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.binary.left.fail79
abi.ok80:
  %t202 = load %xiao.value, ptr %t199
  %t203 = getelementptr inbounds [3 x i8], ptr @.xiao.bytes81, i64 0, i64 0
  %t204 = insertvalue %xiao.bytes zeroinitializer, ptr %t203, 0
  %t205 = insertvalue %xiao.bytes %t204, i64 2, 1
  store %xiao.bytes %t205, ptr %t206
  store %xiao.value %t198, ptr %t207
  store %xiao.value %t202, ptr %t208
  store %xiao.value zeroinitializer, ptr %t209
  %t210 = call i32 @xiao_runtime_value_binary(ptr %t206, ptr %t207, ptr %t208, ptr %t209)
  %t211 = icmp eq i32 %t210, 0
  br i1 %t211, label %abi.ok84, label %abi.status.fail85
abi.status.fail85:
  call i32 @xiao_runtime_error_attach_span(i64 649, i64 663)
  br label %dynamic.binary.fail82
abi.ok84:
  %t212 = load %xiao.value, ptr %t209
  store %xiao.value %t198, ptr %t213
  call void @xiao_runtime_value_release_strong(ptr %t213)
  store %xiao.value %t202, ptr %t214
  call void @xiao_runtime_value_release_strong(ptr %t214)
  br label %dynamic.binary.continue83
dynamic.binary.fail82:
  store %xiao.value %t202, ptr %t215
  call void @xiao_runtime_value_release_strong(ptr %t215)
  br label %dynamic.binary.left.fail79
dynamic.binary.left.fail79:
  store %xiao.value %t198, ptr %t216
  call void @xiao_runtime_value_release_strong(ptr %t216)
  br label %dynamic.for.fail65
dynamic.binary.continue83:
  %t217 = extractvalue %xiao.value %t212, 0
  %t218 = icmp eq i32 %t217, 1
  br i1 %t218, label %dynamic.bool.ok86, label %dynamic.for.fail65
dynamic.bool.ok86:
  %t219 = extractvalue %xiao.value %t212, 1
  %t220 = trunc i64 %t219 to i1
  store %xiao.value %t212, ptr %t221
  call void @xiao_runtime_value_release_strong(ptr %t221)
  br i1 %t220, label %dynamic.if.then87, label %dynamic.if.next88
dynamic.if.then87:
  store %xiao.value zeroinitializer, ptr %t222
  %t223 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t222)
  %t224 = icmp eq i32 %t223, 0
  br i1 %t224, label %abi.ok90, label %abi.status.fail91
abi.status.fail91:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.for.fail65
abi.ok90:
  %t225 = load %xiao.value, ptr %t222
  call void @xiao_runtime_value_release_strong(ptr %slot2)
  store %xiao.value %t225, ptr %slot2
  br label %dynamic.if.merge89
dynamic.if.next88:
  br label %dynamic.if.merge89
dynamic.if.merge89:
  br label %dynamic.for.advance73
dynamic.for.advance73:
  %t226 = add i64 %t187, 1
  store i64 %t226, ptr %t186
  br label %dynamic.for.cond71
dynamic.for.end74:
  br label %dynamic.for.continue66
dynamic.for.fail65:
  %t227 = call i32 @xiao_runtime_error_class()
  %t228 = icmp eq i32 %t227, 2
  br i1 %t228, label %xiao.fn.error, label %dynamic.error-cleanup.recoverable93
dynamic.error-cleanup.recoverable93:
  br label %dynamic.for.source-cleanup92
dynamic.for.source-cleanup92:
  br label %xiao.fn.error
dynamic.for.continue66:
  store %xiao.value zeroinitializer, ptr %t229
  %t230 = call i32 @xiao_runtime_value_copy(ptr %slot2, ptr %t229)
  %t231 = icmp eq i32 %t230, 0
  br i1 %t231, label %abi.ok94, label %abi.status.fail95
abi.status.fail95:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok94:
  %t232 = load %xiao.value, ptr %t229
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t232, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot3)
  call void @xiao_runtime_value_release_strong(ptr %slot2)
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t233 = call i32 @xiao_runtime_error_class()
  %t234 = icmp eq i32 %t233, 0
  br i1 %t234, label %abi.ok96, label %abi.status.fail97
abi.status.fail97:
  call i32 @xiao_runtime_error_attach_span(i64 340, i64 710)
  br label %xiao.fn.error
abi.ok96:
  %t235 = load ptr, ptr %xiao.return.source
  %t236 = icmp ne ptr %t235, null
  br i1 %t236, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t237 = load i1, ptr %xiao.return.temporary
  br i1 %t237, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t238 = call i32 @xiao_runtime_value_copy(ptr %t235, ptr %xiao.function.result)
  %t239 = icmp eq i32 %t238, 0
  br i1 %t239, label %abi.ok98, label %abi.status.fail99
abi.status.fail99:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok98:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t240 = load %xiao.value, ptr %t235
  store %xiao.value %t240, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t235
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.generated.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t241 = call i32 @xiao_runtime_error_class()
  %t242 = icmp eq i32 %t241, 0
  br i1 %t242, label %abi.ok100, label %abi.status.fail101
abi.status.fail101:
  call i32 @xiao_runtime_error_attach_span(i64 340, i64 710)
  br label %xiao.fn.error
abi.ok100:
  ret void
xiao.fn.error:
  %t243 = call i32 @xiao_runtime_error_class()
  %t244 = icmp eq i32 %t243, 2
  br i1 %t244, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot3)
  call void @xiao_runtime_value_release_strong(ptr %slot2)
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.generated.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t245 = getelementptr inbounds [18 x i8], ptr @.xiao.bytes82, i64 0, i64 0
  %t246 = insertvalue %xiao.bytes zeroinitializer, ptr %t245, 0
  %t247 = insertvalue %xiao.bytes %t246, i64 17, 1
  store %xiao.bytes %t247, ptr %t248
  %t249 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes83, i64 0, i64 0
  %t250 = insertvalue %xiao.bytes zeroinitializer, ptr %t249, 0
  %t251 = insertvalue %xiao.bytes %t250, i64 3, 1
  store %xiao.bytes %t251, ptr %t252
  call void @xiao_runtime_error_push_native_frame(ptr %t248, ptr %t252, i64 340, i64 710)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.afa500df1b39a662(ptr %xiao.function.result, ptr %xiao.arg0) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot0 = alloca %xiao.value
  %slot2 = alloca %xiao.value
  %slot1 = alloca %xiao.value
  %slot3 = alloca %xiao.value
  %t4 = alloca %xiao.value
  %t9 = alloca %xiao.bytes
  %t10 = alloca %xiao.value
  %t13 = alloca %xiao.value
  %t14 = alloca i64
  %t18 = alloca i64
  %t22 = alloca %xiao.value
  %t23 = alloca %xiao.value
  %t27 = alloca %xiao.value
  %t32 = alloca %xiao.value
  %t36 = alloca %xiao.bytes
  %t37 = alloca %xiao.value
  %t38 = alloca %xiao.value
  %t39 = alloca %xiao.value
  %t43 = alloca %xiao.value
  %t44 = alloca %xiao.value
  %t45 = alloca %xiao.value
  %t46 = alloca %xiao.value
  %t47 = alloca %xiao.value
  %t52 = alloca %xiao.value
  %t56 = alloca %xiao.bytes
  %t57 = alloca %xiao.value
  %t58 = alloca %xiao.value
  %t59 = alloca %xiao.value
  %t63 = alloca %xiao.value
  %t64 = alloca %xiao.value
  %t65 = alloca %xiao.value
  %t66 = alloca %xiao.value
  %t71 = alloca %xiao.value
  %t75 = alloca %xiao.bytes
  %t76 = alloca ptr
  %t81 = alloca %xiao.value
  %t82 = alloca %xiao.value
  %t86 = alloca %xiao.bytes
  %t87 = alloca %xiao.error.location
  %t92 = alloca %xiao.value
  %t93 = alloca %xiao.value
  %t96 = alloca %xiao.value
  %t97 = alloca %xiao.error.location
  %t107 = alloca %xiao.value
  %t112 = alloca %xiao.bytes
  %t113 = alloca %xiao.value
  %t116 = alloca %xiao.value
  %t117 = alloca i64
  %t121 = alloca i64
  %t125 = alloca %xiao.value
  %t126 = alloca %xiao.value
  %t130 = alloca %xiao.value
  %t134 = alloca %xiao.value
  %t141 = alloca %xiao.bytes
  %t142 = alloca %xiao.value
  %t143 = alloca %xiao.value
  %t144 = alloca %xiao.value
  %t148 = alloca %xiao.value
  %t149 = alloca %xiao.value
  %t150 = alloca %xiao.value
  %t151 = alloca %xiao.value
  %t155 = alloca %xiao.value
  %t174 = alloca %xiao.bytes
  %t178 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot0
  store %xiao.value zeroinitializer, ptr %slot2
  store %xiao.value zeroinitializer, ptr %slot1
  store %xiao.value zeroinitializer, ptr %slot3
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot3)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = load %xiao.value, ptr %slot3
  store %xiao.value zeroinitializer, ptr %t4
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t4, i64 0)
  %t3 = load %xiao.value, ptr %t4
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  store %xiao.value %t3, ptr %slot0
  %t5 = load %xiao.value, ptr %slot3
  %t6 = getelementptr inbounds [9 x i8], ptr @.xiao.bytes84, i64 0, i64 0
  %t7 = insertvalue %xiao.bytes zeroinitializer, ptr %t6, 0
  %t8 = insertvalue %xiao.bytes %t7, i64 8, 1
  store %xiao.bytes %t8, ptr %t9
  store %xiao.value %t5, ptr %t10
  %t11 = call i32 @xiao_runtime_dynamic_check(ptr %t9, ptr %t10)
  %t12 = icmp eq i32 %t11, 0
  br i1 %t12, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 124, i64 130)
  br label %dynamic.for.fail2
abi.ok4:
  store %xiao.value %t5, ptr %t13
  store i64 0, ptr %t14
  %t15 = call i32 @xiao_runtime_value_iter_len(ptr %t13, ptr %t14)
  %t16 = icmp eq i32 %t15, 0
  br i1 %t16, label %abi.ok6, label %abi.status.fail7
abi.status.fail7:
  call i32 @xiao_runtime_error_attach_span(i64 124, i64 130)
  br label %dynamic.for.fail2
abi.ok6:
  %t17 = load i64, ptr %t14
  store i64 0, ptr %t18
  br label %dynamic.for.cond8
dynamic.for.cond8:
  %t19 = load i64, ptr %t18
  %t20 = icmp ult i64 %t19, %t17
  br i1 %t20, label %dynamic.for.body9, label %dynamic.for.end11
dynamic.for.body9:
  %t21 = load %xiao.value, ptr %slot3
  store %xiao.value %t21, ptr %t22
  store %xiao.value zeroinitializer, ptr %t23
  %t24 = call i32 @xiao_runtime_value_iter_get(ptr %t22, i64 %t19, ptr %t23)
  %t25 = icmp eq i32 %t24, 0
  br i1 %t25, label %abi.ok12, label %abi.status.fail13
abi.status.fail13:
  call i32 @xiao_runtime_error_attach_span(i64 115, i64 120)
  br label %dynamic.for.fail2
abi.ok12:
  %t26 = load %xiao.value, ptr %t23
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  store %xiao.value %t26, ptr %slot1
  store %xiao.value zeroinitializer, ptr %t27
  %t28 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t27)
  %t29 = icmp eq i32 %t28, 0
  br i1 %t29, label %abi.ok14, label %abi.status.fail15
abi.status.fail15:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.for.fail2
abi.ok14:
  %t30 = load %xiao.value, ptr %t27
  store %xiao.value zeroinitializer, ptr %t32
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t32, i64 1)
  %t31 = load %xiao.value, ptr %t32
  %t33 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes85, i64 0, i64 0
  %t34 = insertvalue %xiao.bytes zeroinitializer, ptr %t33, 0
  %t35 = insertvalue %xiao.bytes %t34, i64 3, 1
  store %xiao.bytes %t35, ptr %t36
  store %xiao.value %t30, ptr %t37
  store %xiao.value %t31, ptr %t38
  store %xiao.value zeroinitializer, ptr %t39
  %t40 = call i32 @xiao_runtime_value_binary(ptr %t36, ptr %t37, ptr %t38, ptr %t39)
  %t41 = icmp eq i32 %t40, 0
  br i1 %t41, label %abi.ok19, label %abi.status.fail20
abi.status.fail20:
  call i32 @xiao_runtime_error_attach_span(i64 147, i64 156)
  br label %dynamic.binary.fail17
abi.ok19:
  %t42 = load %xiao.value, ptr %t39
  store %xiao.value %t30, ptr %t43
  call void @xiao_runtime_value_release_strong(ptr %t43)
  store %xiao.value %t31, ptr %t44
  call void @xiao_runtime_value_release_strong(ptr %t44)
  br label %dynamic.binary.continue18
dynamic.binary.fail17:
  store %xiao.value %t31, ptr %t45
  call void @xiao_runtime_value_release_strong(ptr %t45)
  br label %dynamic.binary.left.fail16
dynamic.binary.left.fail16:
  store %xiao.value %t30, ptr %t46
  call void @xiao_runtime_value_release_strong(ptr %t46)
  br label %dynamic.for.fail2
dynamic.binary.continue18:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  store %xiao.value %t42, ptr %slot0
  store %xiao.value zeroinitializer, ptr %t47
  %t48 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t47)
  %t49 = icmp eq i32 %t48, 0
  br i1 %t49, label %abi.ok21, label %abi.status.fail22
abi.status.fail22:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.for.fail2
abi.ok21:
  %t50 = load %xiao.value, ptr %t47
  store %xiao.value zeroinitializer, ptr %t52
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t52, i64 65536)
  %t51 = load %xiao.value, ptr %t52
  %t53 = getelementptr inbounds [3 x i8], ptr @.xiao.bytes86, i64 0, i64 0
  %t54 = insertvalue %xiao.bytes zeroinitializer, ptr %t53, 0
  %t55 = insertvalue %xiao.bytes %t54, i64 2, 1
  store %xiao.bytes %t55, ptr %t56
  store %xiao.value %t50, ptr %t57
  store %xiao.value %t51, ptr %t58
  store %xiao.value zeroinitializer, ptr %t59
  %t60 = call i32 @xiao_runtime_value_binary(ptr %t56, ptr %t57, ptr %t58, ptr %t59)
  %t61 = icmp eq i32 %t60, 0
  br i1 %t61, label %abi.ok26, label %abi.status.fail27
abi.status.fail27:
  call i32 @xiao_runtime_error_attach_span(i64 168, i64 181)
  br label %dynamic.binary.fail24
abi.ok26:
  %t62 = load %xiao.value, ptr %t59
  store %xiao.value %t50, ptr %t63
  call void @xiao_runtime_value_release_strong(ptr %t63)
  store %xiao.value %t51, ptr %t64
  call void @xiao_runtime_value_release_strong(ptr %t64)
  br label %dynamic.binary.continue25
dynamic.binary.fail24:
  store %xiao.value %t51, ptr %t65
  call void @xiao_runtime_value_release_strong(ptr %t65)
  br label %dynamic.binary.left.fail23
dynamic.binary.left.fail23:
  store %xiao.value %t50, ptr %t66
  call void @xiao_runtime_value_release_strong(ptr %t66)
  br label %dynamic.for.fail2
dynamic.binary.continue25:
  %t67 = extractvalue %xiao.value %t62, 0
  %t68 = icmp eq i32 %t67, 1
  br i1 %t68, label %dynamic.bool.ok28, label %dynamic.for.fail2
dynamic.bool.ok28:
  %t69 = extractvalue %xiao.value %t62, 1
  %t70 = trunc i64 %t69 to i1
  store %xiao.value %t62, ptr %t71
  call void @xiao_runtime_value_release_strong(ptr %t71)
  br i1 %t70, label %dynamic.if.then29, label %dynamic.if.next30
dynamic.if.then29:
  %t72 = getelementptr inbounds [26 x i8], ptr @.xiao.str87, i64 0, i64 0
  %t73 = insertvalue %xiao.bytes zeroinitializer, ptr %t72, 0
  %t74 = insertvalue %xiao.bytes %t73, i64 25, 1
  store %xiao.bytes %t74, ptr %t75
  store ptr null, ptr %t76
  %t77 = call i32 @xiao_runtime_string_new(ptr %t75, ptr %t76)
  %t78 = icmp eq i32 %t77, 0
  br i1 %t78, label %abi.ok32, label %abi.status.fail33
abi.status.fail33:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.for.fail2
abi.ok32:
  %t79 = load ptr, ptr %t76
  store %xiao.value zeroinitializer, ptr %t81
  call void @xiao_runtime_value_str_owned(ptr sret(%xiao.value) %t81, ptr %t79)
  %t80 = load %xiao.value, ptr %t81
  store %xiao.value %t80, ptr %t82
  %t83 = getelementptr inbounds [14 x i8], ptr @.xiao.bytes88, i64 0, i64 0
  %t84 = insertvalue %xiao.bytes zeroinitializer, ptr %t83, 0
  %t85 = insertvalue %xiao.bytes %t84, i64 13, 1
  store %xiao.bytes %t85, ptr %t86
  %t88 = insertvalue %xiao.error.location zeroinitializer, i64 200, 0
  %t89 = insertvalue %xiao.error.location %t88, i64 249, 1
  %t90 = insertvalue %xiao.error.location %t89, i8 1, 2
  store %xiao.error.location %t90, ptr %t87
  store %xiao.value zeroinitializer, ptr %t92
  call void @xiao_runtime_error_new_values(ptr sret(%xiao.value) %t92, ptr %t86, ptr %t82, ptr null, ptr %t87)
  %t91 = load %xiao.value, ptr %t92
  store %xiao.value %t80, ptr %t93
  call void @xiao_runtime_value_release_strong(ptr %t93)
  %t94 = call i32 @xiao_runtime_error_class()
  %t95 = icmp eq i32 %t94, 0
  br i1 %t95, label %abi.error.ok34, label %abi.error.fail35
abi.error.fail35:
  call i32 @xiao_runtime_error_attach_span(i64 200, i64 249)
  br label %dynamic.for.fail2
abi.error.ok34:
  store %xiao.value %t91, ptr %t96
  %t98 = insertvalue %xiao.error.location zeroinitializer, i64 194, 0
  %t99 = insertvalue %xiao.error.location %t98, i64 249, 1
  %t100 = insertvalue %xiao.error.location %t99, i8 1, 2
  store %xiao.error.location %t100, ptr %t97
  %t101 = call i32 @xiao_runtime_error_raise_value(ptr %t96, ptr %t97)
  call void @xiao_runtime_value_release_strong(ptr %t96)
  %t102 = icmp eq i32 %t101, 0
  br i1 %t102, label %abi.ok36, label %abi.status.fail37
abi.status.fail37:
  call i32 @xiao_runtime_error_attach_span(i64 194, i64 249)
  br label %dynamic.for.fail2
abi.ok36:
  br label %dynamic.for.fail2
dynamic.if.next30:
  br label %dynamic.if.merge31
dynamic.if.merge31:
  br label %dynamic.for.advance10
dynamic.for.advance10:
  %t103 = add i64 %t19, 1
  store i64 %t103, ptr %t18
  br label %dynamic.for.cond8
dynamic.for.end11:
  br label %dynamic.for.continue3
dynamic.for.fail2:
  %t104 = call i32 @xiao_runtime_error_class()
  %t105 = icmp eq i32 %t104, 2
  br i1 %t105, label %xiao.fn.error, label %dynamic.error-cleanup.recoverable39
dynamic.error-cleanup.recoverable39:
  br label %dynamic.for.source-cleanup38
dynamic.for.source-cleanup38:
  br label %xiao.fn.error
dynamic.for.continue3:
  store %xiao.value zeroinitializer, ptr %t107
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t107, i64 0)
  %t106 = load %xiao.value, ptr %t107
  call void @xiao_runtime_value_release_strong(ptr %slot2)
  store %xiao.value %t106, ptr %slot2
  %t108 = load %xiao.value, ptr %slot3
  %t109 = getelementptr inbounds [9 x i8], ptr @.xiao.bytes89, i64 0, i64 0
  %t110 = insertvalue %xiao.bytes zeroinitializer, ptr %t109, 0
  %t111 = insertvalue %xiao.bytes %t110, i64 8, 1
  store %xiao.bytes %t111, ptr %t112
  store %xiao.value %t108, ptr %t113
  %t114 = call i32 @xiao_runtime_dynamic_check(ptr %t112, ptr %t113)
  %t115 = icmp eq i32 %t114, 0
  br i1 %t115, label %abi.ok42, label %abi.status.fail43
abi.status.fail43:
  call i32 @xiao_runtime_error_attach_span(i64 282, i64 288)
  br label %dynamic.for.fail40
abi.ok42:
  store %xiao.value %t108, ptr %t116
  store i64 0, ptr %t117
  %t118 = call i32 @xiao_runtime_value_iter_len(ptr %t116, ptr %t117)
  %t119 = icmp eq i32 %t118, 0
  br i1 %t119, label %abi.ok44, label %abi.status.fail45
abi.status.fail45:
  call i32 @xiao_runtime_error_attach_span(i64 282, i64 288)
  br label %dynamic.for.fail40
abi.ok44:
  %t120 = load i64, ptr %t117
  store i64 0, ptr %t121
  br label %dynamic.for.cond46
dynamic.for.cond46:
  %t122 = load i64, ptr %t121
  %t123 = icmp ult i64 %t122, %t120
  br i1 %t123, label %dynamic.for.body47, label %dynamic.for.end49
dynamic.for.body47:
  %t124 = load %xiao.value, ptr %slot3
  store %xiao.value %t124, ptr %t125
  store %xiao.value zeroinitializer, ptr %t126
  %t127 = call i32 @xiao_runtime_value_iter_get(ptr %t125, i64 %t122, ptr %t126)
  %t128 = icmp eq i32 %t127, 0
  br i1 %t128, label %abi.ok50, label %abi.status.fail51
abi.status.fail51:
  call i32 @xiao_runtime_error_attach_span(i64 273, i64 278)
  br label %dynamic.for.fail40
abi.ok50:
  %t129 = load %xiao.value, ptr %t126
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  store %xiao.value %t129, ptr %slot1
  store %xiao.value zeroinitializer, ptr %t130
  %t131 = call i32 @xiao_runtime_value_copy(ptr %slot2, ptr %t130)
  %t132 = icmp eq i32 %t131, 0
  br i1 %t132, label %abi.ok52, label %abi.status.fail53
abi.status.fail53:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.for.fail40
abi.ok52:
  %t133 = load %xiao.value, ptr %t130
  store %xiao.value zeroinitializer, ptr %t134
  %t135 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t134)
  %t136 = icmp eq i32 %t135, 0
  br i1 %t136, label %abi.ok55, label %abi.status.fail56
abi.status.fail56:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.binary.left.fail54
abi.ok55:
  %t137 = load %xiao.value, ptr %t134
  %t138 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes90, i64 0, i64 0
  %t139 = insertvalue %xiao.bytes zeroinitializer, ptr %t138, 0
  %t140 = insertvalue %xiao.bytes %t139, i64 3, 1
  store %xiao.bytes %t140, ptr %t141
  store %xiao.value %t133, ptr %t142
  store %xiao.value %t137, ptr %t143
  store %xiao.value zeroinitializer, ptr %t144
  %t145 = call i32 @xiao_runtime_value_binary(ptr %t141, ptr %t142, ptr %t143, ptr %t144)
  %t146 = icmp eq i32 %t145, 0
  br i1 %t146, label %abi.ok59, label %abi.status.fail60
abi.status.fail60:
  call i32 @xiao_runtime_error_attach_span(i64 306, i64 320)
  br label %dynamic.binary.fail57
abi.ok59:
  %t147 = load %xiao.value, ptr %t144
  store %xiao.value %t133, ptr %t148
  call void @xiao_runtime_value_release_strong(ptr %t148)
  store %xiao.value %t137, ptr %t149
  call void @xiao_runtime_value_release_strong(ptr %t149)
  br label %dynamic.binary.continue58
dynamic.binary.fail57:
  store %xiao.value %t137, ptr %t150
  call void @xiao_runtime_value_release_strong(ptr %t150)
  br label %dynamic.binary.left.fail54
dynamic.binary.left.fail54:
  store %xiao.value %t133, ptr %t151
  call void @xiao_runtime_value_release_strong(ptr %t151)
  br label %dynamic.for.fail40
dynamic.binary.continue58:
  call void @xiao_runtime_value_release_strong(ptr %slot2)
  store %xiao.value %t147, ptr %slot2
  br label %dynamic.for.advance48
dynamic.for.advance48:
  %t152 = add i64 %t122, 1
  store i64 %t152, ptr %t121
  br label %dynamic.for.cond46
dynamic.for.end49:
  br label %dynamic.for.continue41
dynamic.for.fail40:
  %t153 = call i32 @xiao_runtime_error_class()
  %t154 = icmp eq i32 %t153, 2
  br i1 %t154, label %xiao.fn.error, label %dynamic.error-cleanup.recoverable62
dynamic.error-cleanup.recoverable62:
  br label %dynamic.for.source-cleanup61
dynamic.for.source-cleanup61:
  br label %xiao.fn.error
dynamic.for.continue41:
  store %xiao.value zeroinitializer, ptr %t155
  %t156 = call i32 @xiao_runtime_value_copy(ptr %slot2, ptr %t155)
  %t157 = icmp eq i32 %t156, 0
  br i1 %t157, label %abi.ok63, label %abi.status.fail64
abi.status.fail64:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok63:
  %t158 = load %xiao.value, ptr %t155
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t158, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot3)
  call void @xiao_runtime_value_release_strong(ptr %slot2)
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t159 = call i32 @xiao_runtime_error_class()
  %t160 = icmp eq i32 %t159, 0
  br i1 %t160, label %abi.ok65, label %abi.status.fail66
abi.status.fail66:
  call i32 @xiao_runtime_error_attach_span(i64 64, i64 340)
  br label %xiao.fn.error
abi.ok65:
  %t161 = load ptr, ptr %xiao.return.source
  %t162 = icmp ne ptr %t161, null
  br i1 %t162, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t163 = load i1, ptr %xiao.return.temporary
  br i1 %t163, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t164 = call i32 @xiao_runtime_value_copy(ptr %t161, ptr %xiao.function.result)
  %t165 = icmp eq i32 %t164, 0
  br i1 %t165, label %abi.ok67, label %abi.status.fail68
abi.status.fail68:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok67:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t166 = load %xiao.value, ptr %t161
  store %xiao.value %t166, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t161
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t167 = call i32 @xiao_runtime_error_class()
  %t168 = icmp eq i32 %t167, 0
  br i1 %t168, label %abi.ok69, label %abi.status.fail70
abi.status.fail70:
  call i32 @xiao_runtime_error_attach_span(i64 64, i64 340)
  br label %xiao.fn.error
abi.ok69:
  ret void
xiao.fn.error:
  %t169 = call i32 @xiao_runtime_error_class()
  %t170 = icmp eq i32 %t169, 2
  br i1 %t170, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot3)
  call void @xiao_runtime_value_release_strong(ptr %slot2)
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t171 = getelementptr inbounds [18 x i8], ptr @.xiao.bytes91, i64 0, i64 0
  %t172 = insertvalue %xiao.bytes zeroinitializer, ptr %t171, 0
  %t173 = insertvalue %xiao.bytes %t172, i64 17, 1
  store %xiao.bytes %t173, ptr %t174
  %t175 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes92, i64 0, i64 0
  %t176 = insertvalue %xiao.bytes zeroinitializer, ptr %t175, 0
  %t177 = insertvalue %xiao.bytes %t176, i64 3, 1
  store %xiao.bytes %t177, ptr %t178
  call void @xiao_runtime_error_push_native_frame(ptr %t174, ptr %t178, i64 64, i64 340)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.9dd21f7ca9c49bf7(ptr %xiao.function.result, ptr %xiao.arg0) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.site1 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot0 = alloca %xiao.value
  %t3 = alloca %xiao.value
  %t8 = alloca %xiao.value
  %t12 = alloca %xiao.bytes
  %t13 = alloca %xiao.value
  %t14 = alloca %xiao.value
  %t15 = alloca %xiao.value
  %t19 = alloca %xiao.value
  %t20 = alloca %xiao.value
  %t21 = alloca %xiao.value
  %t22 = alloca %xiao.value
  %t27 = alloca %xiao.value
  %t28 = alloca %xiao.value
  %t35 = alloca %xiao.bytes
  %t36 = alloca %xiao.value
  %t37 = alloca %xiao.value
  %t41 = alloca %xiao.value
  %t42 = alloca %xiao.value
  %t43 = alloca %xiao.value
  %t62 = alloca %xiao.bytes
  %t66 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store %xiao.value zeroinitializer, ptr %xiao.return.site1
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot0
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot0)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = load %xiao.value, ptr %slot0
  store %xiao.value zeroinitializer, ptr %t3
  %t4 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t3)
  %t5 = icmp eq i32 %t4, 0
  br i1 %t5, label %abi.ok2, label %abi.status.fail3
abi.status.fail3:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok2:
  %t6 = load %xiao.value, ptr %t3
  store %xiao.value zeroinitializer, ptr %t8
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t8, i64 0)
  %t7 = load %xiao.value, ptr %t8
  %t9 = getelementptr inbounds [3 x i8], ptr @.xiao.bytes93, i64 0, i64 0
  %t10 = insertvalue %xiao.bytes zeroinitializer, ptr %t9, 0
  %t11 = insertvalue %xiao.bytes %t10, i64 2, 1
  store %xiao.bytes %t11, ptr %t12
  store %xiao.value %t6, ptr %t13
  store %xiao.value %t7, ptr %t14
  store %xiao.value zeroinitializer, ptr %t15
  %t16 = call i32 @xiao_runtime_value_binary(ptr %t12, ptr %t13, ptr %t14, ptr %t15)
  %t17 = icmp eq i32 %t16, 0
  br i1 %t17, label %abi.ok7, label %abi.status.fail8
abi.status.fail8:
  call i32 @xiao_runtime_error_attach_span(i64 109, i64 118)
  br label %dynamic.binary.fail5
abi.ok7:
  %t18 = load %xiao.value, ptr %t15
  store %xiao.value %t6, ptr %t19
  call void @xiao_runtime_value_release_strong(ptr %t19)
  store %xiao.value %t7, ptr %t20
  call void @xiao_runtime_value_release_strong(ptr %t20)
  br label %dynamic.binary.continue6
dynamic.binary.fail5:
  store %xiao.value %t7, ptr %t21
  call void @xiao_runtime_value_release_strong(ptr %t21)
  br label %dynamic.binary.left.fail4
dynamic.binary.left.fail4:
  store %xiao.value %t6, ptr %t22
  call void @xiao_runtime_value_release_strong(ptr %t22)
  br label %xiao.fn.error
dynamic.binary.continue6:
  %t23 = extractvalue %xiao.value %t18, 0
  %t24 = icmp eq i32 %t23, 1
  br i1 %t24, label %dynamic.bool.ok9, label %xiao.fn.error
dynamic.bool.ok9:
  %t25 = extractvalue %xiao.value %t18, 1
  %t26 = trunc i64 %t25 to i1
  store %xiao.value %t18, ptr %t27
  call void @xiao_runtime_value_release_strong(ptr %t27)
  br i1 %t26, label %dynamic.if.then10, label %dynamic.if.next11
dynamic.if.then10:
  store %xiao.value zeroinitializer, ptr %t28
  %t29 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t28)
  %t30 = icmp eq i32 %t29, 0
  br i1 %t30, label %abi.ok13, label %abi.status.fail14
abi.status.fail14:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok13:
  %t31 = load %xiao.value, ptr %t28
  %t32 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes94, i64 0, i64 0
  %t33 = insertvalue %xiao.bytes zeroinitializer, ptr %t32, 0
  %t34 = insertvalue %xiao.bytes %t33, i64 5, 1
  store %xiao.bytes %t34, ptr %t35
  store %xiao.value %t31, ptr %t36
  store %xiao.value zeroinitializer, ptr %t37
  %t38 = call i32 @xiao_runtime_value_unary(ptr %t35, ptr %t36, ptr %t37)
  %t39 = icmp eq i32 %t38, 0
  br i1 %t39, label %abi.ok17, label %abi.status.fail18
abi.status.fail18:
  call i32 @xiao_runtime_error_attach_span(i64 134, i64 140)
  br label %dynamic.unary.fail15
abi.ok17:
  %t40 = load %xiao.value, ptr %t37
  store %xiao.value %t31, ptr %t41
  call void @xiao_runtime_value_release_strong(ptr %t41)
  br label %dynamic.unary.continue16
dynamic.unary.fail15:
  store %xiao.value %t31, ptr %t42
  call void @xiao_runtime_value_release_strong(ptr %t42)
  br label %xiao.fn.error
dynamic.unary.continue16:
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t40, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 true, ptr %xiao.return.temporary
  br label %xiao.fn.return
dynamic.if.next11:
  br label %dynamic.if.merge12
dynamic.if.merge12:
  store %xiao.value zeroinitializer, ptr %t43
  %t44 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t43)
  %t45 = icmp eq i32 %t44, 0
  br i1 %t45, label %abi.ok19, label %abi.status.fail20
abi.status.fail20:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok19:
  %t46 = load %xiao.value, ptr %t43
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site1)
  store %xiao.value %t46, ptr %xiao.return.site1
  store ptr %xiao.return.site1, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t47 = call i32 @xiao_runtime_error_class()
  %t48 = icmp eq i32 %t47, 0
  br i1 %t48, label %abi.ok21, label %abi.status.fail22
abi.status.fail22:
  call i32 @xiao_runtime_error_attach_span(i64 76, i64 159)
  br label %xiao.fn.error
abi.ok21:
  %t49 = load ptr, ptr %xiao.return.source
  %t50 = icmp ne ptr %t49, null
  br i1 %t50, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t51 = load i1, ptr %xiao.return.temporary
  br i1 %t51, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t52 = call i32 @xiao_runtime_value_copy(ptr %t49, ptr %xiao.function.result)
  %t53 = icmp eq i32 %t52, 0
  br i1 %t53, label %abi.ok23, label %abi.status.fail24
abi.status.fail24:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok23:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t54 = load %xiao.value, ptr %t49
  store %xiao.value %t54, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t49
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site1)
  %t55 = call i32 @xiao_runtime_error_class()
  %t56 = icmp eq i32 %t55, 0
  br i1 %t56, label %abi.ok25, label %abi.status.fail26
abi.status.fail26:
  call i32 @xiao_runtime_error_attach_span(i64 76, i64 159)
  br label %xiao.fn.error
abi.ok25:
  ret void
xiao.fn.error:
  %t57 = call i32 @xiao_runtime_error_class()
  %t58 = icmp eq i32 %t57, 2
  br i1 %t58, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site1)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t59 = getelementptr inbounds [12 x i8], ptr @.xiao.bytes95, i64 0, i64 0
  %t60 = insertvalue %xiao.bytes zeroinitializer, ptr %t59, 0
  %t61 = insertvalue %xiao.bytes %t60, i64 11, 1
  store %xiao.bytes %t61, ptr %t62
  %t63 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes96, i64 0, i64 0
  %t64 = insertvalue %xiao.bytes zeroinitializer, ptr %t63, 0
  %t65 = insertvalue %xiao.bytes %t64, i64 3, 1
  store %xiao.bytes %t65, ptr %t66
  call void @xiao_runtime_error_push_native_frame(ptr %t62, ptr %t66, i64 76, i64 159)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.21a25a7a28a1c17b(ptr %xiao.function.result, ptr %xiao.arg0, ptr %xiao.arg1) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.site1 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot0 = alloca %xiao.value
  %slot1 = alloca %xiao.value
  %t6 = alloca %xiao.value
  %t10 = alloca %xiao.value
  %t17 = alloca %xiao.bytes
  %t18 = alloca %xiao.value
  %t19 = alloca %xiao.value
  %t20 = alloca %xiao.value
  %t24 = alloca %xiao.value
  %t25 = alloca %xiao.value
  %t26 = alloca %xiao.value
  %t27 = alloca %xiao.value
  %t32 = alloca %xiao.value
  %t33 = alloca %xiao.value
  %t37 = alloca %xiao.value
  %t56 = alloca %xiao.bytes
  %t60 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store %xiao.value zeroinitializer, ptr %xiao.return.site1
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot0
  store %xiao.value zeroinitializer, ptr %slot1
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot0)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg1, ptr %slot1)
  %t3 = icmp eq i32 %t2, 0
  br i1 %t3, label %abi.ok2, label %abi.status.fail3
abi.status.fail3:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok2:
  %t4 = load %xiao.value, ptr %slot0
  %t5 = load %xiao.value, ptr %slot1
  store %xiao.value zeroinitializer, ptr %t6
  %t7 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t6)
  %t8 = icmp eq i32 %t7, 0
  br i1 %t8, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok4:
  %t9 = load %xiao.value, ptr %t6
  store %xiao.value zeroinitializer, ptr %t10
  %t11 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t10)
  %t12 = icmp eq i32 %t11, 0
  br i1 %t12, label %abi.ok7, label %abi.status.fail8
abi.status.fail8:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.binary.left.fail6
abi.ok7:
  %t13 = load %xiao.value, ptr %t10
  %t14 = getelementptr inbounds [3 x i8], ptr @.xiao.bytes97, i64 0, i64 0
  %t15 = insertvalue %xiao.bytes zeroinitializer, ptr %t14, 0
  %t16 = insertvalue %xiao.bytes %t15, i64 2, 1
  store %xiao.bytes %t16, ptr %t17
  store %xiao.value %t9, ptr %t18
  store %xiao.value %t13, ptr %t19
  store %xiao.value zeroinitializer, ptr %t20
  %t21 = call i32 @xiao_runtime_value_binary(ptr %t17, ptr %t18, ptr %t19, ptr %t20)
  %t22 = icmp eq i32 %t21, 0
  br i1 %t22, label %abi.ok11, label %abi.status.fail12
abi.status.fail12:
  call i32 @xiao_runtime_error_attach_span(i64 296, i64 308)
  br label %dynamic.binary.fail9
abi.ok11:
  %t23 = load %xiao.value, ptr %t20
  store %xiao.value %t9, ptr %t24
  call void @xiao_runtime_value_release_strong(ptr %t24)
  store %xiao.value %t13, ptr %t25
  call void @xiao_runtime_value_release_strong(ptr %t25)
  br label %dynamic.binary.continue10
dynamic.binary.fail9:
  store %xiao.value %t13, ptr %t26
  call void @xiao_runtime_value_release_strong(ptr %t26)
  br label %dynamic.binary.left.fail6
dynamic.binary.left.fail6:
  store %xiao.value %t9, ptr %t27
  call void @xiao_runtime_value_release_strong(ptr %t27)
  br label %xiao.fn.error
dynamic.binary.continue10:
  %t28 = extractvalue %xiao.value %t23, 0
  %t29 = icmp eq i32 %t28, 1
  br i1 %t29, label %dynamic.bool.ok13, label %xiao.fn.error
dynamic.bool.ok13:
  %t30 = extractvalue %xiao.value %t23, 1
  %t31 = trunc i64 %t30 to i1
  store %xiao.value %t23, ptr %t32
  call void @xiao_runtime_value_release_strong(ptr %t32)
  br i1 %t31, label %dynamic.if.then14, label %dynamic.if.next15
dynamic.if.then14:
  store %xiao.value zeroinitializer, ptr %t33
  %t34 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t33)
  %t35 = icmp eq i32 %t34, 0
  br i1 %t35, label %abi.ok17, label %abi.status.fail18
abi.status.fail18:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok17:
  %t36 = load %xiao.value, ptr %t33
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t36, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  br label %xiao.fn.return
dynamic.if.next15:
  br label %dynamic.if.merge16
dynamic.if.merge16:
  store %xiao.value zeroinitializer, ptr %t37
  %t38 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t37)
  %t39 = icmp eq i32 %t38, 0
  br i1 %t39, label %abi.ok19, label %abi.status.fail20
abi.status.fail20:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok19:
  %t40 = load %xiao.value, ptr %t37
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site1)
  store %xiao.value %t40, ptr %xiao.return.site1
  store ptr %xiao.return.site1, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t41 = call i32 @xiao_runtime_error_class()
  %t42 = icmp eq i32 %t41, 0
  br i1 %t42, label %abi.ok21, label %abi.status.fail22
abi.status.fail22:
  call i32 @xiao_runtime_error_attach_span(i64 253, i64 347)
  br label %xiao.fn.error
abi.ok21:
  %t43 = load ptr, ptr %xiao.return.source
  %t44 = icmp ne ptr %t43, null
  br i1 %t44, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t45 = load i1, ptr %xiao.return.temporary
  br i1 %t45, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t46 = call i32 @xiao_runtime_value_copy(ptr %t43, ptr %xiao.function.result)
  %t47 = icmp eq i32 %t46, 0
  br i1 %t47, label %abi.ok23, label %abi.status.fail24
abi.status.fail24:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok23:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t48 = load %xiao.value, ptr %t43
  store %xiao.value %t48, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t43
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site1)
  %t49 = call i32 @xiao_runtime_error_class()
  %t50 = icmp eq i32 %t49, 0
  br i1 %t50, label %abi.ok25, label %abi.status.fail26
abi.status.fail26:
  call i32 @xiao_runtime_error_attach_span(i64 253, i64 347)
  br label %xiao.fn.error
abi.ok25:
  ret void
xiao.fn.error:
  %t51 = call i32 @xiao_runtime_error_class()
  %t52 = icmp eq i32 %t51, 2
  br i1 %t52, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site1)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t53 = getelementptr inbounds [12 x i8], ptr @.xiao.bytes98, i64 0, i64 0
  %t54 = insertvalue %xiao.bytes zeroinitializer, ptr %t53, 0
  %t55 = insertvalue %xiao.bytes %t54, i64 11, 1
  store %xiao.bytes %t55, ptr %t56
  %t57 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes99, i64 0, i64 0
  %t58 = insertvalue %xiao.bytes zeroinitializer, ptr %t57, 0
  %t59 = insertvalue %xiao.bytes %t58, i64 3, 1
  store %xiao.bytes %t59, ptr %t60
  call void @xiao_runtime_error_push_native_frame(ptr %t56, ptr %t60, i64 253, i64 347)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.f44171bb2a26d5df(ptr %xiao.function.result, ptr %xiao.arg0, ptr %xiao.arg1) {
entry:
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot3 = alloca %xiao.value
  %slot4 = alloca %xiao.value
  %slot1 = alloca %xiao.value
  %slot2 = alloca %xiao.value
  %slot0 = alloca %xiao.value
  %t6 = alloca %xiao.value
  %t11 = alloca %xiao.value
  %t15 = alloca %xiao.bytes
  %t16 = alloca %xiao.value
  %t17 = alloca %xiao.value
  %t18 = alloca %xiao.value
  %t22 = alloca %xiao.value
  %t23 = alloca %xiao.value
  %t24 = alloca %xiao.value
  %t25 = alloca %xiao.value
  %t30 = alloca %xiao.value
  %t34 = alloca %xiao.bytes
  %t35 = alloca ptr
  %t40 = alloca %xiao.value
  %t41 = alloca %xiao.value
  %t45 = alloca %xiao.bytes
  %t46 = alloca %xiao.error.location
  %t51 = alloca %xiao.value
  %t52 = alloca %xiao.value
  %t55 = alloca %xiao.value
  %t56 = alloca %xiao.error.location
  %t63 = alloca %xiao.value
  %t64 = alloca %xiao.value
  %t68 = alloca %xiao.value
  %t72 = alloca %xiao.value
  %t77 = alloca %xiao.value
  %t81 = alloca %xiao.bytes
  %t82 = alloca %xiao.value
  %t83 = alloca %xiao.value
  %t84 = alloca %xiao.value
  %t88 = alloca %xiao.value
  %t89 = alloca %xiao.value
  %t90 = alloca %xiao.value
  %t91 = alloca %xiao.value
  %t96 = alloca %xiao.value
  %t97 = alloca %xiao.value
  %t102 = alloca %xiao.value
  %t106 = alloca %xiao.bytes
  %t107 = alloca %xiao.value
  %t108 = alloca %xiao.value
  %t109 = alloca %xiao.value
  %t113 = alloca %xiao.value
  %t114 = alloca %xiao.value
  %t115 = alloca %xiao.value
  %t116 = alloca %xiao.value
  %t118 = alloca %xiao.value
  %t122 = alloca %xiao.bytes
  %t123 = alloca %xiao.value
  %t124 = alloca %xiao.value
  %t125 = alloca %xiao.value
  %t129 = alloca %xiao.value
  %t130 = alloca %xiao.value
  %t131 = alloca %xiao.value
  %t132 = alloca %xiao.value
  %t137 = alloca %xiao.value
  %t138 = alloca %xiao.value
  %t142 = alloca %xiao.value
  %t149 = alloca %xiao.bytes
  %t150 = alloca %xiao.value
  %t151 = alloca %xiao.value
  %t152 = alloca %xiao.value
  %t156 = alloca %xiao.value
  %t157 = alloca %xiao.value
  %t158 = alloca %xiao.value
  %t159 = alloca %xiao.value
  %t160 = alloca %xiao.value
  %t165 = alloca %xiao.value
  %t169 = alloca %xiao.bytes
  %t170 = alloca %xiao.value
  %t171 = alloca %xiao.value
  %t172 = alloca %xiao.value
  %t176 = alloca %xiao.value
  %t177 = alloca %xiao.value
  %t178 = alloca %xiao.value
  %t179 = alloca %xiao.value
  %t180 = alloca %xiao.value
  %t185 = alloca %xiao.value
  %t189 = alloca %xiao.bytes
  %t190 = alloca %xiao.value
  %t191 = alloca %xiao.value
  %t192 = alloca %xiao.value
  %t196 = alloca %xiao.value
  %t197 = alloca %xiao.value
  %t198 = alloca %xiao.value
  %t199 = alloca %xiao.value
  %t204 = alloca %xiao.value
  %t205 = alloca %xiao.value
  %t209 = alloca %xiao.value
  %t216 = alloca %xiao.bytes
  %t217 = alloca %xiao.value
  %t218 = alloca %xiao.value
  %t219 = alloca %xiao.value
  %t223 = alloca %xiao.value
  %t224 = alloca %xiao.value
  %t225 = alloca %xiao.value
  %t226 = alloca %xiao.value
  %t227 = alloca %xiao.value
  %t246 = alloca %xiao.bytes
  %t250 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot3
  store %xiao.value zeroinitializer, ptr %slot4
  store %xiao.value zeroinitializer, ptr %slot1
  store %xiao.value zeroinitializer, ptr %slot2
  store %xiao.value zeroinitializer, ptr %slot0
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot3)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg1, ptr %slot4)
  %t3 = icmp eq i32 %t2, 0
  br i1 %t3, label %abi.ok2, label %abi.status.fail3
abi.status.fail3:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok2:
  %t4 = load %xiao.value, ptr %slot3
  %t5 = load %xiao.value, ptr %slot4
  store %xiao.value zeroinitializer, ptr %t6
  %t7 = call i32 @xiao_runtime_value_copy(ptr %slot4, ptr %t6)
  %t8 = icmp eq i32 %t7, 0
  br i1 %t8, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok4:
  %t9 = load %xiao.value, ptr %t6
  store %xiao.value zeroinitializer, ptr %t11
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t11, i64 0)
  %t10 = load %xiao.value, ptr %t11
  %t12 = getelementptr inbounds [3 x i8], ptr @.xiao.bytes100, i64 0, i64 0
  %t13 = insertvalue %xiao.bytes zeroinitializer, ptr %t12, 0
  %t14 = insertvalue %xiao.bytes %t13, i64 2, 1
  store %xiao.bytes %t14, ptr %t15
  store %xiao.value %t9, ptr %t16
  store %xiao.value %t10, ptr %t17
  store %xiao.value zeroinitializer, ptr %t18
  %t19 = call i32 @xiao_runtime_value_binary(ptr %t15, ptr %t16, ptr %t17, ptr %t18)
  %t20 = icmp eq i32 %t19, 0
  br i1 %t20, label %abi.ok9, label %abi.status.fail10
abi.status.fail10:
  call i32 @xiao_runtime_error_attach_span(i64 1056, i64 1068)
  br label %dynamic.binary.fail7
abi.ok9:
  %t21 = load %xiao.value, ptr %t18
  store %xiao.value %t9, ptr %t22
  call void @xiao_runtime_value_release_strong(ptr %t22)
  store %xiao.value %t10, ptr %t23
  call void @xiao_runtime_value_release_strong(ptr %t23)
  br label %dynamic.binary.continue8
dynamic.binary.fail7:
  store %xiao.value %t10, ptr %t24
  call void @xiao_runtime_value_release_strong(ptr %t24)
  br label %dynamic.binary.left.fail6
dynamic.binary.left.fail6:
  store %xiao.value %t9, ptr %t25
  call void @xiao_runtime_value_release_strong(ptr %t25)
  br label %xiao.fn.error
dynamic.binary.continue8:
  %t26 = extractvalue %xiao.value %t21, 0
  %t27 = icmp eq i32 %t26, 1
  br i1 %t27, label %dynamic.bool.ok11, label %xiao.fn.error
dynamic.bool.ok11:
  %t28 = extractvalue %xiao.value %t21, 1
  %t29 = trunc i64 %t28 to i1
  store %xiao.value %t21, ptr %t30
  call void @xiao_runtime_value_release_strong(ptr %t30)
  br i1 %t29, label %dynamic.if.then12, label %dynamic.if.next13
dynamic.if.then12:
  %t31 = getelementptr inbounds [23 x i8], ptr @.xiao.str101, i64 0, i64 0
  %t32 = insertvalue %xiao.bytes zeroinitializer, ptr %t31, 0
  %t33 = insertvalue %xiao.bytes %t32, i64 22, 1
  store %xiao.bytes %t33, ptr %t34
  store ptr null, ptr %t35
  %t36 = call i32 @xiao_runtime_string_new(ptr %t34, ptr %t35)
  %t37 = icmp eq i32 %t36, 0
  br i1 %t37, label %abi.ok15, label %abi.status.fail16
abi.status.fail16:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok15:
  %t38 = load ptr, ptr %t35
  store %xiao.value zeroinitializer, ptr %t40
  call void @xiao_runtime_value_str_owned(ptr sret(%xiao.value) %t40, ptr %t38)
  %t39 = load %xiao.value, ptr %t40
  store %xiao.value %t39, ptr %t41
  %t42 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes102, i64 0, i64 0
  %t43 = insertvalue %xiao.bytes zeroinitializer, ptr %t42, 0
  %t44 = insertvalue %xiao.bytes %t43, i64 5, 1
  store %xiao.bytes %t44, ptr %t45
  %t47 = insertvalue %xiao.error.location zeroinitializer, i64 1083, 0
  %t48 = insertvalue %xiao.error.location %t47, i64 1121, 1
  %t49 = insertvalue %xiao.error.location %t48, i8 1, 2
  store %xiao.error.location %t49, ptr %t46
  store %xiao.value zeroinitializer, ptr %t51
  call void @xiao_runtime_error_new_values(ptr sret(%xiao.value) %t51, ptr %t45, ptr %t41, ptr null, ptr %t46)
  %t50 = load %xiao.value, ptr %t51
  store %xiao.value %t39, ptr %t52
  call void @xiao_runtime_value_release_strong(ptr %t52)
  %t53 = call i32 @xiao_runtime_error_class()
  %t54 = icmp eq i32 %t53, 0
  br i1 %t54, label %abi.error.ok17, label %abi.error.fail18
abi.error.fail18:
  call i32 @xiao_runtime_error_attach_span(i64 1083, i64 1121)
  br label %xiao.fn.error
abi.error.ok17:
  store %xiao.value %t50, ptr %t55
  %t57 = insertvalue %xiao.error.location zeroinitializer, i64 1077, 0
  %t58 = insertvalue %xiao.error.location %t57, i64 1121, 1
  %t59 = insertvalue %xiao.error.location %t58, i8 1, 2
  store %xiao.error.location %t59, ptr %t56
  %t60 = call i32 @xiao_runtime_error_raise_value(ptr %t55, ptr %t56)
  call void @xiao_runtime_value_release_strong(ptr %t55)
  %t61 = icmp eq i32 %t60, 0
  br i1 %t61, label %abi.ok19, label %abi.status.fail20
abi.status.fail20:
  call i32 @xiao_runtime_error_attach_span(i64 1077, i64 1121)
  br label %xiao.fn.error
abi.ok19:
  br label %xiao.fn.error
dynamic.if.next13:
  br label %dynamic.if.merge14
dynamic.if.merge14:
  store %xiao.value zeroinitializer, ptr %t63
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t63, i64 1)
  %t62 = load %xiao.value, ptr %t63
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  store %xiao.value %t62, ptr %slot0
  store %xiao.value zeroinitializer, ptr %t64
  %t65 = call i32 @xiao_runtime_value_copy(ptr %slot3, ptr %t64)
  %t66 = icmp eq i32 %t65, 0
  br i1 %t66, label %abi.ok21, label %abi.status.fail22
abi.status.fail22:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok21:
  %t67 = load %xiao.value, ptr %t64
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  store %xiao.value %t67, ptr %slot1
  store %xiao.value zeroinitializer, ptr %t68
  %t69 = call i32 @xiao_runtime_value_copy(ptr %slot4, ptr %t68)
  %t70 = icmp eq i32 %t69, 0
  br i1 %t70, label %abi.ok23, label %abi.status.fail24
abi.status.fail24:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok23:
  %t71 = load %xiao.value, ptr %t68
  call void @xiao_runtime_value_release_strong(ptr %slot2)
  store %xiao.value %t71, ptr %slot2
  br label %dynamic.while.cond25
dynamic.while.cond25:
  store %xiao.value zeroinitializer, ptr %t72
  %t73 = call i32 @xiao_runtime_value_copy(ptr %slot2, ptr %t72)
  %t74 = icmp eq i32 %t73, 0
  br i1 %t74, label %abi.ok28, label %abi.status.fail29
abi.status.fail29:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok28:
  %t75 = load %xiao.value, ptr %t72
  store %xiao.value zeroinitializer, ptr %t77
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t77, i64 0)
  %t76 = load %xiao.value, ptr %t77
  %t78 = getelementptr inbounds [3 x i8], ptr @.xiao.bytes103, i64 0, i64 0
  %t79 = insertvalue %xiao.bytes zeroinitializer, ptr %t78, 0
  %t80 = insertvalue %xiao.bytes %t79, i64 2, 1
  store %xiao.bytes %t80, ptr %t81
  store %xiao.value %t75, ptr %t82
  store %xiao.value %t76, ptr %t83
  store %xiao.value zeroinitializer, ptr %t84
  %t85 = call i32 @xiao_runtime_value_binary(ptr %t81, ptr %t82, ptr %t83, ptr %t84)
  %t86 = icmp eq i32 %t85, 0
  br i1 %t86, label %abi.ok33, label %abi.status.fail34
abi.status.fail34:
  call i32 @xiao_runtime_error_attach_span(i64 1190, i64 1203)
  br label %dynamic.binary.fail31
abi.ok33:
  %t87 = load %xiao.value, ptr %t84
  store %xiao.value %t75, ptr %t88
  call void @xiao_runtime_value_release_strong(ptr %t88)
  store %xiao.value %t76, ptr %t89
  call void @xiao_runtime_value_release_strong(ptr %t89)
  br label %dynamic.binary.continue32
dynamic.binary.fail31:
  store %xiao.value %t76, ptr %t90
  call void @xiao_runtime_value_release_strong(ptr %t90)
  br label %dynamic.binary.left.fail30
dynamic.binary.left.fail30:
  store %xiao.value %t75, ptr %t91
  call void @xiao_runtime_value_release_strong(ptr %t91)
  br label %xiao.fn.error
dynamic.binary.continue32:
  %t92 = extractvalue %xiao.value %t87, 0
  %t93 = icmp eq i32 %t92, 1
  br i1 %t93, label %dynamic.bool.ok35, label %xiao.fn.error
dynamic.bool.ok35:
  %t94 = extractvalue %xiao.value %t87, 1
  %t95 = trunc i64 %t94 to i1
  store %xiao.value %t87, ptr %t96
  call void @xiao_runtime_value_release_strong(ptr %t96)
  br i1 %t95, label %dynamic.while.body26, label %dynamic.while.end27
dynamic.while.body26:
  store %xiao.value zeroinitializer, ptr %t97
  %t98 = call i32 @xiao_runtime_value_copy(ptr %slot2, ptr %t97)
  %t99 = icmp eq i32 %t98, 0
  br i1 %t99, label %abi.ok36, label %abi.status.fail37
abi.status.fail37:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok36:
  %t100 = load %xiao.value, ptr %t97
  store %xiao.value zeroinitializer, ptr %t102
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t102, i64 2)
  %t101 = load %xiao.value, ptr %t102
  %t103 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes104, i64 0, i64 0
  %t104 = insertvalue %xiao.bytes zeroinitializer, ptr %t103, 0
  %t105 = insertvalue %xiao.bytes %t104, i64 3, 1
  store %xiao.bytes %t105, ptr %t106
  store %xiao.value %t100, ptr %t107
  store %xiao.value %t101, ptr %t108
  store %xiao.value zeroinitializer, ptr %t109
  %t110 = call i32 @xiao_runtime_value_binary(ptr %t106, ptr %t107, ptr %t108, ptr %t109)
  %t111 = icmp eq i32 %t110, 0
  br i1 %t111, label %abi.ok41, label %abi.status.fail42
abi.status.fail42:
  call i32 @xiao_runtime_error_attach_span(i64 1215, i64 1228)
  br label %dynamic.binary.fail39
abi.ok41:
  %t112 = load %xiao.value, ptr %t109
  store %xiao.value %t100, ptr %t113
  call void @xiao_runtime_value_release_strong(ptr %t113)
  store %xiao.value %t101, ptr %t114
  call void @xiao_runtime_value_release_strong(ptr %t114)
  br label %dynamic.binary.continue40
dynamic.binary.fail39:
  store %xiao.value %t101, ptr %t115
  call void @xiao_runtime_value_release_strong(ptr %t115)
  br label %dynamic.binary.left.fail38
dynamic.binary.left.fail38:
  store %xiao.value %t100, ptr %t116
  call void @xiao_runtime_value_release_strong(ptr %t116)
  br label %xiao.fn.error
dynamic.binary.continue40:
  store %xiao.value zeroinitializer, ptr %t118
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t118, i64 1)
  %t117 = load %xiao.value, ptr %t118
  %t119 = getelementptr inbounds [3 x i8], ptr @.xiao.bytes105, i64 0, i64 0
  %t120 = insertvalue %xiao.bytes zeroinitializer, ptr %t119, 0
  %t121 = insertvalue %xiao.bytes %t120, i64 2, 1
  store %xiao.bytes %t121, ptr %t122
  store %xiao.value %t112, ptr %t123
  store %xiao.value %t117, ptr %t124
  store %xiao.value zeroinitializer, ptr %t125
  %t126 = call i32 @xiao_runtime_value_binary(ptr %t122, ptr %t123, ptr %t124, ptr %t125)
  %t127 = icmp eq i32 %t126, 0
  br i1 %t127, label %abi.ok46, label %abi.status.fail47
abi.status.fail47:
  call i32 @xiao_runtime_error_attach_span(i64 1215, i64 1233)
  br label %dynamic.binary.fail44
abi.ok46:
  %t128 = load %xiao.value, ptr %t125
  store %xiao.value %t112, ptr %t129
  call void @xiao_runtime_value_release_strong(ptr %t129)
  store %xiao.value %t117, ptr %t130
  call void @xiao_runtime_value_release_strong(ptr %t130)
  br label %dynamic.binary.continue45
dynamic.binary.fail44:
  store %xiao.value %t117, ptr %t131
  call void @xiao_runtime_value_release_strong(ptr %t131)
  br label %dynamic.binary.left.fail43
dynamic.binary.left.fail43:
  store %xiao.value %t112, ptr %t132
  call void @xiao_runtime_value_release_strong(ptr %t132)
  br label %xiao.fn.error
dynamic.binary.continue45:
  %t133 = extractvalue %xiao.value %t128, 0
  %t134 = icmp eq i32 %t133, 1
  br i1 %t134, label %dynamic.bool.ok48, label %xiao.fn.error
dynamic.bool.ok48:
  %t135 = extractvalue %xiao.value %t128, 1
  %t136 = trunc i64 %t135 to i1
  store %xiao.value %t128, ptr %t137
  call void @xiao_runtime_value_release_strong(ptr %t137)
  br i1 %t136, label %dynamic.if.then49, label %dynamic.if.next50
dynamic.if.then49:
  store %xiao.value zeroinitializer, ptr %t138
  %t139 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t138)
  %t140 = icmp eq i32 %t139, 0
  br i1 %t140, label %abi.ok52, label %abi.status.fail53
abi.status.fail53:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok52:
  %t141 = load %xiao.value, ptr %t138
  store %xiao.value zeroinitializer, ptr %t142
  %t143 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t142)
  %t144 = icmp eq i32 %t143, 0
  br i1 %t144, label %abi.ok55, label %abi.status.fail56
abi.status.fail56:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.binary.left.fail54
abi.ok55:
  %t145 = load %xiao.value, ptr %t142
  %t146 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes106, i64 0, i64 0
  %t147 = insertvalue %xiao.bytes zeroinitializer, ptr %t146, 0
  %t148 = insertvalue %xiao.bytes %t147, i64 3, 1
  store %xiao.bytes %t148, ptr %t149
  store %xiao.value %t141, ptr %t150
  store %xiao.value %t145, ptr %t151
  store %xiao.value zeroinitializer, ptr %t152
  %t153 = call i32 @xiao_runtime_value_binary(ptr %t149, ptr %t150, ptr %t151, ptr %t152)
  %t154 = icmp eq i32 %t153, 0
  br i1 %t154, label %abi.ok59, label %abi.status.fail60
abi.status.fail60:
  call i32 @xiao_runtime_error_attach_span(i64 1255, i64 1270)
  br label %dynamic.binary.fail57
abi.ok59:
  %t155 = load %xiao.value, ptr %t152
  store %xiao.value %t141, ptr %t156
  call void @xiao_runtime_value_release_strong(ptr %t156)
  store %xiao.value %t145, ptr %t157
  call void @xiao_runtime_value_release_strong(ptr %t157)
  br label %dynamic.binary.continue58
dynamic.binary.fail57:
  store %xiao.value %t145, ptr %t158
  call void @xiao_runtime_value_release_strong(ptr %t158)
  br label %dynamic.binary.left.fail54
dynamic.binary.left.fail54:
  store %xiao.value %t141, ptr %t159
  call void @xiao_runtime_value_release_strong(ptr %t159)
  br label %xiao.fn.error
dynamic.binary.continue58:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  store %xiao.value %t155, ptr %slot0
  br label %dynamic.if.merge51
dynamic.if.next50:
  br label %dynamic.if.merge51
dynamic.if.merge51:
  store %xiao.value zeroinitializer, ptr %t160
  %t161 = call i32 @xiao_runtime_value_copy(ptr %slot2, ptr %t160)
  %t162 = icmp eq i32 %t161, 0
  br i1 %t162, label %abi.ok61, label %abi.status.fail62
abi.status.fail62:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok61:
  %t163 = load %xiao.value, ptr %t160
  store %xiao.value zeroinitializer, ptr %t165
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t165, i64 2)
  %t164 = load %xiao.value, ptr %t165
  %t166 = getelementptr inbounds [10 x i8], ptr @.xiao.bytes107, i64 0, i64 0
  %t167 = insertvalue %xiao.bytes zeroinitializer, ptr %t166, 0
  %t168 = insertvalue %xiao.bytes %t167, i64 9, 1
  store %xiao.bytes %t168, ptr %t169
  store %xiao.value %t163, ptr %t170
  store %xiao.value %t164, ptr %t171
  store %xiao.value zeroinitializer, ptr %t172
  %t173 = call i32 @xiao_runtime_value_binary(ptr %t169, ptr %t170, ptr %t171, ptr %t172)
  %t174 = icmp eq i32 %t173, 0
  br i1 %t174, label %abi.ok66, label %abi.status.fail67
abi.status.fail67:
  call i32 @xiao_runtime_error_attach_span(i64 1291, i64 1305)
  br label %dynamic.binary.fail64
abi.ok66:
  %t175 = load %xiao.value, ptr %t172
  store %xiao.value %t163, ptr %t176
  call void @xiao_runtime_value_release_strong(ptr %t176)
  store %xiao.value %t164, ptr %t177
  call void @xiao_runtime_value_release_strong(ptr %t177)
  br label %dynamic.binary.continue65
dynamic.binary.fail64:
  store %xiao.value %t164, ptr %t178
  call void @xiao_runtime_value_release_strong(ptr %t178)
  br label %dynamic.binary.left.fail63
dynamic.binary.left.fail63:
  store %xiao.value %t163, ptr %t179
  call void @xiao_runtime_value_release_strong(ptr %t179)
  br label %xiao.fn.error
dynamic.binary.continue65:
  call void @xiao_runtime_value_release_strong(ptr %slot2)
  store %xiao.value %t175, ptr %slot2
  store %xiao.value zeroinitializer, ptr %t180
  %t181 = call i32 @xiao_runtime_value_copy(ptr %slot2, ptr %t180)
  %t182 = icmp eq i32 %t181, 0
  br i1 %t182, label %abi.ok68, label %abi.status.fail69
abi.status.fail69:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok68:
  %t183 = load %xiao.value, ptr %t180
  store %xiao.value zeroinitializer, ptr %t185
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t185, i64 0)
  %t184 = load %xiao.value, ptr %t185
  %t186 = getelementptr inbounds [3 x i8], ptr @.xiao.bytes108, i64 0, i64 0
  %t187 = insertvalue %xiao.bytes zeroinitializer, ptr %t186, 0
  %t188 = insertvalue %xiao.bytes %t187, i64 2, 1
  store %xiao.bytes %t188, ptr %t189
  store %xiao.value %t183, ptr %t190
  store %xiao.value %t184, ptr %t191
  store %xiao.value zeroinitializer, ptr %t192
  %t193 = call i32 @xiao_runtime_value_binary(ptr %t189, ptr %t190, ptr %t191, ptr %t192)
  %t194 = icmp eq i32 %t193, 0
  br i1 %t194, label %abi.ok73, label %abi.status.fail74
abi.status.fail74:
  call i32 @xiao_runtime_error_attach_span(i64 1317, i64 1330)
  br label %dynamic.binary.fail71
abi.ok73:
  %t195 = load %xiao.value, ptr %t192
  store %xiao.value %t183, ptr %t196
  call void @xiao_runtime_value_release_strong(ptr %t196)
  store %xiao.value %t184, ptr %t197
  call void @xiao_runtime_value_release_strong(ptr %t197)
  br label %dynamic.binary.continue72
dynamic.binary.fail71:
  store %xiao.value %t184, ptr %t198
  call void @xiao_runtime_value_release_strong(ptr %t198)
  br label %dynamic.binary.left.fail70
dynamic.binary.left.fail70:
  store %xiao.value %t183, ptr %t199
  call void @xiao_runtime_value_release_strong(ptr %t199)
  br label %xiao.fn.error
dynamic.binary.continue72:
  %t200 = extractvalue %xiao.value %t195, 0
  %t201 = icmp eq i32 %t200, 1
  br i1 %t201, label %dynamic.bool.ok75, label %xiao.fn.error
dynamic.bool.ok75:
  %t202 = extractvalue %xiao.value %t195, 1
  %t203 = trunc i64 %t202 to i1
  store %xiao.value %t195, ptr %t204
  call void @xiao_runtime_value_release_strong(ptr %t204)
  br i1 %t203, label %dynamic.if.then76, label %dynamic.if.next77
dynamic.if.then76:
  store %xiao.value zeroinitializer, ptr %t205
  %t206 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t205)
  %t207 = icmp eq i32 %t206, 0
  br i1 %t207, label %abi.ok79, label %abi.status.fail80
abi.status.fail80:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok79:
  %t208 = load %xiao.value, ptr %t205
  store %xiao.value zeroinitializer, ptr %t209
  %t210 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t209)
  %t211 = icmp eq i32 %t210, 0
  br i1 %t211, label %abi.ok82, label %abi.status.fail83
abi.status.fail83:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.binary.left.fail81
abi.ok82:
  %t212 = load %xiao.value, ptr %t209
  %t213 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes109, i64 0, i64 0
  %t214 = insertvalue %xiao.bytes zeroinitializer, ptr %t213, 0
  %t215 = insertvalue %xiao.bytes %t214, i64 3, 1
  store %xiao.bytes %t215, ptr %t216
  store %xiao.value %t208, ptr %t217
  store %xiao.value %t212, ptr %t218
  store %xiao.value zeroinitializer, ptr %t219
  %t220 = call i32 @xiao_runtime_value_binary(ptr %t216, ptr %t217, ptr %t218, ptr %t219)
  %t221 = icmp eq i32 %t220, 0
  br i1 %t221, label %abi.ok86, label %abi.status.fail87
abi.status.fail87:
  call i32 @xiao_runtime_error_attach_span(i64 1352, i64 1367)
  br label %dynamic.binary.fail84
abi.ok86:
  %t222 = load %xiao.value, ptr %t219
  store %xiao.value %t208, ptr %t223
  call void @xiao_runtime_value_release_strong(ptr %t223)
  store %xiao.value %t212, ptr %t224
  call void @xiao_runtime_value_release_strong(ptr %t224)
  br label %dynamic.binary.continue85
dynamic.binary.fail84:
  store %xiao.value %t212, ptr %t225
  call void @xiao_runtime_value_release_strong(ptr %t225)
  br label %dynamic.binary.left.fail81
dynamic.binary.left.fail81:
  store %xiao.value %t208, ptr %t226
  call void @xiao_runtime_value_release_strong(ptr %t226)
  br label %xiao.fn.error
dynamic.binary.continue85:
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  store %xiao.value %t222, ptr %slot1
  br label %dynamic.if.merge78
dynamic.if.next77:
  br label %dynamic.if.merge78
dynamic.if.merge78:
  br label %dynamic.while.cond25
dynamic.while.end27:
  store %xiao.value zeroinitializer, ptr %t227
  %t228 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t227)
  %t229 = icmp eq i32 %t228, 0
  br i1 %t229, label %abi.ok88, label %abi.status.fail89
abi.status.fail89:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok88:
  %t230 = load %xiao.value, ptr %t227
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t230, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot4)
  call void @xiao_runtime_value_release_strong(ptr %slot3)
  call void @xiao_runtime_value_release_strong(ptr %slot2)
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t231 = call i32 @xiao_runtime_error_class()
  %t232 = icmp eq i32 %t231, 0
  br i1 %t232, label %abi.ok90, label %abi.status.fail91
abi.status.fail91:
  call i32 @xiao_runtime_error_attach_span(i64 1010, i64 1386)
  br label %xiao.fn.error
abi.ok90:
  %t233 = load ptr, ptr %xiao.return.source
  %t234 = icmp ne ptr %t233, null
  br i1 %t234, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t235 = load i1, ptr %xiao.return.temporary
  br i1 %t235, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t236 = call i32 @xiao_runtime_value_copy(ptr %t233, ptr %xiao.function.result)
  %t237 = icmp eq i32 %t236, 0
  br i1 %t237, label %abi.ok92, label %abi.status.fail93
abi.status.fail93:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok92:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t238 = load %xiao.value, ptr %t233
  store %xiao.value %t238, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t233
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t239 = call i32 @xiao_runtime_error_class()
  %t240 = icmp eq i32 %t239, 0
  br i1 %t240, label %abi.ok94, label %abi.status.fail95
abi.status.fail95:
  call i32 @xiao_runtime_error_attach_span(i64 1010, i64 1386)
  br label %xiao.fn.error
abi.ok94:
  ret void
xiao.fn.error:
  %t241 = call i32 @xiao_runtime_error_class()
  %t242 = icmp eq i32 %t241, 2
  br i1 %t242, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot4)
  call void @xiao_runtime_value_release_strong(ptr %slot3)
  call void @xiao_runtime_value_release_strong(ptr %slot2)
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t243 = getelementptr inbounds [12 x i8], ptr @.xiao.bytes110, i64 0, i64 0
  %t244 = insertvalue %xiao.bytes zeroinitializer, ptr %t243, 0
  %t245 = insertvalue %xiao.bytes %t244, i64 11, 1
  store %xiao.bytes %t245, ptr %t246
  %t247 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes111, i64 0, i64 0
  %t248 = insertvalue %xiao.bytes zeroinitializer, ptr %t247, 0
  %t249 = insertvalue %xiao.bytes %t248, i64 3, 1
  store %xiao.bytes %t249, ptr %t250
  call void @xiao_runtime_error_push_native_frame(ptr %t246, ptr %t250, i64 1010, i64 1386)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

define void @xiao.fn.3003b006465c2c81(ptr %xiao.function.result, ptr %xiao.arg0) {
entry:
  %xiao.generated.site0 = alloca %xiao.value
  %xiao.return.site0 = alloca %xiao.value
  %xiao.return.source = alloca ptr
  %xiao.return.temporary = alloca i1
  %slot0 = alloca %xiao.value
  %slot2 = alloca %xiao.value
  %slot1 = alloca %xiao.value
  %slot3 = alloca %xiao.value
  %t4 = alloca %xiao.value
  %t9 = alloca %xiao.bytes
  %t10 = alloca %xiao.value
  %t13 = alloca %xiao.value
  %t14 = alloca i64
  %t18 = alloca i64
  %t22 = alloca %xiao.value
  %t23 = alloca %xiao.value
  %t27 = alloca %xiao.value
  %t32 = alloca %xiao.value
  %t36 = alloca %xiao.bytes
  %t37 = alloca %xiao.value
  %t38 = alloca %xiao.value
  %t39 = alloca %xiao.value
  %t43 = alloca %xiao.value
  %t44 = alloca %xiao.value
  %t45 = alloca %xiao.value
  %t46 = alloca %xiao.value
  %t47 = alloca %xiao.value
  %t52 = alloca %xiao.value
  %t56 = alloca %xiao.bytes
  %t57 = alloca %xiao.value
  %t58 = alloca %xiao.value
  %t59 = alloca %xiao.value
  %t63 = alloca %xiao.value
  %t64 = alloca %xiao.value
  %t65 = alloca %xiao.value
  %t66 = alloca %xiao.value
  %t71 = alloca %xiao.value
  %t75 = alloca %xiao.bytes
  %t76 = alloca ptr
  %t81 = alloca %xiao.value
  %t82 = alloca %xiao.value
  %t86 = alloca %xiao.bytes
  %t87 = alloca %xiao.error.location
  %t92 = alloca %xiao.value
  %t93 = alloca %xiao.value
  %t96 = alloca %xiao.value
  %t97 = alloca %xiao.error.location
  %t106 = alloca %xiao.value
  %t111 = alloca %xiao.value
  %t115 = alloca %xiao.bytes
  %t116 = alloca %xiao.value
  %t117 = alloca %xiao.value
  %t118 = alloca %xiao.value
  %t122 = alloca %xiao.value
  %t123 = alloca %xiao.value
  %t124 = alloca %xiao.value
  %t125 = alloca %xiao.value
  %t130 = alloca %xiao.value
  %t134 = alloca %xiao.bytes
  %t135 = alloca ptr
  %t140 = alloca %xiao.value
  %t141 = alloca %xiao.value
  %t145 = alloca %xiao.bytes
  %t146 = alloca %xiao.error.location
  %t151 = alloca %xiao.value
  %t152 = alloca %xiao.value
  %t155 = alloca %xiao.value
  %t156 = alloca %xiao.error.location
  %t162 = alloca %xiao.value
  %t166 = alloca %xiao.value
  %t167 = alloca %xiao.value
  %t177 = alloca %xiao.bytes
  %t178 = alloca %xiao.value
  %t181 = alloca %xiao.value
  %t182 = alloca i64
  %t186 = alloca i64
  %t190 = alloca %xiao.value
  %t191 = alloca %xiao.value
  %t195 = alloca %xiao.value
  %t199 = alloca %xiao.value
  %t206 = alloca %xiao.bytes
  %t207 = alloca %xiao.value
  %t208 = alloca %xiao.value
  %t209 = alloca %xiao.value
  %t213 = alloca %xiao.value
  %t214 = alloca %xiao.value
  %t215 = alloca %xiao.value
  %t216 = alloca %xiao.value
  %t221 = alloca %xiao.value
  %t222 = alloca %xiao.value
  %t229 = alloca %xiao.value
  %t248 = alloca %xiao.bytes
  %t252 = alloca %xiao.bytes
  store %xiao.value zeroinitializer, ptr %xiao.generated.site0
  store %xiao.value zeroinitializer, ptr %xiao.return.site0
  store ptr null, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  store %xiao.value zeroinitializer, ptr %slot0
  store %xiao.value zeroinitializer, ptr %slot2
  store %xiao.value zeroinitializer, ptr %slot1
  store %xiao.value zeroinitializer, ptr %slot3
  %t0 = call i32 @xiao_runtime_value_copy(ptr %xiao.arg0, ptr %slot3)
  %t1 = icmp eq i32 %t0, 0
  br i1 %t1, label %abi.ok0, label %abi.status.fail1
abi.status.fail1:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok0:
  %t2 = load %xiao.value, ptr %slot3
  store %xiao.value zeroinitializer, ptr %t4
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t4, i64 0)
  %t3 = load %xiao.value, ptr %t4
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  store %xiao.value %t3, ptr %slot0
  %t5 = load %xiao.value, ptr %slot3
  %t6 = getelementptr inbounds [9 x i8], ptr @.xiao.bytes112, i64 0, i64 0
  %t7 = insertvalue %xiao.bytes zeroinitializer, ptr %t6, 0
  %t8 = insertvalue %xiao.bytes %t7, i64 8, 1
  store %xiao.bytes %t8, ptr %t9
  store %xiao.value %t5, ptr %t10
  %t11 = call i32 @xiao_runtime_dynamic_check(ptr %t9, ptr %t10)
  %t12 = icmp eq i32 %t11, 0
  br i1 %t12, label %abi.ok4, label %abi.status.fail5
abi.status.fail5:
  call i32 @xiao_runtime_error_attach_span(i64 770, i64 776)
  br label %dynamic.for.fail2
abi.ok4:
  store %xiao.value %t5, ptr %t13
  store i64 0, ptr %t14
  %t15 = call i32 @xiao_runtime_value_iter_len(ptr %t13, ptr %t14)
  %t16 = icmp eq i32 %t15, 0
  br i1 %t16, label %abi.ok6, label %abi.status.fail7
abi.status.fail7:
  call i32 @xiao_runtime_error_attach_span(i64 770, i64 776)
  br label %dynamic.for.fail2
abi.ok6:
  %t17 = load i64, ptr %t14
  store i64 0, ptr %t18
  br label %dynamic.for.cond8
dynamic.for.cond8:
  %t19 = load i64, ptr %t18
  %t20 = icmp ult i64 %t19, %t17
  br i1 %t20, label %dynamic.for.body9, label %dynamic.for.end11
dynamic.for.body9:
  %t21 = load %xiao.value, ptr %slot3
  store %xiao.value %t21, ptr %t22
  store %xiao.value zeroinitializer, ptr %t23
  %t24 = call i32 @xiao_runtime_value_iter_get(ptr %t22, i64 %t19, ptr %t23)
  %t25 = icmp eq i32 %t24, 0
  br i1 %t25, label %abi.ok12, label %abi.status.fail13
abi.status.fail13:
  call i32 @xiao_runtime_error_attach_span(i64 761, i64 766)
  br label %dynamic.for.fail2
abi.ok12:
  %t26 = load %xiao.value, ptr %t23
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  store %xiao.value %t26, ptr %slot1
  store %xiao.value zeroinitializer, ptr %t27
  %t28 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t27)
  %t29 = icmp eq i32 %t28, 0
  br i1 %t29, label %abi.ok14, label %abi.status.fail15
abi.status.fail15:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.for.fail2
abi.ok14:
  %t30 = load %xiao.value, ptr %t27
  store %xiao.value zeroinitializer, ptr %t32
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t32, i64 1)
  %t31 = load %xiao.value, ptr %t32
  %t33 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes113, i64 0, i64 0
  %t34 = insertvalue %xiao.bytes zeroinitializer, ptr %t33, 0
  %t35 = insertvalue %xiao.bytes %t34, i64 3, 1
  store %xiao.bytes %t35, ptr %t36
  store %xiao.value %t30, ptr %t37
  store %xiao.value %t31, ptr %t38
  store %xiao.value zeroinitializer, ptr %t39
  %t40 = call i32 @xiao_runtime_value_binary(ptr %t36, ptr %t37, ptr %t38, ptr %t39)
  %t41 = icmp eq i32 %t40, 0
  br i1 %t41, label %abi.ok19, label %abi.status.fail20
abi.status.fail20:
  call i32 @xiao_runtime_error_attach_span(i64 793, i64 802)
  br label %dynamic.binary.fail17
abi.ok19:
  %t42 = load %xiao.value, ptr %t39
  store %xiao.value %t30, ptr %t43
  call void @xiao_runtime_value_release_strong(ptr %t43)
  store %xiao.value %t31, ptr %t44
  call void @xiao_runtime_value_release_strong(ptr %t44)
  br label %dynamic.binary.continue18
dynamic.binary.fail17:
  store %xiao.value %t31, ptr %t45
  call void @xiao_runtime_value_release_strong(ptr %t45)
  br label %dynamic.binary.left.fail16
dynamic.binary.left.fail16:
  store %xiao.value %t30, ptr %t46
  call void @xiao_runtime_value_release_strong(ptr %t46)
  br label %dynamic.for.fail2
dynamic.binary.continue18:
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  store %xiao.value %t42, ptr %slot0
  store %xiao.value zeroinitializer, ptr %t47
  %t48 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t47)
  %t49 = icmp eq i32 %t48, 0
  br i1 %t49, label %abi.ok21, label %abi.status.fail22
abi.status.fail22:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.for.fail2
abi.ok21:
  %t50 = load %xiao.value, ptr %t47
  store %xiao.value zeroinitializer, ptr %t52
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t52, i64 65536)
  %t51 = load %xiao.value, ptr %t52
  %t53 = getelementptr inbounds [3 x i8], ptr @.xiao.bytes114, i64 0, i64 0
  %t54 = insertvalue %xiao.bytes zeroinitializer, ptr %t53, 0
  %t55 = insertvalue %xiao.bytes %t54, i64 2, 1
  store %xiao.bytes %t55, ptr %t56
  store %xiao.value %t50, ptr %t57
  store %xiao.value %t51, ptr %t58
  store %xiao.value zeroinitializer, ptr %t59
  %t60 = call i32 @xiao_runtime_value_binary(ptr %t56, ptr %t57, ptr %t58, ptr %t59)
  %t61 = icmp eq i32 %t60, 0
  br i1 %t61, label %abi.ok26, label %abi.status.fail27
abi.status.fail27:
  call i32 @xiao_runtime_error_attach_span(i64 814, i64 827)
  br label %dynamic.binary.fail24
abi.ok26:
  %t62 = load %xiao.value, ptr %t59
  store %xiao.value %t50, ptr %t63
  call void @xiao_runtime_value_release_strong(ptr %t63)
  store %xiao.value %t51, ptr %t64
  call void @xiao_runtime_value_release_strong(ptr %t64)
  br label %dynamic.binary.continue25
dynamic.binary.fail24:
  store %xiao.value %t51, ptr %t65
  call void @xiao_runtime_value_release_strong(ptr %t65)
  br label %dynamic.binary.left.fail23
dynamic.binary.left.fail23:
  store %xiao.value %t50, ptr %t66
  call void @xiao_runtime_value_release_strong(ptr %t66)
  br label %dynamic.for.fail2
dynamic.binary.continue25:
  %t67 = extractvalue %xiao.value %t62, 0
  %t68 = icmp eq i32 %t67, 1
  br i1 %t68, label %dynamic.bool.ok28, label %dynamic.for.fail2
dynamic.bool.ok28:
  %t69 = extractvalue %xiao.value %t62, 1
  %t70 = trunc i64 %t69 to i1
  store %xiao.value %t62, ptr %t71
  call void @xiao_runtime_value_release_strong(ptr %t71)
  br i1 %t70, label %dynamic.if.then29, label %dynamic.if.next30
dynamic.if.then29:
  %t72 = getelementptr inbounds [26 x i8], ptr @.xiao.str115, i64 0, i64 0
  %t73 = insertvalue %xiao.bytes zeroinitializer, ptr %t72, 0
  %t74 = insertvalue %xiao.bytes %t73, i64 25, 1
  store %xiao.bytes %t74, ptr %t75
  store ptr null, ptr %t76
  %t77 = call i32 @xiao_runtime_string_new(ptr %t75, ptr %t76)
  %t78 = icmp eq i32 %t77, 0
  br i1 %t78, label %abi.ok32, label %abi.status.fail33
abi.status.fail33:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.for.fail2
abi.ok32:
  %t79 = load ptr, ptr %t76
  store %xiao.value zeroinitializer, ptr %t81
  call void @xiao_runtime_value_str_owned(ptr sret(%xiao.value) %t81, ptr %t79)
  %t80 = load %xiao.value, ptr %t81
  store %xiao.value %t80, ptr %t82
  %t83 = getelementptr inbounds [14 x i8], ptr @.xiao.bytes116, i64 0, i64 0
  %t84 = insertvalue %xiao.bytes zeroinitializer, ptr %t83, 0
  %t85 = insertvalue %xiao.bytes %t84, i64 13, 1
  store %xiao.bytes %t85, ptr %t86
  %t88 = insertvalue %xiao.error.location zeroinitializer, i64 846, 0
  %t89 = insertvalue %xiao.error.location %t88, i64 895, 1
  %t90 = insertvalue %xiao.error.location %t89, i8 1, 2
  store %xiao.error.location %t90, ptr %t87
  store %xiao.value zeroinitializer, ptr %t92
  call void @xiao_runtime_error_new_values(ptr sret(%xiao.value) %t92, ptr %t86, ptr %t82, ptr null, ptr %t87)
  %t91 = load %xiao.value, ptr %t92
  store %xiao.value %t80, ptr %t93
  call void @xiao_runtime_value_release_strong(ptr %t93)
  %t94 = call i32 @xiao_runtime_error_class()
  %t95 = icmp eq i32 %t94, 0
  br i1 %t95, label %abi.error.ok34, label %abi.error.fail35
abi.error.fail35:
  call i32 @xiao_runtime_error_attach_span(i64 846, i64 895)
  br label %dynamic.for.fail2
abi.error.ok34:
  store %xiao.value %t91, ptr %t96
  %t98 = insertvalue %xiao.error.location zeroinitializer, i64 840, 0
  %t99 = insertvalue %xiao.error.location %t98, i64 895, 1
  %t100 = insertvalue %xiao.error.location %t99, i8 1, 2
  store %xiao.error.location %t100, ptr %t97
  %t101 = call i32 @xiao_runtime_error_raise_value(ptr %t96, ptr %t97)
  call void @xiao_runtime_value_release_strong(ptr %t96)
  %t102 = icmp eq i32 %t101, 0
  br i1 %t102, label %abi.ok36, label %abi.status.fail37
abi.status.fail37:
  call i32 @xiao_runtime_error_attach_span(i64 840, i64 895)
  br label %dynamic.for.fail2
abi.ok36:
  br label %dynamic.for.fail2
dynamic.if.next30:
  br label %dynamic.if.merge31
dynamic.if.merge31:
  br label %dynamic.for.advance10
dynamic.for.advance10:
  %t103 = add i64 %t19, 1
  store i64 %t103, ptr %t18
  br label %dynamic.for.cond8
dynamic.for.end11:
  br label %dynamic.for.continue3
dynamic.for.fail2:
  %t104 = call i32 @xiao_runtime_error_class()
  %t105 = icmp eq i32 %t104, 2
  br i1 %t105, label %xiao.fn.error, label %dynamic.error-cleanup.recoverable39
dynamic.error-cleanup.recoverable39:
  br label %dynamic.for.source-cleanup38
dynamic.for.source-cleanup38:
  br label %xiao.fn.error
dynamic.for.continue3:
  store %xiao.value zeroinitializer, ptr %t106
  %t107 = call i32 @xiao_runtime_value_copy(ptr %slot0, ptr %t106)
  %t108 = icmp eq i32 %t107, 0
  br i1 %t108, label %abi.ok40, label %abi.status.fail41
abi.status.fail41:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok40:
  %t109 = load %xiao.value, ptr %t106
  store %xiao.value zeroinitializer, ptr %t111
  call void @xiao_runtime_value_int(ptr sret(%xiao.value) %t111, i64 0)
  %t110 = load %xiao.value, ptr %t111
  %t112 = getelementptr inbounds [3 x i8], ptr @.xiao.bytes117, i64 0, i64 0
  %t113 = insertvalue %xiao.bytes zeroinitializer, ptr %t112, 0
  %t114 = insertvalue %xiao.bytes %t113, i64 2, 1
  store %xiao.bytes %t114, ptr %t115
  store %xiao.value %t109, ptr %t116
  store %xiao.value %t110, ptr %t117
  store %xiao.value zeroinitializer, ptr %t118
  %t119 = call i32 @xiao_runtime_value_binary(ptr %t115, ptr %t116, ptr %t117, ptr %t118)
  %t120 = icmp eq i32 %t119, 0
  br i1 %t120, label %abi.ok45, label %abi.status.fail46
abi.status.fail46:
  call i32 @xiao_runtime_error_attach_span(i64 903, i64 913)
  br label %dynamic.binary.fail43
abi.ok45:
  %t121 = load %xiao.value, ptr %t118
  store %xiao.value %t109, ptr %t122
  call void @xiao_runtime_value_release_strong(ptr %t122)
  store %xiao.value %t110, ptr %t123
  call void @xiao_runtime_value_release_strong(ptr %t123)
  br label %dynamic.binary.continue44
dynamic.binary.fail43:
  store %xiao.value %t110, ptr %t124
  call void @xiao_runtime_value_release_strong(ptr %t124)
  br label %dynamic.binary.left.fail42
dynamic.binary.left.fail42:
  store %xiao.value %t109, ptr %t125
  call void @xiao_runtime_value_release_strong(ptr %t125)
  br label %xiao.fn.error
dynamic.binary.continue44:
  %t126 = extractvalue %xiao.value %t121, 0
  %t127 = icmp eq i32 %t126, 1
  br i1 %t127, label %dynamic.bool.ok47, label %xiao.fn.error
dynamic.bool.ok47:
  %t128 = extractvalue %xiao.value %t121, 1
  %t129 = trunc i64 %t128 to i1
  store %xiao.value %t121, ptr %t130
  call void @xiao_runtime_value_release_strong(ptr %t130)
  br i1 %t129, label %dynamic.if.then48, label %dynamic.if.next49
dynamic.if.then48:
  %t131 = getelementptr inbounds [17 x i8], ptr @.xiao.str118, i64 0, i64 0
  %t132 = insertvalue %xiao.bytes zeroinitializer, ptr %t131, 0
  %t133 = insertvalue %xiao.bytes %t132, i64 16, 1
  store %xiao.bytes %t133, ptr %t134
  store ptr null, ptr %t135
  %t136 = call i32 @xiao_runtime_string_new(ptr %t134, ptr %t135)
  %t137 = icmp eq i32 %t136, 0
  br i1 %t137, label %abi.ok51, label %abi.status.fail52
abi.status.fail52:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok51:
  %t138 = load ptr, ptr %t135
  store %xiao.value zeroinitializer, ptr %t140
  call void @xiao_runtime_value_str_owned(ptr sret(%xiao.value) %t140, ptr %t138)
  %t139 = load %xiao.value, ptr %t140
  store %xiao.value %t139, ptr %t141
  %t142 = getelementptr inbounds [6 x i8], ptr @.xiao.bytes119, i64 0, i64 0
  %t143 = insertvalue %xiao.bytes zeroinitializer, ptr %t142, 0
  %t144 = insertvalue %xiao.bytes %t143, i64 5, 1
  store %xiao.bytes %t144, ptr %t145
  %t147 = insertvalue %xiao.error.location zeroinitializer, i64 928, 0
  %t148 = insertvalue %xiao.error.location %t147, i64 960, 1
  %t149 = insertvalue %xiao.error.location %t148, i8 1, 2
  store %xiao.error.location %t149, ptr %t146
  store %xiao.value zeroinitializer, ptr %t151
  call void @xiao_runtime_error_new_values(ptr sret(%xiao.value) %t151, ptr %t145, ptr %t141, ptr null, ptr %t146)
  %t150 = load %xiao.value, ptr %t151
  store %xiao.value %t139, ptr %t152
  call void @xiao_runtime_value_release_strong(ptr %t152)
  %t153 = call i32 @xiao_runtime_error_class()
  %t154 = icmp eq i32 %t153, 0
  br i1 %t154, label %abi.error.ok53, label %abi.error.fail54
abi.error.fail54:
  call i32 @xiao_runtime_error_attach_span(i64 928, i64 960)
  br label %xiao.fn.error
abi.error.ok53:
  store %xiao.value %t150, ptr %t155
  %t157 = insertvalue %xiao.error.location zeroinitializer, i64 922, 0
  %t158 = insertvalue %xiao.error.location %t157, i64 960, 1
  %t159 = insertvalue %xiao.error.location %t158, i8 1, 2
  store %xiao.error.location %t159, ptr %t156
  %t160 = call i32 @xiao_runtime_error_raise_value(ptr %t155, ptr %t156)
  call void @xiao_runtime_value_release_strong(ptr %t155)
  %t161 = icmp eq i32 %t160, 0
  br i1 %t161, label %abi.ok55, label %abi.status.fail56
abi.status.fail56:
  call i32 @xiao_runtime_error_attach_span(i64 922, i64 960)
  br label %xiao.fn.error
abi.ok55:
  br label %xiao.fn.error
dynamic.if.next49:
  br label %dynamic.if.merge50
dynamic.if.merge50:
  store %xiao.value zeroinitializer, ptr %t162
  %t163 = call i32 @xiao_runtime_value_copy(ptr %slot3, ptr %t162)
  %t164 = icmp eq i32 %t163, 0
  br i1 %t164, label %abi.ok59, label %abi.status.fail60
abi.status.fail60:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.selector.fail57
abi.ok59:
  %t165 = load %xiao.value, ptr %t162
  call void @xiao_runtime_value_release_strong(ptr %xiao.generated.site0)
  store %xiao.value %t165, ptr %xiao.generated.site0
  store %xiao.value %t165, ptr %t166
  store %xiao.value zeroinitializer, ptr %t167
  %t168 = call i32 @xiao_runtime_value_iter_get(ptr %t166, i64 0, ptr %t167)
  %t169 = icmp eq i32 %t168, 0
  br i1 %t169, label %abi.ok61, label %abi.status.fail62
abi.status.fail62:
  call i32 @xiao_runtime_error_attach_span(i64 974, i64 983)
  br label %dynamic.selector.fail57
abi.ok61:
  %t170 = load %xiao.value, ptr %t167
  call void @xiao_runtime_value_release_strong(ptr %xiao.generated.site0)
  br label %dynamic.selector.continue58
dynamic.selector.fail57:
  call void @xiao_runtime_value_release_strong(ptr %xiao.generated.site0)
  br label %xiao.fn.error
dynamic.selector.continue58:
  call void @xiao_runtime_value_release_strong(ptr %slot2)
  store %xiao.value %t170, ptr %slot2
  call void @xiao_runtime_value_release_strong(ptr %xiao.generated.site0)
  %t171 = call i32 @xiao_runtime_error_class()
  %t172 = icmp eq i32 %t171, 0
  br i1 %t172, label %abi.error.ok63, label %abi.error.fail64
abi.error.fail64:
  call i32 @xiao_runtime_error_attach_span(i64 965, i64 983)
  br label %xiao.fn.error
abi.error.ok63:
  %t173 = load %xiao.value, ptr %slot3
  %t174 = getelementptr inbounds [9 x i8], ptr @.xiao.bytes120, i64 0, i64 0
  %t175 = insertvalue %xiao.bytes zeroinitializer, ptr %t174, 0
  %t176 = insertvalue %xiao.bytes %t175, i64 8, 1
  store %xiao.bytes %t176, ptr %t177
  store %xiao.value %t173, ptr %t178
  %t179 = call i32 @xiao_runtime_dynamic_check(ptr %t177, ptr %t178)
  %t180 = icmp eq i32 %t179, 0
  br i1 %t180, label %abi.ok67, label %abi.status.fail68
abi.status.fail68:
  call i32 @xiao_runtime_error_attach_span(i64 1001, i64 1007)
  br label %dynamic.for.fail65
abi.ok67:
  store %xiao.value %t173, ptr %t181
  store i64 0, ptr %t182
  %t183 = call i32 @xiao_runtime_value_iter_len(ptr %t181, ptr %t182)
  %t184 = icmp eq i32 %t183, 0
  br i1 %t184, label %abi.ok69, label %abi.status.fail70
abi.status.fail70:
  call i32 @xiao_runtime_error_attach_span(i64 1001, i64 1007)
  br label %dynamic.for.fail65
abi.ok69:
  %t185 = load i64, ptr %t182
  store i64 0, ptr %t186
  br label %dynamic.for.cond71
dynamic.for.cond71:
  %t187 = load i64, ptr %t186
  %t188 = icmp ult i64 %t187, %t185
  br i1 %t188, label %dynamic.for.body72, label %dynamic.for.end74
dynamic.for.body72:
  %t189 = load %xiao.value, ptr %slot3
  store %xiao.value %t189, ptr %t190
  store %xiao.value zeroinitializer, ptr %t191
  %t192 = call i32 @xiao_runtime_value_iter_get(ptr %t190, i64 %t187, ptr %t191)
  %t193 = icmp eq i32 %t192, 0
  br i1 %t193, label %abi.ok75, label %abi.status.fail76
abi.status.fail76:
  call i32 @xiao_runtime_error_attach_span(i64 992, i64 997)
  br label %dynamic.for.fail65
abi.ok75:
  %t194 = load %xiao.value, ptr %t191
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  store %xiao.value %t194, ptr %slot1
  store %xiao.value zeroinitializer, ptr %t195
  %t196 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t195)
  %t197 = icmp eq i32 %t196, 0
  br i1 %t197, label %abi.ok77, label %abi.status.fail78
abi.status.fail78:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.for.fail65
abi.ok77:
  %t198 = load %xiao.value, ptr %t195
  store %xiao.value zeroinitializer, ptr %t199
  %t200 = call i32 @xiao_runtime_value_copy(ptr %slot2, ptr %t199)
  %t201 = icmp eq i32 %t200, 0
  br i1 %t201, label %abi.ok80, label %abi.status.fail81
abi.status.fail81:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.binary.left.fail79
abi.ok80:
  %t202 = load %xiao.value, ptr %t199
  %t203 = getelementptr inbounds [3 x i8], ptr @.xiao.bytes121, i64 0, i64 0
  %t204 = insertvalue %xiao.bytes zeroinitializer, ptr %t203, 0
  %t205 = insertvalue %xiao.bytes %t204, i64 2, 1
  store %xiao.bytes %t205, ptr %t206
  store %xiao.value %t198, ptr %t207
  store %xiao.value %t202, ptr %t208
  store %xiao.value zeroinitializer, ptr %t209
  %t210 = call i32 @xiao_runtime_value_binary(ptr %t206, ptr %t207, ptr %t208, ptr %t209)
  %t211 = icmp eq i32 %t210, 0
  br i1 %t211, label %abi.ok84, label %abi.status.fail85
abi.status.fail85:
  call i32 @xiao_runtime_error_attach_span(i64 1019, i64 1033)
  br label %dynamic.binary.fail82
abi.ok84:
  %t212 = load %xiao.value, ptr %t209
  store %xiao.value %t198, ptr %t213
  call void @xiao_runtime_value_release_strong(ptr %t213)
  store %xiao.value %t202, ptr %t214
  call void @xiao_runtime_value_release_strong(ptr %t214)
  br label %dynamic.binary.continue83
dynamic.binary.fail82:
  store %xiao.value %t202, ptr %t215
  call void @xiao_runtime_value_release_strong(ptr %t215)
  br label %dynamic.binary.left.fail79
dynamic.binary.left.fail79:
  store %xiao.value %t198, ptr %t216
  call void @xiao_runtime_value_release_strong(ptr %t216)
  br label %dynamic.for.fail65
dynamic.binary.continue83:
  %t217 = extractvalue %xiao.value %t212, 0
  %t218 = icmp eq i32 %t217, 1
  br i1 %t218, label %dynamic.bool.ok86, label %dynamic.for.fail65
dynamic.bool.ok86:
  %t219 = extractvalue %xiao.value %t212, 1
  %t220 = trunc i64 %t219 to i1
  store %xiao.value %t212, ptr %t221
  call void @xiao_runtime_value_release_strong(ptr %t221)
  br i1 %t220, label %dynamic.if.then87, label %dynamic.if.next88
dynamic.if.then87:
  store %xiao.value zeroinitializer, ptr %t222
  %t223 = call i32 @xiao_runtime_value_copy(ptr %slot1, ptr %t222)
  %t224 = icmp eq i32 %t223, 0
  br i1 %t224, label %abi.ok90, label %abi.status.fail91
abi.status.fail91:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %dynamic.for.fail65
abi.ok90:
  %t225 = load %xiao.value, ptr %t222
  call void @xiao_runtime_value_release_strong(ptr %slot2)
  store %xiao.value %t225, ptr %slot2
  br label %dynamic.if.merge89
dynamic.if.next88:
  br label %dynamic.if.merge89
dynamic.if.merge89:
  br label %dynamic.for.advance73
dynamic.for.advance73:
  %t226 = add i64 %t187, 1
  store i64 %t226, ptr %t186
  br label %dynamic.for.cond71
dynamic.for.end74:
  br label %dynamic.for.continue66
dynamic.for.fail65:
  %t227 = call i32 @xiao_runtime_error_class()
  %t228 = icmp eq i32 %t227, 2
  br i1 %t228, label %xiao.fn.error, label %dynamic.error-cleanup.recoverable93
dynamic.error-cleanup.recoverable93:
  br label %dynamic.for.source-cleanup92
dynamic.for.source-cleanup92:
  br label %xiao.fn.error
dynamic.for.continue66:
  store %xiao.value zeroinitializer, ptr %t229
  %t230 = call i32 @xiao_runtime_value_copy(ptr %slot2, ptr %t229)
  %t231 = icmp eq i32 %t230, 0
  br i1 %t231, label %abi.ok94, label %abi.status.fail95
abi.status.fail95:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok94:
  %t232 = load %xiao.value, ptr %t229
  call void @xiao_runtime_error_clear()
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  store %xiao.value %t232, ptr %xiao.return.site0
  store ptr %xiao.return.site0, ptr %xiao.return.source
  store i1 false, ptr %xiao.return.temporary
  br label %xiao.fn.return
xiao.fn.return:
  call void @xiao_runtime_value_release_strong(ptr %slot3)
  call void @xiao_runtime_value_release_strong(ptr %slot2)
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  %t233 = call i32 @xiao_runtime_error_class()
  %t234 = icmp eq i32 %t233, 0
  br i1 %t234, label %abi.ok96, label %abi.status.fail97
abi.status.fail97:
  call i32 @xiao_runtime_error_attach_span(i64 710, i64 1079)
  br label %xiao.fn.error
abi.ok96:
  %t235 = load ptr, ptr %xiao.return.source
  %t236 = icmp ne ptr %t235, null
  br i1 %t236, label %xiao.fn.return.value, label %xiao.fn.return.cleanup
xiao.fn.return.value:
  %t237 = load i1, ptr %xiao.return.temporary
  br i1 %t237, label %xiao.fn.return.copy, label %xiao.fn.return.move
xiao.fn.return.copy:
  %t238 = call i32 @xiao_runtime_value_copy(ptr %t235, ptr %xiao.function.result)
  %t239 = icmp eq i32 %t238, 0
  br i1 %t239, label %abi.ok98, label %abi.status.fail99
abi.status.fail99:
  call i32 @xiao_runtime_error_attach_span(i64 0, i64 2150)
  br label %xiao.fn.error
abi.ok98:
  br label %xiao.fn.return.cleanup
xiao.fn.return.move:
  %t240 = load %xiao.value, ptr %t235
  store %xiao.value %t240, ptr %xiao.function.result
  store %xiao.value zeroinitializer, ptr %t235
  br label %xiao.fn.return.cleanup
xiao.fn.return.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %xiao.generated.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  %t241 = call i32 @xiao_runtime_error_class()
  %t242 = icmp eq i32 %t241, 0
  br i1 %t242, label %abi.ok100, label %abi.status.fail101
abi.status.fail101:
  call i32 @xiao_runtime_error_attach_span(i64 710, i64 1079)
  br label %xiao.fn.error
abi.ok100:
  ret void
xiao.fn.error:
  %t243 = call i32 @xiao_runtime_error_class()
  %t244 = icmp eq i32 %t243, 2
  br i1 %t244, label %xiao.fn.error.return, label %xiao.fn.error.cleanup
xiao.fn.error.cleanup:
  call void @xiao_runtime_value_release_strong(ptr %slot3)
  call void @xiao_runtime_value_release_strong(ptr %slot2)
  call void @xiao_runtime_value_release_strong(ptr %slot1)
  call void @xiao_runtime_value_release_strong(ptr %slot0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.generated.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.return.site0)
  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)
  br label %xiao.fn.error.return
xiao.fn.error.return:
  %t245 = getelementptr inbounds [18 x i8], ptr @.xiao.bytes122, i64 0, i64 0
  %t246 = insertvalue %xiao.bytes zeroinitializer, ptr %t245, 0
  %t247 = insertvalue %xiao.bytes %t246, i64 17, 1
  store %xiao.bytes %t247, ptr %t248
  %t249 = getelementptr inbounds [4 x i8], ptr @.xiao.bytes123, i64 0, i64 0
  %t250 = insertvalue %xiao.bytes zeroinitializer, ptr %t249, 0
  %t251 = insertvalue %xiao.bytes %t250, i64 3, 1
  store %xiao.bytes %t251, ptr %t252
  call void @xiao_runtime_error_push_native_frame(ptr %t248, ptr %t252, i64 710, i64 1079)
  store %xiao.value zeroinitializer, ptr %xiao.function.result
  ret void
}

