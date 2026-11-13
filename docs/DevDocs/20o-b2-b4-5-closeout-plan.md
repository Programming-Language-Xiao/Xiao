# 20O-B2-B4-5. cmath 语言全链路内部收口规划

> **B4-0/B4-1 已完成 Windows 本机内部验收；B4-2A 与 FIX1 已完成 Windows 本机退出；B4-2B 已在246bea46完成Windows本机内部验收；B4-3、B4-4 尚未开始；B4-5 未开始，B5～B7 与 B2-C 后置。**
> 入口为 B4-0/B4-1 记录及后续 B4-2A/B、B4-3、B4-4 各自完成后生成的最终内部证据。
> B4-5 只做整批语言收口、证据对账和 B5 交接，不修改产品源码；发现产品问题时退回所属阶段处理。不升 ABI 1.19/21，不升级 registry/教程。
> 内部 `language_available=true` 不等于最终公共发布；B2-C 才做 721、三平台公共产物和状态升级。

## 一、入口与范围

| 项 | 收口时需要 |
| --- | --- |
| B4-1 | 已完成 Windows 本机验收；15 项类型/IR 与正式向量迁移，[固定收口记录](assets/20o-b2-b/entry-20261110-b4-0-1/README.md)，产品提交 `c7a45e10b76b26915499717563761ed20fda6e6c` |
| B4-2A | 已完成 Windows 本机退出；TAC/字节码 34～48 往返与 FIX1 边界门禁 |
| B4-2B | 已完成Windows本机内部验收；VM15项、002/009/span/清理，见[记录](assets/20o-b2-b/entry-20261113-b4-2b/README.md) |
| B4-3 | 待完成；LLVM 动态/静态、float `**` 共核、O0/O3 |
| B4-4 | 待完成；`std.cmath`/`stdlib.cmath` 双路径、导出/移除、`.xar`/`.xiaoc`、序列化/隔离 |
| 版本 | 仍 ABI 1.18、生成器 20、接口 1；B5 才切 1.19/21 |
| 公共状态 | `stdlib.cmath`、UseDocs、registry 仍 planned；旧 721 未跑 |

B4-5 是 **B4 整批内部收口**，不是 B2-C 最终发布。三平台公共产物、完整 721、最终教程/registry 仍留 B5/B6/B2-C。

Pow 差异的固定对账输入为 [B3-CABI `pow-switch-manifest.json`](assets/20o-b2-b/entry-20261109-b3-cabi/baseline/pow-switch-manifest.json)，SHA-256 `53b751666f42814f94993ed606e3dfe650941b022392d7d04ea57d9f3d16443d`；共 1,075 项：1,011 一致、40 值/错误边界、24 个位模式差异。B4-5 必须核对该固定清单，并以 B4 最终源码在 VM/LLVM 中复核共享 pow 语义；清单不是 B4 运行结果。

### 1.1 要做

1. 对账 B4-1～B4-4 的代码提交、证据、允许路径和源码身份；
2. 复核 15 项 cmath 在类型/IR/TAC/VM/LLVM/归档的完整链路；
3. 复核 30 条导入路径内部可执行、旧 planned 拒绝证据保留；
4. 复核导出/逐函数移除/普通包隔离/归档/序列化；
5. 复核 float `**`/cmath.pow 共核、sfloat 不变，并逐项对账固定的 1,075 组 pow 差异；
6. 复核错误 002/009、span、catch、finally/release、Pure 不删除；
7. 记录 Windows 本机与未执行平台，形成 B5 入口；
8. 更新内部状态说明，但 registry/教程保持 planned。

### 1.2 明确不做

- 不升 ABI 1.19/生成器 21，不改 `.xiaoc`/`.xar`/原生元数据；
- 不升级 `stdlib.cmath` registry/最终教程；
- 不跑旧 721，不关闭 22C/22D、19/11/22；
- 不新增 intrinsic 49+/C 符号，不改 1～48/ID 100；
- 不实现 sfloat cmath；
- 不把 B4-5 内部收口写成 B2-C 发布或三平台产品；
- B4-5 只改收口文档和本批证据；不得在本批顺手修改 B4-1～B4-4 产品源码或测试。发现产品回归时记录失败并退回所属阶段，不以收口批次修补后直接签收。

