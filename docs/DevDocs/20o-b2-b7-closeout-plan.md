# 20O-B2-B7. B2-B 内部收口与 B2-C 交接规划

> **B4-0/B4-1 已完成 Windows 本机内部验收；B4-2A 与 FIX1 已完成 Windows 本机退出；B4-2B已完成Windows内部验收；B4-3～B4-5、B5、B6 尚未完成；B7 与 B2-C 尚未开始。**
> B7 开工前须有 B4-5 内部语言收口、B5 内部 1.19/21 版本矩阵及 B6 资源/性能证据；当前尚不具备这些前置。
> B7 只做 B2-B 内部收口、B1 回归、B2-A 与 B2-B 前序阶段证据对账和 B2-C 交接。
> 不运行旧 721、不升级 registry/教程、不关闭历史债务；B2-C 才做最终三平台、721 和公共发布。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| B4 | B4-0/B4-1 已验收；cmath 内部全链路、双路径/归档收口待完成 |
| B5 | ABI 1.19（生成器21沿用）、旧/新拒绝、缓存（待完成） |
| B6 | 资源/错误/性能/三平台证据（待完成） |
| 版本 | 开工入口须为 ABI 1.19/生成器21、接口1、intrinsic 1～48；当前仓库为1.18/21（生成器21已由B4-3生效），B5只切ABI1.19 |
| 本批目标 | B1 20入口回归、A0～A7与B0～B6证据对账、语言/教程/registry边界、B2-C交接 |
| 未做 | B2-C 721、最终三平台公共产物、registry/教程 verified、历史债务关闭 |

B7 是 **B2-B 内部 1.19/21 收口**，不是 B2 最终发布。B2-C 才在最终新快照重跑 721、收齐三平台公共产物并升级教程/registry。

收口须分别记录证据审计、功能/版本验收、资源/性能及平台覆盖结论。B6 的 `unverifiable` 可作为受限交接中的未验证项保留，但不等于 `perf_pass` 或三平台通过；实测失败不得改记为不可验证或仅凭“已登记”签收通过。

### 1.1 要做

1. 在 B7 固定的最终 1.19/21 源码与 Runtime 快照上重新生成并执行 B1 七库 20 入口、资源 17、双精确入口和 28/28 守卫；
2. 对账 B4-1～B4-5、B5、B6 的代码提交、证据、平台和版本；
3. 回归 A1 完整示例、A2～A5 算术/转换/语言承载及 B2-A 既有资源边界；float 幂等已确认变化按 F-2/F-9 与前序差异清单对账，不允许未登记变化；
4. 语言参考只说明 B2-B 内部范围；`stdlib.cmath` registry条目、cmath教程及 B2-B 新增说明/示例的最终发布仍 planned；既有 verified 条目和页面原有范围不降级也不自动扩大；
5. 记录 Windows/Linux/macOS 实际执行或不可验证项、CI run/job/attempt；
6. 记录旧 721 未运行、历史债务独立；
7. 保存 B2-C 交接入口和最终快照；
8. 形成 B2-C 入口清单。

### 1.2 明确不做

- 不运行或改写旧 721 条 F10；B2-C 才在新快照完整重跑；
- 不升级 `stdlib.cmath`/registry/教程为 verified；
- 不修改 intrinsic 1～48/ID100墓碑，不新增49+或C符号，不改变 ABI1.19/生成器21/标准库接口1；
- 不修改 B1 产品/错误表/预算/工作流阈值；
- 不关闭 22C/22D、19/11/22 或 B1 历史债务；
- 不把 B2-B 内部 1.19/21 写成 B2-C 最终发布或三平台公共产品。

B7 只更新收口文档和本批证据，回归复用前序测试。若发现产品或测试缺口，保留失败并退回所属阶段修复、固定新基线和补验，再继续对账；不能一边更改产品一边沿用 B6 旧测量签收新快照。

## 二、开工闸门

1. B4-5、B5、B6 审计通过，最终源码/证据/平台身份固定；
2. 15 项 cmath、30 条导入、双路径/归档/序列化可复现；
3. ABI 1.19/生成器 21、旧/新拒绝、缓存证据固定；
4. B6 证据清单与逐项结果可复核，功能/资源/性能/平台结论分别登记；不可验证项未被汇总为通过，实测失败有处理或裁定依据；
5. B1 七库源码/错误表/预算/调度释放断言未变；
6. `stdlib.cmath` registry条目与cmath教程仍 planned；B2-B 新增说明不提前 verified，B1七库及 `rust.xiao-error-code` 的既有 verified 身份保留；旧721未跑；
7. 唯一任务根、独立 target/Runtime/缓存已建立。

## 三、分批执行

### B7-1：B1 功能回归

