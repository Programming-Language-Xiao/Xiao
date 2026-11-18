---
id: library.fmt
title: 文本宽度与整数格式
status: verified
audience: learner
module: stdlib.fmt
stage: 20O-B0/B1-FIX-A/B1-IMP/20P/20O-B1-CLOSE
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
  - ../../DevDocs/20o-b1-closeout.md
---

# 文本宽度与整数格式

## 适用范围与状态

按Unicode码点进行左右填充，以及把固定int转换为2～36进制的小写文本。

本页与stdlib.fmt登记为verified。适用Xiao 0.1.0、Runtime ABI 1.18、标准库接口版本1。
已验证范围覆盖VM及Windows/Linux/macOS原生O0/O3、资源边界与稳定错误；
[收口记录](../../DevDocs/20o-b1-closeout.md)列出运行号、源码身份与限制。

## 导入与准备

官方库随工具链提供，无需普通包安装。公共路径为std.fmt，固定身份为stdlib.fmt；
下面的完整示例使用选择导入，所有参数必填，支持既有位置与命名调用规则。
使用包含本批源码的工具链；从仓库根运行示例时，标准库加载根应指向本仓库的stdlib目录。

## 五分钟运行与完整示例

从仓库根执行：

```powershell
xiao run tests/fixtures/20o-b1/imp/fmt/product.xiao
```

以下代码与[实际产品夹具](../../../tests/fixtures/20o-b1/imp/fmt/product.xiao)逐字一致；
输出由[独立黄金](../../../tests/fixtures/20o-b1/imp/fmt/golden.json)核对。

<!-- b1-imp-executable-example -->
```xiao
from std.fmt import pad_left, pad_right, format_int

print(pad_left("雪😀", 4, "."))
print(pad_right("雪😀", 4, "."))
print(pad_left("long", 2, "."))
print(pad_right("", 3, "0"))
print(pad_left("é", 3, "."))
print(pad_right("x", 0, "😀"))
print(pad_left(text = "x", width = 3, fill = "雪"))
print(pad_right(text = "x", width = 3, fill = "."))
minimum = -9223372036854775807 - 1
print(format_int(minimum, 10))
print(format_int(minimum, 2))
print(format_int(minimum, 36))
print(format_int(0, 2))
print(format_int(35, 36))
print(format_int(value = -255, radix = 16))
try
    print(pad_left("x", -1, ""))
catch width as Error
    print(error_code(width))
try
    print(pad_right("long", 0, ""))
catch empty_fill as Error
    print(error_code(empty_fill))
try
    print(pad_left("x", 0, "é"))
catch two_points as Error
    print(error_code(two_points))
try
    print(pad_left("x", 65537, "."))
catch limit as ResourceError
    print(error_code(limit))
try
    print(format_int(minimum, 1))
catch radix as Error
    print(error_code(radix))
try
    print(pad_right("x", 65537, "."))
catch right_limit as ResourceError
    print(error_code(right_limit))
try
    print(pad_left("x", 65537, ""))
catch fill_before_limit as Error
    print(error_code(fill_before_limit))
large = "x"
doublings = 0
while doublings < 16
    large = large + large
    doublings = doublings + 1
oversized = large + "x"
try
    print(pad_left(oversized, 0, "."))
catch text_limit_left as ResourceError
    print(error_code(text_limit_left))
try
    print(pad_right(oversized, 0, "."))
catch text_limit_right as ResourceError
    print(error_code(text_limit_right))
minimum_base = 2
while minimum_base <= 36
    print(format_int(value = minimum, radix = minimum_base))
    minimum_base = minimum_base + 1
```

独立黄金输出：

