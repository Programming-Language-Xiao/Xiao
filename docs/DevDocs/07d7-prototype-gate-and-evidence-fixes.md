# 07-D.7. 原型门禁与证据收口

> **这是 07-D.6 隔离原型的门禁/证据收口批次，不是并发实现阶段。** 原型的 8 个场景、
> 测试、Clippy、格式和测量已经能跑，但 `bun run check:layout` 失败，且
> `tests/research/**` 不在任何 CI paths 中。本批先把门禁修绿、把测量语义写准，再决定
> 原型结果是否足以进入下一道裁定。
>
> **本批一句话**：**让原型真正通过仓库门禁，并让结果表只声称它实际测到的东西；
> 仍然不写语言语法、IR、ABI 或标准库公开契约。**
>
> 状态：**待实施（2026-10-22）**。只改隔离原型、门禁配置和文档；不登记新阶段代号，
> 不预留 `H0-C`、`07-D 实现`、`std.concurrent` 或任何语法/API。

## 一、交接上下文

### 1.1 必读

1. [07-D.6. 最小 OS 线程原型](07d6-minimal-os-thread-prototype.md)
   —— 原型范围、场景、结果和 caveat；
2. [00. 决策基线](00-decisions.md) §07-D 并发模型裁定（2026-10-22）
   —— 8 项裁定；
3. [07-D. 并发模型研究](07d-concurrency-model-research.md) §10–§11
   —— 成本矩阵、取消顺序和入口条件；
4. [21A. 单线程 RC、强环与并发模型决策交接](21a-rc-cycles-and-concurrency-handoff.md)
   §3.2/§五——新阶段前置；
5. [12. 测试与开发里程碑](12-tests-and-milestones.md)
   —— 测试分层和阶段出口。

### 1.2 当前状态（2026-10-22 实跑）

| 环节 | 现状 | 结论 |
| --- | --- | --- |
| 提交 | `7852e5d`、`8aefefc`；本地 3 个提交未推送 | 原型代码在本地 |
| 原型测试 | `cargo test --manifest-path tests/research/concurrency/Cargo.toml`：8 passed | 场景可运行 |
| Clippy/格式 | `-D warnings`、`cargo fmt --check` 通过 | 原型代码质量通过 |
| 文档/UseDocs | 两项通过 | 文档 lint 通过 |
| 测量 | `--iterations 1000` 成功；本次 spawn/join 208087 ns、深拷贝 209 ns、原子 9 ns、锁 20 ns | 只能作当前机器参考 |
| **布局门禁** | **`bun run check:layout` 失败**：A0-LAYOUT-001/002 | **P1，推送即红**；见 F-1 |
| CI 覆盖 | workspace gates 与 maintenance paths 都不含 `tests/research/**` | 原型测试不会在 CI 执行；见 F-2 |
| 原型状态 | 07-D.6 写“隔离原型已完成” | 门禁未绿前不应写完成；见 F-7 |

### 1.3 本批边界

| 项 | 本批 | 说明 |
| --- | --- | --- |
| 布局门禁 | **做** | 补 README/登记 module-registry，跑完整 `bun run check` |
| CI 覆盖 | **做** | 加 `tests/research/**` path + 原型测试步骤，或明确本地口径 |
| 测量语义 | **做** | 修正对象头/spawn-join 标签，必要时拆测量 |
| 场景标注 | **做** | 把模拟/最小模型写清，避免冒充 Runtime/IR 行为 |
| 跨平台测量 | **计划** | Windows-only 不能支撑 ABI 冻结；至少登记 Linux/macOS 待办 |
| 产品代码/语法/IR/ABI | **不做** | 不进入完整工具链契约 |
| 新阶段代号/`std.concurrent` | **不预留** | 原型结果出来后再决定 |

## 二、审核发现

### F-1 **布局门禁失败（P1，推送即红）**

`bun run check:layout` 输出：

```text
[error] A0-LAYOUT-001 tests/research/concurrency/src
  代码目录缺少 README.md。
[error] A0-LAYOUT-002 tests/research/concurrency/src
  发现的源码目录未在 module-registry.json 的 code/tests 路径中登记。
```

`bun run check` 包含 layout；workspace gates 会跑 `tools/gates/run.sh` → `bun run check`。
虽然本批提交也改了 `docs/**`，推送会触发 workspace gates，但会在 layout 这一步失败。

**处理要求**：

1. 给 `tests/research/concurrency/src` 补 README.md（或按仓库规则调整该目录的 README
   口径，但不能删除/弱化 A0-LAYOUT 规则）；
