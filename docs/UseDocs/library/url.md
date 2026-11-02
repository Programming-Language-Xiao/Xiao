---
id: library.url
title: URL 组件转义（B1 教程草案）
status: planned
audience: learner
module: stdlib.url
stage: 20O-B1-IMP-REM-B/20P
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
---

# URL 组件转义（教程草案）

## 适用范围与状态

首批契约只处理组件的百分号编码/解码。完整 URL 分解、路径规范化、网络请求与表单编码不在此已确认 API 范围中。

本页保持 planned：stdlib.url 的产品草稿与完整示例已落地，七库功能和生成器20的已知释放修复已有三平台证据。资源、性能与门禁的当前对账见 [REM证据](../../DevDocs/20o-b1-rem-evidence.md)；本页状态升级归独立B1-CLOSE。
适用 Xiao 0.1.0、Runtime ABI 1.18、标准库接口版本 1。

## 导入与准备

官方库随工具链提供，无需普通包安装。公共路径 std.url，固定身份 stdlib.url：

```xiao
from std.url import encode_component, decode_component
print(encode_component("雪"))
```

输出 %E9%9B%AA。历史 B0 探测保留，产品完整示例另列。

## 五分钟运行与完整示例

从仓库根执行：

```powershell
xiao run tests/fixtures/20o-b1/imp/url/product.xiao
# 历史探测另行保留：
xiao run tests/fixtures/docs/url/probe.xiao
```

[完整源码](../../../tests/fixtures/docs/url/probe.xiao) 与下列代码由测试逐字核对。

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

示例输出 %E9%9B%AA、%F0%9F%98%80、%20%2B%2F，并恢复 雪、😀；最后输出 -1 与码点 38634。

## 核心概念与所有权

百分号后是 UTF-8 字节的两位十六进制。一个雪字需要三个字节，一个 😀 需要四个；str 的索引单位仍是码点。函数返回新字符串，不修改输入。

## 完整产品示例（planned）

以下源与 [产品夹具](../../../tests/fixtures/20o-b1/imp/url/product.xiao) 逐字一致，使用实际模块。UTF-8 临界字节和异常向量独立于产品算法，错误通过 error_code 核对。

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

## 已确认 API 契约（产品草稿）

七项一起冻结已确认；下表是 [B1 逐项契约](../../DevDocs/20o-b1-freeze-draft.md#34-url两项) 的概览，
F1～F13 已由星崽在 FIX-A 逐条确认；所有库函数参数必填，按现有位置/命名规则调用；
输入只读、不消费，失败不改调用方数组。产品草稿已提供下列入口，整体验收仍按 B1 收口。

| 已确认签名 | 已确认边界 |
| --- | --- |
| `encode_component(str text) -> str` | UTF-8；仅保留 ASCII 字母数字及 -._~；大写 %HH |
| `decode_component(str text) -> str` | %HH 接受十六进制大小写；+ 与普通 Unicode 原样保留；拒绝坏转义/非法 UTF-8 |

## 错误与恢复

B1 已确认的稳定 code、类别、触发条件与检查顺序统一见 [错误表](../../DevDocs/20o-b1-freeze-draft.md#六统一错误表f11已确认)，
不在教程另立错误身份。以下为实际产品错误，历史 B0 标签另行保留。

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| Error / URL_INVALID_ESCAPE | 不完整或非十六进制 %HH | 修正转义语法 |
| Error / URL_INVALID_UTF8 | 非最短、代理、越界、截断或孤立续字节 | 使用规范 UTF-8，不替换坏数据 |
| ResourceError / URL_LIMIT_EXCEEDED | 输入或实际结果超过冻结字节预算 | 按预算拆分，整次失败无部分结果 |

`B0_*` 仅是夹具局部错误，不注册为标准库错误身份。示例中的 `caught-*` 是恢复分支标签，
不能作为稳定错误码证据。全局自由函数 `error_code` 已由独立 FIX-B 实现并验收，
其[精确位置/签名/错误表](../../DevDocs/20o-b1-freeze-draft.md#二error_code-的名称位置与签名)与
[三平台验收记录](../../DevDocs/20o-b1-fix-b-closeout.md)已交付；错误成员访问仍未开放。
FIX-B 已实现并验收；stdlib.url 已提供组件产品草稿，完整产品示例调用 error_code。

## 资源、性能与安全边界

**F9 的限额与 F10 的验收目标已确认；当前探测没有实现或证明全部产品资源/性能要求。**

encode 输入最多 65,536 UTF-8 字节、输出最多 196,608 ASCII 字节；decode 输入最多 196,608 UTF-8 字节、结果最多 65,536 字节。大小不是码点数。

精确计量、编码膨胀和超限顺序见 [资源限额](../../DevDocs/20o-b1-freeze-draft.md#四资源限额f9已确认产品未验证)。
文本/编码目标为顺序扫描加至多 O(n log n) 的均衡拼接，禁止隐藏的二次复制/扫描；
翻倍比≤3、最大输入 VM≤10s/原生≤2s、增量内存≤64 MiB 是已确认的验收目标；当前分平台实测见 [REM证据](../../DevDocs/20o-b1-rem-evidence.md)，
计时排除启动/编译/打印，具体方法见 [性能预算](../../DevDocs/20o-b1-freeze-draft.md#五性能预算与测量协议f10)。

入口看门狗只保护测试；F10另按已编译调用计时，不代表库API承诺运行时超时。纯计算不新增宿主能力，打印沿用 stdout。

## 示例如何进入 CI

历史探测位于 tests/fixtures/docs/url/probe.xiao，产品位于 tests/fixtures/20o-b1/imp/url/product.xiao，均与本页完整代码逐字核对。
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
error_code 已随独立 FIX-B 在 ABI 1.18 实现并验收；URL 产品草稿已进入 IMP，整批验收留待 CLOSE。
探测函数不属于公开 API，不进入官方加载器、
普通依赖求解或归档身份。只有产品实现、完整教程、示例、测试与证据均通过，才可升级
为 `verified`；不能仅因本页探测成功就宣称模块完成。

## 故障排查与下一步

组件产品草稿已实现，临界/非法 UTF-8、坏百分号、混合首错及最大 ASCII/Unicode 资源边界
均进入真实产品回归；完整 F10 和生命周期验收另行收口。
补充夹具和证据见 [FIX-A 记录](../../DevDocs/20o-b1-fix-a-contract-closeout.md)。

若无法找到完整示例，先确认位于仓库根；原生测试缺少 LLVM/Runtime 配置时应明确失败，
不能把环境跳过当成功。继续阅读 [CSV 字段与记录](csv.md)、
[官方库索引](README.md) 或 [B1 冻结草案](../../DevDocs/20o-b1-freeze-draft.md)。
