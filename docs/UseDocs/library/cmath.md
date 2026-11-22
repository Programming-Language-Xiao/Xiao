---
id: library.cmath
title: 浮点数学（B2规划）
status: planned
audience: learner
module: stdlib.cmath
stage: 20O-B2-FREEZE
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b2-numeric-foundation-plan.md
  - ../../DevDocs/20o-b2-freeze-draft.md
---

# 浮点数学（B2规划）

## 适用范围与状态

当前内部 1.19/22 快照包含 15 项 Runtime/C ABI、intrinsic 34～48、源码包装、VM 与原生接线，以及双路径归档。教程和库登记仍为 planned，公共发布由 B2-C 验收。
星崽已确认首批15个float入口，floor/ceil/round/trunc也返回float，sfloat与高精度超越函数后置。
F-A已确认定义域、舍入、特殊值和误差；F-5进一步确认参数名、原语及C ABI契约。F-6版本与产物矩阵、F-12完整错误表与优先级也已确认；F-3/F-7也已确认接纳/工作量/性能协议与依赖策略；F-D已实际签字并激活联合冻结，见[B2冻结草案](../../DevDocs/20o-b2-freeze-draft.md)。当前内部实施进度见[B3-3A/B记录](../../DevDocs/20o-b2-b3-3a-b-progress.md)。
Windows、Linux 和 macOS 已有固定快照的内部功能记录。Windows COFF 导出完整性及最终树的跨平台公共产物验证由 B2-C 继续补齐。[B6-5 交接](../../DevDocs/assets/20o-b2-b/entry-20261120-b6-5/README.md)

## 导入与准备

内部工具链可通过 `std.cmath` 和 `stdlib.cmath` 导入。两条路径使用同一个标准库身份 `stdlib.cmath`，接口版本为 1。B5 的 ABI 1.19、生成器 21 矩阵保留为历史记录，当前生成器为 22。[双路径与归档](../../DevDocs/assets/20o-b2-b/entry-20261115-b4-4/README.md) · [B5 版本记录](../../DevDocs/20o-b2-b5-version-matrix-plan.md)

## 五分钟运行与完整示例

tests/fixtures/docs/numeric/cmath-library.xiao及其独立stdout黄金保存内部验证的真实选择导入语料；
正式数值参考向量仍在tests/fixtures/docs/cmath/；
示例将覆盖正常值、域错误、负零、每个公开入口、位置/命名调用及已冻结限制。
原生产品必须分别经过O0/O3，与VM核对稳定stdout、退出码和错误身份；普通超越值按冻结容差独立核对。
错误契约已确认，最终产品尚未交付；本页完整教程示例与三平台CI验收仍后置。

## 核心概念与所有权

cmath是固定64位二进制浮点库，不是复数库，也不是lfloat高精度十进制库。
已确认三角函数使用弧度，round采用最近偶数；负零与逐函数定义域经源码调用同一内部数学内核。
输入/结果为标量，纯计算且无新增宿主能力；实现须保证失败不产生部分结果。

## 首批范围

| 已确认入口组 | 输入/结果 | 规则与待办 |
| --- | --- | --- |
| sqrt、floor、ceil、round、trunc、abs | float到float | 内部语言接线已落地；最终验收后置 |
| min、max、pow | 两个float到float | 内部语言接线已落地；最终验收后置 |
| sin、cos、tan、exp、ln、log10 | float到float | 内部语言接线已落地；最终验收后置 |

底层使用已冻结的 Rust libm 0.2.16。内部快照登记 ID 34～48 和 15 个 `xiao_runtime_cmath_*` 符号，依赖闭包与构建策略见[冻结草案](../../DevDocs/20o-b2-freeze-draft.md)。

## 错误与恢复

F-12已确认：NaN/Inf输入或非有限结果、sqrt负输入、ln/log10非正输入等数学域错误使用ArithmeticError/X06-RUNTIME-009；动态实参类型不符使用TypeError/002，受控输出分配失败使用ResourceError/011。完整顺序和C ABI状态6/out保留规则见冻结草案§5.2；内部Runtime/C ABI错误路径已有本机测试，源码完整示例仍待后续验收。
完整示例将通过catch与error_code读取实际code；不使用只打印恢复标签的方式代替错误身份验证。

## 资源、性能与安全边界

B6-2 的资源与性能实测绑定其独立快照。B2-C 按 F-3 协议核对最终产品、工具链和负载。[B6 性能记录](../../DevDocs/20o-b2-b6-resource-performance-plan.md)
已确认sqrt正确舍入0 ULP，取整/abs/min/max逐位，普通超越与pow非零结果≤4 ULP；零、符号、定义域和错误精确核对。
B1的721条F10不能作为B2当前性能。B2-C必须在新Runtime/ABI快照完整重跑该批数据，并另测B2新增数值路径。

## 示例如何进入 CI

内部回归已执行 `cmath-library.xiao` 的黄金输出、两种导入别名、原生 O0/O3 和错误 009。B2-C 固定最终教程示例后，在三平台逐字核对该候选示例和黄金。[B7 内部回归](../../DevDocs/20o-b2-b7-closeout-plan.md)

## 兼容与废弃

B2-A保持现有文本标签/ABI边界；显式精度函数lfloat_divide与cmath一同纳入B2-B新增原语及ABI升级。
函数、签名、错误或预算的改变须回到冻结，不通过隐藏helper或新私有导出规则绕过公开面约束。
B1七库仍按其历史快照verified；19/11/22与历史22C/22D债务不在本批关闭。

## 故障排查与下一步

内部导入失败时，记录实际诊断、ABI、生成器版本及加载的标准库根。
先查看[B2规划](../../DevDocs/20o-b2-numeric-foundation-plan.md)与[冻结草案](../../DevDocs/20o-b2-freeze-draft.md)，
需要已交付整数运算时使用[math](math.md)，返回[官方库索引](README.md)。
