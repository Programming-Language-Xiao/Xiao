---
id: library.statistics
title: 整数统计
status: verified
audience: learner
module: stdlib.statistics
stage: 20O-B0/B1-FIX-A/B1-IMP/20P/20O-B1-CLOSE
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
  - ../../DevDocs/20o-b1-closeout.md
---

# 整数统计

## 适用范围与状态

整数数组的顺序求和、最小值与最大值；不包含均值、方差、浮点或高精度统计。

本页与stdlib.statistics登记为verified。适用Xiao 0.1.0、Runtime ABI 1.18、标准库接口版本1。
已验证范围覆盖VM及Windows/Linux/macOS原生O0/O3、资源边界与稳定错误；
[收口记录](../../DevDocs/20o-b1-closeout.md)列出运行号、源码身份与限制。

## 导入与准备

官方库随工具链提供，无需普通包安装。公共路径为std.statistics，固定身份为stdlib.statistics；
下面的完整示例使用选择导入，所有参数必填，支持既有位置与命名调用规则。
使用包含本批源码的工具链；从仓库根运行示例时，标准库加载根应指向本仓库的stdlib目录。

## 五分钟运行与完整示例

从仓库根执行：

```powershell
xiao run tests/fixtures/20o-b1/imp/statistics/product.xiao
```

以下代码与[实际产品夹具](../../../tests/fixtures/20o-b1/imp/statistics/product.xiao)逐字一致；
输出由[独立黄金](../../../tests/fixtures/20o-b1/imp/statistics/golden.json)核对。

<!-- b1-imp-executable-example -->
```xiao
from std.statistics import sum, min, max

values = [5, -2, 9, -2]
alias = values
print(sum(values))
print(min(values))
print(max(values))
print(alias[0])
print(sum([]))
minimum = -9223372036854775807 - 1
maximum = 9223372036854775807
print(sum([maximum, minimum]))
print(min([maximum, minimum]))
print(max([minimum, maximum]))
print(sum(values = [1, 2]))
print(min(values = [2, 1]))
print(max(values = [2, 1]))
try
    print(min([]))
catch empty_min as Error
    print(error_code(empty_min))
try
    print(max([]))
catch empty_max as Error
    print(error_code(empty_max))
overflow_values = [maximum, 1, -1]
try
    print(sum(overflow_values))
catch overflow as ArithmeticError
    print(error_code(overflow))
print(overflow_values[0])
large = []
count = 0
while count < 65536
    array_push(large, 1)
    count = count + 1
print(sum(large))
print(min(large))
print(max(large))
over = [maximum, 1]
count = 0
while count < 65535
    array_push(over, 0)
    count = count + 1
try
    print(sum(over))
catch limited_sum as ResourceError
    print(error_code(limited_sum))
try
    print(min(over))
catch limited_min as ResourceError
    print(error_code(limited_min))
try
    print(max(over))
catch limited_max as ResourceError
    print(error_code(limited_max))
print(over[0])
```

独立黄金输出：

```text
10
-2
9
5
0
-1
-9223372036854775808
9223372036854775807
3
1
2
STATISTICS_EMPTY
STATISTICS_EMPTY
X06-RUNTIME-009
9223372036854775807
65536
1
1
STATISTICS_LIMIT_EXCEEDED
STATISTICS_LIMIT_EXCEEDED
STATISTICS_LIMIT_EXCEEDED
9223372036854775807
```

## 核心概念与所有权

sum从0开始从左到右累加，sum([])=0；中间溢出立即失败，即使最终数学结果本可表示也不重排。min/max空数组报错，允许int两端极值，不排序或修改输入。

输入数组只读、不消费、不排序，返回值为int。失败不修改调用方数组，也不返回部分结果；没有需手工关闭的资源。
打印示例沿用stdout能力。

## API

| 已确认签名 | 已确认边界 |
| --- | --- |
| `sum([int] values) -> int` | 空数组为 0；从左到右累加，中途溢出立即失败 |
| `min([int] values) -> int` | 空数组报错；比较不得用减法替代 |
| `max([int] values) -> int` | 空数组报错；原数组不排序、不修改 |

参数名属于契约。完整签名、边界与检查顺序以[B1冻结契约](../../DevDocs/20o-b1-freeze-draft.md#三七库已确认签名与调用边界f2f8f12)为准。

## 错误与恢复

示例在catch中使用全局自由函数error_code读取原错误码，读取不消费错误对象。

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| Error / STATISTICS_EMPTY | min/max输入为空 | 提供至少一个元素 |
| ResourceError / STATISTICS_LIMIT_EXCEEDED | 数组超过65,536项 | 按预算拆分输入 |
| ArithmeticError / X06-RUNTIME-009 | sum的顺序累加中间值不可表示 | 修正数据范围，不重排或自动转浮点 |

调用或静态类型错误沿用X04-TYPE-002，内部动态类型边界沿用X06-RUNTIME-002。
先处理调用/类型和参数域，再按[统一错误顺序](../../DevDocs/20o-b1-freeze-draft.md#61-已确认检查优先级)检查输入、内容与结果预算。
可本地化的message不参与错误身份。error_code的[独立验收](../../DevDocs/20o-b1-fix-b-closeout.md)不开放error.code成员访问。

## 资源、性能与安全边界

每次最多65,536项，返回一个int。产品有界预检后线性归约，归约状态为常量；Runtime迭代快照如复制数组，仍计入实际O(n)存储与内存预算。

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

下面保留[B0探测文件](../../../tests/fixtures/docs/statistics/probe.xiao)的原文，仍进CI；
它的局部probe函数、B0_*错误与caught-*标签只描述历史，不属于当前公开接口。
正常使用本库应运行本页前面的产品示例。

<!-- b0-executable-example -->
```xiao
# B0 探测整数数组归约；D-5 已确认范围，公开签名和错误码尚未冻结。
def probe_sum([int] values) -> int
    total = 0
    for value in values
        total = total + value
    return total

def probe_min([int] values) -> int
    seen = false
    result = 0
    for candidate in values
        if not seen or candidate < result
            result = candidate
        seen = true
    if not seen
        raise Error(code = "B0_EMPTY_STATISTICS")
    return result

def probe_max([int] values) -> int
    seen = false
    result = 0
    for candidate in values
        if not seen or candidate > result
            result = candidate
        seen = true
    if not seen
        raise Error(code = "B0_EMPTY_STATISTICS")
    return result

values = [5, -2, 9, -2]
print(probe_sum(values))
print(probe_min(values))
print(probe_max(values))
print(values[0])
print(probe_sum([]))
try
    print(probe_min([]))
catch empty_min as Error
    print("caught-empty-min")
try
    print(probe_max([]))
catch empty_max as Error
    print("caught-empty-max")
try
    print(probe_sum([9223372036854775807, 1]))
catch overflow as Error
    print("caught-overflow")
```
