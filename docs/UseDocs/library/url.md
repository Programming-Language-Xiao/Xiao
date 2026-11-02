---
id: library.url
title: URL 组件转义
status: verified
audience: learner
module: stdlib.url
stage: 20O-B1-IMP-REM-B/20P/20O-B1-CLOSE
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
  - ../../DevDocs/20o-b1-closeout.md
---

# URL 组件转义

## 适用范围与状态

UTF-8组件的百分号编码与严格解码；适合单个URL组件，不解析完整URL。

本页与stdlib.url登记为verified。适用Xiao 0.1.0、Runtime ABI 1.18、标准库接口版本1。
已验证范围覆盖VM及Windows/Linux/macOS原生O0/O3、资源边界与稳定错误；
[收口记录](../../DevDocs/20o-b1-closeout.md)列出运行号、源码身份与限制。

## 导入与准备

官方库随工具链提供，无需普通包安装。公共路径为std.url，固定身份为stdlib.url；
下面的完整示例使用选择导入，所有参数必填，支持既有位置与命名调用规则。
使用包含本批源码的工具链；从仓库根运行示例时，标准库加载根应指向本仓库的stdlib目录。

## 五分钟运行与完整示例

从仓库根执行：

```powershell
xiao run tests/fixtures/20o-b1/imp/url/product.xiao
```

以下代码与[实际产品夹具](../../../tests/fixtures/20o-b1/imp/url/product.xiao)逐字一致；
输出由[独立黄金](../../../tests/fixtures/20o-b1/imp/url/golden.json)核对。

<!-- b1-imp-executable-example -->
```xiao
from std.url import encode_component, decode_component

print(encode_component("AZaz09-._~"))
print(encode_component(text = " /+%?#=&:"))
print(encode_component("雪😀é"))
print(decode_component("one%20two+three"))
print(decode_component(text = "%e9%9b%aa"))
print(decode_component("雪+é"))
print(decode_component("%2e%2e%2f"))
print(encode_component("") == "")
print(decode_component("") == "")
for point in [0, 127, 128, 2047, 2048, 55295, 57344, 65535, 65536, 1114111]
    print(encode_component(chr(point)))
for encoded_point in ["%00", "%7F", "%C2%80", "%DF%BF", "%E0%A0%80", "%ED%9F%BF", "%EE%80%80", "%EF%BF%BF", "%F0%90%80%80", "%F4%8F%BF%BF"]
    print(int(decode_component(encoded_point)))
print(encode_component(chr(65279) + "a"))
bom = decode_component("%EF%BB%BFa")
print(int(bom[0]))
print(bom[1])
print(decode_component("%65%CC%81") == "é")
for invalid_escape in ["%", "%0", "%G0", "%0g", "%雪0", "abc%2", "%F"]
    try
        print(decode_component(invalid_escape))
    catch escape_error as Error
        print(error_code(escape_error))
for invalid_utf8 in ["%80", "%BF", "%C0%AF", "%C1%BF", "%C2", "%E0%80%80", "%E0%A0", "%ED%A0%80", "%F0%80%80%80", "%F4%90%80%80", "%F5%80%80%80", "%F8%88%80%80%80", "%FE", "%FF", "%E2%28%A1", "%E2%C2%A1", "%C2A", "%E2雪", "%F0%90%80"]
    try
        print(decode_component(invalid_utf8))
    catch utf8_error as Error
        print(error_code(utf8_error))
for first_utf8 in ["%FF%G0", "%E0%80%G0", "%ED%A0%G0", "%F4%90%G0"]
    try
        print(decode_component(first_utf8))
    catch earlier_utf8 as Error
        print(error_code(earlier_utf8))
for first_escape in ["%G0%FF", "%C2%G0"]
    try
        print(decode_component(first_escape))
    catch earlier_escape as Error
        print(error_code(earlier_escape))
large = "x"
doublings = 0
while doublings < 16
    large = large + large
    doublings = doublings + 1
try
    print(encode_component(large + "x"))
catch encode_limit as ResourceError
    print(error_code(encode_limit))
very_large = ""
block = "x"
remaining = 196609
while remaining > 0
    if remaining % 2 == 1
        very_large = very_large + block
    remaining = remaining // 2
    if remaining > 0
        block = block + block
try
    print(decode_component(very_large))
catch decode_limit as ResourceError
    print(error_code(decode_limit))
print(decode_component("%2B"))
```

独立黄金输出：

