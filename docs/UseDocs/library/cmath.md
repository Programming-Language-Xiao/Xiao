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

本页为planned教程提纲，当前没有stdlib/cmath.xiao或对应数学intrinsic，不能按已交付产品调用。
星崽已确认首批15个float入口，floor/ceil/round/trunc也返回float，sfloat与高精度超越函数后置。
F-A已确认定义域、舍入、特殊值和误差；F-5进一步确认参数名、规划原语及C ABI契约。版本矩阵、完整错误表和依赖/性能仍待后续裁定，见[B2冻结草案](../../DevDocs/20o-b2-freeze-draft.md)。
这些是已确认契约，产品与三平台证据尚未交付，本页保持planned。

## 导入与准备

计划公共路径std.cmath，固定身份stdlib.cmath，标准库接口版本1。
产品将随工具链提供，不作为普通包安装。B2-B计划使用Runtime ABI1.19、生成器21，当前仓库仍为1.18/20。
当前只登记文档，不把未来导入命令写成已经可以执行。

## 五分钟运行与完整示例

实现阶段将在tests/fixtures/docs/cmath/保存完整、真实选择导入的产品示例和独立黄金；
示例将覆盖正常值、域错误、负零、每个公开入口、位置/命名调用及已冻结限制。
原生产品必须分别经过O0/O3，与VM核对稳定stdout、退出码和错误身份；普通超越值按冻结容差独立核对。
完整错误表与产品尚未交付，本页暂无可进入CI执行的完整产品示例。

## 核心概念与所有权

cmath是固定64位二进制浮点库，不是复数库，也不是lfloat高精度十进制库。
已确认三角函数使用弧度，round采用最近偶数；负零与逐函数定义域按冻结精表执行，产品尚未实现。
输入/结果为标量，纯计算且无新增宿主能力；实现须保证失败不产生部分结果。

## 首批范围

| 已确认入口组 | 输入/结果 | 规则与待办 |
| --- | --- | --- |
| sqrt、floor、ceil、round、trunc、abs | float到float | 舍入/零/域及value参数名已确认；产品未实现 |
| min、max、pow | 两个float到float | 特殊值、left/right或base/exponent参数名已确认；产品未实现 |
| sin、cos、tan、exp、ln、log10 | float到float | 弧度/下溢/域/误差、value参数及原语契约已确认；产品未实现 |

底层拟使用Rust libm，精确依赖版本仍待F-7；F-5已确认cmath规划ID34～48和xiao_runtime_cmath_*符号，活动表与实际ABI尚未接线。

## 错误与恢复

已确认NaN/Inf不作为正常输入或输出；sqrt负输入、ln/log10非正输入等域规则已确认，完整错误编号与优先级仍待F-B。
完整示例将通过catch与error_code读取实际code；不使用只打印恢复标签的方式代替错误身份验证。

## 资源、性能与安全边界

资源和性能是待实测验收目标，不是当前产品成绩，也不是任意硬件时延保证。
已确认sqrt正确舍入0 ULP，取整/abs/min/max逐位，普通超越与pow非零结果≤4 ULP；零、符号、定义域和错误精确核对。
B1的721条F10不能作为B2当前性能。B2-C必须在新Runtime/ABI快照完整重跑该批数据，并另测B2新增数值路径。

## 示例如何进入 CI

产品实现后，完整示例与执行fixture必须逐字一致，进入默认VM与Windows/Linux/macOS受控原生门禁。
实际执行点、每个入口的移除/错误/资源及版本拒绝证据将在B2实现/收口文档登记。
当前没有对应产品示例执行证据，计划中的测试路径尚不登记为已完成测试。

## 兼容与废弃

B2-A保持现有文本标签/ABI边界；显式精度函数lfloat_divide与cmath一同纳入B2-B新增原语及ABI升级。
函数、签名、错误或预算的改变须回到冻结，不通过隐藏helper或新私有导出规则绕过公开面约束。
B1七库仍按其历史快照verified；19/11/22与历史22C/22D债务不在本批关闭。

## 故障排查与下一步

当前找不到std.cmath是产品尚未实现，不能当作已经验收的环境跳过。
先查看[B2规划](../../DevDocs/20o-b2-numeric-foundation-plan.md)与[冻结草案](../../DevDocs/20o-b2-freeze-draft.md)，
需要已交付整数运算时使用[math](math.md)，返回[官方库索引](README.md)。
