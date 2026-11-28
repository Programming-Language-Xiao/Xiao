; ModuleID = 'D:\project\Xiao\Xiao\docs\DevDocs\assets\20o-b3\entry-20261128-b3-5\logs\c/probe.c'
source_filename = "D:\\project\\Xiao\\Xiao\\docs\\DevDocs\\assets\\20o-b3\\entry-20261128-b3-5\\logs\\c/probe.c"
target datalayout = "e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
target triple = "x86_64-pc-windows-msvc19.51.36256"

%union.anon = type { i64 }
%struct.XiaoValue = type { i32, %union.anon }
%struct.XiaoAbiBytes = type { ptr, i64 }
%struct.XiaoAbiMutBytes = type { ptr, i64 }
%struct.XiaoAbiErrorSnapshot = type { i32, i32, i64, %struct.XiaoAbiBytes, %struct.XiaoAbiBytes, %struct.anon, i32, i32, i32 }
%struct.anon = type { i64, i64, i8, [7 x i8] }

$sprintf = comdat any

$vsprintf = comdat any

$_snprintf = comdat any

$_vsnprintf = comdat any

$fprintf = comdat any

$_vsprintf_l = comdat any

$_vsnprintf_l = comdat any

$__local_stdio_printf_options = comdat any

$_vfprintf_l = comdat any

$"??_C@_06IKJOPBFO@closed?$AA@" = comdat any

$"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@" = comdat any

$"??_C@_1EM@GJOJCMJN@?$AAS?$AAe?$AAt?$AAS?$AAt?$AAd?$AAH?$AAa?$AAn?$AAd?$AAl?$AAe?$AA?$CI?$AAS?$AAT?$AAD?$AA_?$AAO?$AAU?$AAT?$AAP?$AAU?$AAT?$AA_?$AAH?$AAA?$AAN?$AAD?$AAL?$AAE?$AA?0?$AA?5@" = comdat any

$"??_C@_1IC@OPJHALMA@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAe?$AAn?$AAv?$AA_?$AAr?$AAu?$AAn?$AA_?$AAi?$AAn?$AAj?$AAe?$AAc?$AAt?$AAe?$AAd?$AA?$CI?$AAv?$AAe@" = comdat any

$"??_C@_04NIDJFNBE@info?$AA@" = comdat any

$"??_C@_07OEIDNODA@unbound?$AA@" = comdat any

$"??_C@_06DJDOIFHB@stdout?$AA@" = comdat any

$"??_C@_1HK@HFDDFJHJ@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = comdat any

$"??_C@_0BJ@NALBOBEG@C?5ABI?3?5LOG_OUTPUT_FAILED?$AA@" = comdat any

$"??_C@_0DG@NIGKCKPN@C?5ABI?3?50?11?12?16?18?0?5pending?0?5borro@" = comdat any

$"??_C@_03OFAPEBGM@?$CFs?6?$AA@" = comdat any

$"??_C@_0N@MPKLMEOF@C?5ABI?5?f?$LF?$IL?h?$KP?$JF?$AA@" = comdat any

$"??_C@_1HK@JFOMCOEK@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = comdat any

$"??_C@_0BC@JBLGAAOK@LOG_OUTPUT_FAILED?$AA@" = comdat any

$"??_C@_1HG@IJIKIDDE@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AAN?$AAU?$AAL?$AAL?$AA?0?$AA?5?$AA?$CG?$AAm?$AAe@" = comdat any

$"??_C@_0BA@PILDGKCH@X06?9RUNTIME?9012?$AA@" = comdat any

$"??_C@_1HO@ICGHHMKE@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = comdat any

$"??_C@_1HM@LALDBKGK@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = comdat any

$"??_C@_1IK@POPLDDMN@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = comdat any

$"??_C@_1HO@LBMLBEBC@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAw?$AAr?$AAo?$AAn?$AAg?$AA_?$AAt?$AAy@" = comdat any

$"??_C@_0BA@PJHBAABA@X06?9RUNTIME?9002?$AA@" = comdat any

$"??_C@_1HK@JIGIMDPL@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = comdat any

$"??_C@_0BA@JOJJBBHK@X06?9RUNTIME?9026?$AA@" = comdat any

$"??_C@_1HK@GLHINJBC@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAt?$AAa?$AAr?$AAg?$AAe?$AAt?$AA?0?$AA?5@" = comdat any

$"??_C@_1HM@GDLFFFKF@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = comdat any

$"??_C@_01LIIJDEN@?$AN?$AA@" = comdat any

$"??_C@_01EEMJAFIK@?6?$AA@" = comdat any

$"??_C@_01LOCGONAA@?$AA?$AA@" = comdat any

$"??_C@_1IK@HJHFKFIK@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = comdat any

$"??_C@_1HM@JMFIMOJH@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = comdat any

$"??_C@_0BD@BGMAPKLA@LOG_LIMIT_EXCEEDED?$AA@" = comdat any

$"??_C@_1HE@KCPFPJPJ@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = comdat any

$"??_C@_1HO@LGNJPPAO@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAs?$AAt?$AAr?$AAi?$AAn?$AAg?$AA_?$AAn?$AAe?$AAw?$AA?$CI?$AAb?$AAy?$AAt?$AAe?$AAs?$AA?0?$AA?5?$AA?$CG@" = comdat any

$"??_C@_1HK@LAIHHBJG@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = comdat any

$"??_C@_1HG@HKEJBPI@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = comdat any

$"??_C@_1HE@MICEHIGB@?$AAm?$AAe?$AAs?$AAs?$AAa?$AAg?$AAe?$AA?4?$AAt?$AAa?$AAg?$AA?5?$AA?$DN?$AA?$DN?$AA?5?$AA8?$AA?5?$AA?$CG?$AA?$CG?$AA?5?$AAm?$AAe?$AAs?$AAs?$AAa?$AAg?$AAe?$AA?4?$AAp?$AAa?$AAy?$AAl@" = comdat any

$"??_C@_1LG@CADPKNEE@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAs?$AAt?$AAr?$AAi?$AAn?$AAg?$AA_?$AAc?$AAo?$AAp?$AAy?$AA?$CI?$AAo?$AAl?$AAd?$AA_?$AAm?$AAe?$AAs@" = comdat any

$"??_C@_1JI@CLAELEBN@?$AAc?$AAo?$AAp?$AAi?$AAe?$AAd?$AA?5?$AA?$DN?$AA?$DN?$AA?5?$AA1?$AA2?$AA?5?$AA?$CG?$AA?$CG?$AA?5?$AAm?$AAe?$AAm?$AAc?$AAm?$AAp?$AA?$CI?$AAc?$AAo?$AAp?$AAy?$AA?0?$AA?5?$AA?$CC?$AAC?$AA?5@" = comdat any

$"??_C@_1HE@LPOLCLJE@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAs?$AAt?$AAr?$AAi?$AAn?$AAg?$AA_?$AAn?$AAe?$AAw?$AA?$CI?$AAv?$AAi?$AAe?$AAw?$AA?0?$AA?5?$AA?$CG?$AAv@" = comdat any

$"??_C@_1FC@IIBCECNM@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAe?$AAr?$AAr?$AAo?$AAr?$AA_?$AAs?$AAn?$AAa?$AAp?$AAs?$AAh?$AAo?$AAt?$AA?$CI?$AA?$CG?$AAe?$AAr?$AAr@" = comdat any

$"??_C@_1HK@KELFKMPK@?$AAe?$AAr?$AAr?$AAo?$AAr?$AA?4?$AAc?$AAl?$AAa?$AAs?$AAs?$AA_?$AAt?$AAa?$AAg?$AA?5?$AA?$DN?$AA?$DN?$AA?5?$AAc?$AAl?$AAa?$AAs?$AAs?$AA_?$AAt?$AAa?$AAg?$AA?5?$AA?$CG?$AA?$CG?$AA?5@" = comdat any

$"??_C@_1DO@BNAPIDAO@?$AAe?$AAr?$AAr?$AAo?$AAr?$AA?4?$AAc?$AAo?$AAd?$AAe?$AA?4?$AAl?$AAe?$AAn?$AA?5?$AA?$DN?$AA?$DN?$AA?5?$AAs?$AAt?$AAr?$AAl?$AAe?$AAn?$AA?$CI?$AAc?$AAo?$AAd?$AAe?$AA?$CJ?$AA?$AA@" = comdat any

$"??_C@_1GE@HBCAKALM@?$AAm?$AAe?$AAm?$AAc?$AAm?$AAp?$AA?$CI?$AAe?$AAr?$AAr?$AAo?$AAr?$AA?4?$AAc?$AAo?$AAd?$AAe?$AA?4?$AAp?$AAt?$AAr?$AA?0?$AA?5?$AAc?$AAo?$AAd?$AAe?$AA?0?$AA?5?$AAe?$AAr?$AAr@" = comdat any

$"??_C@_1FG@BCDAGFCH@?$AAv?$AAa?$AAl?$AAu?$AAe?$AA?9?$AA?$DO?$AAt?$AAa?$AAg?$AA?5?$AA?$DN?$AA?$DN?$AA?5?$AA0?$AA?5?$AA?$CG?$AA?$CG?$AA?5?$AAv?$AAa?$AAl?$AAu?$AAe?$AA?9?$AA?$DO?$AAp?$AAa?$AAy?$AAl?$AAo?$AAa@" = comdat any

