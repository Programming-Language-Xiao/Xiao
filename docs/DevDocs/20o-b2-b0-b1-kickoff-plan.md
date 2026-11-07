# 20O-B2-B0/B1. 开工基线与依赖/共享内核实施规划

> **这是 B2-B 的开工细化；B2-B 产品仍未开工，B2-C 仍未开始。**
> 入口为 A7 内部收口代码快照 `5749d6e0`、产品基线 `e7bf7a49` 与当前规划提交 `e596cea8`。
> B0 只建立独立基线，B1 只补依赖和私有共享内核；不新增 intrinsic、不改 ABI/生成器、不写 `cmath` 公开入口。
> `stdlib.cmath` 仍 planned，旧 721 条 F10 不运行，22C/22D、19/11/22 与 B1 历史债务保持独立。

## 一、入口与目标

| 项 | 值 |
| --- | --- |
| 入口代码 | `5749d6e0`（A7 内部收口）；当前规划提交 `e596cea8` 仅改 DevDocs |
| 产品基线 | `e7bf7a49` |
| 当前版本 | ABI 1.18、生成器 20、活动 intrinsic 最大 32、标准库接口 1 |
| B0 目标 | 独立工作树内复现 A7 基线，证明 B2-B 从干净入口起步 |
| B1 目标 | 补 `libm 0.2.16` 直接依赖，建立 `lfloat_divide`/`cmath` 私有内核与共享 `pow` 边界 |
| 不在本页 | intrinsic 33～48、C ABI 符号、类型/IR/VM/LLVM 接线、ABI 1.19/21、registry 升级 |
| 后续 | B2 `lfloat_divide`/33；B3 `cmath`/34～48；B4 语言全链路；B5 版本；B6 资源/性能；B7 收口 |

B0/B1 完成后，B2-B 才进入公开入口实现；B2-C 仍负责最终 721、三平台公共产物与教程/registry。

### 1.1 B0/B1 要做

1. 从当前 HEAD 建独立工作树，验证产品树相对 `5749d6e0` 无差异；
2. 用全新 target/Runtime/缓存/临时目录复现 B1 契约、资源、双精确点、B2-A 资源/语言和 28/28 守卫；
3. 记录源码、锁、工具链、目标、PID、命令、日志和前后输入摘要；
4. 按 F-7 补 `libm = "=0.2.16"` 直接依赖，核对六包闭包和 `Cargo.lock`；
5. 建立私有 `lfloat_divide` 内核和 15 项 `cmath` 内核边界，不公开 intrinsic；
6. 建立 float `**` 与 `cmath.pow` 的共享内核设计，但不在 B1 半接线；
7. 建立特殊值/域/有符号零/ULP 的单元测试底座，为 B2/B3 留可复用入口。

### 1.2 明确不做

- 不新增或激活 intrinsic 33～48，不改 `intrinsics.json`；
- 不新增 C ABI 符号，不改 `xiao-runtime-abi` 版本或状态表；
- 不新增 `stdlib/cmath.xiao` 公开包装，不改 `docs/module-registry.json`；
- 不提前升 ABI 1.19/生成器 21，不生成跨版本产物；
- 不运行旧 721 条 F10，不关闭任何历史债务；
- 不把 B0/B1 的绿色写成 B2-B 功能完成或 B2-C 最终发布。

## 二、B0 开工基线

### 2.1 工作树与身份

1. 在当前仓库外新建唯一任务根，例如 `D:\project\Xiao\.b2b-kickoff\<run-id>`；
2. 从当前 HEAD `e596cea8` 建工作树或完整副本，记录 `git rev-parse HEAD`；
3. 验证产品树与 A7 快照一致：

```text
git diff --name-only 5749d6e0 e596cea8 -- ':!docs/DevDocs'
```

结果必须为空；`docs/DevDocs` 的规划差异不算产品差异。

