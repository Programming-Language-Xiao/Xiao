---
id: library.base64
title: Base64 编码
status: verified
audience: learner
module: stdlib.base64
stage: 20O-B1-IMP-REM-A/20P/20O-B1-CLOSE
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
  - ../../DevDocs/20o-b1-closeout.md
---

# Base64 编码

## 适用范围与状态

严格的Base64字节编码，标准与URL-safe字母表均支持，两个入口都必须传入url_safe模式。

本页与stdlib.base64登记为verified。适用Xiao 0.1.0、Runtime ABI 1.18、标准库接口版本1。
已验证范围覆盖VM及Windows/Linux/macOS原生O0/O3、资源边界与稳定错误；
[收口记录](../../DevDocs/20o-b1-closeout.md)列出运行号、源码身份与限制。

## 导入与准备

官方库随工具链提供，无需普通包安装。公共路径为std.base64，固定身份为stdlib.base64；
下面的完整示例使用选择导入，所有参数必填，支持既有位置与命名调用规则。
使用包含本批源码的工具链；从仓库根运行示例时，标准库加载根应指向本仓库的stdlib目录。

## 五分钟运行与完整示例

从仓库根执行：

```powershell
xiao run tests/fixtures/20o-b1/imp/base64/product.xiao
```

以下代码与[实际产品夹具](../../../tests/fixtures/20o-b1/imp/base64/product.xiao)逐字一致；
输出由[独立黄金](../../../tests/fixtures/20o-b1/imp/base64/golden.json)核对。

<!-- b1-imp-executable-example -->
```xiao
from std.base64 import encode, decode

for sample in ["", "f", "fo", "foo", "foob", "fooba", "foobar"]
    sample_bytes = []
    for character in sample
        array_push(sample_bytes, int(character))
    print(encode(sample_bytes, false))
    print(encode(sample_bytes, true))
for encoded_text in ["", "Zg==", "Zm8=", "Zm9v", "Zm9vYg==", "Zm9vYmE=", "Zm9vYmFy"]
    restored_text = ""
    for decoded_byte in decode(encoded_text, false)
        restored_text = restored_text + chr(decoded_byte)
    print(restored_text)
values = [251, 255, 255]
alias = values
print(encode(values, false))
print(encode(values, true))
print(encode(url_safe = true, bytes = [251, 255]))
print(alias[0])
for standard_byte in decode(text = "+///", url_safe = false)
    print(standard_byte)
for safe_byte in decode(url_safe = true, text = "-___")
    print(safe_byte)
for zero_byte in decode("AA==", false)
    print(zero_byte)
empty_count = 0
for empty_byte in decode("", true)
    empty_count = empty_count + 1
print(empty_count)
all_values = []
next_byte = 0
while next_byte < 256
    array_push(all_values, next_byte)
    next_byte = next_byte + 1
print(encode(all_values, false))
print(encode(all_values, true))
expected_byte = 0
for whole_byte in decode("AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0+P0BBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5fYGFiY2RlZmdoaWprbG1ub3BxcnN0dXZ3eHl6e3x9fn+AgYKDhIWGh4iJiouMjY6PkJGSk5SVlpeYmZqbnJ2en6ChoqOkpaanqKmqq6ytrq+wsbKztLW2t7i5uru8vb6/wMHCw8TFxsfIycrLzM3Oz9DR0tPU1dbX2Nna29zd3t/g4eLj5OXm5+jp6uvs7e7v8PHy8/T19vf4+fr7/P3+/w==", false)
    if whole_byte != expected_byte
        raise Error(code = "BASE64_GOLDEN_MISMATCH")
    expected_byte = expected_byte + 1
print(expected_byte)
safe_count = 0
for whole_safe_byte in decode("AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0-P0BBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5fYGFiY2RlZmdoaWprbG1ub3BxcnN0dXZ3eHl6e3x9fn-AgYKDhIWGh4iJiouMjY6PkJGSk5SVlpeYmZqbnJ2en6ChoqOkpaanqKmqq6ytrq-wsbKztLW2t7i5uru8vb6_wMHCw8TFxsfIycrLzM3Oz9DR0tPU1dbX2Nna29zd3t_g4eLj5OXm5-jp6uvs7e7v8PHy8_T19vf4-fr7_P3-_w==", true)
    if whole_safe_byte != safe_count
        raise Error(code = "BASE64_SAFE_GOLDEN_MISMATCH")
    safe_count = safe_count + 1
print(safe_count)
extra_count = 0
while extra_count < 129
    array_push(all_values, 0)
    extra_count = extra_count + 1
print(encode(all_values, false) == "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0+P0BBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5fYGFiY2RlZmdoaWprbG1ub3BxcnN0dXZ3eHl6e3x9fn+AgYKDhIWGh4iJiouMjY6PkJGSk5SVlpeYmZqbnJ2en6ChoqOkpaanqKmqq6ytrq+wsbKztLW2t7i5uru8vb6/wMHCw8TFxsfIycrLzM3Oz9DR0tPU1dbX2Nna29zd3t/g4eLj5OXm5+jp6uvs7e7v8PHy8/T19vf4+fr7/P3+/wAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA==")
first = decode("AA==", false)
second = decode("AA==", false)
array_push(first, 255)
second_count = 0
for second_byte in second
    second_count = second_count + 1
print(second_count)
bad = [7, 256]
try
    print(encode(bad, false))
catch bad_byte as Error
    print(error_code(bad_byte))
print(bad[1])
try
    print(encode([-1], true))
catch negative_byte as Error
    print(error_code(negative_byte))
for invalid_text in ["Zg", "Zg=", "Zg===", " Zg==", "Zg==\n", "A===", "=AAA", "AA=A", "Zg==AAAA", "Zh==", "Zm9=", "-___", "AA_/", "éAAA", "AA A", "!!!!", "AB=="]
    try
        print(decode(invalid_text, false))
    catch invalid_encoding as Error
        print(error_code(invalid_encoding))
for invalid_safe_text in ["+///", "+___", "/w==", "AA+/", "Zh=="]
    try
        print(decode(invalid_safe_text, true))
    catch invalid_safe_encoding as Error
        print(error_code(invalid_safe_encoding))
over = [256]
over_count = 0
while over_count < 65536
    array_push(over, 0)
    over_count = over_count + 1
try
    print(encode(over, true))
catch encode_limit as ResourceError
    print(error_code(encode_limit))
print(over[0])
large = ""
block = "A"
remaining = 87385
while remaining > 0
    if remaining % 2 == 1
        large = large + block
    remaining = remaining // 2
    if remaining > 0
        block = block + block
try
    print(decode(large, false))
catch decode_limit as ResourceError
    print(error_code(decode_limit))
for recovered_byte in decode("AQ==", true)
    print(recovered_byte)
```