@output_failed = internal global i32 0, align 4
@"??_C@_06IKJOPBFO@closed?$AA@" = linkonce_odr dso_local unnamed_addr constant [7 x i8] c"closed\00", comdat, align 1
@"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@" = linkonce_odr dso_local unnamed_addr constant [83 x i16] [i16 68, i16 58, i16 92, i16 112, i16 114, i16 111, i16 106, i16 101, i16 99, i16 116, i16 92, i16 88, i16 105, i16 97, i16 111, i16 92, i16 88, i16 105, i16 97, i16 111, i16 92, i16 100, i16 111, i16 99, i16 115, i16 92, i16 68, i16 101, i16 118, i16 68, i16 111, i16 99, i16 115, i16 92, i16 97, i16 115, i16 115, i16 101, i16 116, i16 115, i16 92, i16 50, i16 48, i16 111, i16 45, i16 98, i16 51, i16 92, i16 101, i16 110, i16 116, i16 114, i16 121, i16 45, i16 50, i16 48, i16 50, i16 54, i16 49, i16 49, i16 50, i16 56, i16 45, i16 98, i16 51, i16 45, i16 53, i16 92, i16 108, i16 111, i16 103, i16 115, i16 92, i16 99, i16 47, i16 112, i16 114, i16 111, i16 98, i16 101, i16 46, i16 99, i16 0], comdat, align 2
@"??_C@_1EM@GJOJCMJN@?$AAS?$AAe?$AAt?$AAS?$AAt?$AAd?$AAH?$AAa?$AAn?$AAd?$AAl?$AAe?$AA?$CI?$AAS?$AAT?$AAD?$AA_?$AAO?$AAU?$AAT?$AAP?$AAU?$AAT?$AA_?$AAH?$AAA?$AAN?$AAD?$AAL?$AAE?$AA?0?$AA?5@" = linkonce_odr dso_local unnamed_addr constant [38 x i16] [i16 83, i16 101, i16 116, i16 83, i16 116, i16 100, i16 72, i16 97, i16 110, i16 100, i16 108, i16 101, i16 40, i16 83, i16 84, i16 68, i16 95, i16 79, i16 85, i16 84, i16 80, i16 85, i16 84, i16 95, i16 72, i16 65, i16 78, i16 68, i16 76, i16 69, i16 44, i16 32, i16 78, i16 85, i16 76, i16 76, i16 41, i16 0], comdat, align 2
@"??_C@_1IC@OPJHALMA@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAe?$AAn?$AAv?$AA_?$AAr?$AAu?$AAn?$AA_?$AAi?$AAn?$AAj?$AAe?$AAc?$AAt?$AAe?$AAd?$AA?$CI?$AAv?$AAe@" = linkonce_odr dso_local unnamed_addr constant [65 x i16] [i16 120, i16 105, i16 97, i16 111, i16 95, i16 114, i16 117, i16 110, i16 116, i16 105, i16 109, i16 101, i16 95, i16 101, i16 110, i16 118, i16 95, i16 114, i16 117, i16 110, i16 95, i16 105, i16 110, i16 106, i16 101, i16 99, i16 116, i16 101, i16 100, i16 40, i16 118, i16 101, i16 114, i16 105, i16 102, i16 121, i16 95, i16 105, i16 110, i16 95, i16 114, i16 117, i16 110, i16 44, i16 32, i16 78, i16 85, i16 76, i16 76, i16 44, i16 32, i16 48, i16 44, i16 32, i16 98, i16 111, i16 116, i16 104, i16 41, i16 32, i16 61, i16 61, i16 32, i16 48, i16 0], comdat, align 2
@"??_C@_04NIDJFNBE@info?$AA@" = linkonce_odr dso_local unnamed_addr constant [5 x i8] c"info\00", comdat, align 1
@"??_C@_07OEIDNODA@unbound?$AA@" = linkonce_odr dso_local unnamed_addr constant [8 x i8] c"unbound\00", comdat, align 1
@"??_C@_06DJDOIFHB@stdout?$AA@" = linkonce_odr dso_local unnamed_addr constant [7 x i8] c"stdout\00", comdat, align 1
@"??_C@_1HK@HFDDFJHJ@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = linkonce_odr dso_local unnamed_addr constant [61 x i16] [i16 120, i16 105, i16 97, i16 111, i16 95, i16 114, i16 117, i16 110, i16 116, i16 105, i16 109, i16 101, i16 95, i16 108, i16 111, i16 103, i16 95, i16 119, i16 114, i16 105, i16 116, i16 101, i16 40, i16 38, i16 108, i16 101, i16 118, i16 101, i16 108, i16 44, i16 32, i16 38, i16 109, i16 101, i16 115, i16 115, i16 97, i16 103, i16 101, i16 44, i16 32, i16 38, i16 116, i16 97, i16 114, i16 103, i16 101, i16 116, i16 44, i16 32, i16 98, i16 111, i16 116, i16 104, i16 41, i16 32, i16 61, i16 61, i16 32, i16 56, i16 0], comdat, align 2
@"??_C@_0BJ@NALBOBEG@C?5ABI?3?5LOG_OUTPUT_FAILED?$AA@" = linkonce_odr dso_local unnamed_addr constant [25 x i8] c"C ABI: LOG_OUTPUT_FAILED\00", comdat, align 1
@"??_C@_0DG@NIGKCKPN@C?5ABI?3?50?11?12?16?18?0?5pending?0?5borro@" = linkonce_odr dso_local unnamed_addr constant [54 x i8] c"C ABI: 0/1/2/6/8, pending, borrowed slots and release\00", comdat, align 1
@"??_C@_03OFAPEBGM@?$CFs?6?$AA@" = linkonce_odr dso_local unnamed_addr constant [4 x i8] c"%s\0A\00", comdat, align 1
@__local_stdio_printf_options._OptionsStorage = internal global i64 0, align 8
@"??_C@_0N@MPKLMEOF@C?5ABI?5?f?$LF?$IL?h?$KP?$JF?$AA@" = linkonce_odr dso_local unnamed_addr constant [13 x i8] c"C ABI \E6\B5\8B\E8\AF\95\00", comdat, align 1
@"??_C@_1HK@JFOMCOEK@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = linkonce_odr dso_local unnamed_addr constant [61 x i16] [i16 120, i16 105, i16 97, i16 111, i16 95, i16 114, i16 117, i16 110, i16 116, i16 105, i16 109, i16 101, i16 95, i16 108, i16 111, i16 103, i16 95, i16 119, i16 114, i16 105, i16 116, i16 101, i16 40, i16 38, i16 108, i16 101, i16 118, i16 101, i16 108, i16 44, i16 32, i16 38, i16 109, i16 101, i16 115, i16 115, i16 97, i16 103, i16 101, i16 44, i16 32, i16 38, i16 116, i16 97, i16 114, i16 103, i16 101, i16 116, i16 44, i16 32, i16 98, i16 111, i16 116, i16 104, i16 41, i16 32, i16 61, i16 61, i16 32, i16 54, i16 0], comdat, align 2
@"??_C@_0BC@JBLGAAOK@LOG_OUTPUT_FAILED?$AA@" = linkonce_odr dso_local unnamed_addr constant [18 x i8] c"LOG_OUTPUT_FAILED\00", comdat, align 1
@__const.verify_in_run.unknown = private unnamed_addr constant { i32, [4 x i8], %union.anon } { i32 255, [4 x i8] zeroinitializer, %union.anon zeroinitializer }, align 8
@__const.verify_in_run.bad_shape = private unnamed_addr constant { i32, [4 x i8], %union.anon } { i32 8, [4 x i8] zeroinitializer, %union.anon zeroinitializer }, align 8
@__const.verify_in_run.wrong_type = private unnamed_addr constant { i32, [4 x i8], %union.anon } { i32 2, [4 x i8] zeroinitializer, %union.anon { i64 12 } }, align 8
@"??_C@_1HG@IJIKIDDE@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AAN?$AAU?$AAL?$AAL?$AA?0?$AA?5?$AA?$CG?$AAm?$AAe@" = linkonce_odr dso_local unnamed_addr constant [59 x i16] [i16 120, i16 105, i16 97, i16 111, i16 95, i16 114, i16 117, i16 110, i16 116, i16 105, i16 109, i16 101, i16 95, i16 108, i16 111, i16 103, i16 95, i16 119, i16 114, i16 105, i16 116, i16 101, i16 40, i16 78, i16 85, i16 76, i16 76, i16 44, i16 32, i16 38, i16 109, i16 101, i16 115, i16 115, i16 97, i16 103, i16 101, i16 44, i16 32, i16 38, i16 116, i16 97, i16 114, i16 103, i16 101, i16 116, i16 44, i16 32, i16 98, i16 111, i16 116, i16 104, i16 41, i16 32, i16 61, i16 61, i16 32, i16 49, i16 0], comdat, align 2
@"??_C@_0BA@PILDGKCH@X06?9RUNTIME?9012?$AA@" = linkonce_odr dso_local unnamed_addr constant [16 x i8] c"X06-RUNTIME-012\00", comdat, align 1
@"??_C@_1HO@ICGHHMKE@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = linkonce_odr dso_local unnamed_addr constant [63 x i16] [i16 120, i16 105, i16 97, i16 111, i16 95, i16 114, i16 117, i16 110, i16 116, i16 105, i16 109, i16 101, i16 95, i16 108, i16 111, i16 103, i16 95, i16 119, i16 114, i16 105, i16 116, i16 101, i16 40, i16 38, i16 108, i16 101, i16 118, i16 101, i16 108, i16 44, i16 32, i16 38, i16 98, i16 97, i16 100, i16 95, i16 115, i16 104, i16 97, i16 112, i16 101, i16 44, i16 32, i16 38, i16 116, i16 97, i16 114, i16 103, i16 101, i16 116, i16 44, i16 32, i16 98, i16 111, i16 116, i16 104, i16 41, i16 32, i16 61, i16 61, i16 32, i16 49, i16 0], comdat, align 2
@"??_C@_1HM@LALDBKGK@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = linkonce_odr dso_local unnamed_addr constant [62 x i16] [i16 120, i16 105, i16 97, i16 111, i16 95, i16 114, i16 117, i16 110, i16 116, i16 105, i16 109, i16 101, i16 95, i16 108, i16 111, i16 103, i16 95, i16 119, i16 114, i16 105, i16 116, i16 101, i16 40, i16 38, i16 108, i16 101, i16 118, i16 101, i16 108, i16 44, i16 32, i16 38, i16 109, i16 101, i16 115, i16 115, i16 97, i16 103, i16 101, i16 44, i16 32, i16 38, i16 117, i16 110, i16 107, i16 110, i16 111, i16 119, i16 110, i16 44, i16 32, i16 98, i16 111, i16 116, i16 104, i16 41, i16 32, i16 61, i16 61, i16 32, i16 50, i16 0], comdat, align 2
@"??_C@_1IK@POPLDDMN@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = linkonce_odr dso_local unnamed_addr constant [69 x i16] [i16 120, i16 105, i16 97, i16 111, i16 95, i16 114, i16 117, i16 110, i16 116, i16 105, i16 109, i16 101, i16 95, i16 108, i16 111, i16 103, i16 95, i16 119, i16 114, i16 105, i16 116, i16 101, i16 40, i16 38, i16 108, i16 101, i16 118, i16 101, i16 108, i16 44, i16 32, i16 38, i16 109, i16 101, i16 115, i16 115, i16 97, i16 103, i16 101, i16 44, i16 32, i16 38, i16 116, i16 97, i16 114, i16 103, i16 101, i16 116, i16 44, i16 32, i16 98, i16 111, i16 116, i16 104, i16 32, i16 124, i16 32, i16 48, i16 120, i16 49, i16 48, i16 48, i16 41, i16 32, i16 61, i16 61, i16 32, i16 50, i16 0], comdat, align 2
@"??_C@_1HO@LBMLBEBC@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAw?$AAr?$AAo?$AAn?$AAg?$AA_?$AAt?$AAy@" = linkonce_odr dso_local unnamed_addr constant [63 x i16] [i16 120, i16 105, i16 97, i16 111, i16 95, i16 114, i16 117, i16 110, i16 116, i16 105, i16 109, i16 101, i16 95, i16 108, i16 111, i16 103, i16 95, i16 119, i16 114, i16 105, i16 116, i16 101, i16 40, i16 38, i16 119, i16 114, i16 111, i16 110, i16 103, i16 95, i16 116, i16 121, i16 112, i16 101, i16 44, i16 32, i16 38, i16 109, i16 101, i16 115, i16 115, i16 97, i16 103, i16 101, i16 44, i16 32, i16 38, i16 116, i16 97, i16 114, i16 103, i16 101, i16 116, i16 44, i16 32, i16 48, i16 41, i16 32, i16 61, i16 61, i16 32, i16 54, i16 0], comdat, align 2
@"??_C@_0BA@PJHBAABA@X06?9RUNTIME?9002?$AA@" = linkonce_odr dso_local unnamed_addr constant [16 x i8] c"X06-RUNTIME-002\00", comdat, align 1
@"??_C@_1HK@JIGIMDPL@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = linkonce_odr dso_local unnamed_addr constant [61 x i16] [i16 120, i16 105, i16 97, i16 111, i16 95, i16 114, i16 117, i16 110, i16 116, i16 105, i16 109, i16 101, i16 95, i16 108, i16 111, i16 103, i16 95, i16 119, i16 114, i16 105, i16 116, i16 101, i16 40, i16 38, i16 108, i16 101, i16 118, i16 101, i16 108, i16 44, i16 32, i16 38, i16 109, i16 101, i16 115, i16 115, i16 97, i16 103, i16 101, i16 44, i16 32, i16 38, i16 116, i16 97, i16 114, i16 103, i16 101, i16 116, i16 44, i16 32, i16 109, i16 97, i16 115, i16 107, i16 41, i16 32, i16 61, i16 61, i16 32, i16 54, i16 0], comdat, align 2
@"??_C@_0BA@JOJJBBHK@X06?9RUNTIME?9026?$AA@" = linkonce_odr dso_local unnamed_addr constant [16 x i8] c"X06-RUNTIME-026\00", comdat, align 1
@"??_C@_1HK@GLHINJBC@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAt?$AAa?$AAr?$AAg?$AAe?$AAt?$AA?0?$AA?5@" = linkonce_odr dso_local unnamed_addr constant [61 x i16] [i16 120, i16 105, i16 97, i16 111, i16 95, i16 114, i16 117, i16 110, i16 116, i16 105, i16 109, i16 101, i16 95, i16 108, i16 111, i16 103, i16 95, i16 119, i16 114, i16 105, i16 116, i16 101, i16 40, i16 38, i16 116, i16 97, i16 114, i16 103, i16 101, i16 116, i16 44, i16 32, i16 38, i16 109, i16 101, i16 115, i16 115, i16 97, i16 103, i16 101, i16 44, i16 32, i16 38, i16 108, i16 101, i16 118, i16 101, i16 108, i16 44, i16 32, i16 98, i16 111, i16 116, i16 104, i16 41, i16 32, i16 61, i16 61, i16 32, i16 54, i16 0], comdat, align 2
@"??_C@_1HM@GDLFFFKF@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = linkonce_odr dso_local unnamed_addr constant [62 x i16] [i16 120, i16 105, i16 97, i16 111, i16 95, i16 114, i16 117, i16 110, i16 116, i16 105, i16 109, i16 101, i16 95, i16 108, i16 111, i16 103, i16 95, i16 119, i16 114, i16 105, i16 116, i16 101, i16 40, i16 38, i16 108, i16 101, i16 118, i16 101, i16 108, i16 44, i16 32, i16 38, i16 109, i16 101, i16 115, i16 115, i16 97, i16 103, i16 101, i16 44, i16 32, i16 38, i16 109, i16 101, i16 115, i16 115, i16 97, i16 103, i16 101, i16 44, i16 32, i16 98, i16 111, i16 116, i16 104, i16 41, i16 32, i16 61, i16 61, i16 32, i16 54, i16 0], comdat, align 2
@"??_C@_01LIIJDEN@?$AN?$AA@" = linkonce_odr dso_local unnamed_addr constant [2 x i8] c"\0D\00", comdat, align 1
@"??_C@_01EEMJAFIK@?6?$AA@" = linkonce_odr dso_local unnamed_addr constant [2 x i8] c"\0A\00", comdat, align 1
@"??_C@_01LOCGONAA@?$AA?$AA@" = linkonce_odr dso_local unnamed_addr constant [2 x i8] zeroinitializer, comdat, align 1
@__const.verify_in_run.invalid = private unnamed_addr constant [3 x ptr] [ptr @"??_C@_01LIIJDEN@?$AN?$AA@", ptr @"??_C@_01EEMJAFIK@?6?$AA@", ptr @"??_C@_01LOCGONAA@?$AA?$AA@"], align 16
@"??_C@_1IK@HJHFKFIK@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = linkonce_odr dso_local unnamed_addr constant [69 x i16] [i16 120, i16 105, i16 97, i16 111, i16 95, i16 114, i16 117, i16 110, i16 116, i16 105, i16 109, i16 101, i16 95, i16 108, i16 111, i16 103, i16 95, i16 119, i16 114, i16 105, i16 116, i16 101, i16 40, i16 38, i16 108, i16 101, i16 118, i16 101, i16 108, i16 44, i16 32, i16 38, i16 105, i16 110, i16 118, i16 97, i16 108, i16 105, i16 100, i16 95, i16 109, i16 101, i16 115, i16 115, i16 97, i16 103, i16 101, i16 44, i16 32, i16 38, i16 116, i16 97, i16 114, i16 103, i16 101, i16 116, i16 44, i16 32, i16 98, i16 111, i16 116, i16 104, i16 41, i16 32, i16 61, i16 61, i16 32, i16 54, i16 0], comdat, align 2
@"??_C@_1HM@JMFIMOJH@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = linkonce_odr dso_local unnamed_addr constant [62 x i16] [i16 120, i16 105, i16 97, i16 111, i16 95, i16 114, i16 117, i16 110, i16 116, i16 105, i16 109, i16 101, i16 95, i16 108, i16 111, i16 103, i16 95, i16 119, i16 114, i16 105, i16 116, i16 101, i16 40, i16 38, i16 108, i16 101, i16 118, i16 101, i16 108, i16 44, i16 32, i16 38, i16 116, i16 111, i16 111, i16 95, i16 108, i16 111, i16 110, i16 103, i16 44, i16 32, i16 38, i16 116, i16 97, i16 114, i16 103, i16 101, i16 116, i16 44, i16 32, i16 98, i16 111, i16 116, i16 104, i16 41, i16 32, i16 61, i16 61, i16 32, i16 54, i16 0], comdat, align 2
@"??_C@_0BD@BGMAPKLA@LOG_LIMIT_EXCEEDED?$AA@" = linkonce_odr dso_local unnamed_addr constant [19 x i8] c"LOG_LIMIT_EXCEEDED\00", comdat, align 1
@"??_C@_1HE@KCPFPJPJ@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = linkonce_odr dso_local unnamed_addr constant [58 x i16] [i16 120, i16 105, i16 97, i16 111, i16 95, i16 114, i16 117, i16 110, i16 116, i16 105, i16 109, i16 101, i16 95, i16 108, i16 111, i16 103, i16 95, i16 119, i16 114, i16 105, i16 116, i16 101, i16 40, i16 38, i16 108, i16 101, i16 118, i16 101, i16 108, i16 44, i16 32, i16 38, i16 102, i16 117, i16 108, i16 108, i16 44, i16 32, i16 38, i16 116, i16 97, i16 114, i16 103, i16 101, i16 116, i16 44, i16 32, i16 98, i16 111, i16 116, i16 104, i16 41, i16 32, i16 61, i16 61, i16 32, i16 48, i16 0], comdat, align 2
@__const.verify_in_run.invalid_utf8 = private unnamed_addr constant [2 x i8] c"\C0\AF", align 1
@"??_C@_1HO@LGNJPPAO@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAs?$AAt?$AAr?$AAi?$AAn?$AAg?$AA_?$AAn?$AAe?$AAw?$AA?$CI?$AAb?$AAy?$AAt?$AAe?$AAs?$AA?0?$AA?5?$AA?$CG@" = linkonce_odr dso_local unnamed_addr constant [63 x i16] [i16 120, i16 105, i16 97, i16 111, i16 95, i16 114, i16 117, i16 110, i16 116, i16 105, i16 109, i16 101, i16 95, i16 115, i16 116, i16 114, i16 105, i16 110, i16 103, i16 95, i16 110, i16 101, i16 119, i16 40, i16 98, i16 121, i16 116, i16 101, i16 115, i16 44, i16 32, i16 38, i16 104, i16 97, i16 110, i16 100, i16 108, i16 101, i16 41, i16 32, i16 61, i16 61, i16 32, i16 52, i16 32, i16 38, i16 38, i16 32, i16 104, i16 97, i16 110, i16 100, i16 108, i16 101, i16 32, i16 61, i16 61, i16 32, i16 78, i16 85, i16 76, i16 76, i16 0], comdat, align 2
@"??_C@_1HK@LAIHHBJG@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = linkonce_odr dso_local unnamed_addr constant [61 x i16] [i16 120, i16 105, i16 97, i16 111, i16 95, i16 114, i16 117, i16 110, i16 116, i16 105, i16 109, i16 101, i16 95, i16 108, i16 111, i16 103, i16 95, i16 119, i16 114, i16 105, i16 116, i16 101, i16 40, i16 38, i16 108, i16 101, i16 118, i16 101, i16 108, i16 44, i16 32, i16 38, i16 109, i16 101, i16 115, i16 115, i16 97, i16 103, i16 101, i16 44, i16 32, i16 38, i16 116, i16 97, i16 114, i16 103, i16 101, i16 116, i16 44, i16 32, i16 98, i16 111, i16 116, i16 104, i16 41, i16 32, i16 61, i16 61, i16 32, i16 48, i16 0], comdat, align 2
@"??_C@_1HG@HKEJBPI@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@" = linkonce_odr dso_local unnamed_addr constant [59 x i16] [i16 120, i16 105, i16 97, i16 111, i16 95, i16 114, i16 117, i16 110, i16 116, i16 105, i16 109, i16 101, i16 95, i16 108, i16 111, i16 103, i16 95, i16 119, i16 114, i16 105, i16 116, i16 101, i16 40, i16 38, i16 108, i16 101, i16 118, i16 101, i16 108, i16 44, i16 32, i16 38, i16 108, i16 101, i16 118, i16 101, i16 108, i16 44, i16 32, i16 38, i16 116, i16 97, i16 114, i16 103, i16 101, i16 116, i16 44, i16 32, i16 98, i16 111, i16 116, i16 104, i16 41, i16 32, i16 61, i16 61, i16 32, i16 48, i16 0], comdat, align 2
@"??_C@_1HE@MICEHIGB@?$AAm?$AAe?$AAs?$AAs?$AAa?$AAg?$AAe?$AA?4?$AAt?$AAa?$AAg?$AA?5?$AA?$DN?$AA?$DN?$AA?5?$AA8?$AA?5?$AA?$CG?$AA?$CG?$AA?5?$AAm?$AAe?$AAs?$AAs?$AAa?$AAg?$AAe?$AA?4?$AAp?$AAa?$AAy?$AAl@" = linkonce_odr dso_local unnamed_addr constant [58 x i16] [i16 109, i16 101, i16 115, i16 115, i16 97, i16 103, i16 101, i16 46, i16 116, i16 97, i16 103, i16 32, i16 61, i16 61, i16 32, i16 56, i16 32, i16 38, i16 38, i16 32, i16 109, i16 101, i16 115, i16 115, i16 97, i16 103, i16 101, i16 46, i16 112, i16 97, i16 121, i16 108, i16 111, i16 97, i16 100, i16 46, i16 104, i16 97, i16 110, i16 100, i16 108, i16 101, i16 32, i16 61, i16 61, i16 32, i16 111, i16 108, i16 100, i16 95, i16 109, i16 101, i16 115, i16 115, i16 97, i16 103, i16 101, i16 0], comdat, align 2
@"??_C@_1LG@CADPKNEE@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAs?$AAt?$AAr?$AAi?$AAn?$AAg?$AA_?$AAc?$AAo?$AAp?$AAy?$AA?$CI?$AAo?$AAl?$AAd?$AA_?$AAm?$AAe?$AAs@" = linkonce_odr dso_local unnamed_addr constant [91 x i16] [i16 120, i16 105, i16 97, i16 111, i16 95, i16 114, i16 117, i16 110, i16 116, i16 105, i16 109, i16 101, i16 95, i16 115, i16 116, i16 114, i16 105, i16 110, i16 103, i16 95, i16 99, i16 111, i16 112, i16 121, i16 40, i16 111, i16 108, i16 100, i16 95, i16 109, i16 101, i16 115, i16 115, i16 97, i16 103, i16 101, i16 44, i16 32, i16 40, i16 88, i16 105, i16 97, i16 111, i16 65, i16 98, i16 105, i16 77, i16 117, i16 116, i16 66, i16 121, i16 116, i16 101, i16 115, i16 41, i16 123, i16 99, i16 111, i16 112, i16 121, i16 44, i16 32, i16 115, i16 105, i16 122, i16 101, i16 111, i16 102, i16 40, i16 99, i16 111, i16 112, i16 121, i16 41, i16 125, i16 44, i16 32, i16 38, i16 99, i16 111, i16 112, i16 105, i16 101, i16 100, i16 41, i16 32, i16 61, i16 61, i16 32, i16 48, i16 0], comdat, align 2
@"??_C@_1JI@CLAELEBN@?$AAc?$AAo?$AAp?$AAi?$AAe?$AAd?$AA?5?$AA?$DN?$AA?$DN?$AA?5?$AA1?$AA2?$AA?5?$AA?$CG?$AA?$CG?$AA?5?$AAm?$AAe?$AAm?$AAc?$AAm?$AAp?$AA?$CI?$AAc?$AAo?$AAp?$AAy?$AA?0?$AA?5?$AA?$CC?$AAC?$AA?5@" = linkonce_odr dso_local unnamed_addr constant [76 x i16] [i16 99, i16 111, i16 112, i16 105, i16 101, i16 100, i16 32, i16 61, i16 61, i16 32, i16 49, i16 50, i16 32, i16 38, i16 38, i16 32, i16 109, i16 101, i16 109, i16 99, i16 109, i16 112, i16 40, i16 99, i16 111, i16 112, i16 121, i16 44, i16 32, i16 34, i16 67, i16 32, i16 65, i16 66, i16 73, i16 32, i16 92, i16 120, i16 101, i16 54, i16 92, i16 120, i16 98, i16 53, i16 92, i16 120, i16 56, i16 98, i16 92, i16 120, i16 101, i16 56, i16 92, i16 120, i16 97, i16 102, i16 92, i16 120, i16 57, i16 53, i16 34, i16 44, i16 32, i16 99, i16 111, i16 112, i16 105, i16 101, i16 100, i16 41, i16 32, i16 61, i16 61, i16 32, i16 48, i16 0], comdat, align 2
@__const.string_bytes.value = private unnamed_addr constant { i32, [4 x i8], %union.anon } { i32 8, [4 x i8] zeroinitializer, %union.anon zeroinitializer }, align 8
@"??_C@_1HE@LPOLCLJE@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAs?$AAt?$AAr?$AAi?$AAn?$AAg?$AA_?$AAn?$AAe?$AAw?$AA?$CI?$AAv?$AAi?$AAe?$AAw?$AA?0?$AA?5?$AA?$CG?$AAv@" = linkonce_odr dso_local unnamed_addr constant [58 x i16] [i16 120, i16 105, i16 97, i16 111, i16 95, i16 114, i16 117, i16 110, i16 116, i16 105, i16 109, i16 101, i16 95, i16 115, i16 116, i16 114, i16 105, i16 110, i16 103, i16 95, i16 110, i16 101, i16 119, i16 40, i16 118, i16 105, i16 101, i16 119, i16 44, i16 32, i16 38, i16 118, i16 97, i16 108, i16 117, i16 101, i16 46, i16 112, i16 97, i16 121, i16 108, i16 111, i16 97, i16 100, i16 46, i16 104, i16 97, i16 110, i16 100, i16 108, i16 101, i16 41, i16 32, i16 61, i16 61, i16 32, i16 48, i16 0], comdat, align 2
@"??_C@_1FC@IIBCECNM@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAe?$AAr?$AAr?$AAo?$AAr?$AA_?$AAs?$AAn?$AAa?$AAp?$AAs?$AAh?$AAo?$AAt?$AA?$CI?$AA?$CG?$AAe?$AAr?$AAr@" = linkonce_odr dso_local unnamed_addr constant [41 x i16] [i16 120, i16 105, i16 97, i16 111, i16 95, i16 114, i16 117, i16 110, i16 116, i16 105, i16 109, i16 101, i16 95, i16 101, i16 114, i16 114, i16 111, i16 114, i16 95, i16 115, i16 110, i16 97, i16 112, i16 115, i16 104, i16 111, i16 116, i16 40, i16 38, i16 101, i16 114, i16 114, i16 111, i16 114, i16 41, i16 32, i16 61, i16 61, i16 32, i16 48, i16 0], comdat, align 2
@"??_C@_1HK@KELFKMPK@?$AAe?$AAr?$AAr?$AAo?$AAr?$AA?4?$AAc?$AAl?$AAa?$AAs?$AAs?$AA_?$AAt?$AAa?$AAg?$AA?5?$AA?$DN?$AA?$DN?$AA?5?$AAc?$AAl?$AAa?$AAs?$AAs?$AA_?$AAt?$AAa?$AAg?$AA?5?$AA?$CG?$AA?$CG?$AA?5@" = linkonce_odr dso_local unnamed_addr constant [61 x i16] [i16 101, i16 114, i16 114, i16 111, i16 114, i16 46, i16 99, i16 108, i16 97, i16 115, i16 115, i16 95, i16 116, i16 97, i16 103, i16 32, i16 61, i16 61, i16 32, i16 99, i16 108, i16 97, i16 115, i16 115, i16 95, i16 116, i16 97, i16 103, i16 32, i16 38, i16 38, i16 32, i16 101, i16 114, i16 114, i16 111, i16 114, i16 46, i16 101, i16 120, i16 105, i16 116, i16 95, i16 99, i16 111, i16 100, i16 101, i16 32, i16 61, i16 61, i16 32, i16 101, i16 120, i16 105, i16 116, i16 95, i16 99, i16 111, i16 100, i16 101, i16 0], comdat, align 2
@"??_C@_1DO@BNAPIDAO@?$AAe?$AAr?$AAr?$AAo?$AAr?$AA?4?$AAc?$AAo?$AAd?$AAe?$AA?4?$AAl?$AAe?$AAn?$AA?5?$AA?$DN?$AA?$DN?$AA?5?$AAs?$AAt?$AAr?$AAl?$AAe?$AAn?$AA?$CI?$AAc?$AAo?$AAd?$AAe?$AA?$CJ?$AA?$AA@" = linkonce_odr dso_local unnamed_addr constant [31 x i16] [i16 101, i16 114, i16 114, i16 111, i16 114, i16 46, i16 99, i16 111, i16 100, i16 101, i16 46, i16 108, i16 101, i16 110, i16 32, i16 61, i16 61, i16 32, i16 115, i16 116, i16 114, i16 108, i16 101, i16 110, i16 40, i16 99, i16 111, i16 100, i16 101, i16 41, i16 0], comdat, align 2
@"??_C@_1GE@HBCAKALM@?$AAm?$AAe?$AAm?$AAc?$AAm?$AAp?$AA?$CI?$AAe?$AAr?$AAr?$AAo?$AAr?$AA?4?$AAc?$AAo?$AAd?$AAe?$AA?4?$AAp?$AAt?$AAr?$AA?0?$AA?5?$AAc?$AAo?$AAd?$AAe?$AA?0?$AA?5?$AAe?$AAr?$AAr@" = linkonce_odr dso_local unnamed_addr constant [50 x i16] [i16 109, i16 101, i16 109, i16 99, i16 109, i16 112, i16 40, i16 101, i16 114, i16 114, i16 111, i16 114, i16 46, i16 99, i16 111, i16 100, i16 101, i16 46, i16 112, i16 116, i16 114, i16 44, i16 32, i16 99, i16 111, i16 100, i16 101, i16 44, i16 32, i16 101, i16 114, i16 114, i16 111, i16 114, i16 46, i16 99, i16 111, i16 100, i16 101, i16 46, i16 108, i16 101, i16 110, i16 41, i16 32, i16 61, i16 61, i16 32, i16 48, i16 0], comdat, align 2
@"??_C@_1FG@BCDAGFCH@?$AAv?$AAa?$AAl?$AAu?$AAe?$AA?9?$AA?$DO?$AAt?$AAa?$AAg?$AA?5?$AA?$DN?$AA?$DN?$AA?5?$AA0?$AA?5?$AA?$CG?$AA?$CG?$AA?5?$AAv?$AAa?$AAl?$AAu?$AAe?$AA?9?$AA?$DO?$AAp?$AAa?$AAy?$AAl?$AAo?$AAa@" = linkonce_odr dso_local unnamed_addr constant [43 x i16] [i16 118, i16 97, i16 108, i16 117, i16 101, i16 45, i16 62, i16 116, i16 97, i16 103, i16 32, i16 61, i16 61, i16 32, i16 48, i16 32, i16 38, i16 38, i16 32, i16 118, i16 97, i16 108, i16 117, i16 101, i16 45, i16 62, i16 112, i16 97, i16 121, i16 108, i16 111, i16 97, i16 100, i16 46, i16 114, i16 97, i16 119, i16 32, i16 61, i16 61, i16 32, i16 48, i16 0], comdat, align 2

