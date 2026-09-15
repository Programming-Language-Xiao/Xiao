# 05. 表、模块与工程模型

> 本阶段把单文件脚本提升为可构建工程，并定义 TOML 风格表、源码模块和后续包边界。
> 当前已完成 05-A、05-B，以及 05-C 的表语法与静态生命周期契约闭环；表的运行时释放、
> `config.xiao` 配置读取和外部包仍是后续子阶段。

## Agent 交接上下文

### 接手前必须阅读

1. [00. 决策基线](00-decisions.md)：全局语法、国际化字段和禁止高度耦合约束。
2. [00A. 工程框架与目录布局](00a-project-layout.md)：crate、测试、README 和登记门槛。
3. [01E. P1 表达式与选择器](01e-p1-expression-selectors.md)：名称、成员表达式和源码区间契约。
4. [04. 函数与控制流](04-functions-and-control.md)：递归语句体、入口模式和作用域边界。
5. [12. 测试与开发里程碑](12-tests-and-milestones.md)：05 阶段退出条件与后置债项。

### 当前交付物

| 子阶段 | 状态 | 主要位置 |
| --- | --- | --- |
| 05-A 导入 AST 与解析 | 已完成 | `xiao-syntax/src/imports.rs`、`src/parser/imports.rs`、`tests/d0_imports.rs` |
| 05-B 本地发现与解析 | 已完成 | `xiao-modules/src/discovery.rs`、`resolver.rs`、`model.rs`、`tests/d0_modules.rs` |
| 05-C 表语法与生命周期静态闭环 | 已完成静态阶段 | `xiao-syntax`、`xiao-types`、`xiao-modules`；运行时生命周期留给 06 |
| 05-D `config.xiao` 与包边界 | 未开始 | `xiao-config` 及第 11A 包管理阶段 |

### 不负责事项

05-A/B 不执行导入或模块初始化，不创建 Runtime 模块值，不进行跨文件类型统一，不读取
`config.xiao`，不下载/安装外部依赖，不实现字节码、LLVM、CLI、Shell 激活或锁文件。
Runtime 只在执行到导入语句时初始化目标且每个具体模块至多一次；这一行为由第 06、09 阶段
消费本阶段的静态结果。

## 一级工程目标：表的两种语言身份

### 单例表 `[TableName]`

`[TableName]` 表示一个单例命名空间，可承载静态字段、函数和模块级状态，适合配置和工具
模块。表头和字段顶格，函数/控制流体使用缩进：

```xiao
[App]
    name = "Xiao"
    version = 1

    def start()
        print(name)
```

### 可实例化表 `[[TableName]]`

`[[TableName]]` 表示可由 `new` 创建多个实例的类型表：

```xiao
[[User]]
    name = ""

    def init(self, name)
        self.name = name

    def drop(self)
        close(self.name)
```

字段布局、方法接收者、构造参数映射和可见性在 05-C 冻结。花括号集合/字典表仍是运行时
值，不会仅凭外观生成源码表或类。

### 生命周期与入口（分层交付）

05-C 只验证 `init(self, ...)`、`drop(self)` 的静态签名和 `new` 的参数契约：表方法的首个
参数必须是普通名称 `self`，`drop` 只能接收 `self`，两个生命周期方法的返回类型必须为
`none`。类型检查器不会执行任何方法，也不会创建实例、插入释放点或开启引用计数。
实例分配、构造调用、作用域退出、错误展开时的清理和逃逸分析统一留给 06/R0；脚本模式和
`[main]` 工程模式继续沿用 04 阶段入口元数据。构造失败、清理失败和多个入口的精确传播
规则尚未实现，不能由本阶段代码猜测。

## 一级工程目标：05-C 表语法与静态生命周期闭环

### 表头、成员和可见性

