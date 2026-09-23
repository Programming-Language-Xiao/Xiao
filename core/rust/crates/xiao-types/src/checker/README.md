# `checker/`

这里是 `xiao-types` 的静态检查器职责子模块。上层 `checker.rs` 仍是唯一的检查器门面，
负责保存 `TypeChecker` 状态、注册顶层声明、分派语句和组装 `TypeCheckResult`；公开入口
`xiao_types::check`、`xiao_types::TypeChecker`、`xiao_types::RuntimeCheckKind` 以及结果
类型的路径保持不变。

## 工程期

11X0-G（`checker.rs` 职责解耦）。本目录只调整实现组织和依赖方向，不新增类型规则、诊断码
或 Runtime 能力；拆分结果以父级交接文档的分步提交和验收记录为准。

## 模块职责

| 模块 | 职责 | 允许依赖 |
| --- | --- | --- |
| `result.rs` | `RuntimeCheckKind`、`RuntimeCheck`、`TypedNode`、`TypeCheckResult` 公开结果模型 | 类型、环境、函数签名、容器计划和诊断值；不依赖检查器实现 |
| `constant.rs` | 无状态常量转换、字符串解码、二元/一元常量折叠和运行时检查判定 | `numeric`、纯转换矩阵、`container_checker::decode_escape`；不持有 `TypeChecker` |
| `conversion.rs` | 显式转换、常量目标范围、调用者名称和带环境的编译期求值 | `constant.rs`、诊断上报、数值规则和纯转换矩阵 |
| `diagnostic.rs` | 名称键、诊断构造、运行时检查标记和统一错误上报 | 结果模型、环境/数值/统一错误值和 `TypeChecker` 状态 |
| `expression.rs` | 字面量、名称、单/二元运算、调用、转换和类型变量推导 | `constant.rs`、集合运算、容器/函数/表/选择器规则和 `TypeChecker` 状态 |
| `statement.rs` | 语句分派、声明、赋值、常量绑定和显式目标校验 | `constant.rs`、集合运算、纯转换/数值规则和 `TypeChecker` 状态 |

依赖方向固定为“结果/无状态规则 → 带状态规则 → 门面装配”。`expression.rs` 与
`statement.rs` 通过 `TypeChecker` 的内部方法协作，不直接互相导入；转换子模块与根级
`conversion.rs` 不同，前者负责检查器状态和常量目标，后者只提供纯转换矩阵。新增规则应
继续下沉到职责最小的模块，不能把实现重新塞回 `checker.rs`，也不能建立表达式、语句、
诊断之间的反向依赖。

## 稳定边界

- `RuntimeCheckKind` 仍从 `xiao_types::RuntimeCheckKind` 可达，`as_name`、`from_name` 和
  `all` 的行为是跨 crate 的单一来源，不能在后端复制名单。
- `TypeCheckResult` 的字段、节点顺序、诊断顺序和运行时检查顺序保持不变；本次只调整
  实现文件位置，不新增类型规则、诊断码或运行时能力。
- 检查器只消费 AST 和源码区间并生成静态结果，不依赖 Runtime、VM、LLVM 或 CLI，也不
  执行用户代码。

## 架构回归

`checker_architecture_tests.rs` 在 `cfg(test)` 下由门面接入，使用源码级断言锁定门面不再
承载表达式、语句、诊断和常量求值实现，并检查子模块不出现反向导入。`xiao-types/tests/`
仍是行为与快照测试的单一来源，本批不修改其中任何文件。
