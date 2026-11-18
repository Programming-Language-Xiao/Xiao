---
id: library.math
title: 整数计算
status: verified
audience: learner
module: stdlib.math
stage: 20O-B0/B1-FIX-A/B1-IMP/20P/20O-B1-CLOSE
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
  - ../../DevDocs/20o-b1-closeout.md
---

# 整数计算

## 适用范围与状态

整数绝对值、大小比较、闭区间钳位、最大公约数和非负整数幂；只接受固定64位有符号int。

本页与stdlib.math登记为verified。适用Xiao 0.1.0、Runtime ABI 1.18、标准库接口版本1。
已验证范围覆盖VM及Windows/Linux/macOS原生O0/O3、资源边界与稳定错误；
[收口记录](../../DevDocs/20o-b1-closeout.md)列出运行号、源码身份与限制。

## 导入与准备

官方库随工具链提供，无需普通包安装。公共路径为std.math，固定身份为stdlib.math；
下面的完整示例使用选择导入，所有参数必填，支持既有位置与命名调用规则。
使用包含本批源码的工具链；从仓库根运行示例时，标准库加载根应指向本仓库的stdlib目录。

## 五分钟运行与完整示例

从仓库根执行：

```powershell
xiao run tests/fixtures/20o-b1/imp/math/product.xiao
```

以下代码与[实际产品夹具](../../../tests/fixtures/20o-b1/imp/math/product.xiao)逐字一致；
输出由[独立黄金](../../../tests/fixtures/20o-b1/imp/math/golden.json)核对。

<!-- b1-imp-executable-example -->
```xiao
from std.math import abs, min, max, clamp, gcd, pow

minimum = -9223372036854775807 - 1
maximum = 9223372036854775807
print(abs(-7))
print(abs(0))
print(min(minimum, maximum))
print(max(minimum, maximum))
print(clamp(5, 0, 3))
print(clamp(minimum, minimum, maximum))
print(clamp(0, 2, 2))
print(gcd(54, -24))
print(gcd(0, 0))
print(gcd(minimum, 2))
print(gcd(minimum, -1))
print(gcd(minimum, maximum))
print(pow(minimum, 1))
print(pow(0, 0))
print(pow(0, maximum))
print(pow(1, maximum))
print(pow(-1, maximum))
print(pow(2, 62))
print(abs(value = -maximum))
print(min(left = maximum, right = maximum))
print(max(left = minimum, right = minimum))
print(clamp(value = 5, lower = 3, upper = 4))
print(gcd(left = minimum, right = 6))
print(pow(base = -2, exponent = 63))
try
    print(clamp(0, 1, -1))
catch bounds as Error
    print(error_code(bounds))
try
    print(pow(2, -1))
catch exponent as Error
    print(error_code(exponent))
try
    print(abs(minimum))
catch absolute as ArithmeticError
    print(error_code(absolute))
try
    print(gcd(minimum, 0))
catch gcd_zero as ArithmeticError
    print(error_code(gcd_zero))
try
    print(gcd(minimum, minimum))
catch gcd_both as ArithmeticError
    print(error_code(gcd_both))
try
    print(pow(2, 63))
catch power as ArithmeticError
    print(error_code(power))
```

独立黄金输出：

```text
7
0
-9223372036854775808
9223372036854775807
3
-9223372036854775808
2
6
0
2
1
1
-9223372036854775808
1
0
1
-1
4611686018427387904
9223372036854775807
9223372036854775807
-9223372036854775808
4
2
-9223372036854775808
MATH_INVALID_BOUNDS
MATH_NEGATIVE_EXPONENT
X06-RUNTIME-009
X06-RUNTIME-009
X06-RUNTIME-009
X06-RUNTIME-009
```

## 核心概念与所有权

gcd允许最小int作为输入，只在最终非负结果不可表示时溢出；gcd(0,0)=0。pow的指数必须非负，0^0=1，MIN^1合法，指数可到最大int。首批不提供浮点或高精度重载。

参数和返回值均为int，按值传入。结果不可表示时抛出算术错误，不回绕、截断或返回部分结果；没有需手工关闭的资源。
打印示例沿用stdout能力。

## API

| 已确认签名 | 已确认边界 |
| --- | --- |
| `abs(int value) -> int` | 绝对值；最小 int 的绝对值报既有溢出 |
| `min(int left, int right) -> int` | 二元最小值 |
| `max(int left, int right) -> int` | 二元最大值 |
| `clamp(int value, int lower, int upper) -> int` | 闭区间钳位，倒置区间先报错 |
| `gcd(int left, int right) -> int` | 非负结果，gcd(0,0)=0；允许最小 int 输入，仅结果不可表示时溢出 |
| `pow(int base, int exponent) -> int` | 非负指数，0^0=1；MIN^1 合法，禁止无必要中间平方 |