`[Table]` 是单例表，`[[Table]]` 是可实例化表。表头必须位于文件顶层并顶格；成员体缩进
一级，方法体继续缩进。表体只允许字段赋值、字段声明、`const` 字段和 `def` 方法。字段与
方法统一进入 `TableSignature.members`；成员名以下划线开头时默认 `Private`，其他成员默认
`Public`。表不能作为数组/集合索引对象，本阶段也不把表作为集合的可哈希元素。

### 静态类型与访问

表名在类型环境中绑定为 `Type::Table`：单例表是 `Singleton`，可实例化表声明在 `new`
之前是 `Constructor`，`new` 的结果是 `Instance`。字段类型来自显式声明或初始化表达式；
字段初始化器必须是字面量、常量、纯一元/二元表达式或纯容器，`input`、`print`、普通函数
调用、`new`、动态成员链和选择器都会产生 `X05-TYPE-006`。`self.field`、`Table.field`
和实例字段访问复用同一成员解析；表外访问私有成员产生 `X05-TYPE-003`。

### 构造和生命周期契约

只有 `[[Table]]` 可以作为 `new` 目标；没有 `init` 时只能无参构造。存在 `init` 时，构造
参数按位置/关键字与 `init` 去掉隐式 `self` 后的参数匹配，数量或类型错误使用
`X05-TYPE-004`。`drop` 的参数和返回类型不满足契约时使用 `X05-TYPE-005`。本阶段只输出
签名和类型结果，不执行构造或析构。

### 稳定诊断

| 编号 | `message_id` 示例 | 触发条件 |
| --- | --- | --- |
| `X05-PARSE-005` | `x05.parse.invalid_table_name` | 表头名称、尾部或顶层位置非法 |
| `X05-PARSE-006` | `x05.parse.invalid_table_member` | 表体出现不在白名单内的语句 |
| `X05-PARSE-007` | `x05.parse.missing_table_indent` | 缺少表体、缩进或成员 |
| `X05-TYPE-001` | `x05.type.duplicate_table` | 表名重复或与当前作用域名称冲突 |
| `X05-TYPE-002` | `x05.type.unknown_table_member` | 成员不存在、重复或字段类型冲突 |
| `X05-TYPE-003` | `x05.type.private_table_member` | 表外访问下划线私有成员 |
| `X05-TYPE-004` | `x05.type.constructor_argument` | `new` 目标/参数不符合构造契约 |
| `X05-TYPE-005` | `x05.type.method_requires_self` | `init`/`drop` 或普通方法的首参契约错误 |
| `X05-TYPE-006` | `x05.type.dynamic_table_initializer` | 字段初始化器含动态或有副作用表达式 |

完整的交接上下文、SOP 和未负责事项见 [05C. 表静态闭环交接记录](05c-table-static-closure.md)。
面向使用者的分层页面见 [表与生命周期](../UseDocs/language/tables/README.md)。

## 一级工程目标：05-A 绝对导入语法

### 冻结语法

```xiao
import net.http
import net.http as http
import app.http, app.models as models
from app.models import User
from app.models import User as ModelUser, `显示名` as `用户显示名`
```

路径段只接受大小写精确匹配的 ASCII 标识符；选择名称和别名可以是普通名称或反引号名称。
一个文件中允许多个导入项，导入语句可以出现在顶层、函数、分支和循环体。

当前明确拒绝相对导入、通配导入、动态字符串导入、括号续行、跨逻辑行导入和尾逗号。没有
`as` 时 `import a.b` 默认绑定根名称 `a`；有别名时绑定完整目标名称。

### AST 与解析边界

`xiao-syntax` 的 `ImportPath`、`ModuleImport`、`SelectedImport` 和 `ImportStatement` 只
保存名称、别名和 `SourceSpan`。`Statement::Import` 挂接文档注释，并参与语句区间、节点
索引和错误恢复；解析器不访问文件系统。稳定解析诊断为 `X05-PARSE-001` 至
`X05-PARSE-004`，机器字段包括 `code`、`message_id` 和参数，中文只作预览。

