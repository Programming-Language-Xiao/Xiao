# 00C. 文档 lint 接线实现交接

> 本文保留接线前的实测缺口和方案比较，并记录已执行的方案 B。它现在是门禁变更的交接记录，
> 不是待决策的预研究。
>
> 当前结论：**20 个 Rust workspace 成员已经统一接入 `missing_docs`，代码文档和两套检查口径均为满覆盖。**

## Agent 交接上下文

### 接手前提

- 先读 [00A. 工程框架与目录布局](00a-project-layout.md)、
  [A0. 工作区与质量门禁实现方案](00a-a0-workspace-and-checkers.md)、
  [00B. UseDocs 同步政策](00b-usedocs-policy.md)。
- 本文与 A0 检查器直接相关：它记录 A0 文档覆盖率门禁之外的 Rust 编译器 lint 接线。

### 当前状态

方案 B 已于 2026-09-18 独立落地：20/20 个 crate 的 `Cargo.toml` 增加
`[lints] workspace = true`，并补齐接线前 rustc `missing_docs` 报出的 44 个公共字段缺口。
workspace 仍将 `missing_docs` 声明为 `warn`，标准 Clippy 门禁的 `-D warnings` 将其提升为失败；
直接运行普通 `cargo check` 时仍会显示警告，这是 Cargo 的预期分层。

复测结果：`cargo clippy --workspace --all-targets -- -D warnings`、
`cargo doc --workspace --no-deps`、`cargo fmt --all -- --check` 均通过；
`bun run check:coverage` 为总体 `3832/3832`、公共 API `2065/2065`，不再有
`A0-COVERAGE-002`。实现提交号会在提交后补入本节和文档索引。

### 本阶段交付与不负责

**交付**：缺口的事实、证据、实测数据、方案决策、20 个 crate 的 lint 接线、44 个字段的
Rustdoc 补齐，以及可重复的门禁验证记录。

**不负责**：不改变文档覆盖率工具的 AST 口径，不把 UseDocs 当作代码文档抵扣，也不为后续
新 crate 自动生成 manifest；新成员必须显式加入 workspace 并同步声明 `lints.workspace = true`。

---

## 一、接线前缺口

接线前，`core/rust/Cargo.toml` 的 workspace 配置写着：

```toml
[workspace.lints.rust]
missing_docs = "warn"
```

但 **Cargo 的 `[workspace.lints]` 只对声明了 `[lints] workspace = true` 的成员 crate 生效**。
实测 `core/rust/crates/` 下 **20 个 crate 无一 opt-in**，因此这条 lint 在接线前没有对任何
crate 生效。

实际在兜底的是 `tools/repo-check` 的 `A0-COVERAGE-002`，但它报的是 **`[warning]`**，
`bun run check` 的退出码仍为 0。

**接线前后果**：凡是「新增 `pub` 项须 100% Rustdoc」「不用 `#[allow]` 掩盖警告」这类验收条件，
都没有 Cargo 强制力。R2D 的 `encode.rs` 一次就积了 **110 项**，门禁依然是绿的。

这是本仓第一号病史（「同一规则两处各写一份然后漂移」）在工具层的形态：规则写在配置里，
接线断了，而所有人都以为它生效。

---

## 二、接线前证据与复测

### 2.1 没有任何 crate opt-in

接线前检查命令：

```bash
for f in core/rust/crates/*/Cargo.toml; do
  grep -q "lints.workspace = true\|\[lints\]" "$f" && echo "yes $f" || echo "no $f"
done
```

输出是 20 个全部为 `no`：`xiao-artifacts`、`xiao-bytecode`、`xiao-codegen-llvm`、
`xiao-config`、`xiao-diagnostics`、`xiao-doc-coverage-rust`、`xiao-driver`、`xiao-i18n`、
`xiao-ir`、`xiao-lifetime`、`xiao-modules`、`xiao-optimizer`、`xiao-package`、
`xiao-platform`、`xiao-runtime`、`xiao-source`、`xiao-syntax`、`xiao-types`、`xiao-vm`、
`xiao-xar`。

