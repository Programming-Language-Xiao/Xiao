---
id: language.basics.types.conversions
title: 转换与数值运算
status: verified
audience: learner
module: rust.xiao-types
stage: "02A"
version: "0.1.0"
related:
  - README.md
  - variables.md
  - constants.md
---

# 转换与数值运算

显式转换使用“源值 `as` 目标类型”，构造式写法与它共享同一规则：

```xiao
str raw = input("true/false")
bool flag = raw as bool
bool same = bool(raw)
str text = flag as str       # 结果是小写 true 或 false
```

`str` 到 `bool` 只接受 `true`、`True`、`false`、`False`；其他字符串在运行时检查或
已知为常量时在编译期报告 `X02-TYPE-006`。整数和浮点的安全加宽可以隐式发生，窄化
必须显式转换，并可能产生范围检查。显式浮点到整数转换按向零截断，例如
`1.9 as int` 的结果为 `1`；截断后的值超出目标固定宽度时仍报告
`X02-TYPE-007`。隐式声明不会借用这条截断规则。

数值提升顺序为 `sint < int < lint` 与 `sfloat < float < lfloat`。`/` 返回浮点族，
`//` 与 `%` 原有范围只接受整数；B2新增的lfloat路径见下方planned段落。布尔值有一个专用规则：只有布尔在左、整数在右时，偶数保持、
奇数翻转，结果仍是 `bool`：

```xiao
flag = false
flag += 3   # true
flag -= 2   # true
```

反向运算、布尔与布尔、布尔与浮点数的加减都会报告 `X02-TYPE-005`。

## B2-A高精度基础：内部验收通过，最终发布（planned）

B2-A已将精确内核接入类型检查、VM、原生、字节码/归档和并发值承载，并完成三平台内部验收。这里记录ABI1.18/生成器20内部快照的能力；上文verified状态继续覆盖原有规则，新增范围的最终公共发布仍planned。`lfloat_divide`、`cmath`及最终1.19/21产物未交付，B2-C再完成最终教程/registry和721条F10。

`lint`/`lfloat`继续使用十进制文本边界。`1.50`与`1.5`相等且同哈希，分别显示各自精度；非零显示尾零保留，所有十进制零统一显示0。负scale或普通表示超过65,536字节时使用整数系数和e指数，例如1e100000。计算与转换均受下方固定预算约束，不表示无界分配。

| 方向 | B2-A内部已验证规则 |
| --- | --- |
| 整数族→lint/lfloat | 精确；进入lfloat的整数显示scale=0，例如10显示10；不隐式窄化 |
| str→int/sint/lint | 保留单码点转换；`"1"`得到49，`"42"`因多个码点而拒绝。数字文本先经lfloat再显式转整数 |
| str→lfloat | ASCII十进制解析；允许符号、.5、5.、e/E；下划线只许夹在同一数字段的两位数字之间。拒绝空白、BOM、NUL、NaN/Inf、非ASCII数字和不完整文本 |
| lfloat→lint/int/sint | 显式向零截断；lint检查系数位数，int/sint检查截断后的定宽范围 |
| float/sfloat→lfloat | 精确承接实际二进制值；整数值显示scale=0，±0统一为十进制0；拒绝非有限值 |
| lint/lfloat→float/sfloat | 显式最近偶数舍入；支持子正规数和非零负来源舍入成-0，正确舍入到非有限结果才报溢出 |
| lfloat→str | 按上述显示尺度输出；不使用上游默认精度；float/sfloat→str沿既有有限显示并保留-0 |
| str→float/sfloat | 仍不开放；显式经lfloat解析再窄化 |

A1身份和A5完整转换示例分别验证早期接纳与完整转换路径，示例内容继续由现有VM和原生测试逐字核对。内部收口范围见[A7记录](../../../../DevDocs/20o-b2-a7-b2a-closeout.md)，精确契约见[冻结草案](../../../../DevDocs/20o-b2-freeze-draft.md)。

A1的完整示例保留非零显示精度，但等价数值比较相等；这里使用数值声明演示身份；str→lfloat的新解析方向另由后面的完整转换示例验证：

```xiao
lint big = 18446744073709551616
lfloat first = 1.50
lfloat second = 15e-1
lfloat zero = -0.00
lfloat large = 1e100000
print(big)
print(first)
print(second)
print(first == second)
print(zero)
print(large)
```

预期输出：

```text
18446744073709551616
1.50
1.5
true
0
1e100000
```

数值接纳预算超限是可恢复错误，可以捕获后继续执行：

```xiao
try
    lfloat over_budget = 1e100001
    print(over_budget)
catch failure as ResourceError
    print(error_code(failure))
print("continued")
```

预期输出为`X06-RUNTIME-033`和`continued`，各占一行。
两份完整源码分别位于[身份示例](../../../../../tests/fixtures/docs/numeric/a1_identity.xiao)与[接纳错误示例](../../../../../tests/fixtures/docs/numeric/a1_admission_error.xiao)，由同一VM/受控原生测试逐字核对，默认VM测试进入既有workspace CI。
A1证据只覆盖接纳和身份；A5转换示例另列于下。语言字典继续使用str键。