; Function Attrs: noinline nounwind optnone uwtable
define linkonce_odr dso_local i32 @sprintf(ptr noundef %0, ptr noundef %1, ...) #0 comdat {
  %3 = alloca ptr, align 8
  %4 = alloca ptr, align 8
  %5 = alloca i32, align 4
  %6 = alloca ptr, align 8
  store ptr %1, ptr %3, align 8
  store ptr %0, ptr %4, align 8
  call void @llvm.va_start.p0(ptr %6)
  %7 = load ptr, ptr %6, align 8
  %8 = load ptr, ptr %3, align 8
  %9 = load ptr, ptr %4, align 8
  %10 = call i32 @_vsprintf_l(ptr noundef %9, ptr noundef %8, ptr noundef null, ptr noundef %7)
  store i32 %10, ptr %5, align 4
  call void @llvm.va_end.p0(ptr %6)
  %11 = load i32, ptr %5, align 4
  ret i32 %11
}

; Function Attrs: noinline nounwind optnone uwtable
define linkonce_odr dso_local i32 @vsprintf(ptr noundef %0, ptr noundef %1, ptr noundef %2) #0 comdat {
  %4 = alloca ptr, align 8
  %5 = alloca ptr, align 8
  %6 = alloca ptr, align 8
  store ptr %2, ptr %4, align 8
  store ptr %1, ptr %5, align 8
  store ptr %0, ptr %6, align 8
  %7 = load ptr, ptr %4, align 8
  %8 = load ptr, ptr %5, align 8
  %9 = load ptr, ptr %6, align 8
  %10 = call i32 @_vsnprintf_l(ptr noundef %9, i64 noundef -1, ptr noundef %8, ptr noundef null, ptr noundef %7)
  ret i32 %10
}