2. 在 `docs/module-registry.json` 登记该原型目录为 code/tests 路径；必要时同步
   `tools/repo-check/repository.manifest.json`；
3. 重跑 `bun run check:layout`，再跑完整 `bun run check`；
4. 通过前不要推送“原型已完成”的结论。

### F-2 **原型测试没有 CI 覆盖（P2）**

工作区门禁的 `paths` 不包含 `tests/**`；维护回归只含 `tests/fuzz/**`、
`tests/benchmarks/**`、`tests/fixtures/**`，不含 `tests/research/**`。即使 layout 修好，
`tests/research/concurrency` 的 `cargo test` 也不会在 CI 执行。

**处理要求**：二选一并写清：

1. **推荐**：把 `tests/research/**` 加入 workspace gates 的 paths，并在 `run.sh` 或
   workflow 中增加：
   ```bash
   cargo test --manifest-path tests/research/concurrency/Cargo.toml
   cargo clippy --manifest-path tests/research/concurrency/Cargo.toml \
     --all-targets -- -D warnings
   cargo fmt --manifest-path tests/research/concurrency/Cargo.toml -- --check
   ```
   评估是否能接受额外构建时间；原型 crate 没有外部依赖，成本应可控。
2. 明确写成“研究原型仅本地验证”，并在 07-D.6/README 登记为已知 CI 缺口；后续若原型
   结果进入决策，必须先补 CI 或一次性受控运行记录。

### F-3 **`read_state_bytes` 不是对象头尺寸（P2/P3）**

`tests/research/concurrency/src/lib.rs:789`：

```rust
read_state_bytes: size_of::<ReadState<u64>>(),
```

`ReadState<u64>` 是 `payload: u64`（8 bytes）+ `drops: Arc<AtomicUsize>`（8 bytes），
不是原子对象头（没有强/弱计数、tag 或布局字段）。07-D.6 结果表
`:192` 写“只读原型对象头尺寸 16 bytes”，`:56`/`:86` 也写“对象头开销”，会误导 ABI。

**处理要求**：

1. 把字段/表项改名为“只读状态尺寸（载荷 + 计数指针）”，或单独定义/测量真正的句柄与
   对象头布局；
2. 不要把 std `Arc`/`Weak` 的尺寸当成 Xiao 共享句柄 ABI 的尺寸；
3. 在结果表注明“原型模型，不是冻结布局”。

### F-4 **多个场景是模拟，不是实际执行（P3）**

- `simulate_copy_failure` 用 `CopyPart` 编号模拟深拷贝失败，没有实际嵌套值复制失败；
- `simulate_cancellation` 对四个 `CancelPosition` 返回同一组步骤，位置只被记录；
- `BoundaryFacts` 是布尔结构体，不是 lifetime/IR 里的真实事实；
- `candidate_a_accepts` 是手写枚举表，没有接入真实类型检查器。

作为隔离原型可以接受，但结果表必须写“模拟/最小模型”。否则容易被读成 Runtime、IR 或
类型层已经实现。

**处理要求**：在 07-D.6 §7.1 每行加“模拟/模型”限定；在 §7.3 明确这些结果不能作为
Runtime/IR 行为证据。

### F-5 **`spawn_join_ns` 包含深拷贝（P3）**

`measure()` 每次循环先 `source.deep_copy()`，再 `thread::spawn` 和 `join`；所以
`spawn_join_ns` 实际是“深拷贝 + spawn + join”。`deep_copy_ns` 另行测量，二者标签会
让读者以为 spawn/join 是纯线程成本。

**处理要求**：把 spawn/join 测量改成使用预先复制好的值，或改名为“复制 + spawn/join”，
并在结果表注明包含关系。

### F-6 **只有 Windows x86_64 测量（P3，计划项）**

当前结果只代表 Windows x86_64；OS 线程、锁和调度成本对平台敏感。07-D.6 §7.3 已写
“下一道裁定应先审阅跨平台重复测量”，本批应把 Linux/macOS 测量列为明确待办，不能
用单平台数字冻结 ABI 或调度方案。

**处理要求**：在结果表/入口条件中登记 Linux amd64/arm64、macOS arm64 的待测项；若
平台无法本地运行，至少记录运行环境、命令和缺失原因。

### F-7 **“隔离原型已完成”的状态过早（P3）**

07-D.6 状态写“隔离原型已完成”，但 layout 未过、CI 未覆盖。建议在 F-1/F-2 关闭前改为
“本地场景通过，仓库门禁待修”；修绿后再写完成。