```text
AZaz09-._~
%20%2F%2B%25%3F%23%3D%26%3A
%E9%9B%AA%F0%9F%98%80e%CC%81
one two+three
雪
雪+é
../
true
true
%00
%7F
%C2%80
%DF%BF
%E0%A0%80
%ED%9F%BF
%EE%80%80
%EF%BF%BF
%F0%90%80%80
%F4%8F%BF%BF
0
127
128
2047
2048
55295
57344
65535
65536
1114111
%EF%BB%BFa
65279
a
true
URL_INVALID_ESCAPE
URL_INVALID_ESCAPE
URL_INVALID_ESCAPE
URL_INVALID_ESCAPE
URL_INVALID_ESCAPE
URL_INVALID_ESCAPE
URL_INVALID_ESCAPE
URL_INVALID_UTF8
URL_INVALID_UTF8
URL_INVALID_UTF8
URL_INVALID_UTF8
URL_INVALID_UTF8
URL_INVALID_UTF8
URL_INVALID_UTF8
URL_INVALID_UTF8
URL_INVALID_UTF8
URL_INVALID_UTF8
URL_INVALID_UTF8
URL_INVALID_UTF8
URL_INVALID_UTF8
URL_INVALID_UTF8
URL_INVALID_UTF8
URL_INVALID_UTF8
URL_INVALID_UTF8
URL_INVALID_UTF8
URL_INVALID_UTF8
URL_INVALID_UTF8
URL_INVALID_UTF8
URL_INVALID_UTF8
URL_INVALID_UTF8
URL_INVALID_ESCAPE
URL_INVALID_ESCAPE
URL_LIMIT_EXCEEDED
URL_LIMIT_EXCEEDED
+
```

## 核心概念与所有权

编码只保留ASCII字母、数字和-._~，其余按UTF-8字节输出大写%HH。解码接受十六进制大小写，保留+和普通Unicode，拒绝非法转义及非最短、代理、越界、截断等非法UTF-8。NUL、BOM和组合字符保留，不做路径或Unicode规范化。

输入字符串不可变、不消费调用方引用；返回字符串保持独立有效的生命周期，允许共享底层不可变存储。失败不返回部分结果；没有需手工关闭的资源。
打印示例沿用stdout能力。

## API

| 已确认签名 | 已确认边界 |
| --- | --- |
| `encode_component(str text) -> str` | UTF-8；仅保留 ASCII 字母数字及 -._~；大写 %HH |
| `decode_component(str text) -> str` | %HH 接受十六进制大小写；+ 与普通 Unicode 原样保留；拒绝坏转义/非法 UTF-8 |

参数名属于契约。完整签名、边界与检查顺序以[B1冻结契约](../../DevDocs/20o-b1-freeze-draft.md#三七库已确认签名与调用边界f2f8f12)为准。

## 错误与恢复

示例在catch中使用全局自由函数error_code读取原错误码，读取不消费错误对象。

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| Error / URL_INVALID_ESCAPE | %HH不完整或包含非十六进制字符 | 修正百分号转义 |
| Error / URL_INVALID_UTF8 | 还原字节不构成规范UTF-8 | 提供有效UTF-8，不用替换字符掩盖错误 |
| ResourceError / URL_LIMIT_EXCEEDED | 输入或实际结果超限 | 按字节预算拆分输入 |

调用或静态类型错误沿用X04-TYPE-002，内部动态类型边界沿用X06-RUNTIME-002。
先处理调用/类型和参数域，再按[统一错误顺序](../../DevDocs/20o-b1-freeze-draft.md#61-已确认检查优先级)检查输入、内容与结果预算。
可本地化的message不参与错误身份。error_code的[独立验收](../../DevDocs/20o-b1-fix-b-closeout.md)不开放error.code成员访问。

## 资源、性能与安全边界

encode输入最多65,536 UTF-8字节，输出最多196,608 ASCII字节；decode输入最多196,608 UTF-8字节，结果最多65,536字节。大小按字节计算。每个转义先查%HH语法，再查UTF-8，保持从左到右的首错。

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

下面保留[B0探测文件](../../../tests/fixtures/docs/url/probe.xiao)的原文，仍进CI；
它的局部probe函数、B0_*错误与caught-*标签只描述历史，不属于当前公开接口。
正常使用本库应运行本页前面的产品示例。

<!-- b0-executable-example -->
```xiao
# B0 探测 UTF-8 字节编码所需的码点运算，不是完整 URL 解析器。
from stdlib.text import find

def probe_percent_byte(int byte) -> str
    digits = "0123456789ABCDEF"
    high = byte // 16
    low = byte % 16
    return "%" + digits[high] + digits[low]

snow = int("雪")
print(probe_percent_byte(224 + snow // 4096) + probe_percent_byte(128 + snow // 64 % 64) + probe_percent_byte(128 + snow % 64))
face = int("😀")
print(probe_percent_byte(240 + face // 262144) + probe_percent_byte(128 + face // 4096 % 64) + probe_percent_byte(128 + face // 64 % 64) + probe_percent_byte(128 + face % 64))
print(probe_percent_byte(32) + probe_percent_byte(43) + probe_percent_byte(47))
print(chr((233 - 224) * 4096 + (155 - 128) * 64 + (170 - 128)))
print(chr((240 - 240) * 262144 + (159 - 128) * 4096 + (152 - 128) * 64 + (128 - 128)))
print(find("0123456789ABCDEF", "G"))
print(int("雪"))
```
