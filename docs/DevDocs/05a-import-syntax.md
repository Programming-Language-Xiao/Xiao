# 05A. 绝对导入语法实现交接记录

> 本文是 05 阶段的语法子阶段交接。它只扩展 `xiao-syntax` 的 AST 和可恢复解析器，
> 不访问文件系统，不建立模块图，不执行导入。

## Agent 交接上下文

### 接手前必须阅读

1. [00. 决策基线](00-decisions.md)：源码位置、国际化字段和低耦合约束。
2. [01F. P2-A 语法模块解耦](01f-p2a-syntax-decoupling.md)：解析器模块装配方式。
3. [04. 函数与控制流](04-functions-and-control.md)：递归代码块和文档注释挂接。
4. [05. 表、模块与工程模型](05-tables-and-projects.md)：05 阶段总边界。
5. [12. 测试与开发里程碑](12-tests-and-milestones.md)：语法测试和 UseDocs 门槛。

### 已完成子任务

| 编号 | 交付 | 状态 |
| --- | --- | --- |
| 05A.1 | `ImportPath`、`ModuleImport`、`SelectedImport`、`ImportStatement` | 已完成 |
| 05A.2 | `Statement::Import`、遍历辅助和门面重导出 | 已完成 |
| 05A.3 | `import`/`from` 多项、别名、反引号选择名称解析 | 已完成 |
| 05A.4 | 相对/通配/尾逗号/缺失路径错误恢复 | 已完成 |
| 05A.5 | 定向测试、规格快照、README 和 UseDocs | 已完成 |

## 一级工程目标：稳定导入 AST

### 数据模型

- `ImportPath` 保存点号路径段和完整 `SourceSpan`。
- `ModuleImport` 保存路径、可选别名和导入项区间。
- `SelectedImport` 保存选择名称、可选别名和导入项区间。
- `ImportStatement::Modules` 表示一个或多个完整模块导入；`From` 表示一个目标路径和
  一个或多个选择项。
- `Statement::Import` 与其他语句一样保存文档注释和语句区间；导入项内部不拆成可执行
  表达式节点，`NodeIndex` 只登记语句节点。

### 语法冻结

```text
import path[.segment] [, path[.segment] ...]
import path[.segment] as name [, ...]
from path[.segment] import name [as name] [, ...]
```

路径段只能是普通 ASCII `Identifier`；选择名称和别名接受 `Identifier` 或
`BacktickIdentifier`。导入是绝对路径；不支持前导点、通配符、动态字符串、括号续行、
跨逻辑行和尾逗号。语句可在顶层、函数、分支和循环体出现。

## 二级实现任务（已执行 SOP）

### 05A.1 解析器接线

1. 在 `parser.rs` 只增加窄分派，把具体逻辑放入 `src/parser/imports.rs`。
2. 解析完成后要求当前 Token 是换行、反缩进或 EOF；多余内容进入稳定诊断并同步到边界。
3. 不在解析器中查询模块文件、配置或类型信息。

### 05A.2 错误与恢复

| 编号 | `message_id` | 触发条件 |
| --- | --- | --- |
| `X05-PARSE-001` | `x05.parse.invalid_import_path` | 路径段缺失或不是 ASCII 名称 |
| `X05-PARSE-002` | `x05.parse.invalid_import_target` | 目标/选择项/列表结构缺失 |
| `X05-PARSE-003` | `x05.parse.invalid_import_alias` | `as` 后缺少合法名称 |
| `X05-PARSE-004` | `x05.parse.unsupported_import_form` | 相对或通配形式 |

诊断保留稳定 `code`、`message_id`、参数和源码区间；中文文本只是当前预览，不能被测试或
后续模块层反解析。

### 05A.3 验证与交接

`core/rust/crates/xiao-syntax/tests/d0_imports.rs` 覆盖三种导入形态、多目标/多名称、
别名、反引号名称、嵌套块、文档注释、节点索引和非法形式；`tests/spec/06-modules` 保存
语言无关正反快照。UseDocs 入口为[导入本地模块](../UseDocs/language/modules/imports.md)。

## 非负责事项与后续接口

模块发现、名称绑定、顶层再导出、循环检测由 [05B. 本地模块解析](05b-local-module-resolution.md)
负责；跨模块类型统一由 08 阶段负责。任何新增导入形式必须先更新 `00-decisions.md`、本页、
AST/诊断测试和 UseDocs，不得在解析器里以字符串或文件扫描偷偷扩展语义。
