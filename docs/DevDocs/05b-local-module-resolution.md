# 05B. 本地模块发现与依赖图实现交接记录

> 本文是 05 阶段的本地静态解析子阶段交接。它消费 05-A 的公开 AST，在项目根内发现
> `.xiao` 文件并建立模块/命名空间接口；不读取 `config.xiao`，不执行 Runtime 或下载包。

## Agent 交接上下文

### 接手前必须阅读

1. [00. 决策基线](00-decisions.md)：模块、国际化和低耦合边界。
2. [05. 表、模块与工程模型](05-tables-and-projects.md)：本阶段总设计和后续债项。
3. [05A. 绝对导入语法](05a-import-syntax.md)：导入 AST 字段与诊断。
4. [04. 函数与控制流](04-functions-and-control.md)：递归语句体和作用域规则。
5. [12. 测试与开发里程碑](12-tests-and-milestones.md)：模块测试退出条件。

### 已完成子任务

| 编号 | 交付 | 状态 |
| --- | --- | --- |
| 05B.1 | 文件发现和逻辑模块名映射 | 已完成 |
| 05B.2 | 纯目录命名空间、边界和冲突诊断 | 已完成 |
| 05B.3 | 直接/选择导入目标、作用域绑定和限定访问 | 已完成 |
| 05B.4 | 顶层再导出、依赖聚合、循环和初始化顺序 | 已完成 |
| 05B.5 | 集成测试、规格快照、README 和分层 UseDocs | 已完成 |

## 一级工程目标：公开项目分析结果

### 入口与模型

`xiao_modules::analyze_project(path)` 接收项目目录并返回 `ProjectModuleResult`。结果包含：

- `modules`：逻辑 `ModuleName` 到 `ModuleRecord` 的确定性映射；文件记录含源码、AST 和
  当前符号接口。
- `namespaces`：纯目录 `NamespaceRecord`；没有初始化代码。
- `graph`：聚合的 `Import`/`QualifiedUse` 边和依赖优先初始化序列。
- `bindings`：模块、作用域、局部名称和 `BindingKind` 的导入绑定记录。
- `diagnostics`：带模块/路径上下文的统一诊断。

模块名由项目根相对路径和文件 stem 组成；根 `config.xiao` 被排除，嵌套 `config.xiao` 停止
扫描其子树。点目录排除，符号链接不跟随，`target`/`node_modules` 不特殊排除。

### 名称冲突

文件/目录段必须是 ASCII 标识符且不能是 Xiao 保留字。大小写比较对导入保持精确，对文件
和命名空间冲突采用 ASCII 折叠；冲突使用 `X05-MODULE-003`，不选择任一胜者。

## 一级工程目标：解析导入和接口

### 目标与绑定

`import a.b` 无别名绑定根名称 `a`；`import a.b as c` 绑定完整目标 `c`。多个共享根命名
空间的无别名导入可合并。`from file import value` 绑定可赋值值符号；`from namespace import
child` 绑定子模块/命名空间限定符。导入绑定只存在于出现它的词法作用域，块内绑定不泄漏。

限定符不是一等值，单独读取或赋值使用 `X05-MODULE-008`；限定到文件成员时登记
`QualifiedUse` 边，成员之后的对象属性交给后续类型层。

### 顶层再导出

顶层选择导入加入当前文件的符号接口，并以 `ExportOrigin::Reexport` 保留原始模块/名称；
函数、分支和循环内导入不加入接口。接口传播按依赖优先顺序进行，缺失文件符号使用
`X05-MODULE-005`。

## 一级工程目标：依赖和错误

### 初始化顺序

直接导入边只对具体文件模块产生初始化依赖；目录命名空间不初始化。DFS 在依赖返回后写入
模块，因此结果天然是依赖优先且按 `ModuleName` 稳定排序。检测到循环时报告
`X05-MODULE-007` 并清空整个初始化序列，不能把部分序列交给后端。

### 稳定诊断

| 编号 | `message_id` | 触发条件 |
| --- | --- | --- |
| `X05-MODULE-001` | `x05.module.*` | 项目/目录/源码读取失败 |
| `X05-MODULE-002` | `x05.module.*` | 路径不能映射为模块名 |
| `X05-MODULE-003` | `x05.module.*` | 文件/命名空间或大小写冲突 |
| `X05-MODULE-004` | `x05.module.missing_import_target` | 模块或命名空间子模块不存在 |
| `X05-MODULE-005` | `x05.module.missing_import_symbol` | 文件中没有选择符号 |
| `X05-MODULE-006` | `x05.module.import_binding_conflict` | 作用域导入名称冲突 |
| `X05-MODULE-007` | `x05.module.import_cycle` | 文件模块循环依赖 |
| `X05-MODULE-008` | `x05.module.invalid_qualifier_use` | 限定符单独读取/赋值 |

诊断参数独立于展示文本，模块分析错误时仍尽可能返回部分结果；调用方必须在
`is_success() == false` 时停止 Runtime/IR 生成。

命名空间限定访问（例如 `app.missing.value`）在找不到下一段子模块时直接报告
`X05-MODULE-004`；具体文件模块后的名称则按模块导出接口检查并报告
`X05-MODULE-005`。文件/命名空间冲突的错误结果可以保留确定性的部分记录以便继续收集诊断，
但这类结果不是可消费的胜者，调用方必须先检查 `is_success()`。

## 二级实现任务（已执行 SOP）

### 05B.1 发现器与解析器边界

1. `discovery.rs` 只负责文件系统遍历、源码读取、语法解析和本地顶层符号收集。
2. `resolver.rs` 只消费公开 AST/模型，负责目标解析、绑定、接口和图；不打开文件或配置。
3. `model.rs` 保存跨模块公开契约；`lib.rs` 只装配/重导出。任何新增跨层字段先更新决策和
   模块登记，禁止循环依赖。

### 05B.2 测试和文档同步

`core/rust/crates/xiao-modules/tests/d0_modules.rs` 覆盖布局、边界、冲突、绑定、再导出、
限定边、缺失错误和循环；规格快照位于 `tests/spec/06-modules`。面向使用者的入口为[模块与工程](../UseDocs/language/modules/README.md)，其下分层页面记录导入、布局、作用域和错误。

## 后续接棒

05-C 在不破坏上述模型的前提下增加 `[Table]`/`[[Table]]`、字段可见性和生命周期；05-D
由 `xiao-config` 解析根配置，再由 11A 接入外部包、锁文件和多源索引。Runtime 的延迟初始化、
模块失败缓存和跨模块类型检查不得回写为本阶段已实现能力。
