# `tools/repo-check`

## 目录职责

存放 A0 的仓库完整性检查器。工具读取 workspace manifest、仓库政策清单、项目维护源码、模块登记和 Markdown 文档，报告目录、README、单文件行数、UseDocs 链接及 workspace 漂移；不实现 Xiao 语言语义，也不自动修改用户文件。

## 工程期

A0 建立最小检查命令；A0.2 接入目录与 workspace 交叉核对；A0.4 接入 CI、JSON/SARIF 报告和 UseDocs 同步门禁；00E 增加 2500 物理行门禁和结构化大纲。后续 11、18、19 只扩展稳定报告接口，不把编译器逻辑复制到这里。

## 依赖边界

- 可以读取 `core/rust/Cargo.toml`、仓库根 `package.json`、`docs/module-registry.json` 和 `docs/UseDocs/`。
- 可以调用 `cargo metadata`，并仅在源文件超标时按需调用文档覆盖率工具的 Rust/TypeScript 大纲适配器。
- 不得依赖 Rust crate 的内部内存布局、解析 Xiao 源码或根据本地化文本判断结果。

`check:layout` 和 `check` 在超标 Rust 文件存在时依赖 Rust 工具链，也可通过
`XIAO_RUST_DOC_ADAPTER` 使用预编译适配器。当前 `xiao-bytecode` 研究编码器和
`xiao-syntax/parser.rs` 均已完成拆分，两个命令应保持全绿；不得为已经可拆分的文件增加
旁置豁免说明。

## 交付规则

实现、测试和 [UseDocs 工具说明](../../docs/UseDocs/tooling/cli/README.md) 必须在同一可审计变更集中完成。所有公共导出项使用 JSDoc；本目录自身也纳入全仓库 90% 文档覆盖率统计。

## 子目录

- [`src/`](src/README.md)：检查器实现、规则适配器和报告模型。
- `fixtures/`（实现时建立）：故意缺失 README、断链、workspace 漂移等正/负样本；样本不是生产代码。
