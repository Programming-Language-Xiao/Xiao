# 00C. 文档 lint 接线预研究

> **预研究文档，不是施工图。** 它记录一个已确认、但尚未决策的工具层缺口，并给出实测依据，
> 供决定「要不要接、怎么接、什么时候接」。
>
> 结论一句话：**仓库以为自己在强制「公开项必须有文档」，实际上没有。**

## Agent 交接上下文

### 接手前提

- 先读 [00A. 工程框架与目录布局](00a-project-layout.md)、
  [A0. 工作区与质量门禁实现方案](00a-a0-workspace-and-checkers.md)、
  [00B. UseDocs 同步政策](00b-usedocs-policy.md)。
- 本文与 A0 检查器直接相关：它描述的正是「A0 文档覆盖率门禁之外，还缺哪一层强制」。

### 本阶段交付与不负责

**交付**：缺口的事实、证据、实测数据、选项与代价、触发条件。

**不负责**：不修任何 crate、不改 workspace 配置、不动门禁行为——**本文只提供决策依据**。

---

## 一、缺口

`core/rust/Cargo.toml` 的 `:33-34` 写着：

```toml
[workspace.lints.rust]
missing_docs = "warn"
```

但 **Cargo 的 `[workspace.lints]` 只对声明了 `[lints] workspace = true` 的成员 crate 生效**。
实测：`core/rust/crates/` 下 **20 个 crate 无一 opt-in**，因此这条 lint
**从写下那天起就没有对任何 crate 生效过**。

实际在兜底的是 `tools/repo-check` 的 `A0-COVERAGE-002`，但它报的是 **`[warning]`**，
`bun run check` 的**退出码仍为 0**。

**后果**：凡是「新增 `pub` 项须 100% Rustdoc」「不用 `#[allow]` 掩盖警告」这类验收条件，
**都没有强制力**。R2D 的 `encode.rs` 一次就积了 **110 项**，门禁依然是绿的。

这是本仓第一号病史（「同一规则两处各写一份然后漂移」）在**工具层**的形态：
规则写在配置里，接线断了，而所有人都以为它生效。

---

## 二、证据

### 2.1 没有任何 crate opt-in

```bash
for f in core/rust/crates/*/Cargo.toml; do
  grep -q "lints.workspace = true\|\[lints\]" "$f" && echo "✅ $f" || echo "❌ $f"
done
```

**输出：20 个全部为 ❌**（`xiao-artifacts`、`xiao-bytecode`、`xiao-codegen-llvm`、
`xiao-config`、`xiao-diagnostics`、`xiao-doc-coverage-rust`、`xiao-driver`、`xiao-i18n`、
`xiao-ir`、`xiao-lifetime`、`xiao-modules`、`xiao-optimizer`、`xiao-package`、
`xiao-platform`、`xiao-runtime`、`xiao-source`、`xiao-syntax`、`xiao-types`、`xiao-vm`、`xiao-xar`）。

因此 `cargo clippy -- -D warnings` 也**不会**因为缺文档而失败——这条 lint 根本没进编译。

### 2.2 退出码实测

| 命令 | 退出码 | 说明 |
| --- | --- | --- |
| `bun run check` | **0** | 110 条 `A0-COVERAGE-002` 全部是 warning |
| `bun run check:coverage` | 0 | |
| `bun test` | 0 | |

---

## 三、两套口径不同，别把它们当成一件事

用 `RUSTFLAGS="-W missing_docs"` 模拟「如果接上 lint 会怎样」：

```bash
RUSTFLAGS="-W missing_docs" cargo check --manifest-path core/rust/Cargo.toml --workspace --all-targets
```

**全仓 44 项**，分布：

| 文件 | 项数 | 归属 |
| --- | --- | --- |
| `xiao-ir/src/model.rs` | 30 | **既有**，与任何当前批次无关 |
| `xiao-bytecode/src/research/encode.rs` | 11 | R2D |
| `xiao-types/src/selection_random.rs` | 2 | 既有 |
| `xiao-types/src/path_constraints.rs` | 1 | 既有 |

而 `repo-check` 对**同一个 `encode.rs`** 报 **110 项**（72 function / 30 method /
5 constant / 2 struct / 1 module）。

**差异原因**：rustc 的 `missing_docs` 只看**crate 根可达的公开 API**；
repo-check 的口径更宽（含实现块内的关联方法等）。
**接线时必须同时确认两套口径**，否则会出现「rustc 不报、repo-check 仍报」的夹缝。

---

## 四、选项与代价

| 方案 | 做什么 | 代价 |
| --- | --- | --- |
| **A. 只补文档，不接线** | 补齐 `encode.rs` 的缺口，把本缺口如实登记 | 规则仍无强制力，下一个批次还会重演 |
| **B. 一次到位** | 补齐 **44 项**（含既有 33 项）+ 给 **20 个 crate** 加 `[lints] workspace = true` | 门槛变更影响全仓；提交会带上与当前批次无关的既有改动；**`cargo clippy -- -D warnings` 从此会因缺文档直接失败**，需要确认全仓确实干净 |
| **C. 只给部分 crate 接线** | 例如只给 `xiao-bytecode` / `xiao-vm` | 规则在仓内不一致，**容易让人误以为全仓已受管**——比不接更危险 |

**方案 C 不推荐**：不一致的强制比没有强制更容易误导。

---

## 五、建议与触发条件

**建议按 B 执行，但必须独立成一批**，理由：

1. 它把 33 项**既有**缺口和门槛变更混在一起，塞进任何功能性批次都会污染该批次的评审；
2. 接线后 `cargo clippy -- -D warnings` 行为改变，属于**门禁变更**，应当单独提交、单独验证；
3. 需要同时确认 rustc 与 repo-check **两套口径**都干净（见第三节），否则会留下夹缝。

**触发条件**：在任何「新增大量公开声明」的批次开始前做。
R2D 的 `encode.rs` 就是反例——**它一次新增了 110 项缺口，而当时门禁是绿的**。

**在此之前的最低要求**：凡是验收条件里写了「新增 `pub` 项 100% Rustdoc」的批次，
必须**手动跑一次** `bun run check | grep A0-COVERAGE-002` 并确认计数为 0，
**不能以「`bun run check` 退出码为 0」当作通过**。