; Function Attrs: noinline nounwind optnone uwtable
define linkonce_odr dso_local i32 @_snprintf(ptr noundef %0, i64 noundef %1, ptr noundef %2, ...) #0 comdat {
  %4 = alloca ptr, align 8
  %5 = alloca i64, align 8
  %6 = alloca ptr, align 8
  %7 = alloca i32, align 4
  %8 = alloca ptr, align 8
  store ptr %2, ptr %4, align 8
  store i64 %1, ptr %5, align 8
  store ptr %0, ptr %6, align 8
  call void @llvm.va_start.p0(ptr %8)
  %9 = load ptr, ptr %8, align 8
  %10 = load ptr, ptr %4, align 8
  %11 = load i64, ptr %5, align 8
  %12 = load ptr, ptr %6, align 8
  %13 = call i32 @_vsnprintf(ptr noundef %12, i64 noundef %11, ptr noundef %10, ptr noundef %9)
  store i32 %13, ptr %7, align 4
  call void @llvm.va_end.p0(ptr %8)
  %14 = load i32, ptr %7, align 4
  ret i32 %14
}

; Function Attrs: noinline nounwind optnone uwtable
define linkonce_odr dso_local i32 @_vsnprintf(ptr noundef %0, i64 noundef %1, ptr noundef %2, ptr noundef %3) #0 comdat {
  %5 = alloca ptr, align 8
  %6 = alloca ptr, align 8
  %7 = alloca i64, align 8
  %8 = alloca ptr, align 8
  store ptr %3, ptr %5, align 8
  store ptr %2, ptr %6, align 8
  store i64 %1, ptr %7, align 8
  store ptr %0, ptr %8, align 8
  %9 = load ptr, ptr %5, align 8
  %10 = load ptr, ptr %6, align 8
  %11 = load i64, ptr %7, align 8
  %12 = load ptr, ptr %8, align 8
  %13 = call i32 @_vsnprintf_l(ptr noundef %12, i64 noundef %11, ptr noundef %10, ptr noundef null, ptr noundef %9)
  ret i32 %13
}

