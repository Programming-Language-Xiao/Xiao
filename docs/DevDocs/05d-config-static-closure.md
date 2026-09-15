# 05D. `config.xiao` 声明式配置静态闭环

> 本文是 05-D 的实现与交接记录。该阶段已经完成：`xiao-config` 能把根
> `config.xiao` 转换为不可执行配置树，并在读取阶段拒绝可执行构造。依赖求解、
> 锁文件、多源索引、虚拟环境、语言目录和优化参数仍由后续阶段负责。

## Agent 交接上下文

### 接手前必须阅读

1. [00. 决策基线](00-decisions.md)：配置安全边界、国际化字段和低耦合约束。
2. [00A. 工程框架与目录布局](00a-project-layout.md)：Rust crate、目录 README 和登记门槛。
3. [05. 表、模块与工程模型](05-tables-and-projects.md)：05-A/B/C 的模块与表静态接口。
4. [05C. 表静态闭环](05c-table-static-closure.md)：源码 `[main]` 与配置文件的边界。
5. [11A. 虚拟环境与包管理](11a-environments-and-packages.md)：后续依赖消费者的输入契约。
6. [12. 测试与开发里程碑](12-tests-and-milestones.md)：阶段退出条件和测试分层。

### 输入与输出

输入是已经由 `xiao-source::SourceFile` 验证的 UTF-8 文本。解析器复用
`xiao-syntax::Lexer` 产生的 Token 和 `SourceSpan`，输出 `ConfigDocument`；校验通过后
输出同形的 `NormalizedConfig`。配置模型不包含 `Statement`、表达式、函数或 Runtime 值。

### 明确不负责

- 不调用普通 Xiao `Parser`，不解析 `.xiao` 源文件中的 `[main]`。
- 不执行函数、控制流、导入、常量、环境变量或任何用户代码。
- 不扫描模块、不下载/安装依赖、不生成锁文件、不选择包源、不激活虚拟环境。
- 不解释 `[CLI]`、`[debug]`、`[VM]`、`[language]` 等扩展表的后续专属行为。

## 一级工程目标：建立独立配置模型

### D1.1 模型边界

`xiao-config/src/model.rs` 提供以下稳定类型：

- `ConfigDocument`：按规范化表名保存 `BTreeMap`，并保留文档 `SourceSpan`。
- `ConfigTable`：表名、按键保存的 `ConfigEntry` 和表区间。
- `ConfigEntry`：键名、`ConfigValue` 和条目区间。
- `ConfigValue`：`String`、`Integer(i128)`、`Float(f64)`、`Boolean`、递归 `Array` 和
  `Dictionary`。
- `ConfigValueKind`：供诊断和后续消费者使用的稳定种类名称。

`BTreeMap` 是确定性输出要求的一部分；后续包管理器不得依赖哈希遍历顺序。

### D1.2 Token 解析

`src/parser.rs` 只接受单层 `[name]` 表头和 `key = value` 成员。允许字符串、数值、
布尔值、数组和字典表递归嵌套，数组允许尾逗号，字典键值必须使用 `=`。顶层普通赋值、
双层 `[[name]]`、点号表头和缺失分隔符都产生 `X05-CONFIG-*` 诊断。

## 一级工程目标：校验与规范化

### D2.1 表和字段白名单

`src/validation.rs` 登记保留顶层表。`project` 存在时必须包含非空字符串 `name` 和
`version`；`exports` 的每个键映射到一个安全的项目根相对 `.xiao` 路径；`language`
只接受首版 `locale` 字段。CLI、Debug、VM、Runtime、依赖、源和构建相关表只保留静态
节点，具体字段语义在后续阶段冻结。

严格字段表中的未知字段和未知顶层表直接报错。重复键、重复表头和跨表/运行时引用不被
覆盖或求值。需要强制项目身份的调用方使用 `parse_config_project` 或
`validate_project_config`；全局配置可使用通用 `parse_config`。

### D2.2 稳定诊断

配置诊断使用独立 `X05-CONFIG-001` 至 `X05-CONFIG-012` 编号，保留 `message_id`、
结构化参数和 `SourceSpan`。中文文本只是当前预览，不能作为 CLI 或国际化层的判断接口。

## 二级工程 SOP

### D3.1 测试

1. `core/rust/crates/xiao-config/src` 单元测试覆盖模型访问、字面量解码、数组/字典
   恢复和校验辅助。
2. `core/rust/crates/xiao-config/tests/d05_config.rs` 覆盖合法项目、递归静态值、重复
   结构、未知节点、路径穿越、缺少身份和可执行构造拒绝。
3. 后续新增字段必须先增加决策记录和正反规格，再扩展测试；不得用兼容性需求绕过执行边界。

### D3.2 文档与登记

1. 同步 `xiao-config/README.md`、`xiao-config/src/README.md` 和 `xiao-config/tests/README.md`。
2. 同步 `docs/UseDocs/tooling/config/` 的索引、语法和错误页面；UseDocs 不得平铺到 DevDocs。
3. 更新 `docs/module-registry.json` 的代码、测试、UseDocs 路径和验证状态。
4. 更新 [05. 阶段总文档](05-tables-and-projects.md)、[README 索引](README.md) 和
   [测试里程碑](12-tests-and-milestones.md)。

### D3.3 质量门禁

```text
cargo fmt --all -- --check
cargo check --workspace
cargo test -p xiao-config
cargo clippy -p xiao-config --all-targets -- -D warnings
bun tools/repo-check/src/cli.ts all
```

公共 Rust API 文档覆盖率必须为 100%，仓库声明项覆盖率不得低于 90%。代码、测试、
UseDocs、README 和登记必须在同一提交中完成。

## 后续交接

11/11A 接手时直接消费 `ConfigDocument`/`NormalizedConfig`，不得重新扫描文本或在
TypeScript 中复制字面量语义。11C 只在已解析的 `[language].locale` 节点上实现语言
规范化和回退。若要增加依赖字段、优化字段或可执行构造，先更新 `00-decisions.md` 和
本记录，再进行独立阶段设计。