正常构造数值也不会替换正在展开的原错误：

```xiao
try
    try
        raise Error(code = "A1_ORIGINAL")
    finally
        lfloat value = 1.50
        print(value)
catch original as Error
    print(error_code(original))
```

这份[完整finally示例](../../../../../tests/fixtures/docs/numeric/a1_pending_finally.xiao)预期输出`1.50`和`A1_ORIGINAL`，各占一行，已完成B2-A内部两后端与三平台验证；最终发布仍planned，同样进入上述执行测试。

## 完整转换示例：B2-A内部已验证，最终发布（planned）

str到整数仍是单码点转换；数值文本先经lfloat解析。显式lfloat到整数向零截断，定宽结果检查范围；float/sfloat到lfloat承接实际二进制数值，因此二进制0.1与十进制0.1不同。整数值转入lfloat时scale为0，文本输入保留非零显示精度。
涉及lint/lfloat的值错误保留Runtime检查，可在语言中捕获；其余定宽常量检查保持原规则，静态非法类型仍编译拒绝。

<!-- b2-a5-conversions-executable-example -->
```xiao
float binary = 0.1
sfloat small_binary = 0.1
lfloat exact = lfloat(binary)
lfloat parsed = lfloat("0.1")
lfloat precision = lfloat("+1_234.50_0e-2")
lint wide = lint(lfloat("340282366920938463463374607431768211456"))
int fixed = 9007199254740993
lint large = 9007199254740993
float boundary = 9007199254740992.0
print(exact)
print(lfloat(small_binary))
print(parsed == exact)
print(parsed < exact)
print(precision)
print(str(precision))
print(lfloat(10))
print(lfloat(float(10)))
print(lint(lfloat("-1.9")))
print(int(lfloat("9223372036854775807.999")))
print(sint(lfloat("-2147483648.999")))
print(wide)
print(str(-0.0))
print(str(sfloat(-0.0)))
print(lint("1"))
print(int("雪"))
print(fixed == boundary)
print(large == boundary)
print(lfloat(large) == large)
```

预期输出：

```text
0.1000000000000000055511151231257827021181583404541015625
0.100000001490116119384765625
false
true
12.34500
12.34500
10
10
-1
9223372036854775807
-2147483648
340282366920938463463374607431768211456
-0
-0
49
38634
true
false
true
```

[完整源码](../../../../../tests/fixtures/docs/numeric/a5_conversions.xiao)由默认VM和维护原生两个精确入口执行，O0/O3逐字核对。完整六类型转换、资源/错误及并发承载另有独立回归；本段已完成B2-A内部验收，最终发布仍planned。

## B2-A内部资源与错误边界

| 接纳或结果 | 固定上限 |
| --- | --- |
| 数值原文 | 65,536个UTF-8字节 |
| 输入和最终显示系数 | 去前导零、保留显示尾零后最多32,768位；零记1位 |
| 显示scale、去尾零后的规范scale、原文显式指数 | 各自绝对值最多100,000；零仍检查显式指数 |
| 显式物化的中间整数系数 | 最多65,536位；对齐/强消去先查工作量再按实际结果验收 |
| 非平凡底数的幂指数 | 最多1,000,000，还须满足工作和结果预算；0/±1快路径仍先检查输入和指数域 |

| 错误 | 稳定身份与行为 |
| --- | --- |
| 动态类型不符 | TypeError / X06-RUNTIME-002；静态非法类型仍由编译器拒绝 |
| 非法幂域、非有限值或定宽转换溢出 | ArithmeticError / X06-RUNTIME-009 |
| 实际受控输出分配失败 | ResourceError / X06-RUNTIME-011；不是数值预算超限，真实不可恢复OOM仍沿Fatal边界 |
| 文本语法非法 | TypeError / X06-RUNTIME-012 |
| `/`、`//`、`%`零分母 | ArithmeticError / X06-RUNTIME-013 |
| 输入、工作或结果预算超限 | ResourceError / X06-RUNTIME-033 |
| 合法非零分母的精确`/`非终止 | ArithmeticError / X06-RUNTIME-034；`//`与`%`不使用此错误 |

按既有表达式顺序先求值，再检查动态类型、按参数顺序接纳输入，随后检查非有限值与算子域/除零，再判断精确除法非终止、工作/结果预算和受控输出分配。文本入口内部先查原文长度，再从左到右检查完整语法，最后查系数/指数/scale；超长原文先报033，长度合格但语法坏时先报012。混合算子完成两侧接纳后先拒绝非有限值，再检查除零/幂域。

失败不修改输入或返回部分结果。涉及lint/lfloat的值错误保留可捕获Runtime检查；其他定宽常量检查维持原规则。身份、错误保留、算术与转换的完整示例均已进入CI；A6受控性能是特定平台测量，不是API超时或任意硬件保证。

[上一页：变量声明](variables.md) · [下一页：编译期常量](constants.md)