1. 固定最终 B7 产品提交与平台 Runtime 内容，从 B1 源码在1.19/21下重生成回归产物，执行七库20入口功能矩阵；
2. 覆盖 std/stdlib 双路径、导出/移除、重新生成的真实 `.xiaoc`/`.xar`、普通包隔离和教程一致性；历史1.18原件只作为B5拒绝矩阵的输入；
3. 执行资源17项；VM/原生精确入口分别要求退出0、恰好1 passed/0 failed/0 ignored，原生需保留七库O0/O3实际标记与黄金验证。最终工作区重跑28/28守卫，报告绑定工作流/脚本摘要和产品提交；守卫只验证计数判断，不代替产品测试；
4. 保持 B1 产品源码/错误表/预算/调度释放断言不变；
5. 旧 721 条 F10 只作历史，不启用 `b1_f10`；
6. 保存命令、PID、独占输出路径、两个Cargo.lock、源码/stdlib加载根/工具链/目标、Runtime与实际执行文件摘要，复核执行前后源码不变；
7. 失败样本保留原归属；22C/22D/19/11/22 不因全绿关闭。

退出：B1 20 入口回归通过；没有把旧 721 写成 B7 成绩。

精确入口复用既有测试，使用Rust1.96.0和受控 `--locked` 构建；原生入口须先配置与本次源码对应的release Runtime和平台LLVM/链接器，不能用旧Runtime或忽略结果代替执行：

```powershell
cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_probe -- b1_capabilities_have_vm_golden_outputs --exact --nocapture
cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_probe -- --ignored b1_capabilities_match_native_golden_outputs --exact --nocapture
```

### B7-2：A0～A7 与 B0～B6 证据对账

1. 按完整源码提交、证据清单SHA-256、实际平台/目标、版本、Runtime与工具摘要逐阶段对账 B2-A的A0～A7和B2-B的B0～B6；
2. 在B7最终快照回归A1完整示例、A2～A5算术/转换/语言及归档/并发承载、既有资源与错误边界；历史A6/A7成绩不代替本次功能回归；
3. 区分B1七库历史证据、B2-B的B0/B1私有底座、B2显式除法33、B3正式参考/共享pow/C ABI与34～48声明/源码包装、B4语言、B5版本、B6性能的适用范围；
4. 复核旧/新产物拒绝、缓存重生成、B6的F-9正式参考、固定负载应测/实测集合、五批样本、时间/内存/翻倍比及平台覆盖；阶段提交可不同，但须按受保护产品树、Runtime及工具/负载内容与最终产品对账。不对应时回所属阶段补验，不凭相同1.19/21版本号继承结论；
5. A0原始 `result.json` 的失败保留，有效结论引用其 `closeout.json`，不得把原始失败写成通过；A7按历史源码身份读取。CRLF原始摘要与UTF-8/LF规范摘要分开校验，不重写历史样本；
6. B7只读审计以固定Git提交/tree/blob核对产品与锁，不要求当前HEAD与历史产品相同，也不改写 `evidence.json`/`closeout.json`；清单由独立生成器按实际字节生成，包含 `.gitattributes`，审计输出与收口回执逐项同义；
7. 在临时副本验证改清单、删已登记文件、伪造阶段身份/状态及子集冒充全量的自反例；即使重建清单，语义错误也须按预期原因拒绝；核对实际退出码、拒绝文本及审计前后字节不变；
8. 保存阶段对账表、摘要、逐项通过/失败/不可验证及后续归属，不把不同阶段/平台证据拼成B2-C或721成绩。

退出：前序证据链与B7最终产品可逐阶段对账；取证完整性、功能验收及性能/平台结论分开，不跨阶段覆盖或冒充。

### B7-3：语言说明、教程与 registry 边界

1. 语言参考只提升到 B2-B 内部 1.19/21 范围；
2. `stdlib.cmath` registry条目和cmath教程保持 planned；`lfloat_divide` 等B2-B新增说明/示例仍标最终发布planned；保留B1七库、error_code及页面原有verified范围；
3. 不新增未交付模块或提前 verified；
4. 页面既有 verified 范围不自动扩展到 B2-B 新增段落；
5. B2-C 再按最终721/三平台证据升级对应新增范围；不把整份registry概称planned或重新升级既有verified条目；
6. 保存状态核对与文档门禁。

退出：语言/教程/registry 状态与内部证据一致；cmath 仍 planned。

### B7-4：CI、三平台与门禁

1. 记录 B5/B6/B7 实际 CI run/job/attempt；
2. push 路径过滤未触发按事实写明；
3. 记录 Windows/Linux/macOS 实际执行或不可验证；
4. 按实际平台范围保存性能/资源/拒绝矩阵；B6不可验证项不能因B7审计或CI整体绿色改成通过，失败不得改写为不可验证；
5. 使用Rust1.96.0和受控Cargo环境运行完整workspace gates：`cargo +1.96.0 test --manifest-path core/rust/Cargo.toml --workspace --locked`、`cargo +1.96.0 clippy --manifest-path core/rust/Cargo.toml --workspace --all-targets --locked -- -D warnings`、`cargo +1.96.0 fmt --manifest-path core/rust/Cargo.toml --all -- --check`、`bun run check:lock`、`bun tools/repo-check/src/cli.ts all`；最大负载仍由独立release测量入口处理；
6. 不修改工作流阈值/超时或删除失败样本。

