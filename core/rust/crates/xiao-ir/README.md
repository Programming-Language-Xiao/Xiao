# `xiao-ir`

## 目录职责

定义类型化 Xiao IR：值类别、控制流、容器选择、效果、别名、所有权、释放计划、错误边界、模块身份和源码映射。

## 工程期

08 建立统一 IR；13 为共享优化器提供稳定输入；09/10 分别降低到字节码和 LLVM。

## 模块放置

值/类型节点、控制流、效果、所有权、释放计划、源码映射和验证器放在 `src/`；不要把 CLI 请求对象混入 IR。

## 稳定性要求

IR 节点必须可验证、可快照和可版本化；后端不能重新解析源码或重新推断语言语义。

## 08A/U0 交付

`src/model.rs` 保存版本化递归 IR，`src/lower.rs` 只从 AST、类型结果和生命周期结果
单向降低，`src/validate.rs` 在后端前检查结构不变量，`src/snapshot.rs` 提供版本化 JSON
往返。定向规格位于 `tests/u0_ir.rs`，对应 UseDocs 为
`docs/UseDocs/language/compiler/ir/README.md`。

## 09R2 交付

`src/validate.rs` 新增并列入口 `reconcile_release_plans` 与纯数据观测类型
`ObservedRelease`：后端记录实际发出的释放序列，IR 侧比对它与冻结计划是否逐条
相等，不一致报 `X08-IR-003`。这是后端「没有重排、去重或漏放」的唯一校验手段，
因为释放计划本身是每个作用域乘以全部退出边的无条件笛卡尔积，覆盖检查没有区分度。
该入口吃自己定义的数据结构，不引入对后端的反向依赖。定向规格位于
`tests/r2_release_reconciliation.rs`。

## 边界

本 crate 不执行 Xiao 代码、不创建 Runtime 对象、不解析 CLI 参数，也不实现字节码、LLVM
或优化 Pass。新增节点必须同时更新降低器、验证器、快照测试和 UseDocs，禁止把语义实现
复制到后端。