详细交接见 [05A. 绝对导入语法](05a-import-syntax.md)，面向使用者的页面见
[导入本地模块](../UseDocs/language/modules/imports.md)。

## 一级工程目标：05-B 本地模块发现与解析

### 文件模块和目录命名空间

当前项目根直接作为源码根；普通 `.xiao` 文件映射为文件模块，目录自动形成纯命名空间，
无需 `__init__.py`：

```text
main.xiao             -> main
app/user.xiao         -> app.user
app/http/client.xiao  -> app.http.client
```

命名空间没有初始化代码。根 `config.xiao` 不作为模块；嵌套目录中的 `config.xiao` 标记
后续包边界并停止当前项目扫描该子树。点目录排除，`target`/`node_modules` 本阶段不特殊
排除，符号链接不跟随。

文件/目录段必须是 ASCII 标识符且不能是 Xiao 保留字。大小写精确匹配；文件模块与同名
命名空间、大小写折叠冲突都报告 `X05-MODULE-003`，不静默选胜者。

### 绑定、再导出和限定访问

导入绑定属于出现它的词法作用域。块内导入离开块后不成为模块属性；同一作用域重复别名
报告 `X05-MODULE-006`，共享根命名空间的无别名导入可以合并。模块/命名空间限定符不是
一等值，不能单独读取、赋值或放入容器；限定成员访问可登记具体文件的延迟边。

顶层 `from module import name` 会加入当前文件的导出接口，因此可以逐层再导出，并保留
原始符号来源。包外公开清单尚未实现，不能把本阶段的顶层接口当作已发布 API。

### 依赖图与初始化计划

源码导入边聚合为 `ModuleGraph`：直接导入是 `Import`，通过目录命名空间限定到具体文件
是 `QualifiedUse`。文件模块初始化序列按依赖优先排列；纯命名空间不初始化。循环
依赖立即报告 `X05-MODULE-007`，初始化序列为空，后端不得使用部分 DFS 结果。

缺失目标、缺失符号和限定符误用分别使用 `X05-MODULE-004`、`X05-MODULE-005`、
`X05-MODULE-008`。模块分析尽可能保留可解析结果，但 `is_success()` 为假时不得生成
Runtime/IR 产物。

详细交接见 [05B. 本地模块发现与依赖图](05b-local-module-resolution.md)，面向使用者的
页面见[项目文件布局](../UseDocs/language/modules/project-layout.md)和[作用域与顶层导出](../UseDocs/language/modules/scope-and-exports.md)。

## 一级工程目标：`config.xiao` 与外部包（后续）

### 配置职责

根 `config.xiao` 将集中声明项目身份、直接外部依赖、包外导出和约束，但不抄写源码文件
之间的实际导入边，也不要求用户手写传递依赖树。配置解析必须复用 Token/源码区间，拒绝
函数、调用、控制流和模块导入等可执行构造，输出供 CLI、模块加载器和第 11A 环境管理器
共同消费的声明树。

### 包源与锁定职责

外部包允许多个独立源（注册表、静态目录、本地目录和符合规范的 Git/GitHub 仓库）。源
选择遵循“显式源优先，其次配置顺序，仍不唯一则歧义错误”，禁止跨源按版本号择高。版本
求解、联邦源索引、锁文件、缓存和环境物化由 [11A. 虚拟环境与包管理](11a-environments-and-packages.md)
负责；05 阶段只提供稳定的包身份/公共接口接入点。

## 二级实现任务（SOP）

### 05-A.1 语法模型

1. 在 `xiao-syntax` 独立模块中定义路径、导入项、选择项和语句变体，全部保留 `SourceSpan`。
2. 在解析器扩展模块中实现多目标、多名称、别名、反引号选择名称和边界错误恢复。
3. 更新 AST 的遍历辅助、节点索引、类型检查显式空分支和门面重导出。