; Function Attrs: noinline nounwind optnone uwtable
define dso_local i32 @main(i32 noundef %0, ptr noundef %1) #0 {
  %3 = alloca i32, align 4
  %4 = alloca ptr, align 8
  %5 = alloca i32, align 4
  %6 = alloca %struct.XiaoValue, align 8
  %7 = alloca %struct.XiaoValue, align 8
  %8 = alloca %struct.XiaoValue, align 8
  store i32 0, ptr %3, align 4
  store ptr %1, ptr %4, align 8
  store i32 %0, ptr %5, align 4
  call void @xiao_runtime_release_trace_begin()
  %9 = load i32, ptr %5, align 4
  %10 = icmp eq i32 %9, 2
  %11 = zext i1 %10 to i32
  store i32 %11, ptr @output_failed, align 4
  %12 = load i32, ptr @output_failed, align 4
  %13 = icmp ne i32 %12, 0
  br i1 %13, label %14, label %27

14:                                               ; preds = %2
  %15 = load ptr, ptr %4, align 8
  %16 = getelementptr inbounds ptr, ptr %15, i64 1
  %17 = load ptr, ptr %16, align 8
  %18 = call i32 @strcmp(ptr noundef %17, ptr noundef @"??_C@_06IKJOPBFO@closed?$AA@") #6
  %19 = icmp eq i32 %18, 0
  br i1 %19, label %20, label %27

20:                                               ; preds = %14
  %21 = call i32 @SetStdHandle(i32 noundef -11, ptr noundef null)
  %22 = icmp ne i32 %21, 0
  br i1 %22, label %24, label %23

