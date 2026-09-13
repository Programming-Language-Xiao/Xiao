# 02A. P2-B/S0 静态标量类型实现交接记录

> 本记录描述 P2 在 P2-A 语法解耦之后的首个可交付闭环：标量声明、名称环境、HM
> 基础算法、转换矩阵、数值边界和只检查不执行的 AST 类型检查器。容器、函数、
> 控制流、`@f`/`free` 与 Runtime 均不在本交接范围。

## Agent 交接上下文

### 接手前必须阅读

1. [00. 决策基线](00-decisions.md)：类型、位宽、转换和布尔加减的冻结规则。
2. [00A. 工程框架与目录布局](00a-project-layout.md)：依赖方向、目录 README 和文档门槛。
3. [01F. P2-A 语法模块解耦](01f-p2a-syntax-decoupling.md)：语法门面与 AST 兼容契约。
4. [02. 类型与值系统](02-type-system.md)：用户可见规则和后续容器边界。
5. [12. 测试与开发里程碑](12-tests-and-milestones.md)：S0 退出条件。
6. [00B. UseDocs 同步政策](00b-usedocs-policy.md)：代码、测试和使用文档必须同批交付。

### 当前状态

| 子任务 | 状态 | 交付位置 |
| --- | --- | --- |
| P2-B.1 标量声明 AST | 已完成 | `xiao-syntax/src/ast.rs`、`parser.rs` |
| P2-B.2 类型表示和 HM 变量 | 已完成 | `xiao-types/src/types.rs` |
| P2-B.3 统一、occurs-check、泛化、实例化 | 已完成 | `xiao-types/src/unify.rs` |
| P2-B.4 作用域、初始化和常量绑定 | 已完成 | `xiao-types/src/environment.rs` |
| P2-B.5 转换矩阵和数值规则 | 已完成 | `conversion.rs`、`numeric.rs` |
| P2-B.6 AST 静态检查器与运行时检查标记 | 已完成首批 | `xiao-types/src/checker.rs` |
| P2-B.7 S0 规格、UseDocs 和模块登记 | 已完成首批 | `xiao-types/tests/s0_types.rs`、`tests/spec/04-types`、`docs/UseDocs/language/basics/types` |

## 冻结的语法与语义

### 声明语法

```text
typed_declaration := scalar_type name ["=" expression]
const_declaration := "const" [scalar_type] name "=" expression
```

`str name` 可以无初始化声明；在赋值前读取由类型检查器报告
`X02-TYPE-003`。`const` 必须带初始化表达式。`int(value)` 等标量构造调用仍是
表达式，不会被解析成声明。容器类型前缀和路径约束在 C0 再加入。

### 类型和赋值

- `a = value` 首次出现时建立普通单态类型槽，后续赋值不能换成不兼容类型。
- 显式标量声明锁定 `sint`、`int`、`lint`、`sfloat`、`float`、`lfloat`、`str` 或 `bool`。
- 常量字面量若确实落在较窄数值槽内，可以初始化该槽；变量表达式的窄化必须显式转换。
- `const` 方案可泛化；普通可变绑定保持单态，避免值变化破坏推断。
- `lint` 的十进制字面量不受 `i128` 解析上限影响；超大数的完整算术由后续高精度运行库接管。

### HM API

`TypeContext` 分配 `TypeVarId`，持有 `Substitution`，并提供 `unify`、`occurs_check`、
`generalize` 和 `instantiate`。统一器支持标量、函数、元组和数组类型；动态类型
与任意类型统一时保留另一侧，供后续动态边界使用。统一失败不会直接生成诊断，由检查器
映射为 `X02-TYPE-009`。

### 数值和转换

- 整数提升为 `sint < int < lint`，浮点提升为 `sfloat < float < lfloat`；整数与浮点混合时进入浮点族。
- `/` 返回提升后的浮点族，`//` 和 `%` 仅接受整数并返回整数族。
- `bool +/- integer` 只接受布尔在左、整数在右；偶数保持、奇数翻转，结果仍为 `bool`。
- `value as bool` 与 `bool(value)` 共享矩阵；`str` 只接受 `true`、`True`、`false`、`False`，
  `bool as str` 产生小写字符串。
- 显式浮点到整数转换按向零截断；静态常量先检查截断后的目标范围，动态值由后续
  Runtime 执行范围检查。隐式声明不会把浮点到整数转换当作可用的截断。
- 可静态证明的固定宽度溢出、浮点非有限值和除零在检查阶段报告；动态表达式输出
  `RuntimeCheck`，由后续 Runtime/后端插入检查。

## 实现边界和禁止事项

1. `xiao-types` 只能依赖 `xiao-syntax` 的公开 AST、`xiao-source` 和统一诊断接口。
2. 检查器不得执行函数、输入、随机选择或容器访问；`input`/`print` 只作为预留内建签名。
3. 类型化结果通过 `TypedNode` 旁路表返回，不向 AST 节点写入类型字段。
4. 不在本阶段实现函数语法、控制流、容器、生命周期或 LLVM/VM 指令。
5. 新容器和动态规则必须提取独立模块，禁止继续扩大 `checker.rs` 的跨层职责。

## 二级 SOP

### S0-1：语法入口

1. 先由 `Parser` 生成 `Declaration`/`ConstDeclaration`。
2. 检查 `SourceSpan`、文档注释、EOF 和错误恢复。
3. 更新 `tests/spec/04-types` 和 `xiao-syntax` UseDocs。

### S0-2：类型核心

1. 用 `Type`、`TypeVarId` 和 `TypeScheme` 表示类型，不复制宿主布局。
2. 用 `Substitution` 实现统一和 occurs-check；为函数/元组预留结构分支。
3. 用 `TypeEnvironment` 管理作用域、初始化、可变和常量标志。

### S0-3：检查和诊断

1. 按源码顺序累积诊断，不因首个错误停止后续名称解析。
2. 对声明、赋值、复合赋值、标量运算和转换调用执行静态检查。
3. 将不可静态证明的范围/内容检查记录为 `RuntimeCheck`，不把它伪装成成功求值。

### S0-4：验证与交接

```text
cargo fmt --manifest-path core/rust/Cargo.toml --all -- --check
cargo test --manifest-path core/rust/Cargo.toml -p xiao-syntax -p xiao-types
cargo clippy --manifest-path core/rust/Cargo.toml -p xiao-types --all-targets -- -D warnings
cargo doc --manifest-path core/rust/Cargo.toml -p xiao-types --no-deps
bun run check
```

接手 C0 时，先阅读 [03. 容器、集合与索引路径](03-collections.md)，并把容器类型放入
独立模块；不要把数组路径和集合可哈希规则直接塞进当前标量匹配分支。