### 05-A.2 语法规格与文档

1. 用 `d0_imports.rs` 覆盖三种导入形态、嵌套体、文档注释和非法形式。
2. 在 `tests/spec/06-modules` 维护不依赖展示语言的正反快照。
3. 同步 `xiao-syntax` README、05A 交接文档和 `docs/UseDocs/language/modules` 索引；未测试
   的行为只能写成 planned/draft。

### 05-B.1 发现器

1. 从项目根递归发现 `.xiao`，按平台无关逻辑段建立文件模块和纯目录命名空间。
2. 实施根/嵌套 `config.xiao`、点目录、符号链接、非法段和大小写/同名冲突规则。
3. 读取 UTF-8 源码并调用公开语法解析器，保留可恢复诊断和本地顶层符号表。

### 05-B.2 解析器与图

1. 为所有作用域收集导入边，解析直接目标、选择目标和缺失错误。
2. 建立限定符/值绑定，限制块作用域泄漏，传播顶层再导出。
3. 计算确定性依赖优先序，循环时清空初始化计划并保留结构化诊断。
4. 通过 `d0_modules.rs` 覆盖发现、冲突、绑定、再导出、限定访问和循环；同步本目录 README、
   05B 交接文档、UseDocs 和模块登记。

### 05-C. 表静态闭环（已完成静态阶段）

1. 在 `xiao-syntax` 中增加 `TableKind` 和 `Statement::Table`，解析顶层 `[Name]`/`[[Name]]`、
   表体白名单、缩进恢复、文档注释和节点索引；解析器不执行表语义。
2. 在 `xiao-types` 独立的 `table_checker.rs` 中预登记表/成员签名，检查字段纯初始化、
   成员可见性、`new` 参数和 `init`/`drop` 契约；结果通过 `TableSignature` 暴露给后续前端。
3. 在 `xiao-modules` 中把表加入本地导出符号，并递归检查方法体词法作用域；模块层不依赖
   `xiao-types`，不执行构造和生命周期。
4. 通过 `xiao-syntax/tests/c05_tables.rs`、`xiao-types/tests/c05_tables.rs` 和
   `xiao-modules/tests/c05_tables.rs` 验证正反路径；同步 UseDocs、目录 README、模块登记和
   国际化字段。

### 05-D. 后续配置与包接口

1. 在不修改 05-A/B/C 公共模型的前提下，由 `xiao-config` 解析根 `config.xiao` 和 `[main]`
   初始化计划。
2. 再由 `xiao-package` 接入外部包身份、依赖锁定和多源索引。
3. 任何跨层新增字段先更新 `00-decisions.md`、模块登记和交接文档，禁止在 resolver 中偷偷
   读取配置或下载依赖。

## 验收标准

### 05-A/B/C 当前验收

- 绝对导入 AST、错误恢复、源码区间、文档注释和节点顺序通过语法测试。
- 不创建 `__init__.py` 即可发现目录命名空间，根/嵌套配置和点目录规则稳定。
- 导入目标、块作用域绑定、顶层再导出和限定访问结果可确定复现。
- 缺失、冲突、循环和限定符错误具有稳定 `code`、`message_id`、参数和源码/模块上下文。
- 循环图不暴露部分初始化序列；分析错误不会触发 Runtime 或外部副作用。
- 表头、成员白名单、字段静态类型、私有成员、构造参数和 `init`/`drop` 签名通过 05-C
  三个 crate 的规格测试；类型结果不执行生命周期。
- Rust/TypeScript 模块职责保持低耦合，所有新增代码目录含 README，UseDocs 与测试同一提交。

### 后续验收债项

- 表实例的运行时分配/释放、`config.xiao` 白名单、包外导出和外部依赖锁定尚未实现；这些
  内容不得标记为 Runtime verified，也不能阻塞 05-A/B/C 的静态消费者建设。