23:                                               ; preds = %20
  call void @_wassert(ptr noundef @"??_C@_1EM@GJOJCMJN@?$AAS?$AAe?$AAt?$AAS?$AAt?$AAd?$AAH?$AAa?$AAn?$AAd?$AAl?$AAe?$AA?$CI?$AAS?$AAT?$AAD?$AA_?$AAO?$AAU?$AAT?$AAP?$AAU?$AAT?$AA_?$AAH?$AAA?$AAN?$AAD?$AAL?$AAE?$AA?0?$AA?5@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 164)
  br label %24

24:                                               ; preds = %23, %20
  %25 = phi i1 [ true, %20 ], [ false, %23 ]
  %26 = zext i1 %25 to i32
  br label %27

27:                                               ; preds = %24, %14, %2
  %28 = call i32 @xiao_runtime_env_run_injected(ptr noundef @verify_in_run, ptr noundef null, i64 noundef 0, i64 noundef 34)
  %29 = icmp eq i32 %28, 0
  br i1 %29, label %31, label %30

30:                                               ; preds = %27
  call void @_wassert(ptr noundef @"??_C@_1IC@OPJHALMA@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAe?$AAn?$AAv?$AA_?$AAr?$AAu?$AAn?$AA_?$AAi?$AAn?$AAj?$AAe?$AAc?$AAt?$AAe?$AAd?$AA?$CI?$AAv?$AAe@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 169)
  br label %31

31:                                               ; preds = %30, %27
  %32 = phi i1 [ true, %27 ], [ false, %30 ]
  %33 = zext i1 %32 to i32
  %34 = load i32, ptr @output_failed, align 4
  %35 = icmp ne i32 %34, 0
  br i1 %35, label %43, label %36

36:                                               ; preds = %31
  call void @string(ptr dead_on_unwind writable sret(%struct.XiaoValue) align 8 %6, ptr noundef @"??_C@_04NIDJFNBE@info?$AA@")
  call void @string(ptr dead_on_unwind writable sret(%struct.XiaoValue) align 8 %7, ptr noundef @"??_C@_07OEIDNODA@unbound?$AA@")
  call void @string(ptr dead_on_unwind writable sret(%struct.XiaoValue) align 8 %8, ptr noundef @"??_C@_06DJDOIFHB@stdout?$AA@")
  %37 = call i32 @xiao_runtime_log_write(ptr noundef %6, ptr noundef %7, ptr noundef %8, i64 noundef 34)
  %38 = icmp eq i32 %37, 8
  br i1 %38, label %40, label %39

39:                                               ; preds = %36
  call void @_wassert(ptr noundef @"??_C@_1HK@HFDDFJHJ@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 174)
  br label %40

40:                                               ; preds = %39, %36
  %41 = phi i1 [ true, %36 ], [ false, %39 ]
  %42 = zext i1 %41 to i32
  call void @pending(i32 noundef 2, ptr noundef null, i32 noundef 4)
  call void @release(ptr noundef %8)
  call void @release(ptr noundef %7)
  call void @release(ptr noundef %6)
  br label %43

43:                                               ; preds = %40, %31
  call void @xiao_runtime_release_trace_flush()
  %44 = load i32, ptr @output_failed, align 4
  %45 = icmp ne i32 %44, 0
  %46 = zext i1 %45 to i64
  %47 = select i1 %45, ptr @"??_C@_0BJ@NALBOBEG@C?5ABI?3?5LOG_OUTPUT_FAILED?$AA@", ptr @"??_C@_0DG@NIGKCKPN@C?5ABI?3?50?11?12?16?18?0?5pending?0?5borro@"
  %48 = call ptr @__acrt_iob_func(i32 noundef 2)
  %49 = call i32 (ptr, ptr, ...) @fprintf(ptr noundef %48, ptr noundef @"??_C@_03OFAPEBGM@?$CFs?6?$AA@", ptr noundef %47) #6
  ret i32 0
}

declare dso_local void @xiao_runtime_release_trace_begin() #1

; Function Attrs: nounwind
declare dso_local i32 @strcmp(ptr noundef, ptr noundef) #2

declare dllimport i32 @SetStdHandle(i32 noundef, ptr noundef) #1

declare dso_local void @_wassert(ptr noundef, ptr noundef, i32 noundef) #1

declare dso_local i32 @xiao_runtime_env_run_injected(ptr noundef, ptr noundef, i64 noundef, i64 noundef) #1

; Function Attrs: noinline nounwind optnone uwtable
define internal i32 @verify_in_run() #0 {
  %1 = alloca %struct.XiaoValue, align 8
  %2 = alloca %struct.XiaoValue, align 8
  %3 = alloca %struct.XiaoValue, align 8
  %4 = alloca %struct.XiaoValue, align 8
  %5 = alloca %struct.XiaoValue, align 8
  %6 = alloca %struct.XiaoValue, align 8
  %7 = alloca i64, align 8
  %8 = alloca [3 x ptr], align 16
  %9 = alloca i64, align 8
  %10 = alloca %struct.XiaoValue, align 8
  %11 = alloca [4097 x i8], align 16
  %12 = alloca %struct.XiaoValue, align 8
  %13 = alloca %struct.XiaoValue, align 8
  %14 = alloca [2 x i8], align 1
  %15 = alloca %struct.XiaoAbiBytes, align 8
  %16 = alloca ptr, align 8
  %17 = alloca %struct.XiaoAbiBytes, align 8
  %18 = alloca ptr, align 8
  %19 = alloca [32 x i8], align 16
  %20 = alloca i64, align 8
  %21 = alloca %struct.XiaoAbiMutBytes, align 8
  %22 = alloca %struct.XiaoAbiMutBytes, align 8
  call void @string(ptr dead_on_unwind writable sret(%struct.XiaoValue) align 8 %1, ptr noundef @"??_C@_04NIDJFNBE@info?$AA@")
  call void @string(ptr dead_on_unwind writable sret(%struct.XiaoValue) align 8 %2, ptr noundef @"??_C@_0N@MPKLMEOF@C?5ABI?5?f?$LF?$IL?h?$KP?$JF?$AA@")
  call void @string(ptr dead_on_unwind writable sret(%struct.XiaoValue) align 8 %3, ptr noundef @"??_C@_06DJDOIFHB@stdout?$AA@")
  %23 = load i32, ptr @output_failed, align 4
  %24 = icmp ne i32 %23, 0
  br i1 %24, label %25, label %32

25:                                               ; preds = %0
  %26 = call i32 @xiao_runtime_log_write(ptr noundef %1, ptr noundef %2, ptr noundef %3, i64 noundef 34)
  %27 = icmp eq i32 %26, 6
  br i1 %27, label %29, label %28

28:                                               ; preds = %25
  call void @_wassert(ptr noundef @"??_C@_1HK@JFOMCOEK@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 94)
  br label %29

29:                                               ; preds = %28, %25
  %30 = phi i1 [ true, %25 ], [ false, %28 ]
  %31 = zext i1 %30 to i32
  call void @pending(i32 noundef 1, ptr noundef @"??_C@_0BC@JBLGAAOK@LOG_OUTPUT_FAILED?$AA@", i32 noundef 3)
  br label %181

32:                                               ; preds = %0
  call void @llvm.memcpy.p0.p0.i64(ptr align 8 %4, ptr align 8 @__const.verify_in_run.unknown, i64 16, i1 false)
  call void @llvm.memcpy.p0.p0.i64(ptr align 8 %5, ptr align 8 @__const.verify_in_run.bad_shape, i64 16, i1 false)
  call void @llvm.memcpy.p0.p0.i64(ptr align 8 %6, ptr align 8 @__const.verify_in_run.wrong_type, i64 16, i1 false)
  %33 = call i32 @xiao_runtime_log_write(ptr noundef null, ptr noundef %2, ptr noundef %3, i64 noundef 34)
  %34 = icmp eq i32 %33, 1
  br i1 %34, label %36, label %35

35:                                               ; preds = %32
  call void @_wassert(ptr noundef @"??_C@_1HG@IJIKIDDE@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AAN?$AAU?$AAL?$AAL?$AA?0?$AA?5?$AA?$CG?$AAm?$AAe@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 100)
  br label %36

36:                                               ; preds = %35, %32
  %37 = phi i1 [ true, %32 ], [ false, %35 ]
  %38 = zext i1 %37 to i32
  call void @pending(i32 noundef 1, ptr noundef @"??_C@_0BA@PILDGKCH@X06?9RUNTIME?9012?$AA@", i32 noundef 3)
  %39 = call i32 @xiao_runtime_log_write(ptr noundef %1, ptr noundef %5, ptr noundef %3, i64 noundef 34)
  %40 = icmp eq i32 %39, 1
  br i1 %40, label %42, label %41

41:                                               ; preds = %36
  call void @_wassert(ptr noundef @"??_C@_1HO@ICGHHMKE@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 102)
  br label %42

42:                                               ; preds = %41, %36
  %43 = phi i1 [ true, %36 ], [ false, %41 ]
  %44 = zext i1 %43 to i32
  call void @pending(i32 noundef 1, ptr noundef @"??_C@_0BA@PILDGKCH@X06?9RUNTIME?9012?$AA@", i32 noundef 3)
  %45 = call i32 @xiao_runtime_log_write(ptr noundef %1, ptr noundef %2, ptr noundef %4, i64 noundef 34)
  %46 = icmp eq i32 %45, 2
  br i1 %46, label %48, label %47

47:                                               ; preds = %42
  call void @_wassert(ptr noundef @"??_C@_1HM@LALDBKGK@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 104)
  br label %48

48:                                               ; preds = %47, %42
  %49 = phi i1 [ true, %42 ], [ false, %47 ]
  %50 = zext i1 %49 to i32
  call void @pending(i32 noundef 1, ptr noundef @"??_C@_0BA@PILDGKCH@X06?9RUNTIME?9012?$AA@", i32 noundef 3)
  %51 = call i32 @xiao_runtime_log_write(ptr noundef %1, ptr noundef %2, ptr noundef %3, i64 noundef 290)
  %52 = icmp eq i32 %51, 2
  br i1 %52, label %54, label %53

53:                                               ; preds = %48
  call void @_wassert(ptr noundef @"??_C@_1IK@POPLDDMN@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 106)
  br label %54

54:                                               ; preds = %53, %48
  %55 = phi i1 [ true, %48 ], [ false, %53 ]
  %56 = zext i1 %55 to i32
  call void @pending(i32 noundef 1, ptr noundef @"??_C@_0BA@PILDGKCH@X06?9RUNTIME?9012?$AA@", i32 noundef 3)
  %57 = call i32 @xiao_runtime_log_write(ptr noundef %6, ptr noundef %2, ptr noundef %3, i64 noundef 0)
  %58 = icmp eq i32 %57, 6
  br i1 %58, label %60, label %59

59:                                               ; preds = %54
  call void @_wassert(ptr noundef @"??_C@_1HO@LBMLBEBC@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAw?$AAr?$AAo?$AAn?$AAg?$AA_?$AAt?$AAy@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 108)
  br label %60

60:                                               ; preds = %59, %54
  %61 = phi i1 [ true, %54 ], [ false, %59 ]
  %62 = zext i1 %61 to i32
  call void @pending(i32 noundef 1, ptr noundef @"??_C@_0BA@PJHBAABA@X06?9RUNTIME?9002?$AA@", i32 noundef 3)
  store i64 2, ptr %7, align 8
  br label %63

63:                                               ; preds = %74, %60
  %64 = load i64, ptr %7, align 8
  %65 = icmp ule i64 %64, 32
  br i1 %65, label %66, label %77

66:                                               ; preds = %63
  %67 = load i64, ptr %7, align 8
  %68 = call i32 @xiao_runtime_log_write(ptr noundef %1, ptr noundef %2, ptr noundef %3, i64 noundef %67)
  %69 = icmp eq i32 %68, 6
  br i1 %69, label %71, label %70

70:                                               ; preds = %66
  call void @_wassert(ptr noundef @"??_C@_1HK@JIGIMDPL@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 111)
  br label %71

71:                                               ; preds = %70, %66
  %72 = phi i1 [ true, %66 ], [ false, %70 ]
  %73 = zext i1 %72 to i32
  call void @pending(i32 noundef 1, ptr noundef @"??_C@_0BA@JOJJBBHK@X06?9RUNTIME?9026?$AA@", i32 noundef 3)
  br label %74

74:                                               ; preds = %71
  %75 = load i64, ptr %7, align 8
  %76 = shl i64 %75, 4
  store i64 %76, ptr %7, align 8
  br label %63, !llvm.loop !9

77:                                               ; preds = %63
  %78 = call i32 @xiao_runtime_log_write(ptr noundef %3, ptr noundef %2, ptr noundef %1, i64 noundef 34)
  %79 = icmp eq i32 %78, 6
  br i1 %79, label %81, label %80

80:                                               ; preds = %77
  call void @_wassert(ptr noundef @"??_C@_1HK@GLHINJBC@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAt?$AAa?$AAr?$AAg?$AAe?$AAt?$AA?0?$AA?5@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 114)
  br label %81

81:                                               ; preds = %80, %77
  %82 = phi i1 [ true, %77 ], [ false, %80 ]
  %83 = zext i1 %82 to i32
  call void @pending(i32 noundef 1, ptr noundef @"??_C@_0BA@PILDGKCH@X06?9RUNTIME?9012?$AA@", i32 noundef 3)
  %84 = call i32 @xiao_runtime_log_write(ptr noundef %1, ptr noundef %2, ptr noundef %2, i64 noundef 34)
  %85 = icmp eq i32 %84, 6
  br i1 %85, label %87, label %86

86:                                               ; preds = %81
  call void @_wassert(ptr noundef @"??_C@_1HM@GDLFFFKF@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 116)
  br label %87

87:                                               ; preds = %86, %81
  %88 = phi i1 [ true, %81 ], [ false, %86 ]
  %89 = zext i1 %88 to i32
  call void @pending(i32 noundef 1, ptr noundef @"??_C@_0BA@PILDGKCH@X06?9RUNTIME?9012?$AA@", i32 noundef 3)
  call void @llvm.memcpy.p0.p0.i64(ptr align 16 %8, ptr align 16 @__const.verify_in_run.invalid, i64 24, i1 false)
  store i64 0, ptr %9, align 8
  br label %90

90:                                               ; preds = %103, %87
  %91 = load i64, ptr %9, align 8
  %92 = icmp ult i64 %91, 3
  br i1 %92, label %93, label %106

93:                                               ; preds = %90
  %94 = load i64, ptr %9, align 8
  %95 = getelementptr inbounds nuw [3 x ptr], ptr %8, i64 0, i64 %94
  %96 = load ptr, ptr %95, align 8
  call void @string_bytes(ptr dead_on_unwind writable sret(%struct.XiaoValue) align 8 %10, ptr noundef %96, i64 noundef 1)
  %97 = call i32 @xiao_runtime_log_write(ptr noundef %1, ptr noundef %10, ptr noundef %3, i64 noundef 34)
  %98 = icmp eq i32 %97, 6
  br i1 %98, label %100, label %99

99:                                               ; preds = %93
  call void @_wassert(ptr noundef @"??_C@_1IK@HJHFKFIK@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 121)
  br label %100

100:                                              ; preds = %99, %93
  %101 = phi i1 [ true, %93 ], [ false, %99 ]
  %102 = zext i1 %101 to i32
  call void @pending(i32 noundef 1, ptr noundef @"??_C@_0BA@PILDGKCH@X06?9RUNTIME?9012?$AA@", i32 noundef 3)
  call void @release(ptr noundef %10)
  br label %103

103:                                              ; preds = %100
  %104 = load i64, ptr %9, align 8
  %105 = add i64 %104, 1
  store i64 %105, ptr %9, align 8
  br label %90, !llvm.loop !11

106:                                              ; preds = %90
  %107 = getelementptr inbounds [4097 x i8], ptr %11, i64 0, i64 0
  call void @llvm.memset.p0.i64(ptr align 16 %107, i8 97, i64 4097, i1 false)
  %108 = getelementptr inbounds [4097 x i8], ptr %11, i64 0, i64 0
  call void @string_bytes(ptr dead_on_unwind writable sret(%struct.XiaoValue) align 8 %12, ptr noundef %108, i64 noundef 4097)
  %109 = call i32 @xiao_runtime_log_write(ptr noundef %1, ptr noundef %12, ptr noundef %3, i64 noundef 34)
  %110 = icmp eq i32 %109, 6
  br i1 %110, label %112, label %111

111:                                              ; preds = %106
  call void @_wassert(ptr noundef @"??_C@_1HM@JMFIMOJH@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 128)
  br label %112

112:                                              ; preds = %111, %106
  %113 = phi i1 [ true, %106 ], [ false, %111 ]
  %114 = zext i1 %113 to i32
  call void @pending(i32 noundef 1, ptr noundef @"??_C@_0BD@BGMAPKLA@LOG_LIMIT_EXCEEDED?$AA@", i32 noundef 3)
  call void @release(ptr noundef %12)
  %115 = getelementptr inbounds [4097 x i8], ptr %11, i64 0, i64 0
  call void @string_bytes(ptr dead_on_unwind writable sret(%struct.XiaoValue) align 8 %13, ptr noundef %115, i64 noundef 4096)
  %116 = call i32 @xiao_runtime_log_write(ptr noundef %1, ptr noundef %13, ptr noundef %3, i64 noundef 34)
  %117 = icmp eq i32 %116, 0
  br i1 %117, label %119, label %118

118:                                              ; preds = %112
  call void @_wassert(ptr noundef @"??_C@_1HE@KCPFPJPJ@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 132)
  br label %119

119:                                              ; preds = %118, %112
  %120 = phi i1 [ true, %112 ], [ false, %118 ]
  %121 = zext i1 %120 to i32
  call void @pending(i32 noundef 0, ptr noundef null, i32 noundef 0)
  call void @release(ptr noundef %13)
  call void @llvm.memcpy.p0.p0.i64(ptr align 1 %14, ptr align 1 @__const.verify_in_run.invalid_utf8, i64 2, i1 false)
  %122 = getelementptr inbounds nuw %struct.XiaoAbiBytes, ptr %15, i32 0, i32 0
  %123 = getelementptr inbounds [2 x i8], ptr %14, i64 0, i64 0
  store ptr %123, ptr %122, align 8
  %124 = getelementptr inbounds nuw %struct.XiaoAbiBytes, ptr %15, i32 0, i32 1
  store i64 2, ptr %124, align 8
  store ptr null, ptr %16, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr align 8 %17, ptr align 8 %15, i64 16, i1 false)
  %125 = call i32 @xiao_runtime_string_new(ptr dead_on_return noundef %17, ptr noundef %16)
  %126 = icmp eq i32 %125, 4
  br i1 %126, label %127, label %130

127:                                              ; preds = %119
  %128 = load ptr, ptr %16, align 8
  %129 = icmp eq ptr %128, null
  br i1 %129, label %131, label %130

130:                                              ; preds = %127, %119
  call void @_wassert(ptr noundef @"??_C@_1HO@LGNJPPAO@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAs?$AAt?$AAr?$AAi?$AAn?$AAg?$AA_?$AAn?$AAe?$AAw?$AA?$CI?$AAb?$AAy?$AAt?$AAe?$AAs?$AA?0?$AA?5?$AA?$CG@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 139)
  br label %131

131:                                              ; preds = %130, %127
  %132 = phi i1 [ true, %127 ], [ false, %130 ]
  %133 = zext i1 %132 to i32
  call void @pending(i32 noundef 0, ptr noundef null, i32 noundef 0)
  %134 = getelementptr inbounds nuw %struct.XiaoValue, ptr %2, i32 0, i32 1
  %135 = load ptr, ptr %134, align 8
  store ptr %135, ptr %18, align 8
  %136 = call i32 @xiao_runtime_log_write(ptr noundef %1, ptr noundef %2, ptr noundef %3, i64 noundef 34)
  %137 = icmp eq i32 %136, 0
  br i1 %137, label %139, label %138

138:                                              ; preds = %131
  call void @_wassert(ptr noundef @"??_C@_1HK@LAIHHBJG@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 142)
  br label %139

139:                                              ; preds = %138, %131
  %140 = phi i1 [ true, %131 ], [ false, %138 ]
  %141 = zext i1 %140 to i32
  call void @pending(i32 noundef 0, ptr noundef null, i32 noundef 0)
  %142 = call i32 @xiao_runtime_log_write(ptr noundef %1, ptr noundef %1, ptr noundef %3, i64 noundef 34)
  %143 = icmp eq i32 %142, 0
  br i1 %143, label %145, label %144

144:                                              ; preds = %139
  call void @_wassert(ptr noundef @"??_C@_1HG@HKEJBPI@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAl?$AAo?$AAg?$AA_?$AAw?$AAr?$AAi?$AAt?$AAe?$AA?$CI?$AA?$CG?$AAl?$AAe?$AAv?$AAe?$AAl?$AA?0?$AA?5?$AA?$CG@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 144)
  br label %145

145:                                              ; preds = %144, %139
  %146 = phi i1 [ true, %139 ], [ false, %144 ]
  %147 = zext i1 %146 to i32
  call void @pending(i32 noundef 0, ptr noundef null, i32 noundef 0)
  %148 = getelementptr inbounds nuw %struct.XiaoValue, ptr %2, i32 0, i32 0
  %149 = load i32, ptr %148, align 8
  %150 = icmp eq i32 %149, 8
  br i1 %150, label %151, label %156

151:                                              ; preds = %145
  %152 = getelementptr inbounds nuw %struct.XiaoValue, ptr %2, i32 0, i32 1
  %153 = load ptr, ptr %152, align 8
  %154 = load ptr, ptr %18, align 8
  %155 = icmp eq ptr %153, %154
  br i1 %155, label %157, label %156

156:                                              ; preds = %151, %145
  call void @_wassert(ptr noundef @"??_C@_1HE@MICEHIGB@?$AAm?$AAe?$AAs?$AAs?$AAa?$AAg?$AAe?$AA?4?$AAt?$AAa?$AAg?$AA?5?$AA?$DN?$AA?$DN?$AA?5?$AA8?$AA?5?$AA?$CG?$AA?$CG?$AA?5?$AAm?$AAe?$AAs?$AAs?$AAa?$AAg?$AAe?$AA?4?$AAp?$AAa?$AAy?$AAl@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 146)
  br label %157

157:                                              ; preds = %156, %151
  %158 = phi i1 [ true, %151 ], [ false, %156 ]
  %159 = zext i1 %158 to i32
  store i64 0, ptr %20, align 8
  %160 = getelementptr inbounds nuw %struct.XiaoAbiMutBytes, ptr %21, i32 0, i32 0
  %161 = getelementptr inbounds [32 x i8], ptr %19, i64 0, i64 0
  store ptr %161, ptr %160, align 8
  %162 = getelementptr inbounds nuw %struct.XiaoAbiMutBytes, ptr %21, i32 0, i32 1
  store i64 32, ptr %162, align 8
  %163 = load ptr, ptr %18, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr align 8 %22, ptr align 8 %21, i64 16, i1 false)
  %164 = call i32 @xiao_runtime_string_copy(ptr noundef %163, ptr dead_on_return noundef %22, ptr noundef %20)
  %165 = icmp eq i32 %164, 0
  br i1 %165, label %167, label %166

166:                                              ; preds = %157
  call void @_wassert(ptr noundef @"??_C@_1LG@CADPKNEE@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAs?$AAt?$AAr?$AAi?$AAn?$AAg?$AA_?$AAc?$AAo?$AAp?$AAy?$AA?$CI?$AAo?$AAl?$AAd?$AA_?$AAm?$AAe?$AAs@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 150)
  br label %167

167:                                              ; preds = %166, %157
  %168 = phi i1 [ true, %157 ], [ false, %166 ]
  %169 = zext i1 %168 to i32
  %170 = load i64, ptr %20, align 8
  %171 = icmp eq i64 %170, 12
  br i1 %171, label %172, label %177

172:                                              ; preds = %167
  %173 = load i64, ptr %20, align 8
  %174 = getelementptr inbounds [32 x i8], ptr %19, i64 0, i64 0
  %175 = call i32 @memcmp(ptr noundef %174, ptr noundef @"??_C@_0N@MPKLMEOF@C?5ABI?5?f?$LF?$IL?h?$KP?$JF?$AA@", i64 noundef %173)
  %176 = icmp eq i32 %175, 0
  br i1 %176, label %178, label %177

177:                                              ; preds = %172, %167
  call void @_wassert(ptr noundef @"??_C@_1JI@CLAELEBN@?$AAc?$AAo?$AAp?$AAi?$AAe?$AAd?$AA?5?$AA?$DN?$AA?$DN?$AA?5?$AA1?$AA2?$AA?5?$AA?$CG?$AA?$CG?$AA?5?$AAm?$AAe?$AAm?$AAc?$AAm?$AAp?$AA?$CI?$AAc?$AAo?$AAp?$AAy?$AA?0?$AA?5?$AA?$CC?$AAC?$AA?5@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 151)
  br label %178

178:                                              ; preds = %177, %172
  %179 = phi i1 [ true, %172 ], [ false, %177 ]
  %180 = zext i1 %179 to i32
  br label %181

181:                                              ; preds = %178, %29
  call void @release(ptr noundef %3)
  call void @release(ptr noundef %2)
  call void @release(ptr noundef %1)
  ret i32 0
}

; Function Attrs: noinline nounwind optnone uwtable
define internal void @string(ptr dead_on_unwind noalias writable sret(%struct.XiaoValue) align 8 %0, ptr noundef %1) #0 {
  %3 = alloca ptr, align 8
  %4 = alloca ptr, align 8
  store ptr %0, ptr %3, align 8
  store ptr %1, ptr %4, align 8
  %5 = load ptr, ptr %4, align 8
  %6 = call i64 @strlen(ptr noundef %5) #6
  %7 = load ptr, ptr %4, align 8
  call void @string_bytes(ptr dead_on_unwind writable sret(%struct.XiaoValue) align 8 %0, ptr noundef %7, i64 noundef %6)
  ret void
}

declare dso_local i32 @xiao_runtime_log_write(ptr noundef, ptr noundef, ptr noundef, i64 noundef) #1

; Function Attrs: noinline nounwind optnone uwtable
define internal void @pending(i32 noundef %0, ptr noundef %1, i32 noundef %2) #0 {
  %4 = alloca i32, align 4
  %5 = alloca ptr, align 8
  %6 = alloca i32, align 4
  %7 = alloca %struct.XiaoAbiErrorSnapshot, align 8
  store i32 %2, ptr %4, align 4
  store ptr %1, ptr %5, align 8
  store i32 %0, ptr %6, align 4
  call void @llvm.memset.p0.i64(ptr align 8 %7, i8 0, i64 88, i1 false)
  %8 = call i32 @xiao_runtime_error_snapshot(ptr noundef %7)
  %9 = icmp eq i32 %8, 0
  br i1 %9, label %11, label %10

10:                                               ; preds = %3
  call void @_wassert(ptr noundef @"??_C@_1FC@IIBCECNM@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAe?$AAr?$AAr?$AAo?$AAr?$AA_?$AAs?$AAn?$AAa?$AAp?$AAs?$AAh?$AAo?$AAt?$AA?$CI?$AA?$CG?$AAe?$AAr?$AAr@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 75)
  br label %11

11:                                               ; preds = %10, %3
  %12 = phi i1 [ true, %3 ], [ false, %10 ]
  %13 = zext i1 %12 to i32
  %14 = getelementptr inbounds nuw %struct.XiaoAbiErrorSnapshot, ptr %7, i32 0, i32 0
  %15 = load i32, ptr %14, align 8
  %16 = load i32, ptr %6, align 4
  %17 = icmp eq i32 %15, %16
  br i1 %17, label %18, label %23

18:                                               ; preds = %11
  %19 = getelementptr inbounds nuw %struct.XiaoAbiErrorSnapshot, ptr %7, i32 0, i32 6
  %20 = load i32, ptr %19, align 8
  %21 = load i32, ptr %4, align 4
  %22 = icmp eq i32 %20, %21
  br i1 %22, label %24, label %23

23:                                               ; preds = %18, %11
  call void @_wassert(ptr noundef @"??_C@_1HK@KELFKMPK@?$AAe?$AAr?$AAr?$AAo?$AAr?$AA?4?$AAc?$AAl?$AAa?$AAs?$AAs?$AA_?$AAt?$AAa?$AAg?$AA?5?$AA?$DN?$AA?$DN?$AA?5?$AAc?$AAl?$AAa?$AAs?$AAs?$AA_?$AAt?$AAa?$AAg?$AA?5?$AA?$CG?$AA?$CG?$AA?5@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 76)
  br label %24

24:                                               ; preds = %23, %18
  %25 = phi i1 [ true, %18 ], [ false, %23 ]
  %26 = zext i1 %25 to i32
  %27 = load ptr, ptr %5, align 8
  %28 = icmp ne ptr %27, null
  br i1 %28, label %29, label %53

29:                                               ; preds = %24
  %30 = getelementptr inbounds nuw %struct.XiaoAbiErrorSnapshot, ptr %7, i32 0, i32 3
  %31 = getelementptr inbounds nuw %struct.XiaoAbiBytes, ptr %30, i32 0, i32 1
  %32 = load i64, ptr %31, align 8
  %33 = load ptr, ptr %5, align 8
  %34 = call i64 @strlen(ptr noundef %33) #6
  %35 = icmp eq i64 %32, %34
  br i1 %35, label %37, label %36

36:                                               ; preds = %29
  call void @_wassert(ptr noundef @"??_C@_1DO@BNAPIDAO@?$AAe?$AAr?$AAr?$AAo?$AAr?$AA?4?$AAc?$AAo?$AAd?$AAe?$AA?4?$AAl?$AAe?$AAn?$AA?5?$AA?$DN?$AA?$DN?$AA?5?$AAs?$AAt?$AAr?$AAl?$AAe?$AAn?$AA?$CI?$AAc?$AAo?$AAd?$AAe?$AA?$CJ?$AA?$AA@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 78)
  br label %37

37:                                               ; preds = %36, %29
  %38 = phi i1 [ true, %29 ], [ false, %36 ]
  %39 = zext i1 %38 to i32
  %40 = getelementptr inbounds nuw %struct.XiaoAbiErrorSnapshot, ptr %7, i32 0, i32 3
  %41 = getelementptr inbounds nuw %struct.XiaoAbiBytes, ptr %40, i32 0, i32 1
  %42 = load i64, ptr %41, align 8
  %43 = load ptr, ptr %5, align 8
  %44 = getelementptr inbounds nuw %struct.XiaoAbiErrorSnapshot, ptr %7, i32 0, i32 3
  %45 = getelementptr inbounds nuw %struct.XiaoAbiBytes, ptr %44, i32 0, i32 0
  %46 = load ptr, ptr %45, align 8
  %47 = call i32 @memcmp(ptr noundef %46, ptr noundef %43, i64 noundef %42)
  %48 = icmp eq i32 %47, 0
  br i1 %48, label %50, label %49

49:                                               ; preds = %37
  call void @_wassert(ptr noundef @"??_C@_1GE@HBCAKALM@?$AAm?$AAe?$AAm?$AAc?$AAm?$AAp?$AA?$CI?$AAe?$AAr?$AAr?$AAo?$AAr?$AA?4?$AAc?$AAo?$AAd?$AAe?$AA?4?$AAp?$AAt?$AAr?$AA?0?$AA?5?$AAc?$AAo?$AAd?$AAe?$AA?0?$AA?5?$AAe?$AAr?$AAr@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 79)
  br label %50

50:                                               ; preds = %49, %37
  %51 = phi i1 [ true, %37 ], [ false, %49 ]
  %52 = zext i1 %51 to i32
  br label %53

53:                                               ; preds = %50, %24
  call void @xiao_runtime_error_clear()
  ret void
}

; Function Attrs: noinline nounwind optnone uwtable
define internal void @release(ptr noundef %0) #0 {
  %2 = alloca ptr, align 8
  store ptr %0, ptr %2, align 8
  %3 = load ptr, ptr %2, align 8
  call void @xiao_runtime_value_release(ptr noundef %3)
  %4 = load ptr, ptr %2, align 8
  %5 = getelementptr inbounds nuw %struct.XiaoValue, ptr %4, i32 0, i32 0
  %6 = load i32, ptr %5, align 8
  %7 = icmp eq i32 %6, 0
  br i1 %7, label %8, label %13

8:                                                ; preds = %1
  %9 = load ptr, ptr %2, align 8
  %10 = getelementptr inbounds nuw %struct.XiaoValue, ptr %9, i32 0, i32 1
  %11 = load i64, ptr %10, align 8
  %12 = icmp eq i64 %11, 0
  br i1 %12, label %14, label %13

13:                                               ; preds = %8, %1
  call void @_wassert(ptr noundef @"??_C@_1FG@BCDAGFCH@?$AAv?$AAa?$AAl?$AAu?$AAe?$AA?9?$AA?$DO?$AAt?$AAa?$AAg?$AA?5?$AA?$DN?$AA?$DN?$AA?5?$AA0?$AA?5?$AA?$CG?$AA?$CG?$AA?5?$AAv?$AAa?$AAl?$AAu?$AAe?$AA?9?$AA?$DO?$AAp?$AAa?$AAy?$AAl?$AAo?$AAa@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 86)
  br label %14

14:                                               ; preds = %13, %8
  %15 = phi i1 [ true, %8 ], [ false, %13 ]
  %16 = zext i1 %15 to i32
  ret void
}

declare dso_local void @xiao_runtime_release_trace_flush() #1

; Function Attrs: noinline nounwind optnone uwtable
define linkonce_odr dso_local i32 @fprintf(ptr noundef %0, ptr noundef %1, ...) #0 comdat {
  %3 = alloca ptr, align 8
  %4 = alloca ptr, align 8
  %5 = alloca i32, align 4
  %6 = alloca ptr, align 8
  store ptr %1, ptr %3, align 8
  store ptr %0, ptr %4, align 8
  call void @llvm.va_start.p0(ptr %6)
  %7 = load ptr, ptr %6, align 8
  %8 = load ptr, ptr %3, align 8
  %9 = load ptr, ptr %4, align 8
  %10 = call i32 @_vfprintf_l(ptr noundef %9, ptr noundef %8, ptr noundef null, ptr noundef %7)
  store i32 %10, ptr %5, align 4
  call void @llvm.va_end.p0(ptr %6)
  %11 = load i32, ptr %5, align 4
  ret i32 %11
}

declare dso_local ptr @__acrt_iob_func(i32 noundef) #1

; Function Attrs: nocallback nofree nosync nounwind willreturn
declare void @llvm.va_start.p0(ptr) #3

; Function Attrs: noinline nounwind optnone uwtable
define linkonce_odr dso_local i32 @_vsprintf_l(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3) #0 comdat {
  %5 = alloca ptr, align 8
  %6 = alloca ptr, align 8
  %7 = alloca ptr, align 8
  %8 = alloca ptr, align 8
  store ptr %3, ptr %5, align 8
  store ptr %2, ptr %6, align 8
  store ptr %1, ptr %7, align 8
  store ptr %0, ptr %8, align 8
  %9 = load ptr, ptr %5, align 8
  %10 = load ptr, ptr %6, align 8
  %11 = load ptr, ptr %7, align 8
  %12 = load ptr, ptr %8, align 8
  %13 = call i32 @_vsnprintf_l(ptr noundef %12, i64 noundef -1, ptr noundef %11, ptr noundef %10, ptr noundef %9)
  ret i32 %13
}

; Function Attrs: nocallback nofree nosync nounwind willreturn
declare void @llvm.va_end.p0(ptr) #3

; Function Attrs: noinline nounwind optnone uwtable
define linkonce_odr dso_local i32 @_vsnprintf_l(ptr noundef %0, i64 noundef %1, ptr noundef %2, ptr noundef %3, ptr noundef %4) #0 comdat {
  %6 = alloca ptr, align 8
  %7 = alloca ptr, align 8
  %8 = alloca ptr, align 8
  %9 = alloca i64, align 8
  %10 = alloca ptr, align 8
  %11 = alloca i32, align 4
  store ptr %4, ptr %6, align 8
  store ptr %3, ptr %7, align 8
  store ptr %2, ptr %8, align 8
  store i64 %1, ptr %9, align 8
  store ptr %0, ptr %10, align 8
  %12 = load ptr, ptr %6, align 8
  %13 = load ptr, ptr %7, align 8
  %14 = load ptr, ptr %8, align 8
  %15 = load i64, ptr %9, align 8
  %16 = load ptr, ptr %10, align 8
  %17 = call ptr @__local_stdio_printf_options()
  %18 = load i64, ptr %17, align 8
  %19 = or i64 %18, 1
  %20 = call i32 @__stdio_common_vsprintf(i64 noundef %19, ptr noundef %16, i64 noundef %15, ptr noundef %14, ptr noundef %13, ptr noundef %12)
  store i32 %20, ptr %11, align 4
  %21 = load i32, ptr %11, align 4
  %22 = icmp slt i32 %21, 0
  br i1 %22, label %23, label %24

23:                                               ; preds = %5
  br label %26

24:                                               ; preds = %5
  %25 = load i32, ptr %11, align 4
  br label %26

26:                                               ; preds = %24, %23
  %27 = phi i32 [ -1, %23 ], [ %25, %24 ]
  ret i32 %27
}

declare dso_local i32 @__stdio_common_vsprintf(i64 noundef, ptr noundef, i64 noundef, ptr noundef, ptr noundef, ptr noundef) #1

; Function Attrs: noinline nounwind optnone uwtable
define linkonce_odr dso_local ptr @__local_stdio_printf_options() #0 comdat {
  ret ptr @__local_stdio_printf_options._OptionsStorage
}

; Function Attrs: nocallback nofree nounwind willreturn memory(argmem: readwrite)
declare void @llvm.memcpy.p0.p0.i64(ptr noalias writeonly captures(none), ptr noalias readonly captures(none), i64, i1 immarg) #4

; Function Attrs: noinline nounwind optnone uwtable
define internal void @string_bytes(ptr dead_on_unwind noalias writable sret(%struct.XiaoValue) align 8 %0, ptr noundef %1, i64 noundef %2) #0 {
  %4 = alloca ptr, align 8
  %5 = alloca i64, align 8
  %6 = alloca ptr, align 8
  %7 = alloca %struct.XiaoAbiBytes, align 8
  %8 = alloca %struct.XiaoAbiBytes, align 8
  store ptr %0, ptr %4, align 8
  store i64 %2, ptr %5, align 8
  store ptr %1, ptr %6, align 8
  call void @llvm.memcpy.p0.p0.i64(ptr align 8 %0, ptr align 8 @__const.string_bytes.value, i64 16, i1 false)
  %9 = getelementptr inbounds nuw %struct.XiaoAbiBytes, ptr %7, i32 0, i32 0
  %10 = load ptr, ptr %6, align 8
  store ptr %10, ptr %9, align 8
  %11 = getelementptr inbounds nuw %struct.XiaoAbiBytes, ptr %7, i32 0, i32 1
  %12 = load i64, ptr %5, align 8
  store i64 %12, ptr %11, align 8
  %13 = getelementptr inbounds nuw %struct.XiaoValue, ptr %0, i32 0, i32 1
  call void @llvm.memcpy.p0.p0.i64(ptr align 8 %8, ptr align 8 %7, i64 16, i1 false)
  %14 = call i32 @xiao_runtime_string_new(ptr dead_on_return noundef %8, ptr noundef %13)
  %15 = icmp eq i32 %14, 0
  br i1 %15, label %17, label %16

16:                                               ; preds = %3
  call void @_wassert(ptr noundef @"??_C@_1HE@LPOLCLJE@?$AAx?$AAi?$AAa?$AAo?$AA_?$AAr?$AAu?$AAn?$AAt?$AAi?$AAm?$AAe?$AA_?$AAs?$AAt?$AAr?$AAi?$AAn?$AAg?$AA_?$AAn?$AAe?$AAw?$AA?$CI?$AAv?$AAi?$AAe?$AAw?$AA?0?$AA?5?$AA?$CG?$AAv@", ptr noundef @"??_C@_1KG@KHEOFKHE@?$AAD?$AA?3?$AA?2?$AAp?$AAr?$AAo?$AAj?$AAe?$AAc?$AAt?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAX?$AAi?$AAa?$AAo?$AA?2?$AAd?$AAo?$AAc?$AAs?$AA?2?$AAD?$AAe?$AAv?$AAD?$AAo?$AAc@", i32 noundef 65)
  br label %17

17:                                               ; preds = %16, %3
  %18 = phi i1 [ true, %3 ], [ false, %16 ]
  %19 = zext i1 %18 to i32
  ret void
}

; Function Attrs: nocallback nofree nounwind willreturn memory(argmem: write)
declare void @llvm.memset.p0.i64(ptr writeonly captures(none), i8, i64, i1 immarg) #5

declare dso_local i32 @xiao_runtime_string_new(ptr dead_on_return noundef, ptr noundef) #1

declare dso_local i32 @xiao_runtime_string_copy(ptr noundef, ptr dead_on_return noundef, ptr noundef) #1

declare dso_local i32 @memcmp(ptr noundef, ptr noundef, i64 noundef) #1

; Function Attrs: nounwind
declare dso_local i64 @strlen(ptr noundef) #2

declare dso_local i32 @xiao_runtime_error_snapshot(ptr noundef) #1

declare dso_local void @xiao_runtime_error_clear() #1

declare dso_local void @xiao_runtime_value_release(ptr noundef) #1

; Function Attrs: noinline nounwind optnone uwtable
define linkonce_odr dso_local i32 @_vfprintf_l(ptr noundef %0, ptr noundef %1, ptr noundef %2, ptr noundef %3) #0 comdat {
  %5 = alloca ptr, align 8
  %6 = alloca ptr, align 8
  %7 = alloca ptr, align 8
  %8 = alloca ptr, align 8
  store ptr %3, ptr %5, align 8
  store ptr %2, ptr %6, align 8
  store ptr %1, ptr %7, align 8
  store ptr %0, ptr %8, align 8
  %9 = load ptr, ptr %5, align 8
  %10 = load ptr, ptr %6, align 8
  %11 = load ptr, ptr %7, align 8
  %12 = load ptr, ptr %8, align 8
  %13 = call ptr @__local_stdio_printf_options()
  %14 = load i64, ptr %13, align 8
  %15 = call i32 @__stdio_common_vfprintf(i64 noundef %14, ptr noundef %12, ptr noundef %11, ptr noundef %10, ptr noundef %9)
  ret i32 %15
}

declare dso_local i32 @__stdio_common_vfprintf(i64 noundef, ptr noundef, ptr noundef, ptr noundef, ptr noundef) #1

attributes #0 = { noinline nounwind optnone uwtable "min-legal-vector-width"="0" "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #1 = { "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #2 = { nounwind "no-trapping-math"="true" "stack-protector-buffer-size"="8" "target-cpu"="x86-64" "target-features"="+cmov,+cx8,+fxsr,+mmx,+sse,+sse2,+x87" "tune-cpu"="generic" }
attributes #3 = { nocallback nofree nosync nounwind willreturn }
attributes #4 = { nocallback nofree nounwind willreturn memory(argmem: readwrite) }
attributes #5 = { nocallback nofree nounwind willreturn memory(argmem: write) }
attributes #6 = { nounwind }

!llvm.dbg.cu = !{!0}
!llvm.linker.options = !{!2, !2}
!llvm.module.flags = !{!3, !4, !5, !6, !7}
!llvm.ident = !{!8}

!0 = distinct !DICompileUnit(language: DW_LANG_C11, file: !1, producer: "clang version 22.1.2 (https://github.com/msys2/MINGW-packages 4f19d31560faf73257116b7c95f0b322ba83d720)", isOptimized: false, runtimeVersion: 0, emissionKind: NoDebug, splitDebugInlining: false, nameTableKind: None)
!1 = !DIFile(filename: "D:\\project\\Xiao\\Xiao\\docs\\DevDocs\\assets\\20o-b3\\entry-20261128-b3-5\\logs\\c/probe.c", directory: "D:/project/Xiao/.xiao-orchestrator/worktrees/b3-5c/cases/c-probe")
!2 = !{!"/DEFAULTLIB:uuid.lib"}
!3 = !{i32 2, !"Debug Info Version", i32 3}
!4 = !{i32 1, !"wchar_size", i32 2}
!5 = !{i32 8, !"PIC Level", i32 2}
!6 = !{i32 7, !"uwtable", i32 2}
!7 = !{i32 1, !"MaxTLSAlign", i32 65536}
!8 = !{!"clang version 22.1.2 (https://github.com/msys2/MINGW-packages 4f19d31560faf73257116b7c95f0b322ba83d720)"}
!9 = distinct !{!9, !10}
!10 = !{!"llvm.loop.mustprogress"}
!11 = distinct !{!11, !10}