4. 固定 Rust 1.96.0、目标平台、`Cargo.lock`、`--locked`，清理继承的 `RUST_BIGDECIMAL_*`；
5. 固定独立 `CARGO_TARGET_DIR`、`XIAO_HOME`、`TEMP`/`TMP`、`XIAO_STDLIB_ROOT`；
6. 保留 `XIAO_B1_F10=0`、`XIAO_B2_A_PERF=0`；B0 不启用旧 F10 或 A6 性能开关。

### 2.2 基线命令与期望

以下命令在独立工作树、全新输出目录执行；每条保存 argv、cwd、PID、起止、退出码、日志和前后源码摘要。

| 检查 | 命令要点 | 期望 |
| --- | --- | --- |
| B1 契约 | `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_imp_contract` | 9 passed / 0 ignored |
| B1 资源 | 同上 `--test 20o_b1_imp_resources` | 17 passed / 0 ignored |
| B2-A 资源 | 同上 `--test 20o_b2_a_resources --test 20o_b2_a_performance_protocol` | 3 与 8 passed / 0 ignored |
| B2-A 语言 | 同上 `--test 20o_b2_a_contract -- --nocapture` | 12 passed / 1 ignored |
| Driver 单元 | 同上 `-p xiao-driver --lib` | 64 passed / 1 ignored |
| B1 VM 精确 | `--test 20o_b1_probe -- b1_capabilities_have_vm_golden_outputs --exact --nocapture` | 恰好 1 passed |
| B1 原生精确 | `--test 20o_b1_probe -- --ignored b1_capabilities_match_native_golden_outputs --exact --nocapture` | 恰好 1 passed |
| A1 VM 精确 | `--test 20o_b2_a1_identity a1_numeric_values_and_errors_have_vm_golden_outputs -- --exact --nocapture` | 恰好 1 passed |
| A1 原生精确 | `--test 20o_b2_a1_identity a1_numeric_values_and_errors_match_native_golden_outputs -- --ignored --exact --nocapture` | 恰好 1 passed |
| 原 28 守卫 | `pwsh -NoProfile -File tools/gates/b1-probe-guards.ps1 -BashPath <bash> -ReportPath <b0>/b1-guards-report.json` | 28 passed / 28 total |
| 工程门禁 | workspace `check`、benchmark `check --locked`、`clippy -D warnings`、`cargo fmt --check` | 全部退出 0 |
| 文档门禁 | `bun tools/repo-check/src/cli.ts all` | 退出 0；既有覆盖率警告照常记录 |

若计数与 A7 不一致，停止并定位，不能改断言、放宽阈值或重跑补绿。

工程门禁的实际命令：

```text
cargo +1.96.0 check --locked --manifest-path core/rust/Cargo.toml --workspace
cargo +1.96.0 check --locked --manifest-path tests/benchmarks/Cargo.toml
cargo +1.96.0 clippy --locked --manifest-path core/rust/Cargo.toml --workspace --all-targets -- -D warnings
rustup run 1.96.0 cargo fmt --manifest-path core/rust/Cargo.toml --all -- --check
bun tools/repo-check/src/cli.ts all
pwsh -NoProfile -File tools/gates/b1-probe-guards.ps1 -BashPath <bash> -ReportPath <b0>/b1-guards-report.json
```

### 2.3 B0 证据

```text
docs/DevDocs/assets/20o-b2-b/entry-<date>/b0/
  bootstrap.json            任务根、源码 HEAD、产品基线、工具链
  environment.json          全部 overrides 与剥离的环境变量
  source-manifest.json      被测源码清单及摘要
  checks/*.json             每条命令的 argv/cwd/PID/退出码/前后摘要
  logs/*.log                原始 stdout/stderr
  b1-guards-report.json     28/28 原始报告
  closeout.json             B0 结论、计数、未执行项
```

B0 退出：独立工作树与 A7 产品树一致；上述计数全部匹配；没有产品、锁、ABI、intrinsic、registry 或工作流改动。