独立黄金输出：

```text


Zg==
Zg==
Zm8=
Zm8=
Zm9v
Zm9v
Zm9vYg==
Zm9vYg==
Zm9vYmE=
Zm9vYmE=
Zm9vYmFy
Zm9vYmFy

f
fo
foo
foob
fooba
foobar
+///
-___
-_8=
251
251
255
255
251
255
255
0
0
AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0+P0BBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5fYGFiY2RlZmdoaWprbG1ub3BxcnN0dXZ3eHl6e3x9fn+AgYKDhIWGh4iJiouMjY6PkJGSk5SVlpeYmZqbnJ2en6ChoqOkpaanqKmqq6ytrq+wsbKztLW2t7i5uru8vb6/wMHCw8TFxsfIycrLzM3Oz9DR0tPU1dbX2Nna29zd3t/g4eLj5OXm5+jp6uvs7e7v8PHy8/T19vf4+fr7/P3+/w==
AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0-P0BBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5fYGFiY2RlZmdoaWprbG1ub3BxcnN0dXZ3eHl6e3x9fn-AgYKDhIWGh4iJiouMjY6PkJGSk5SVlpeYmZqbnJ2en6ChoqOkpaanqKmqq6ytrq-wsbKztLW2t7i5uru8vb6_wMHCw8TFxsfIycrLzM3Oz9DR0tPU1dbX2Nna29zd3t_g4eLj5OXm5-jp6uvs7e7v8PHy8_T19vf4-fr7_P3-_w==
256
256
true
1
BASE64_INVALID_BYTE
256
BASE64_INVALID_BYTE
BASE64_INVALID_ENCODING
BASE64_INVALID_ENCODING
BASE64_INVALID_ENCODING
BASE64_INVALID_ENCODING
BASE64_INVALID_ENCODING
BASE64_INVALID_ENCODING
BASE64_INVALID_ENCODING
BASE64_INVALID_ENCODING
BASE64_INVALID_ENCODING
BASE64_INVALID_ENCODING
BASE64_INVALID_ENCODING
BASE64_INVALID_ENCODING
BASE64_INVALID_ENCODING
BASE64_INVALID_ENCODING
BASE64_INVALID_ENCODING
BASE64_INVALID_ENCODING
BASE64_INVALID_ENCODING
BASE64_INVALID_ENCODING
BASE64_INVALID_ENCODING
BASE64_INVALID_ENCODING
BASE64_INVALID_ENCODING
BASE64_INVALID_ENCODING
BASE64_LIMIT_EXCEEDED
256
BASE64_LIMIT_EXCEEDED
1
```