参数名属于契约。完整签名、边界与检查顺序以[B1冻结契约](../../DevDocs/20o-b1-freeze-draft.md#三七库已确认签名与调用边界f2f8f12)为准。

## 错误与恢复

示例在catch中使用全局自由函数error_code读取原错误码，读取不消费错误对象。

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| Error / MATH_INVALID_BOUNDS | clamp的lower大于upper | 先修正闭区间上下界 |
| Error / MATH_NEGATIVE_EXPONENT | pow的exponent小于0 | 使用非负整数指数 |
| ArithmeticError / X06-RUNTIME-009 | abs、gcd或pow的结果不可表示 | 缩小输入；不截断、回绕或自动转浮点 |

调用或静态类型错误沿用X04-TYPE-002，内部动态类型边界沿用X06-RUNTIME-002。
先处理调用/类型和参数域，再按[统一错误顺序](../../DevDocs/20o-b1-freeze-draft.md#61-已确认检查优先级)检查输入、内容与结果预算。
可本地化的message不参与错误身份。error_code的[独立验收](../../DevDocs/20o-b1-fix-b-closeout.md)不开放error.code成员访问。

## 资源、性能与安全边界

参数和结果限于int，不额外限制合法指数。abs/min/max/clamp为常量成本；gcd为余数迭代，pow按指数二进制位计算。

等于上限合法，超过上限整次失败，不静默截断。具体数值见[资源限额](../../DevDocs/20o-b1-freeze-draft.md#四资源限额f9已验收)。

固定源码身份工具的[完整F10](../../DevDocs/assets/20o-b1-imp/rem-identity-final-41464439400/README.md)
共721条，覆盖20入口/31负载：标量记最大规模，其余还测4,096→8,192→16,384；原生O0/O3分记。
受控平台测得翻倍比≤3、最大合法输入VM≤10秒/原生≤2秒、驻留增量上界≤64MiB。
一次预热、五批采样取中位数，编译、输入准备和终端I/O在窗口外；内存按OS历史驻留峰值减当前基线估计，
是包含暂存与输出的保守上界。这些是已测硬件上的验收结果，不是任意机器的时延保证或API超时语义。
详见[测量协议](../../DevDocs/20o-b1-freeze-draft.md#五性能预算与测量协议f10)。

## 示例如何进入 CI

产品示例与历史探测均由既有两个精确目标执行。VM与原生逐字核对stdout、退出码与stderr；
默认契约测试另检查真实导出、逐函数移除、真实.xar重放、普通包隔离、双路径、参数负例和教程源码一致性。

```powershell
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_probe -- b1_capabilities_have_vm_golden_outputs --exact --nocapture
# 配置受控LLVM/Runtime工具链后执行：
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_probe -- --ignored b1_capabilities_match_native_golden_outputs --exact --nocapture
```

维护回归的四个执行点各要求恰好1 passed、0 failed、0 ignored；批量路径排除这两个名字，计数守卫28/28。
仅修改教程也会触发维护回归，三平台记录与源码身份见[收口证据](../../DevDocs/20o-b1-closeout.md)。

## 兼容与废弃

本模块按标准库接口版本1兼容新增，固定身份不进入普通依赖求解，也不把官方函数体复制进应用归档。
当前Runtime ABI为1.18，生成器20；error_code前置已独立验收。
历史B0/FIX-A探测最初使用ABI1.17，仅用于解释当时能力，不作为本页产品或性能成绩。
后续新增入口、资源限额或错误规则应另立批次；本次状态升级没有修改产品函数、ABI、生成器或预算。

## 故障排查与下一步

导入失败时先检查工具链是否包含本批源码及标准库加载根；不要把官方库安装成普通包。
原生测试缺少LLVM、Runtime或目标平台配置时应明确失败，不能把环境跳过当成功。
遇到数据错误按上表修正参数，遇到资源超限按完整输入与输出预算拆分。
继续阅读[官方库索引](README.md)、[冻结契约](../../DevDocs/20o-b1-freeze-draft.md)和[收口记录](../../DevDocs/20o-b1-closeout.md)。

## 附：B0历史探测

下面保留[B0探测文件](../../../tests/fixtures/docs/math/probe.xiao)的原文，仍进CI；
它的局部probe函数、B0_*错误与caught-*标签只描述历史，不属于当前公开接口。
正常使用本库应运行本页前面的产品示例。

<!-- b0-executable-example -->
```xiao
# B0 能力探测；这些局部函数不是 std.math 的产品实现。
def probe_gcd(int left, int right) -> int
    if left < 0
        left = -left
    if right < 0
        right = -right
    while right != 0
        remainder = left % right
        left = right
        right = remainder
    return left

def probe_power(int base, int exponent) -> int
    if exponent < 0
        raise Error(code = "B0_NEGATIVE_EXPONENT")
    result = 1
    remaining = exponent
    while remaining > 0
        result = result * base
        remaining = remaining - 1
    return result

print(probe_gcd(54, 24))
print(probe_gcd(-54, 24))
print(probe_gcd(0, 0))
print(probe_power(2, 10))
print(probe_power(0, 0))
try
    print(probe_power(2, -1))
catch negative as Error
    print("caught-negative-exponent")
try
    print(probe_power(2, 63))
catch overflow as Error
    print("caught-overflow")
```
