---
id: library.fmt
title: 文本宽度与整数格式（B1 教程草案）
status: planned
audience: learner
module: stdlib.fmt
stage: 20O-B0/B1-FIX-A/B1-IMP/20P
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
---

# 文本宽度与整数格式（教程草案）

## 适用范围与状态

为填充、对齐和整数进制提供有语义增量的便利 API，不再建立等价于 str(value) 的占位函数。

本页保持 planned：stdlib.fmt 的产品草稿与完整示例已落地，七库功能和生成器20的已知释放修复已有三平台证据。资源、性能与门禁的当前对账见 [REM证据](../../DevDocs/20o-b1-rem-evidence.md)；本页状态升级归独立B1-CLOSE。
本页保留 B0 历史探测，另附直接导入产品函数的完整示例。适用基线：Xiao 0.1.0、Runtime ABI 1.18、
标准库接口版本 1。逐平台结果见 [B0 探测记录](../../DevDocs/20o-b0-baseline-and-b1-draft.md)。

## 导入与准备

官方库随工具链提供，无需普通包安装；用户路径为 `std.fmt`，
固定身份为 `stdlib.fmt`。检出本批源码后可以导入产品函数；已安装 Xiao 且检出仓库时，
可以直接运行下面的探测文件；源码中的 `probe_` 名称仅在这个示例中有效。

## 五分钟运行与完整示例

从仓库根执行：

```powershell
xiao run tests/fixtures/docs/fmt/probe.xiao
```

[完整源码](../../../tests/fixtures/docs/fmt/probe.xiao) 与下列代码由测试逐字核对。

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

示例输出 ..雪😀、long、000、ff、-9223372036854775808，再输出错误恢复标签。

## 核心概念与所有权

宽度按 Unicode 码点数计算，不能等同终端显示列宽或字素簇。填充字符必须恰好一个码点；不提供 locale 规则。输入只读，返回新字符串。

## B1-IMP 产品完整示例（planned）

以下源码与 [CI 产品夹具](../../../tests/fixtures/20o-b1/imp/fmt/product.xiao) 逐字一致，已接入原有 VM/原生精确目标；命名调用前置已修复；当前七库的功能、性能与释放记录见 [REM证据](../../DevDocs/20o-b1-rem-evidence.md)。B0 探测文件独立保留。

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
```

## 已实现 API（B1-IMP，验收未收口）

七项一起冻结已确认；下表是 [B1 逐项契约](../../DevDocs/20o-b1-freeze-draft.md#36-fmt三项) 的概览，
F1～F13 已由星崽在 FIX-A 逐条确认；所有库函数参数必填，按现有位置/命名规则调用；
输入只读、不消费，失败不改调用方数据。产品示例调用下列冻结入口；B0 probe 继续作独立历史对照。

| 已确认签名 | 已确认边界 |
| --- | --- |
| `pad_left(str text, int width, str fill) -> str` | 按码点左填充，负宽度拒绝；即使无需填充也检查 fill 是单码点 |
| `pad_right(str text, int width, str fill) -> str` | 同样规则的右填充；原文不截断 |
| `format_int(int value, int radix) -> str` | 2～36 进制、小写、无前缀、负号最前；全范围 int，0 为 0 |

## 错误与恢复

B1 已确认的稳定 code、类别、触发条件与检查顺序统一见 [错误表](../../DevDocs/20o-b1-freeze-draft.md#六统一错误表f11已确认)，
不在教程另立错误身份。下面保留的是**当前 B0 探测**的错误与边界，不能当作已交付产品契约。

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| `B0_FILL_WIDTH` | 探测多码点填充被拒绝，非公开错误码 | 使用单码点填充 |
| B0 未覆盖 | 非法进制、宽度或结果超限 | 使用冻结范围内参数 |

`B0_*` 仅是夹具局部错误，不注册为标准库错误身份。示例中的 `caught-*` 是恢复分支标签，
不能作为稳定错误码证据。全局自由函数 `error_code` 已由独立 FIX-B 实现并验收，
其[精确位置/签名/错误表](../../DevDocs/20o-b1-freeze-draft.md#二error_code-的名称位置与签名)与
[三平台验收记录](../../DevDocs/20o-b1-fix-b-closeout.md)已交付；错误成员访问仍未开放。
B0 示例保留原探测写法，通过驱动器机器结果核对 code，不调用产品函数；
新增产品示例直接调用公开入口，用 `error_code` 核对真实错误身份。

## 资源、性能与安全边界

**F9 的限额与 F10 的验收目标已确认；当前探测没有实现或证明全部产品资源/性能要求。**

padding 原文最多 65,536 UTF-8 字节，width 最多 65,536 码点，结果最多 262,144 字节；format_int 最多输出 65 个 ASCII 字符。最小 int 不能先取正绝对值。

精确计量、编码膨胀和超限顺序见 [资源限额](../../DevDocs/20o-b1-freeze-draft.md#四资源限额f9已确认产品未验证)。
文本/编码目标为顺序扫描加至多 O(n log n) 的均衡拼接，禁止隐藏的二次复制/扫描；
翻倍比≤3、最大输入 VM≤10s/原生≤2s、增量内存≤64 MiB 是已确认的验收目标；当前分平台实测见 [REM证据](../../DevDocs/20o-b1-rem-evidence.md)，
计时排除启动/编译/打印，具体方法见 [性能预算](../../DevDocs/20o-b1-freeze-draft.md#五性能预算与测量协议f10)。

入口看门狗只保护测试；F10另按已编译调用计时，不代表库API承诺运行时超时。纯计算不新增宿主能力，打印沿用 stdout。

## 示例如何进入 CI

完整示例仍在 `tests/fixtures/docs/fmt/probe.xiao`，本页完整代码与文件逐字核对。
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
error_code 已随独立 FIX-B 在 ABI 1.18 实现并验收；本模块源码与样例已进入 IMP，完整验收和状态升级留待 CLOSE。
探测函数不属于公开 API，不进入官方加载器、
普通依赖求解或归档身份。只有产品实现、完整教程、示例、测试与证据均通过，才可升级
为 `verified`；不能仅因本页探测成功就宣称模块完成。

## 故障排查与下一步

三项产品草稿已有基础 VM 与真实导出黄金，MIN 的实现避免对 MIN 求正绝对值或直接对其正除模。
命名调用已修复，三平台功能回归及最大左右填充的默认VM测试已通过；完整F10与生成器20释放修复的当前记录见
[REM证据](../../DevDocs/20o-b1-rem-evidence.md)；历史边界见 [FIX-A记录](../../DevDocs/20o-b1-fix-a-contract-closeout.md)。

若无法找到完整示例，先确认位于仓库根；原生测试缺少 LLVM/Runtime 配置时应明确失败，
不能把环境跳过当成功。继续阅读 [整数计算](math.md)、
[官方库索引](README.md) 或 [B1 冻结草案](../../DevDocs/20o-b1-freeze-draft.md)。
