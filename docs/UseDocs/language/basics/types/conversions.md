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

## B2高精度基础（规划中）

上文的verified状态覆盖原有规则。B2-A5将A1～A4精确内核接入语言路径，lint/lfloat继续以十进制文本承载；本节新增范围保持planned，A6性能与A7/B2-C整批收口后置。

[B2冻结草案](../../../../DevDocs/20o-b2-freeze-draft.md)已确认未来方向：保留文本ABI边界，
精确整数/十进制底座、显式精度除法、保留码点式str到整数转换，并由str到lfloat处理数值文本。
A5完整算术与转换逐步验收，显式精度除法lfloat_divide仍留B2-B；不能按本页verified状态推断新增范围已完成最终交付。
本节新增范围继续planned；完整三平台证据和后续收口齐备后再升级。

A1的完整示例保留非零显示精度，但等价数值比较相等；这里使用既有数值声明，未新增字符串到lfloat的转换入口：

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

这份[完整finally示例](../../../../../tests/fixtures/docs/numeric/a1_pending_finally.xiao)预期输出`1.50`和`A1_ORIGINAL`，各占一行，同样保持planned并进入上述执行测试。

## A5完整转换示例（planned）

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

[完整源码](../../../../../tests/fixtures/docs/numeric/a5_conversions.xiao)由默认VM和维护原生两个精确入口执行，O0/O3逐字核对。完整六类型转换、资源/错误及并发承载另有独立回归；本段仍为planned。

[上一页：变量声明](variables.md) · [下一页：编译期常量](constants.md)