退出：CI/平台/门禁证据完整；未执行项明确。

### B7-5：B2-C 交接

1. 固定 B2-B 最终 1.19/21 源码/证据/平台边界；
2. 列出 B2-C 必须执行的完整 721、三平台公共产物、最终教程/registry；
3. 列出未验证平台/性能的具体负载与补验要求、历史债务独立身份，不能只写“无三平台声明”就跳过缺项；
4. 复核旧 721 未跑、registry 未升级、历史债务未关闭；
5. 形成 B2-C 入口清单。

退出：B2-C入口明确且未开始；B2-B内部收口按已验证范围记录。若带不可验证项交接，须注明受限范围与必补项，不声明相应性能或三平台通过；实测失败须解决或取得对应裁定后才能签收验收结论。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| B1功能 | 七库20入口、双路径、导出/移除/归档/隔离/教程 |
| B1资源/精确 | 新快照17项、VM/原生精确各1 passed/0 failed/0 ignored；原生七库O0/O3；28/28计数守卫单列 |
| B4～B6 | 语言/双路径/归档、1.19/21拒绝/缓存、F-9正式参考、固定负载覆盖/五批性能/内存/翻倍比 |
| B2-A回归 | A1完整示例、A2～A5算术/转换/语言与归档/并发承载、既有资源边界；A0/A6/A7历史身份单列 |
| 语言/教程 | 内部1.19/21范围；stdlib.cmath条目及cmath教程planned，新增说明不提前verified；既有verified保留 |
| 平台/CI | Windows/Linux/macOS、run/job/attempt、不可验证项 |
| 收口审计 | 固定Git对象只读审计；清单/文件缺失/阶段身份/状态/子集冒进自反例；回执与审计输出一致 |
| 工程门禁 | Rust1.96.0 workspace测试、Clippy、fmt、锁与docs/UseDocs/layout门禁 |
| 历史 | 721未跑、22C/22D、19/11/22、B1旧证据独立 |

## 五、退出条件

1. B1源码在最终1.19/21快照重生成后的七库20入口/资源17/双精确/28守卫通过，完整workspace gates通过；
2. B2-A的A0～A7与B2-B的B0～B6证据链/版本/平台可对账，固定Git对象只读审计和自反例通过；
3. A1示例与A2～A5算术/转换/语言承载及既有资源回归通过；A0有效closeout和原始失败、A7历史身份保留；
4. 语言说明只认领已验证的B2-B内部范围；stdlib.cmath条目、cmath教程及新增说明的最终发布仍planned，既有verified不扩大或降级；
5. CI/三平台证据按项登记通过/失败/不可验证，不把审计通过当性能通过；不可验证项限定交接范围并列出补验清单，实测失败不靠登记签收；
6. 旧721未运行、历史债务未关闭；
7. B2-C未开始；B2-B内部1.19/21收口结论与实际功能、性能及平台覆盖范围一致；
8. B2-C最终721/三平台/教程/registry未完成。

## 六、风险与禁止

- 用B4～B6证据冒充B2-C 721/发布；
- 旧721被运行或写成B7成绩；
- 提前升级registry/教程；
- 用不对应最终产品身份的阶段记录拼接通过，或把某平台成绩当成另一平台成绩；
- 为B1回归修改产品/预算/断言/工作流；
- 关闭22C/22D、19/11/22或B1历史债务；
- 把B2-B内部1.19/21写成最终公共产品。

## 七、交接

B7完成后进入[B2-C 最终公共收口规划](20o-b2-c-final-closeout-plan.md)：最终1.19/21快照完整721、三平台公共产物、最终教程/registry与发布收口。B2-C未完成前，B2-B仍只是内部收口。

新 Agent 先读：

1. [B2-B实施规划](20o-b2-b-implementation-plan.md)；
2. [B6资源/性能](20o-b2-b6-resource-performance-plan.md)；
3. [B5版本矩阵](20o-b2-b5-version-matrix-plan.md)；
4. [B4-5收口](20o-b2-b4-5-closeout-plan.md)；
5. [B1收口](20o-b1-closeout.md)。

## 八、落点

```text
docs/DevDocs/20o-b2-b7-closeout-plan.md              本规划
docs/DevDocs/assets/20o-b2-b/entry-<date>-b7/        B7证据
tests/fixtures/                                       只读复用已交付的B1/cmath/归档语料
docs/UseDocs/language/                                语言状态边界
docs/module-registry.json                             仅核对stdlib.cmath=planned与既有条目状态
```

本页只做B7；B2-C仍后置。
