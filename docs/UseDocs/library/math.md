---
id: library.math
title: 整数计算（B1 教程草案）
status: planned
audience: learner
module: stdlib.math
stage: 20O-B0/B1-FIX-A/20P
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
---

# 整数计算（教程草案）

## 适用范围与状态

整数边界、最大公约数和非负整数幂。首批不涉及浮点、高精度或数值重载。

本页为 `planned`：官方 `std.math` 尚未交付。完整示例只使用现有语言和已交付模块，
演示 B0 探测，不提供可导入的产品函数。适用基线：Xiao 0.1.0、Runtime ABI 1.18、
标准库接口版本 1。逐平台结果见 [B0 探测记录](../../DevDocs/20o-b0-baseline-and-b1-draft.md)。

## 导入与准备

官方库随工具链提供，无需普通包安装；`std.math` 是拟定用户路径，
`stdlib.math` 是拟定内部身份。本轮不能导入该模块。已安装 Xiao 且检出仓库时，
可以直接运行下面的探测文件；源码中的 `probe_` 名称仅在这个示例中有效。

## 五分钟运行与完整示例

从仓库根执行：

```powershell
xiao run tests/fixtures/docs/math/probe.xiao
```

[完整源码](../../../tests/fixtures/docs/math/probe.xiao) 与下列代码由测试逐字核对。

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

示例输出依次为 6、6、0、1024、1，再输出两个错误恢复标签；验证了负数、零、负指数和 2 的 63 次方溢出。

## 核心概念与所有权

int 是固定 64 位有符号整数。值按值传入，没有可关闭资源；结果溢出应报错而非回绕。

## 已确认 API 契约（尚未实现）

七项一起冻结已确认；下表是 [B1 逐项契约](../../DevDocs/20o-b1-freeze-draft.md#31-math六项) 的概览，
F1～F13 已由星崽在 FIX-A 逐条确认；所有库函数参数必填，按现有位置/命名规则调用；
输入只读、不消费，失败不改调用方数组。**这些入口尚未实现，不能替换上面的 probe 示例调用。**

| 已确认签名 | 已确认边界 |
| --- | --- |
| `abs(int value) -> int` | 绝对值；最小 int 的绝对值报既有溢出 |
| `min(int left, int right) -> int` | 二元最小值 |
| `max(int left, int right) -> int` | 二元最大值 |
| `clamp(int value, int lower, int upper) -> int` | 闭区间钳位，倒置区间先报错 |
| `gcd(int left, int right) -> int` | 非负结果，gcd(0,0)=0；允许最小 int 输入，仅结果不可表示时溢出 |
| `pow(int base, int exponent) -> int` | 非负指数，0^0=1；MIN^1 合法，禁止无必要中间平方 |

## 错误与恢复

B1 已确认的稳定 code、类别、触发条件与检查顺序统一见 [错误表](../../DevDocs/20o-b1-freeze-draft.md#六统一错误表f11已确认)，
不在教程另立错误身份。下面保留的是**当前 B0 探测**的错误与边界，不能当作已交付产品契约。

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| `X06-RUNTIME-009` | 已有整数溢出；机器身份由专用夹具核对 | 缩小输入，或等待任意精度批次 |
| `B0_NEGATIVE_EXPONENT` | 仅探测负指数拒绝，不是公开错误码 | 示例使用非负指数 |

`B0_*` 仅是夹具局部错误，不注册为标准库错误身份。示例中的 `caught-*` 是恢复分支标签，
不能作为稳定错误码证据。已确认采用自由函数 `error_code`，不开放错误成员访问；
其[精确位置/签名/错误表](../../DevDocs/20o-b1-freeze-draft.md#二error_code-的名称位置与签名)已冻结为独立 FIX-B 前置，仍未实现。
当前示例从驱动器机器结果核对 code，未调用尚未实现的 error_code。

## 资源、性能与安全边界

**F9 的限额与 F10 的验收目标已确认；当前探测没有实现或证明全部产品资源/性能要求。**

参数/结果限于 int；指数允许到最大 int。abs/min/max/clamp 为常量成本，gcd/pow 目标为对数步数；最小负数不能先取正绝对值。

精确计量、编码膨胀和超限顺序见 [资源限额](../../DevDocs/20o-b1-freeze-draft.md#四资源限额f9已确认产品未验证)。
文本/编码目标为顺序扫描加至多 O(n log n) 的均衡拼接，禁止隐藏的二次复制/扫描；
翻倍比≤3、最大输入 VM≤10s/原生≤2s、增量内存≤64 MiB 是已确认、仍待实测的目标，
计时排除启动/编译/打印，具体方法见 [性能预算](../../DevDocs/20o-b1-freeze-draft.md#五性能预算与测量协议f10)。

本例的十秒看门狗只保护测试；不代表库 API 承诺运行时超时。纯计算不新增宿主能力，打印沿用 stdout。

## 示例如何进入 CI

完整示例仍在 `tests/fixtures/docs/math/probe.xiao`，本页完整代码与文件逐字核对。
维护回归默认作业独立精确执行 VM 探测，Windows/Linux/macOS 原生作业独立精确执行原生探测；
对应批量命令排除这两个名字，计数须恰好 1 passed、0 failed、0 ignored，命令失败不得吞掉。
文档一致性与错误/语言边界仍由默认 workspace 的其余测试执行。

```powershell
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_probe -- b1_capabilities_have_vm_golden_outputs --exact --nocapture
# 配置受控工具链后执行：
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_probe -- --ignored b1_capabilities_match_native_golden_outputs --exact --nocapture
```

仅修改本目录教程也会触发维护回归。源码/输出逐字比较和退出码要求不变；
接线与实际取得的证据见 [B1 CI 记录](../../DevDocs/20o-b1-freeze-draft.md#八本轮-ci-接线与验证)。
没有对应提交的三平台证据时仍为 planned。

## 兼容与废弃

七个库按接口版本 1 兼容新增；历史 B0/FIX-A 证据基于 ABI 1.17，当前仓库探测使用 1.18。
error_code 已确认在独立 FIX-B 升级至 ABI 1.18，再交付七库；不能把读取前置混在纯库提交中。探测函数不属于公开 API，不进入官方加载器、
普通依赖求解或归档身份。只有产品实现、完整教程、示例、测试与证据均通过，才可升级
为 `verified`；不能仅因本页探测成功就宣称模块完成。

## 故障排查与下一步

六项边界已确认，FIX-A 新增 MIN/MAX、gcd 和平方求幂探测已接入两路精确入口；产品导入/移除/归档与完整资源验收仍待实现批。
补充夹具和证据见 [FIX-A 记录](../../DevDocs/20o-b1-fix-a-contract-closeout.md)。

若无法找到完整示例，先确认位于仓库根；原生测试缺少 LLVM/Runtime 配置时应明确失败，
不能把环境跳过当成功。继续阅读 [整数统计](statistics.md)、
[官方库索引](README.md) 或 [B1 冻结草案](../../DevDocs/20o-b1-freeze-draft.md)。