## 三、B1 依赖、私有内核与共享 `pow`

### 3.1 依赖

1. 在 `core/rust/Cargo.toml` 的 `[workspace.dependencies]` 增加精确版本：

```text
libm = "=0.2.16"
```

2. 在 `core/rust/crates/xiao-runtime/Cargo.toml` 增加 `libm.workspace = true`；
3. 保留默认/`arch` 能力，不启用 `unstable`；
4. `Cargo.lock` 预期不新增包（`libm 0.2.16` 已作为 `bigdecimal` 传递依赖存在）；若发生变化，必须逐包核对是否仍在 F-7 六包闭包内；
5. 复核许可、校验值、`--locked` 和受控环境清理；
6. 不改 `rust-version` 声明，不推断未实测 MSRV。

### 3.2 私有内核边界

建议落点（名称可在实现时微调，但不得越过私有边界）：

```text
core/rust/crates/xiao-runtime/src/value/cmath/mod.rs       15 项 cmath 私有内核
core/rust/crates/xiao-runtime/src/value/cmath/reference.rs 特殊值/ULP 测试辅助
core/rust/crates/xiao-runtime/src/value/arithmetic/explicit_division.rs  lfloat_divide 内核
core/rust/crates/xiao-runtime/src/value/ops.rs            仅保留现有 float ** 调用点，不在 B1 切换
```

要求：

1. `cmath` 内核只接受已校验的 `f64`，返回结果或明确域/错误身份；不直接暴露 `XiaoValue`；
2. 全部 cmath 入口先拒绝 NaN/Inf；域错误、非有限结果按 F-2 为 009；
3. `sqrt(-0)=-0`、`round(-0.5)=-0`、min/max 零符号等按 F-2 逐项测试；
4. 普通非零超越与 `pow` 结果按 4 ULP 验收，取整/abs/min/max 逐位；
5. `lfloat_divide` 内核只做精确商余 + 七种最终舍入，不接 intrinsic；
6. `pow` 共享边界先定义清楚：`cmath.pow` 与 float `**` 的域、NaN/Inf、负底、零指数和末位规则只能有一套；
7. B1 **不切换** `ops.rs` 的 float `**` 调用点；B3 再把 `cmath.pow` 与 float `**` 一次性接到同一内核，避免半接线。

### 3.3 共享 `pow` 的行为差

B1 必须产出一张“现有 float `**` vs F-2/F-9 共享内核”的行为差表，至少覆盖：

- NaN/Inf 输入是拒绝还是传播；
- 负底非整数指数、`0**0`、`0` 的负幂；
- `-0` 的奇偶正整数幂、非有限结果；
- 普通非零结果的 4 ULP 边界；
- 现有 `finite_float64` 的非有限结果错误身份是否保持。

如果差表显示切换共享内核会改变 B2-A 已验收行为，先停下回冻结，不在 B1 静默改。

### 3.4 B1 测试与证据

1. 15 项 cmath 特殊值/域/有符号零单元测试；
2. 15 项 cmath 的 NaN/Inf 拒绝测试；
3. ULP/位模式比较工具与至少一组独立参考向量；参考只用离线 MPFR 4.2.2/GMP 6.3.0，登记生成器版本/摘要；
4. `lfloat_divide` 七种舍入、precision 1/32,768、0/超界、短结果的私有内核测试；
5. `cargo +1.96.0 test --locked -p xiao-runtime` 全绿；
6. 复核 `git diff --name-only` 只含允许文件；`intrinsics.json`、`xiao-runtime-abi`、`CODEGEN_VERSION`、`docs/module-registry.json`、`stdlib/cmath.xiao` 必须无差异；
7. 保存依赖解析、锁摘要、内核测试日志、源码清单和 closeout。

允许改动：