## 二、开工闸门

1. B4-1～B4-4 各自审计通过，代码/证据身份未漂移；
2. 30 条 cmath 导入路径已按 B4-4 转为内部可执行，旧拒绝证据保留；
3. 15 项类型/IR/TAC/VM/LLVM/归档证据可复现；
4. float `**`/cmath.pow 共核、静态无 `llvm.pow.f64` 第二实现；
5. ABI 1.18/生成器 20/接口 1/registry planned 未变；
6. B1/B2/B3 固定历史审计可复核；28/28 守卫须在 B4 最终源码对应工作区重跑并保存报告，不能拿 B3 历史报告冒充本次结果；
7. 唯一任务根、独立 target/Runtime/缓存已建立。

## 三、分批执行

### B4-5A：代码与证据对账

1. 固定 B4-1 的完整提交 `c7a45e10b76b26915499717563761ed20fda6e6c`、B4-2A/B、B4-3、B4-4 的最终完整提交和证据摘要；
2. 逐阶段核对允许路径、锁、Runtime、工具链、目标、PID、日志；旧 1.18 证据只作为各自历史阶段记录，不冒充 B4-5 执行；
3. 复核 B4-4 后的产品树与 B4-5 固定产品提交一致，收口后产品源码不再变化；
4. 固定 pow-switch-manifest 字节摘要，并核对 1,075 项分类、F-2/F-9 归属及其 B4 VM/LLVM 对应回归；
5. B4-5 审计从固定 Git commit/tree/blob 校验产品和锁身份，不要求当前 HEAD 等于产品提交；审计只读，不写 evidence/closeout；
6. 运行收口审计自反例：篡改 evidence 清单、删除已登记文件、伪造阶段提交/源码身份或把 planned/public 状态冒进都须按登记原因拒绝，且基线审计前后证据字节不变；
7. 保存阶段对账表与身份摘要。

退出：B4-1～B4-4 证据链可逐阶段对账。

### B4-5B：语言全链路回归

1. 15 项 cmath 经类型/IR/TAC/VM/LLVM/归档；
2. 双路径导入、导出、逐函数移除、普通包隔离；
3. 002/009、span、catch、finally/release、Pure 不删除；
4. 30 条路径内部可执行；旧 planned 拒绝证据保留；
5. float `**`/cmath.pow 共核、sfloat `**` 未变；对固定 1,075 项逐项核算：1,011 项新旧结果一致；40 项值/错误边界符合 F-2 特殊值顺序；24 项新旧位模式差异均为 1 ULP，且各自新结果满足 F-9 参考容差；
6. B1/B2/B3 固定历史审计与当前 B4 功能回归分开记录；28/28 守卫在最终工作区重新执行。B4 证据记录同时保存维护工作流 SHA-256、守卫报告 SHA-256 和 B4 源码提交，不能将守卫模拟当成产品测试；
7. 使用 Rust `1.96.0` 运行完整 workspace gates：`cargo +1.96.0 test --manifest-path core/rust/Cargo.toml --workspace --locked`、`cargo +1.96.0 clippy --manifest-path core/rust/Cargo.toml --workspace --all-targets --locked -- -D warnings`、`cargo +1.96.0 fmt --manifest-path core/rust/Cargo.toml --all -- --check`、`bun run check:lock`、`bun tools/repo-check/src/cli.ts all`；
8. 保存 VM/LLVM/归档/序列化日志。

退出：15 项 cmath 内部全链路通过；pow 清单无未登记差异；workspace gates 全部通过；无未登记行为变化。

### B4-5C：状态与文档边界

1. 记录 B4 内部 `language_available=true`；
2. `stdlib.cmath`、UseDocs、registry 仍 planned；
3. 不写最终公共教程/verified 示例；
4. 记录 Windows 本机与未执行平台、无三平台 CI；
5. 不把内部可用写成 B2-C 最终发布；
6. 保存状态核对与文档门禁。

