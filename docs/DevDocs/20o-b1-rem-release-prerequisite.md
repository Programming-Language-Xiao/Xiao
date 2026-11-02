# 20O-REM 释放差异的复现与独立前置草案

状态：星崽已直接授权独立前置修复并将生成器19→20（2026-11-02）；本机修复与六组同源回归通过，三平台待取证。
依据 [REM §三/§4.2](20o-b1-imp-remainder.md#42-释放差异门槛)，本批不得修改
CODEGEN_VERSION；本页记录的独立前置是星崽明确授权的版本例外，不修改IR释放计划或22D规则/断言。

## 可复跑结果

显式工具为 core/rust/crates/xiao-driver/examples/b1_release_diagnostic.rs，使用已有受控
原生环境与相同 fixture。stdout、退出码和空 stderr 均匹配后，分别记录 VM/native
完整 trace、IR、LLVM 与结果 JSON；出现差异返回 1，不计作通过，也不新增 ignored 目标。

本机 Windows O0/O3 都复现：成功段 native 12 / VM 13，含越界段 native 14 / VM 16。
成功段只有对象 13（末次循环字符）缺 destroy；含越界段另有对象 1（输入文本）缺 destroy。
完整日志、同源 fixture、VM/native trace 与归因摘录已登记在
[REM 失败证据目录](assets/20o-b1-imp/rem-release-local-20261102/README.md)。

```text
cargo run --manifest-path core/rust/Cargo.toml -p xiao-driver --example b1_release_diagnostic
```

## 归因

成功段的现有 IR 已有 loop scope=1、value=7（ascii:character）的 normal/break/continue/
error 等 strong 释放动作，输入 text 则属于 scope=0、value=1。生成的 LLVM 在
dynamic.for.end 直接跳到 continue，随后只执行根作用域 text 的归还，未消费循环 normal 计划。
因此差异不来自 CSV/URL 产品或新的字符索引数组；无需改变 IR 所有权语义来补齐该执行边。

越界段另外命中 emit_selector：先复制来源到拥有的 SSA 值，再调用会失败的索引 getter；
checked status 失败边在后面的 release_value 之前跳入 catch，使该临时来源强引用留存。
Runtime getter 内部借读/临时复制会正常结束，缺口在原生调用方的失败边。

## 已授权的独立修复

1. 独立前置只修改 xiao-codegen-llvm 的普通 for/selector 降低，消费已有循环释放计划，
   覆盖正常、continue、break、错误和嵌套退出；不改 22D 的排序规则、断言或 IR 计划。
2. 索引 getter 的失败边归还调用方临时来源；正常路径仍恰好归还一次，保留 pending 主错。
3. 因生成文本发生变化，CODEGEN_VERSION 19→20，ABI 继续 1.18，无新 intrinsic/语言语法。
4. 把相同成功/越界、嵌套/continue/break 向量加入既有差分目标，逐对象核对销毁数与释放顺序；
   复跑原有 22 相关门禁，保留所有原断言，不把既有发布债务纳入此修复。
5. 前置独立提交并取证后回到 REM-C；三平台证据齐备前仍不宣称生命周期整体验收或 B1-CLOSE 完成。

## 实现与本机验证

普通for按每轮正常、break、continue和错误边消费既有loop计划，并把临时来源移入已初始化的帧槽，
使循环内return也能归还来源。selector把来源、索引/step及中间路径的拥有引用放进已置空的槽，
成功和失败边各归还一次；较晚的实参失败不会读取未支配SSA，也不会改变挂起主错。

完整向量固定在 tests/fixtures/20o-b1/imp/release-edges/，默认VM、既有20C原生差分目标和显式
诊断共用。没有新增ignored目标。以下六项均在本机VM及原生O0/O3逐字核对stdout、退出码、
空stderr与逐对象销毁顺序，12条原生记录全部匹配：

| 向量 | VM / 原生O0 / 原生O3销毁数 |
| --- | --- |
| 原成功段 | 13 / 13 / 13 |
| 原含越界段 | 16 / 16 / 16 |
| 嵌套continue/break | 7 / 7 / 7 |
| 临时来源return及空循环 | 3 / 3 / 3 |
| 多级索引、getter与索引实参失败 | 6 / 6 / 6 |
| 循环体索引失败 | 3 / 3 / 3 |

失败基线继续保留；修复后的完整trace及机器清单见
[本机生成器20证据](assets/20o-b1-imp/rem-release-cg20-local-20261102/README.md)。