```text
..雪😀
雪😀..
long
000
.é
x
雪雪x
x..
-9223372036854775808
-1000000000000000000000000000000000000000000000000000000000000000
-1y2p0ij32e8e8
0
z
-ff
FMT_INVALID_WIDTH
FMT_INVALID_FILL
FMT_INVALID_FILL
FMT_LIMIT_EXCEEDED
FMT_INVALID_RADIX
FMT_LIMIT_EXCEEDED
FMT_INVALID_FILL
FMT_LIMIT_EXCEEDED
FMT_LIMIT_EXCEEDED
-1000000000000000000000000000000000000000000000000000000000000000
-2021110011022210012102010021220101220222
-20000000000000000000000000000000
-1104332401304422434310311213
-1540241003031030222122212
-22341010611245052052301
-1000000000000000000000
-67404283172107811828
-9223372036854775808
-1728002635214590698
-41a792678515120368
-10b269549075433c38
-4340724c6c71dc7a8
-160e2ad3246366808
-8000000000000000
-33d3d8307b214009
-16agh595df825fa8
-ba643dci0ffeehi
-5cbfjia3fh26ja8
-2heiciiie82dh98
-1adaibb21dckfa8
-i6k448cf4192c3
-acd772jnc9l0l8
-64ie1focnn5g78
-3igoecjbmca688
-27c48l5b37oaoq
-1bk39f3ah3dmq8
-q1se8f0m04isc
-hajppbc1fc208
-bm03i95hia438
-8000000000000
-5hg4ck9jd4u38
-3tdtk1v8j6tpq
-2pijmikexrxp8
-1y2p0ij32e8e8
```

## 核心概念与所有权

width按码点计数，不是字节数、字素簇或终端列宽。fill必须恰好一个码点，即使无需填充也检查；原文不截断。format_int无前缀、不补零、负号在最前，最小int在全部支持进制均可格式化。不提供locale、浮点或模板格式化。

输入字符串不可变、不消费调用方引用；返回字符串保持独立有效的生命周期，允许共享底层不可变存储。失败不返回部分结果；没有需手工关闭的资源。
打印示例沿用stdout能力。

## API

| 已确认签名 | 已确认边界 |
| --- | --- |
| `pad_left(str text, int width, str fill) -> str` | 按码点左填充，负宽度拒绝；即使无需填充也检查 fill 是单码点 |
| `pad_right(str text, int width, str fill) -> str` | 同样规则的右填充；原文不截断 |
| `format_int(int value, int radix) -> str` | 2～36 进制、小写、无前缀、负号最前；全范围 int，0 为 0 |

参数名属于契约。完整签名、边界与检查顺序以[B1冻结契约](../../DevDocs/20o-b1-freeze-draft.md#三七库已确认签名与调用边界f2f8f12)为准。

## 错误与恢复

示例在catch中使用全局自由函数error_code读取原错误码，读取不消费错误对象。

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| Error / FMT_INVALID_WIDTH | width小于0 | 使用非负宽度 |
| Error / FMT_INVALID_FILL | fill不是恰好一个Unicode码点 | 使用单码点填充文本 |
| Error / FMT_INVALID_RADIX | radix不在2～36 | 选择支持的进制 |
| ResourceError / FMT_LIMIT_EXCEEDED | 原文、width或结果超限 | 按相应字节或码点预算缩小参数 |

调用或静态类型错误沿用X04-TYPE-002，内部动态类型边界沿用X06-RUNTIME-002。
先处理调用/类型和参数域，再按[统一错误顺序](../../DevDocs/20o-b1-freeze-draft.md#61-已确认检查优先级)检查输入、内容与结果预算。
可本地化的message不参与错误身份。error_code的[独立验收](../../DevDocs/20o-b1-fix-b-closeout.md)不开放error.code成员访问。

## 资源、性能与安全边界

padding原文最多65,536 UTF-8字节，width为0～65,536码点，结果最多262,144字节。format_int最多输出65个ASCII字符。width、fill和radix的参数域先检查，再检查输入预算；结果预算通过后才构造。

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

下面保留[B0探测文件](../../../tests/fixtures/docs/fmt/probe.xiao)的原文，仍进CI；
它的局部probe函数、B0_*错误与caught-*标签只描述历史，不属于当前公开接口。
正常使用本库应运行本页前面的产品示例。

<!-- b0-executable-example -->
```xiao
# B0 探测码点宽度、填充与整数进制所需操作；不是通用格式化器。
from stdlib.text import length, repeat

def probe_padding(str value, int width, str fill) -> str
    if length(fill) != 1
        raise Error(code = "B0_FILL_WIDTH")
    missing = width - length(value)
    if missing <= 0
        return value
    return repeat(fill, missing) + value

print(probe_padding("雪😀", 4, "."))
print(probe_padding("long", 2, "."))
print(probe_padding("", 3, "0"))
digits = "0123456789abcdef"
number = 255
high = number // 16
low = number % 16
print(digits[high] + digits[low])
negative = -9223372036854775807 - 1
print(str(negative))
try
    print(probe_padding("雪", 3, "ab"))
catch invalid_fill as Error
    print("caught-fill-width")
```