退出：内部语言状态明确，公共状态仍 planned。

### B4-5D：B5 交接

1. 列出 B4 最终代码/证据身份；
2. 列出 B5 需要切换的 ABI 1.19/生成器 21、旧/新拒绝矩阵、缓存重生成；
3. 列出 B6 资源/性能、B2-C 721/教程/registry；
4. 列出未执行平台和已知债务；
5. 形成 B5 入口文档/证据索引。

退出：B5 入口明确；B5 未开始。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| 对账 | B4-1～B4-4 代码/证据/允许路径/源码身份 |
| 类型/IR | 15 项、ID/参数/span |
| TAC/VM | 34～48 往返、VM 15 项、002/009/清理 |
| LLVM | 动态/静态、float `**` 共核、O0/O3 |
| 双路径 | std/stdlib、导出/移除、普通包隔离 |
| 归档/序列化 | `.xar`/`.xiaoc`、加载器、无格式变化 |
| 导入迁移 | 30 条内部可执行；旧拒绝证据保留 |
| Pow 差异 | 固定 1,075 项：1,011 一致、40 值/错误边界、24 个 1 ULP；与 B4 最终 VM/LLVM 及 F-2/F-9 核对 |
| 回归 | B1/B2/B3 固定历史审计（只证明各自历史快照）、B4-1～4 当前功能回归；最终快照 28/28 守卫 |
| workspace gates | workspace 测试、Clippy、fmt、锁、docs/UseDocs/layout 门禁 |
| 收口审计 | 固定 Git 对象只读审计；清单/缺文件/身份/状态冒进自反例及只读性 |
| 边界 | ABI1.18/20、registry planned、721 未跑 |

## 五、退出条件

1. B4-1～B4-4 证据链可对账；收口审计按固定 Git 对象通过且自反例按预期拒绝；
2. 15 项 cmath 内部全链路通过；
3. 30 条导入路径内部可执行，旧证据保留；
4. float `**`/cmath.pow 共核，无第二实现；1,075 组差异固定输入逐项对账完成，无未登记行为变化；
5. 内部 `language_available=true`，公共 registry/教程仍 planned；
6. 无 ABI 1.19/21、无旧 721、无历史债务关闭；
7. 当前 B4 源码快照的 28/28 守卫和完整 workspace gates 通过，报告可复核；
8. B5 入口明确；B5 未开始；
9. 三平台/性能仍未声称，B2-C 仍未开始。

## 六、风险与禁止

- 把 B4 内部可用写成 B2-C 最终发布；
- 提前升级 registry/教程或改 ABI；
- 用旧 planned 拒绝测试或旧证据冒充内部可用；
- 删掉 B4-1～B4-4 历史失败/旧证据；
- 只做 VM 不做 LLVM，或只做动态不做静态；
- 跑旧 721 或关闭历史债务。

## 七、交接

B4-5 完成后进入 B5：ABI 1.19、生成器 21、旧/新产物拒绝矩阵和缓存重生成。B6 做资源/性能，B7 收口，B2-C 做最终 721、三平台公共产物与教程/registry。

新 Agent 先读：

1. [B4 语言全链路规划](20o-b2-b4-language-integration-plan.md)；
2. [B4-0/B4-1 记录](assets/20o-b2-b/entry-20261110-b4-0-1/README.md)；
3. [B4-2A/B](20o-b2-b4-2a-b-execution-plan.md)；
4. [B4-3 LLVM](20o-b2-b4-3-llvm-plan.md)；
5. [B4-4 双路径/归档](20o-b2-b4-4-dual-path-archive-plan.md)。

## 八、落点

```text
docs/DevDocs/assets/20o-b2-b/entry-<date>-b4-final/   B4-5 收口证据
docs/DevDocs/20o-b2-b4-5-closeout-plan.md             本规划
core/rust/crates/xiao-*/                              B4-1～B4-4 最终产品
stdlib/cmath.xiao                                     15 个包装
```

本页只做 B4-5；B5～B7 与 B2-C 仍后置。
