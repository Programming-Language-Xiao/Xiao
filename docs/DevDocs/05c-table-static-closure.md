# 05C. 表语法与静态生命周期闭环交接记录

> 本文是 05-C 的可交接实施记录。它描述已经交付的 AST、静态类型和模块作用域边界，
> 不把后续 Runtime 的实例分配、RAII、逃逸分析或引用计数写成当前能力。

## Agent 交接上下文

### 接手前必须阅读

1. [00. 决策基线](00-decisions.md)：强类型、国际化字段和禁止高度耦合的总约束。
2. [00A. 工程框架与目录布局](00a-project-layout.md)：crate、测试、README 和登记门槛。
3. [01E. P1 表达式与选择器](01e-p1-expression-selectors.md)：`Member`、`NewCall` 和源码区间。
4. [04. 函数与控制流](04-functions-and-control.md)：函数参数、返回统一和词法作用域。
5. [05. 表、模块与工程模型](05-tables-and-projects.md)：05-A/B 接口与本阶段总规格。
6. [12. 测试与开发里程碑](12-tests-and-milestones.md)：静态阶段与 Runtime 后置门槛。

### 本阶段输入

- `xiao-syntax::Program`，其中表头已解析为 `Statement::Table`。
- `xiao-source::SourceFile`，所有诊断和节点都保留 `SourceSpan`。
- 04 阶段的函数签名/返回检查辅助；不能反向依赖 Runtime、VM 或 CLI。

### 本阶段交付

| 交付物 | 位置 | 状态 |
| --- | --- | --- |
| 表头与表体 AST | `core/rust/crates/xiao-syntax/src/ast.rs`、`parser.rs` | 已完成 |
| 表值类型和成员签名 | `core/rust/crates/xiao-types/src/tables.rs`、`types.rs` | 已完成 |
| 表字段/方法/构造静态检查 | `core/rust/crates/xiao-types/src/table_checker.rs` | 已完成 |
| 本地模块表符号与方法作用域 | `core/rust/crates/xiao-modules/src/discovery.rs`、`resolver.rs` | 已完成 |
| 正反规格测试 | 三个 crate 的 `tests/c05_tables.rs` | 已完成 |
| 用户文档 | `docs/UseDocs/language/tables/` | 已完成 |

### 明确不负责

- 不创建表实例，不执行 `init` 或 `drop`，不插入作用域退出释放点。
- 不实现 RAII、逃逸分析、引用计数、`Weak`、错误展开清理或 Runtime 对象布局。
- 不实现继承、接口、泛型、`config.xiao`、包边界、跨文件类型统一和后端 lowering。
- 不在模块解析器中读取配置、下载依赖或执行用户代码。

## 一级工程目标：静态语义闭环

### 表身份

`[Name]` 产生 `TableType { kind: Singleton }`，`[[Name]]` 在构造前产生
`TableType { kind: Constructor }`，`new Name(...)` 的结果为 `TableType { kind: Instance }`。
表名目前限定为顶层、非保留 ASCII 标识符；成员名可使用普通或反引号名称。

### 成员和可见性

字段赋值、字段声明、`const` 字段和 `def` 方法进入同一个 `TableSignature.members`。成员
键沿用环境键（`ascii:name` 或 `backtick:name`）；以下划线开头的去前缀名称默认私有。
成员表达式由类型检查器统一解析，方法的外部函数类型会去掉隐式 `self`。

### 初始化和生命周期

字段初始化器只能由字面量、已知常量、纯一元/二元表达式和纯容器组成；标量转换调用
（例如 `bool("true")`）在参数纯净时允许。`input`、`print`、普通函数、`new`、成员链和
选择器都被标记为动态表初始化错误。方法首参必须是普通 `self`；`drop` 只能有该参数，
`init`/`drop` 返回类型必须为 `none`。这些是静态契约，不代表执行。

## 二级实现 SOP

### C1：语法和 AST

1. 在 `ast.rs` 定义 `TableKind` 和 `Statement::Table`，保留表头、成员体、文档注释和区间。
2. 在 `parser.rs` 识别顶层 `[Name]`/`[[Name]]`，消费一层成员缩进；方法体交给既有函数
   代码块解析器。
3. 表体只接收字段赋值/声明、`const` 和 `def`；嵌套表、额外缩进、空体和非法表名使用
   `X05-PARSE-005` 至 `X05-PARSE-007`。
4. 更新 `Statement` 辅助方法、`NodeIndex` 和门面重导出；解析器不得导入类型 crate。

### C2：类型和构造契约

1. `table_checker.rs` 先登记顶层表和成员占位，再检查字段表达式与方法体，避免源码顺序
   破坏前向成员访问。
2. 用 `TableSignature` 暴露字段/方法签名；普通方法沿用 04 阶段参数和返回统一逻辑。
3. `new` 只接受 `Constructor`；无 `init` 时参数必须为空，有 `init` 时按去掉 `self` 后的
   位置/关键字参数匹配。
4. 访问不存在成员、类型冲突、私有成员和生命周期签名错误分别保持 `X05-TYPE-002` 至
   `X05-TYPE-006`，每条诊断都要有稳定 `message_id` 和参数。

### C3：模块作用域

1. `discovery.rs` 将表登记为 `ModuleSymbolKind::Table`，使其可以参与顶层导出。
2. `resolver.rs` 递归进入表体和方法体，只做词法名称/导入检查，不复制类型规则。
3. 保持 `xiao-modules` 不依赖 `xiao-types`，表的跨文件类型检查留给 08 前端。

### C4：测试与文档

1. 运行 `cargo test --manifest-path core/rust/Cargo.toml -p xiao-syntax --test c05_tables`。
2. 运行 `cargo test --manifest-path core/rust/Cargo.toml -p xiao-types --test c05_tables`。
3. 运行 `cargo test --manifest-path core/rust/Cargo.toml -p xiao-modules --test c05_tables`。
4. 同一变更集中更新 `docs/UseDocs/language/tables`、crate README、`12-tests-and-milestones.md`
   和 `docs/module-registry.json`；UseDocs 不得平铺到 DevDocs。

### C5：交接检查

1. `cargo fmt --all -- --check`、`cargo check --workspace`、定向/全量测试和 Clippy 通过。
2. 检查公共 API 文档注释覆盖率 100%，全仓库声明项覆盖率至少 90%。
3. 确认本阶段没有 Runtime 副作用、没有配置读取、没有下载依赖，也没有把单文件检查器重新
   扩大为跨层中心。
4. 后续 06/R0 接手时，应先消费 `TableType`、`TableSignature` 和静态诊断，不重新解析表头
   或重复实现可见性/构造规则。

## 后续债项

实例布局、字段写入、`init`/`drop` 执行顺序、作用域退出释放、异常展开、逃逸提升和跨后端
一致性全部登记在 06、08、09、10 阶段；在这些阶段完成前，UseDocs 只能描述静态检查和
规格，不承诺可运行的表对象。