```text
core/rust/Cargo.toml
core/rust/crates/xiao-runtime/Cargo.toml
core/rust/Cargo.lock（仅在确有必要时）
core/rust/crates/xiao-runtime/src/value/cmath/**
core/rust/crates/xiao-runtime/src/value/arithmetic/explicit_division.rs
core/rust/crates/xiao-runtime/src/value/mod.rs
docs/DevDocs/assets/20o-b2-b/**
```

B1 退出：

- `libm` 依赖身份和闭包可核对；
- 私有 cmath/lfloat_divide 内核与测试可复现；
- float `**` 与 `cmath.pow` 的行为差已记录，尚未半接线；
- 没有 intrinsic、C ABI、ABI/生成器、registry、公开 stdlib 改动；
- B2 的 intrinsic 33 入口可以开始。

## 四、证据与门禁

| 层 | B0 | B1 |
| --- | --- | --- |
| 源码 | HEAD/产品基线/工作树身份 | 允许文件 diff 复核 |
| 依赖 | 锁文件、工具链、目标 | `libm` 直接依赖、六包闭包、锁摘要 |
| 功能 | B1 9/17、B2-A 3/8/12、双精确、64/1 | 内核单元测试 |
| 守卫 | 28/28、workspace/clippy/fmt | Runtime 全测试 |
| 文档 | docs/usedocs/layout/all | 证据索引与 closeout |
| 禁止 | 不改产品/锁/ABI/registry | 不改 intrinsic/ABI/registry/公开 stdlib |

B0/B1 的绿色只证明“干净开工”和“私有内核可用”，不证明 `lfloat_divide`/`cmath` 已交付。

## 五、风险与禁止

- 复用 A7 的 target、Runtime、缓存或可执行文件，导致身份混用；
- 在 B1 提前改 `ops.rs` 的 float `**`，留下 cmath 未接、`**` 已变的半状态；
- 把 `libm` 直接依赖写成宽版本或启用 `unstable`，引入未批准闭包；
- 用宿主 `f64` 结果直接当 ULP 黄金，或用被测 libm 生成自己的参考；
- 在 B1 提前加 intrinsic 33～48、C 符号、`stdlib/cmath.xiao` 或 registry；
- 为让基线变绿改断言、删场景、放宽阈值或重跑补绿；
- 把 B0/B1 写成 B2-B 完成、B2-C 发布或 721 条成绩。

## 六、交接

B1 完成后进入 B2：`lfloat_divide` 与 intrinsic 33。B2 才新增公开入口、C 符号和 VM/LLVM 绑定；B3 再统一 `cmath.pow` 与 float `**`。

新 Agent 先读：

1. [B2-B 实施规划](20o-b2-b-implementation-plan.md)；
2. [已签冻结正文](20o-b2-freeze-draft.md) §五～§九；
3. [F-3 资源/性能](20o-b2-freeze-c-resource-dependency-plan.md)；
4. [F-5 原语/C ABI](20o-b2-freeze-b1-closeout-plan.md)；
5. [F-9 误差与共享内核](20o-b2-freeze-a-numeric-semantics-plan.md)；
6. [F-7 依赖](20o-b2-freeze-c-resource-dependency-plan.md)；
7. [A7 内部收口记录](20o-b2-a7-b2a-closeout.md)。

## 七、落点

```text
core/rust/Cargo.toml                                      libm 精确依赖
core/rust/crates/xiao-runtime/Cargo.toml                  libm.workspace
core/rust/crates/xiao-runtime/src/value/cmath/            15 项私有内核
core/rust/crates/xiao-runtime/src/value/arithmetic/       lfloat_divide 内核
core/rust/crates/xiao-runtime/src/value/ops.rs            现有 float ** 调用点（B1 不切换）
docs/DevDocs/assets/20o-b2-b/entry-<date>/b0/             B0 基线证据
docs/DevDocs/assets/20o-b2-b/entry-<date>/b1/             B1 依赖/内核证据
```

本页只细化 B0/B1；B2-B 公开入口、ABI 1.19/21 和 B2-C 仍后置。
