---
id: library.hex
title: 十六进制编码
status: verified
audience: learner
module: stdlib.hex
stage: 20O-B1-IMP-REM-A/20P/20O-B1-CLOSE
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
  - ../../DevDocs/20o-b1-closeout.md
---

# 十六进制编码

## 适用范围与状态

在字节整数数组与无前缀十六进制文本之间转换，编码输出小写，解码接受大小写。

本页与stdlib.hex登记为verified。适用Xiao 0.1.0、Runtime ABI 1.18、标准库接口版本1。
已验证范围覆盖VM及Windows/Linux/macOS原生O0/O3、资源边界与稳定错误；
[收口记录](../../DevDocs/20o-b1-closeout.md)列出运行号、源码身份与限制。

## 导入与准备

官方库随工具链提供，无需普通包安装。公共路径为std.hex，固定身份为stdlib.hex；
下面的完整示例使用选择导入，所有参数必填，支持既有位置与命名调用规则。
使用包含本批源码的工具链；从仓库根运行示例时，标准库加载根应指向本仓库的stdlib目录。

## 五分钟运行与完整示例

从仓库根执行：

```powershell
xiao run tests/fixtures/20o-b1/imp/hex/product.xiao
```

以下代码与[实际产品夹具](../../../tests/fixtures/20o-b1/imp/hex/product.xiao)逐字一致；
输出由[独立黄金](../../../tests/fixtures/20o-b1/imp/hex/golden.json)核对。

<!-- b1-imp-executable-example -->
```xiao
from std.hex import encode, decode

values = [0, 1, 15, 16, 127, 128, 255]
alias = values
print(encode(values))
print(encode(bytes = [0, 255]))
print(alias[0])
print(encode([]) == "")
for byte in decode(text = "00aAFF")
    print(byte)
empty_count = 0
for empty_byte in decode("")
    empty_count = empty_count + 1
print(empty_count)
all_bytes = []
next_byte = 0
while next_byte < 256
    array_push(all_bytes, next_byte)
    next_byte = next_byte + 1
print(encode(all_bytes))
array_push(all_bytes, 0)
print(encode(all_bytes) == "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff00")
expected_byte = 0
for decoded_byte in decode("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff")
    if decoded_byte != expected_byte
        raise Error(code = "HEX_GOLDEN_MISMATCH")
    expected_byte = expected_byte + 1
print(expected_byte)
first = decode("00")
second = decode("00")
array_push(first, 1)
second_count = 0
for second_byte in second
    second_count = second_count + 1
print(second_count)
bad = [7, 256]
try
    print(encode(bad))
catch invalid_byte as Error
    print(error_code(invalid_byte))
print(bad[1])
try
    print(encode([-1]))
catch negative_byte as Error
    print(error_code(negative_byte))
try
    print(decode("g"))
catch invalid_length as Error
    print(error_code(invalid_length))
try
    print(decode("é"))
catch odd_codepoints as Error
    print(error_code(odd_codepoints))
try
    print(decode("é0"))
catch invalid_unicode as Error
    print(error_code(invalid_unicode))
for invalid_text in ["0x", "00 ", "00\n0", "gg"]
    try
        print(decode(invalid_text))
    catch invalid_character as Error
        print(error_code(invalid_character))
over = [256]
over_count = 0
while over_count < 65536
    array_push(over, 0)
    over_count = over_count + 1
try
    print(encode(over))
catch encode_limit as ResourceError
    print(error_code(encode_limit))
print(over[0])
large = "0"
doublings = 0
while doublings < 17
    large = large + large
    doublings = doublings + 1
try
    print(decode("g" + large))
catch decode_limit as ResourceError
    print(error_code(decode_limit))
for recovered_byte in decode("01")
    print(recovered_byte)
```

独立黄金输出：

```text
00010f107f80ff
00ff
0
true
0
170
255
0
000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff
true
256
1
HEX_INVALID_BYTE
256
HEX_INVALID_BYTE
HEX_INVALID_LENGTH
HEX_INVALID_LENGTH
HEX_INVALID_CHARACTER
HEX_INVALID_CHARACTER
HEX_INVALID_LENGTH
HEX_INVALID_CHARACTER
HEX_INVALID_CHARACTER
HEX_LIMIT_EXCEEDED
256
HEX_LIMIT_EXCEEDED
1
```

## 核心概念与所有权

字节使用[int]表示，每项为0～255。解码拒绝奇数长度、0x前缀、空白、分隔符和非十六进制字符；空数组与空文本互转。字符码点不能直接当作UTF-8字节。

输入字节数组只读、不消费；decode返回独立的新数组，encode返回保持有效生命周期的不可变字符串。失败不修改输入，也不返回部分结果；没有需手工关闭的资源。
打印示例沿用stdout能力。

## API

| 已确认签名 | 已确认边界 |
| --- | --- |
| `encode([int] bytes) -> str` | 字节 0～255；每字节两位小写、无前缀；空数组变空串 |
| `decode(str text) -> [int]` | 大小写均接受；拒绝奇数长度、空白、0x 和坏字符；空串变空数组 |

参数名属于契约。完整签名、边界与检查顺序以[B1冻结契约](../../DevDocs/20o-b1-freeze-draft.md#三七库已确认签名与调用边界f2f8f12)为准。

## 错误与恢复

示例在catch中使用全局自由函数error_code读取原错误码，读取不消费错误对象。

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| Error / HEX_INVALID_BYTE | encode元素不在0～255 | 修正字节值 |
| Error / HEX_INVALID_LENGTH | decode码点数为奇数，先于坏字符检查 | 补齐每个字节的两位编码 |
| Error / HEX_INVALID_CHARACTER | 含非十六进制字符、前缀、空白或Unicode | 提供无分隔的ASCII十六进制文本 |
| ResourceError / HEX_LIMIT_EXCEEDED | 输入或结果超过资源上限 | 按预算拆分输入 |

调用或静态类型错误沿用X04-TYPE-002，内部动态类型边界沿用X06-RUNTIME-002。
先处理调用/类型和参数域，再按[统一错误顺序](../../DevDocs/20o-b1-freeze-draft.md#61-已确认检查优先级)检查输入、内容与结果预算。
可本地化的message不参与错误身份。error_code的[独立验收](../../DevDocs/20o-b1-fix-b-closeout.md)不开放error.code成员访问。

## 资源、性能与安全边界

encode最多65,536项，结果最多131,072 ASCII字节；decode输入最多131,072 UTF-8字节，结果最多65,536项。输入预算先于偶数长度检查，随后从左到右校验字符。

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

下面保留[B0探测文件](../../../tests/fixtures/docs/hex/probe.xiao)的原文，仍进CI；
它的局部probe函数、B0_*错误与caught-*标签只描述历史，不属于当前公开接口。
正常使用本库应运行本页前面的产品示例。

<!-- b0-executable-example -->
```xiao
# B0 字节整数、字符查表与错误传播探测，不是公开编码器。
from stdlib.text import find, lower_ascii

def probe_hex_byte(int value) -> str
    if value < 0 or value > 255
        raise Error(code = "B0_BYTE_RANGE")
    digits = "0123456789abcdef"
    high = value // 16
    low = value % 16
    return digits[high] + digits[low]

encoded = ""
for byte in [0, 1, 15, 16, 127, 128, 255]
    encoded = encoded + probe_hex_byte(byte)
print(encoded)
digits = "0123456789abcdef"
print(find(digits, "f") * 16 + find(digits, lower_ascii("F")))
print(find(digits, "g"))
try
    print(probe_hex_byte(256))
catch invalid_byte as Error
    print("caught-byte-range")
```