因此接线前 `cargo clippy -- -D warnings` 不会因为缺文档失败：这条 lint 根本没有进入成员 crate。

### 2.2 接线前退出码实测

| 命令 | 退出码 | 说明 |
| --- | --- | --- |
| `bun run check` | 0 | 110 条 `A0-COVERAGE-002` 全部是 warning |
| `bun run check:coverage` | 0 | 覆盖率工具本身通过 |
| `bun test` | 0 | 工具测试通过 |

### 2.3 接线后复测

| 检查 | 结果 | 说明 |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | 通过 | 20 个成员均实际读取 workspace lint 配置 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 通过 | 缺失文档现在会阻断标准 Rust 门禁 |
| `cargo doc --workspace --no-deps` | 通过 | Rustdoc 无缺失文档警告 |
| `cargo fmt --all -- --check` | 通过 | 格式无漂移 |
| `bun run check:coverage` | 通过 | 总体 `3832/3832`，公共 API `2065/2065` |
| `bun run check` | 通过 | 无 `A0-COVERAGE-002` |

---

## 三、两套口径不同，必须同时保留

接线前用 `RUSTFLAGS="-W missing_docs"` 模拟「如果接上 lint 会怎样」：

```bash
RUSTFLAGS="-W missing_docs" cargo check --manifest-path core/rust/Cargo.toml --workspace --all-targets
```

接线前全仓 44 项，分布如下：

| 文件 | 项数 | 归属 |
| --- | --- | --- |
| `xiao-ir/src/model.rs` | 30 | 既有缺口 |
| `xiao-bytecode/src/research/encode.rs` | 11 | R2D 缺口 |
| `xiao-types/src/selection_random.rs` | 2 | 既有缺口 |
| `xiao-types/src/path_constraints.rs` | 1 | 既有缺口 |

而接线前 repo-check 对同一个 `encode.rs` 报 110 项（72 function、30 method、5 constant、
2 struct、1 module）。差异来自统计范围：rustc `missing_docs` 只看 crate 根可达的公开 API，
repo-check 的口径更宽，包含实现块内的关联方法等。

本次先补齐 rustc 口径的 44 项，再由 repo-check 复测全仓；两套口径都为零缺口，但分母仍不相同，
后续不能用其中一套替代另一套。

---

## 四、方案决策

| 方案 | 做法 | 结论 |
| --- | --- | --- |
| A. 只补文档，不接线 | 只补 `encode.rs` 缺口 | 拒绝：规则仍无强制力 |
| B. 一次到位 | 补齐 44 项，并给 20 个 crate 加 `[lints] workspace = true` | **采用并完成** |
| C. 只给部分 crate 接线 | 只接 `xiao-bytecode` / `xiao-vm` 等 | 拒绝：规则不一致，容易误导 |

采用 B 的原因：门槛变更独立成批，避免污染功能批次；接线后标准 Clippy 命令确实会失败于缺失
Rustdoc；rustc 与 repo-check 两套口径可以在同一变更中共同验收。

---

## 五、后续触发条件

任何新增 Rust workspace crate 的变更都必须在其 `Cargo.toml` 写入：

```toml
[lints]
workspace = true
```

任何新增公共声明都必须在同一变更中补 Rustdoc，并同时运行 Clippy 和 `bun run check:coverage`。
接线完成后，不再把 `bun run check` 的退出码单独当作 Rust API 文档证明；它和
`cargo clippy --workspace --all-targets -- -D warnings` 必须成对执行。

---

## 六、交接记录

本阶段包含 20 个 manifest 的 opt-in 和 44 个字段 Rustdoc；实际提交号在提交完成后补入本节、
`docs/DevDocs/README.md` 和 [12. 测试与开发里程碑](12-tests-and-milestones.md)。