## 三、证据记录

| 项 | 结果 |
| --- | --- |
| 原型测试 | 8 passed |
| Clippy | `-D warnings` 通过 |
| 格式 | 通过 |
| 文档/UseDocs | 通过 |
| Layout | **失败**：A0-LAYOUT-001/002 |
| 测量 | Windows x86_64，1000 iterations；单次结果只作参考 |
| 产品边界 | 仅 `docs/**` 与 `tests/research/concurrency/**` |

## 四、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 补 `tests/research/concurrency/src/README.md`，登记 module-registry/manifest | `bun run check:layout` |
| 1 | 决定 CI 覆盖口径并接线 | workspace gates / 文档例外 |
| 2 | 修正 `read_state_bytes`、spawn/join 测量标签和对象头声明 | 代码/结果表 |
| 3 | 把模拟场景和 Windows-only 限制写清 | 07-D.6 §7 |
| 4 | 跑完整门禁：原型测试/Clippy/fmt、layout、`bun run check`、docs/usedocs | 本地/CI |
| 5 | 推送并记录 CI 运行号；通过后才更新“已完成” | CI |

**本批不写产品代码**；不改语言、类型、IR、Runtime ABI 或标准库公开契约。

## 五、验收

1. **Layout**：`bun run check:layout` 与完整 `bun run check` 通过；`tests/research/concurrency/src`
   有 README 且已在 module-registry 登记；
2. **CI**：`tests/research/**` 有明确覆盖（推荐加 path + 测试步骤），或在 07-D.6/README
   写明本地-only 例外和理由；最终 HEAD 有对应运行号；
3. **测量**：不再把 `ReadState` 尺寸写成对象头；spawn/join 与深拷贝关系写清；跨平台
   待办已登记；
4. **场景**：模拟/最小模型已标注，没有被写成 Runtime/IR 行为证据；
5. **原型**：8 个测试、Clippy、格式仍通过；没有产品 crate 引用或 ABI 变化；
6. **状态**：门禁通过后才把 07-D.6 写成“完成”；未通过前保持“本地场景通过、门禁待修”。

## 六、不负责

- 不把原型升级成完整并发实现或产品 Runtime；
- 不修改语言语法、类型系统、IR、Runtime ABI、对象布局或标准库公开接口；
- 不实现任务池、Actor、序列化、冻结共享或官方并发容器；
- 不删除/弱化 A0-LAYOUT 规则来换取通过；
- 不用单平台测量冻结 ABI、调度或成本结论；
- 不登记新阶段代号或 `std.concurrent`。

## 七、落点与取证

```text
docs/DevDocs/07d7-prototype-gate-and-evidence-fixes.md  本文
docs/DevDocs/07d6-minimal-os-thread-prototype.md        状态/结果表/限制
docs/DevDocs/README.md                                  07-D.7 索引
tests/research/concurrency/src/README.md                新增（layout）
docs/module-registry.json                               登记原型目录
tools/repo-check/repository.manifest.json               必要时同步
tests/research/concurrency/src/lib.rs                   测量/标签修正
.github/workflows/workspace-gates.yml                   tests/research CI 覆盖（如采用）
tools/gates/run.sh                                      原型测试步骤（如采用）
```

```bash
# 布局与完整门禁
bun run check:layout
bun run check

# 原型
cargo test --manifest-path tests/research/concurrency/Cargo.toml
cargo clippy --manifest-path tests/research/concurrency/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path tests/research/concurrency/Cargo.toml -- --check
cargo run --release --manifest-path tests/research/concurrency/Cargo.toml -- --iterations 1000

# 文档
bun run check:docs
bun run check:usedocs
git diff --check
```

## 八、风险

1. **为了变绿而绕过 layout**：不能在 manifest/规则里豁免原型目录；必须按仓库约定补齐
   README 和登记。
2. **CI 成本**：原型 crate 无外部依赖，测试很快；如果加入 workspace gates，确认构建
   时间和 runner 成本可接受。
3. **把模型当 ABI**：std `Arc`/`Weak`/`Mutex` 的尺寸和语义不是 Xiao 共享句柄布局；
   结果表必须保留“原型模型”限定。
4. **单平台结论**：Windows 数字不能代表 Linux/macOS；跨平台测量完成前不要冻结 ABI。
5. **模拟场景被误读**：深拷贝失败、取消、CrossThread 和候选 A 都是最小模型，不是
   Runtime/IR 实现；文档必须直说。
