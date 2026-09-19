# 09R2E. 研究编码器模块解耦交接记录

> 本记录描述 09R2 研究编码器从单文件实现拆为门面与职责模块的交付。此次变更只调整
> 文件组织和内部依赖方向，不新增语言语义，不把研究编码误升格为正式 `.xiaoc` 格式。

## Agent 交接上下文

### 接手前必须阅读

1. [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md)：研究编码器的
   字节字段、源码映射和 ABI 冻结依据。
2. [00E. 单文件行数门禁交接](00e-file-size-gate.md)：`A0-SIZE-001` 的拆分门槛与当前
   `parser.rs` 债务。
3. [00. 决策基线](00-decisions.md)：架构耦合硬约束和新增模块的 README/依赖/测试要求。
4. [`research/encode/README.md`](../../core/rust/crates/xiao-bytecode/src/research/encode/README.md)：
   当前模块职责、允许依赖和禁止依赖的唯一局部边界说明。

## 解耦后的结构

`research::encode` 仍是唯一公开门面，内部实现按下面的 DAG 组织：

```text
encode.rs（类型、错误、公开入口与装配）
├── codec.rs    （字节读写叶子）
├── tags.rs     （opcode/枚举标签叶子）
├── validate.rs （编码输入不变量，依赖 codec/tags）
├── encoder.rs  （依赖 codec/tags/validate）
└── decoder.rs  （依赖 codec/tags/validate，与 encoder 互不依赖）
```

`tests.rs` 只在 `cfg(test)` 下编译，允许访问测试需要的内部辅助接口，但生产模块不得
依赖它。`codec` 和 `tags` 是叶子；任何新增共享逻辑必须继续下沉到合适的下层，不能在
`encoder` 与 `decoder` 之间建立横向依赖。`module_dependency_direction_is_acyclic` 是
对应的源码级架构检查用例。

## 兼容契约

- `research::encode` 的公开路径、公开类型、错误变体和函数签名保持不变；调用方不需要
  改成访问 `encode/` 私有模块。
- 研究编码的魔数、布局版本、opcode 数值、字段顺序、LEB128/定宽 `u16` 规则、可选值
  标记、长度上限和源码映射增量格式保持不变。
- `encode`、`decode`、`decode_encoded`、`build_pc_map` 和 `validate_encoded` 的成功/拒绝
  边界保持不变；未知标签、版本不匹配、截断、尾部字节、越界引用和定宽溢出仍然拒绝。
- `EncodedProgram` 的函数/基本块物理目录、指令 pc 和 `pc -> span` 查询语义保持不变。
- 本批不实现正式 `.xiaoc` 文件容器，不改变 TAC、ABI、生命周期或类型推断语义。

## 验证与交付

本批必须执行：

1. `cargo fmt --manifest-path core/rust/Cargo.toml --all -- --check`。
2. `cargo test --manifest-path core/rust/Cargo.toml -p xiao-bytecode`。
3. `cargo clippy --manifest-path core/rust/Cargo.toml -p xiao-bytecode --all-targets -- -D warnings`。
4. `bun test`、`bun run check:docs`、`bun run check:usedocs` 和 `bun run check:coverage`。
5. `bun run check:layout` 与 `bun run check` 必须继续只报告已登记的
   `core/rust/crates/xiao-syntax/src/parser.rs` 尺寸债务；`encode.rs` 不得重新出现。

后续接手者若继续拆分研究模块，必须在同一提交更新局部 README、允许/禁止依赖、架构
回归测试和本记录的交付状态；不得用新增跨层依赖绕过这张 DAG。