## 核心概念与所有权

false使用标准字母表，true使用URL-safe字母表；模式专属字符不可混用。两种模式均要求正确补位，拒绝空白、缺少或多余补位、中间补位和非零未使用尾位，不自动修正输入。空输入互转。

输入字节数组只读、不消费；decode返回独立的新数组，encode返回保持有效生命周期的不可变字符串。失败不修改输入，也不返回部分结果；没有需手工关闭的资源。
打印示例沿用stdout能力。

## API

| 已确认签名 | 已确认边界 |
| --- | --- |
| `encode([int] bytes, bool url_safe) -> str` | 标准/URL-safe 由必填参数选择；强制补位、不换行；字节 0～255 |
| `decode(str text, bool url_safe) -> [int]` | 严格检查所选模式/字母表、长度、补位和尾位；拒绝混用和空白；空输入互转 |

参数名属于契约。完整签名、边界与检查顺序以[B1冻结契约](../../DevDocs/20o-b1-freeze-draft.md#三七库已确认签名与调用边界f2f8f12)为准。

## 错误与恢复

示例在catch中使用全局自由函数error_code读取原错误码，读取不消费错误对象。

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| Error / BASE64_INVALID_BYTE | 字节不在0～255 | 修正输入字节 |
| Error / BASE64_INVALID_ENCODING | 长度、所选字母表、补位或尾位不规范，或包含空白 | 使用匹配模式的规范编码 |
| ResourceError / BASE64_LIMIT_EXCEEDED | 输入或实际解码结果超限 | 按资源预算拆分输入 |

调用或静态类型错误沿用X04-TYPE-002，内部动态类型边界沿用X06-RUNTIME-002。
先处理调用/类型和参数域，再按[统一错误顺序](../../DevDocs/20o-b1-freeze-draft.md#61-已确认检查优先级)检查输入、内容与结果预算。
可本地化的message不参与错误身份。error_code的[独立验收](../../DevDocs/20o-b1-fix-b-closeout.md)不开放error.code成员访问。

## 资源、性能与安全边界

encode最多65,536字节整数，结果最多87,384 ASCII字节；decode输入最多87,384 UTF-8字节，实际结果最多65,536项。文本长度合规仍可能结果超限，最后四元组先查字母表、补位和尾位，再查真实结果预算。

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

下面保留[B0探测文件](../../../tests/fixtures/docs/base64/probe.xiao)的原文，仍进CI；
它的局部probe函数、B0_*错误与caught-*标签只描述历史，不属于当前公开接口。
正常使用本库应运行本页前面的产品示例。

<!-- b0-executable-example -->
```xiao
# B0 探测 24 位分组、余数补位及反向查表；不是完整 Base64 API。
from stdlib.text import find

alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
bytes = [102, 111, 111]
packed = bytes[0] * 65536 + bytes[1] * 256 + bytes[2]
a = packed // 262144
b = packed // 4096 % 64
c = packed // 64 % 64
d = packed % 64
print(alphabet[a] + alphabet[b] + alphabet[c] + alphabet[d])
one = bytes[0]
one_a = one // 4
one_b = one % 4 * 16
print(alphabet[one_a] + alphabet[one_b] + "==")
two = bytes[0] * 256 + bytes[1]
two_a = two // 1024
two_b = two // 16 % 64
two_c = two % 16 * 4
print(alphabet[two_a] + alphabet[two_b] + alphabet[two_c] + "=")
decoded = find(alphabet, "Z") * 262144 + find(alphabet, "m") * 4096 + find(alphabet, "9") * 64 + find(alphabet, "v")
print(decoded // 65536)
print(decoded // 256 % 256)
print(decoded % 256)
print(find(alphabet, "!"))
print(find(alphabet, "g") % 16 == 0)
print(find(alphabet, "h") % 16 == 0)
```
