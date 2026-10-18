# 18B-收口. `build -xar` 与 18B 验收对账

> **背景**：[18B](18b-backend-calls-and-workflow.md) 的首批已接通 `verify`/`cache`、
> 字节码 O1–O3、`run_xiaoc`、缓存展示与四入口回归；但 **`xiao build -xar` 还没有接线**：
> CLI 的 `executeBuild` 直接调 `buildSource`，`command.xar` 被忽略。
> 同时 20E 的索引字段需要一个**生产者**：`stdlib_modules` 与 `required_capabilities`
> 必须从入口可达闭包推导出来。
>
> **一句话概括本批**：**让 `build -xar` 真的产出可运行的归档，并把 18B 的验收清单逐条关掉。**

状态：**已完成并验收通过（2026-10-18）**。四道裁定全部取甲；O-1、归档协议、精确闭包、缓存校验与本地模块运行均已接通，18B 的 13 条验收已逐条核销。

## 一、为什么是这一批

1. **18B 的索引仍写「进行中」**，而它列的剩余项里有几项在 §十 已实际落地——
   需要一次**逐条对账**，不能只看索引状态；
2. **`build -xar` 是 20E 的既定边界**：[20E](20e-archive-wiring.md) §12 已交付
   索引字段与启动检查，但实际字段的生产者（18B）还没有；
3. **O-1 已挂为下一批前置**：`validate_requirements` 把请求掩码的未知位也报成归档错误，
   本批必须收掉；
4. **规范 `:57` 明确入口**：`xiao build -xar "<源路径>"`，不是新命令。

## 二、冻结已定死的（**输入，不许重新论证**）

| 项 | 冻结内容 | 出处 |
| --- | --- | --- |
| CLI 入口 | `xiao build -xar "<源路径>"`；不新增 `pack` | [18 规范](18-optimization-cli.md) `:57` |
| 后端边界 | TS **不重写** Pass/哈希/Protobuf/ZIP；新能力加在 Rust 并暴露协议 | [18B](18b-backend-calls-and-workflow.md) §2.2 |
| 归档校验 | 缓存命中只免“重新生成”，**不免“重新验证”**；不允许因命中跳过完整 SHA-256 | 同上 §2.6 |
| 输出前验证 | 写出/交给运行器前必须经既有校验器；不得自动运行未验证文件 | 同上 §2.7 |
| 归档边界 | 不携带官方 Runtime/标准库；只带第三方/项目模块与**显式声明的资源** | [17](17-xar-archive.md) §一 |
| 20E 字段 | `stdlib_interface_version`、`language_version`、`stdlib_modules`、`required_capabilities`；版本精确相等；缺位 026、未知位 011 | [20E](20e-archive-wiring.md) §三 |
| 能力推导 | 构建侧从 **intrinsic 契约**推导所需能力；不引入手写配置 | 同上 §3.3 |
| O-1 | 请求掩码与归档掩码分开校验；请求错误不得报成“归档要求无效” | [20E](20e-archive-wiring.md) §13.2 |

## 三、实施前核出的现状

### 3.1 `build -xar` 目前不存在

- `parser.ts:35` 的 `build` 命令带 `xar: boolean`，但 `executeBuild`
  （`cli/ts/src/commands/index.ts`）**没有读取它**，始终调用 `client.buildSource`；
- `protocol/build.rs` 只有 native build 路径，没有归档请求/响应；
- `build.rs` 的产物是原生可执行文件，不会调用 [17B](17b-resources-and-index-integration.md)
  的 `XarBuilder`。

### 3.2 前端能提供闭包，但没有消费点

- 前端已把入口与本地模块编译成类型化 IR；intrinsic 调用在 IR 里有稳定 ID；
- 标准库模块由加载器编译（`stdlib/os.xiao` 等），它们的 intrinsic 调用也可达；
- 但 `XarBuilder::with_requirements`（`xiao-xar/src/lib.rs`）目前**没有生产者**——
  20E 的测试是直接构造的。

### 3.3 缓存与验证已经可用

- 源码运行的 `.xiaoc` 已写全局内容寻址缓存（`xiao-driver/src/run.rs:818` 的
  `ArtifactStore::open(...).put_xiaoc(...)`），不是写在源码旁边；
- `read_checked` / `decode_xiaoc` / `validate_xar` 都是既有验证入口；
- `verify`/`cache` 命令已存在，`build -xar` 只应复用它们，不新写检查。

### 3.4 18B 的验收需要逐条对账

索引行的「进行中」列了 LLVM Pass、全局 `.xiaoc` 缓存、四入口统一运行契约和完整缓存展示；
但 §十 已记录其中多项落地（字节码 O1–O3、`run_xiaoc`、四入口回归、缓存展示、`verify --detail`）。
**不能凭索引状态判断**——本批要按 §八 的 13 条逐项核销，缺的才补。

### 3.5 O-1 的位置

`xiao-xar/src/runner.rs:394` 把 `index.required_capabilities` 与
`options.capabilities` 的未知位合并映射为 `X17-XAR-011`；
协议层虽在 `protocol/run.rs:106` 单独报请求错误，但库层调用会得到误导性的归档错误。

## 四、本批冻结的契约（四道裁定之后）

### 4.1 `build -xar` 的行为

```text
xiao build -xar <config.xiao | 含 [main] 的 .xiao | 任意 .xiao> [-O0..-O3] [-o <path>] [-debug]
```

- **默认输出**：输入同目录的 `<输入文件 stem>.xar`（`main.xiao` → `main.xar`，
  `config.xiao` → `config.xar`）；`-o` 覆盖；
- **输入形态**：复用 18A 的输入路由；`config.xiao` 走项目入口与依赖锁，
  单个 `.xiao` 走脚本顶层入口；
- **项目入口**（星崽补充裁定，2026-10-18）：静态查找项目中唯一带 `[main]` 的 `.xiao`；缺失或多个时拒绝，不执行源码猜测入口；
- **产物**：项目本地模块编译/缓存为 `.xiaoc`，加上显式声明的资源；
  **不复制**官方 Runtime/标准库；
- **第三方依赖**：首版**不支持**；遇到时给**稳定诊断**并拒绝，
  **不得**静默省略后产出一个缺模块的归档；
- **输出前**：组装完成后必须用既有 `validate_xar`/`decode_xar` 完整验证，
  通过后才原子写出。

### 4.2 可达闭包（精确）

- **标准库模块**：从入口出发，沿模块导入与**实际函数调用**，收集可达的
  `stdlib.*` 模块；只记要求，不复制对象；
- **能力位**：对同一闭包内所有可达 intrinsic，取
  `IntrinsicDecl.capability` 的并集；**不按模块整体放大**；
- **判据用例**：只调用 `stdlib.os.open_file` 的程序**不得**要求 `process`；
  调用 `stdlib.os.spawn` 的程序必须要求 `process`（以及 `filesystem`，若同时使用）；
- 闭包为空时，`stdlib_modules`/`required_capabilities` 分别为空/0，
  这正是 20E 允许的“没有官方依赖”的归档。

### 4.3 缓存与验证

- `.xiaoc` 对象从全局缓存读取时必须 `read_checked`（每次重算摘要）；
- 缓存命中只跳过“重新生成”，不跳过“重新验证”；
- 构造“索引说命中但对象已损坏”的测试，断言 `build -xar` **拒绝**。

### 4.4 O-1

- `index.required_capabilities` 的未知位 → `X17-XAR-011`；
- `options.capabilities` 的未知位 → **请求/校验错误**（`XarRunError::Validation`
  或等价的稳定请求错误），不再冒充归档要求无效；
- 协议层现有的请求错误检查保持。

## 五、本批要做的事（顺序有依赖）

1. **O-1**（依赖：无）
   - 拆分 `validate_requirements` 的两个掩码校验；
   - 补库层直调用例：归档合法但请求掩码未知位 → 请求错误，不是 011；
2. **精确闭包提取**（依赖：无）
   - 在驱动器/前端提供入口可达的本地模块、`stdlib.*` 模块与 intrinsic ID；
   - 沿函数调用与 intrinsic 调用遍历；标准库模块只解析/编译，不把对象放进归档；
   - 输出给 `XarBuilder::with_requirements`；
3. **`build -xar` 协议**（依赖 2）
   - 新增 `build_archive` 请求/响应；Rust 侧完成编译、缓存读取、资源收集、归档组装；
   - CLI 的 `executeBuild` 读取 `command.xar` 并路由；TS 只搬运结果；
   - 默认输出 `<stem>.xar`，`-o` 覆盖；
4. **归档组装**（依赖 2、3）
   - 17B 的 `XarBuilder` + 资源收集（`config.xiao` 的显式声明，不扫描目录）；
   - `-debug` 时按 17B 的独立调试/源码对象选项；默认不带完整符号/源码正文；
   - 20E 的四个字段由闭包填入；归档索引的 `entry` 指向入口 `.xiaoc`；
5. **输出前验证与缓存**（依赖 3、4）
   - 组装后 `validate_xar`/`decode_xar`；通过后原子写出；
   - 缓存对象 `read_checked`；损坏命中场景拒绝；
6. **第三方依赖边界**（依赖 3）
   - 检测到第三方包依赖时给稳定诊断并拒绝（码族选择见 §六.3）；
   - 不静默省略、不产出缺模块归档；
7. **18B 对账**（依赖 1–6）
   - 按 18B §八 13 条逐项核销；索引里剩余四项逐条判定：
     LLVM Pass（**报告** clang `-O` 与 `xiao_passes_registered=false`，
     实现不在 18B）、全局 `.xiaoc` 缓存、四入口、缓存展示；
   - 更新 18B 实施记录与阶段索引状态；
8. **文档与登记**（依赖全部）
   - 用户文档补 `build -xar` 的输入/输出/第三方边界；
   - `docs/module-registry.json` 更新 18B 条目；
   - 18B 的验收对账表写入本文 §八。

## 六、不许

1. **不许**在 TS 重写 Pass/哈希/Protobuf/ZIP/`.xiaoc` 校验；
2. **不许**因缓存命中跳过完整 SHA-256 校验；
3. **不许**给 `build -xar` 的第三方依赖做“静默省略”；遇到就拒绝并给稳定诊断
   （优先复用既有码；没有合适码时新登记，**不许**用运行期 `X17-XAR-008` 冒充构建期限制）；
4. **不许**把官方 Runtime/标准库复制进归档；
5. **不许**按模块整体放大能力闭包（必须沿实际调用）；
6. **不许**把 `build -xar` 的默认产物写到源码目录以外的地方（如隐式 `dist/`）——
   默认是输入同目录 `<stem>.xar`；
7. **不许**在未验证的情况下写出归档或自动运行它；
8. **不许**只改索引行就声称 18B 的 13 条验收完成——必须有逐条证据。

## 七、四道裁决（已落定，均甲）

| # | 问题 | 裁定 |
| --- | --- | --- |
| 1 | 可达闭包的精度 | **甲：调用图精确闭包**，标准库模块与 intrinsic 都按实际可达收集 |
| 2 | `build -xar` 默认输出 | **甲：输入同目录 `<stem>.xar`**，`-o` 覆盖 |
| 3 | 本批收口范围 | **甲：O-1 + 18B 13 条对账**；O-2/O-3 不塞进同一批 |
| 4 | 首版依赖范围 | **甲：本地多模块 + 显式资源 + `stdlib.*` 要求**；第三方依赖拒绝并留待后续 |

**来源**：星崽直接裁定（AskUserQuestion，2026-10-18）；四条均取甲。

## 八、18B 验收对账（逐条证据）

| # | 18B §八 条目 | 判定 | 证据 |
| --- | --- | --- | --- |
| 1 | `verify` 可用 | 核销 | `tests/release_report.rs::verify_detail_reports_rust_computed_integrity_and_reproducibility_fields` / `verify_detail_reports_archive_members_and_object_digests` 分别验证 `.xiaoc`、`.xar`，不执行程序 |
| 2 | `cache` 可用（两阶段 clean） | 核销 | `protocol_tests.rs::cache_list_returns_structured_result`、`cache_clean_protocol_*`；`xiao-artifacts/src/tests.rs::cleanup_plan_protects_project_lock_references_and_apply_is_explicit` 验证 plan/apply 分离 |
| 3 | TS 不重写后端 | 核销 | `commands/build-archive.ts` 只调用 18A 路由、归一化与 `ProtocolClient.buildArchive`；编译/优化、SHA-256、索引、ZIP、校验集中在 Rust `archive.rs`，TS 无这些实现 |
| 4 | 缓存展示四类字段 | 核销 | 既有 `run.rs::CacheObservation` 及协议镜像保留 `status`、`recompiled/recompile_reason`、`verified`、`reason`、Pass 的 `skip_reason`；归档结果追加同形状逐模块证据；CLI 命令测试覆盖 JSON 结果 |
| 5 | `.xiaoc` 默认进全局缓存、源码目录不出现 | 核销 | `18b_build_archive.rs::script_builds_runs_and_keeps_xiaoc_in_global_cache` 检查 O0–O3 缓存对象和源码目录；`served_protocol_builds_runs_stdlib_and_keeps_repl_out_of_global_cache` 实际进程验证源码运行副作用 |
| 6 | REPL 片段不进全局缓存 | 核销 | 同一实际进程用例发送 `source.path=None`，断言 `status=session` 且全局 `.xiaoc` 数量不增加；独立 `XIAO_HOME` 不触及用户缓存 |
| 7 | 归档不跳过校验 | 核销 | `indexed_corrupt_hit_is_rejected_without_replacing_output` 先形成真实索引命中，再破坏对象，断言拒绝；生产读取始终经 `read_checked` 与 `decode_xiaoc` |
| 8 | 输出前验证 | 核销 | `archive.rs` 在原子提交前调用 `validate_xar`、`decode_xar`；缺失资源等负例不产生归档，损坏命中保留既有完整输出；运行器先完整验证后创建 VM |
| 9 | 四入口一致 | 核销 | `d19a_differential.rs::all_vm_sides_agree_on_every_case` 为非 ignored 测试，实际比较源码 O0/O1、REPL、未优化/优化 `.xiaoc`、`.xar` 的输出、错误、退出码、释放事件；本轮 workspace 已实际执行 |
| 10 | 承接 18A 两处 | 核销 | `commands/index.test.ts` 的 `run -O2` 探针检查配置/锁内容和 mtime；18A §实施记录说明 `build -O1` 既有测试变更；本批再加真实锁文件只读构建回归 |
| 11 | O1–O3 可运行；LLVM 侧明确报告 | 核销 | 本批 O0–O3 归档均实际运行；`protocol_run_accepts_nonzero_optimization_level` 及既有逐 Pass 报告；原生 `protocol/build.rs` 报告 clang `-O`、`xiao_passes_registered=false`，不把 Xiao LLVM Pass 实现纳入本批 |
| 12 | 不回归 | 核销 | 18A 参数缺省、优先级和五类路由与全工作区测试均通过；最终命令结果见 §十二 |
| 13 | 门禁全绿 | 核销 | fmt、clippy、Bun、类型检查、完整 `check` 均退出 0，最终结果见 §十二 |

## 九、落点

```text
docs/DevDocs/18b-build-xar-and-closeout.md             本文
cli/ts/src/commands/{index.ts,parser.ts}                build -xar 路由与默认输出
core/rust/crates/xiao-driver/src/protocol/{build.rs,request.rs,run.rs}
                                                        build_archive 请求/响应
core/rust/crates/xiao-driver/src/frontend.rs            可达闭包提取
core/rust/crates/xiao-xar/src/{lib.rs,runner.rs}         O-1 拆分、组装入口
core/rust/crates/xiao-xar/tests 或 driver 测试          build -xar 端到端/损坏缓存/闭包精度
docs/UseDocs/tooling/cli/build.md（若存在）              build -xar 用户文档
docs/module-registry.json                               18B 条目
docs/DevDocs/README.md                                  18B 状态与本文一行
```

## 十、验收

1. **`build -xar` 可用**：单文件、`config.xiao` 与多模块项目都能产出归档；
   默认 `<stem>.xar`，`-o` 覆盖；`xiao -xar` 能运行，输出与源码一致；
2. **字段正确**：归档索引的 `stdlib_modules`/`required_capabilities` 与精确闭包一致；
   只读文件的程序不要求 `process`；spawn 的程序要求 `process`；
3. **校验**：缓存命中仍重算 SHA-256；损坏命中被拒绝；归档写出前完整验证；
4. **边界**：不复制官方模块；第三方依赖被拒绝且有稳定诊断；`-debug` 只按选项带调试对象；
5. **O-1**：请求掩码未知位不再报 011；
6. **18B 对账**：13 条逐条有证据，索引状态更新；
7. **通用**：`cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、
   `bunx tsc --noEmit` 全绿；提交标题带类型前缀、正文非空。

## 十一、裁定结果（已执行）

见 §七 的四行表格；四条均取甲，来源为星崽直接裁定（AskUserQuestion，2026-10-18）。

## 十二、实施与验证记录（2026-10-18）

- **O-1**：`d3ebb0a` 拆分归档要求与请求能力校验，库层新增
  `unknown_request_capabilities_are_validation_errors`；请求未知位返回 `Validation/009`，
  归档未知位仍为 `InvalidRequirements/011`。
- **入口补充裁定**：星崽明确选择「静态查找项目中唯一带 `[main]` 的 `.xiao`，缺失或多个时拒绝」。
  构建复用项目发现和语法解析的入口模式，不引入配置入口字段，不执行源码猜测。
- **Rust 编排**：驱动器 `archive_closure.rs` 沿可达函数、模块初始化、函数值复制、参数、
  返回值及本地重导出计算闭包；`archive.rs` 消费统一前端，官方导入保留外部依赖，
  只将本地模块加入 `XarBuilder`。能力并集完全来自 `IntrinsicDecl.capability`。
  字节码 `lower/imports.rs` 补齐函数读取稳定顶层导入的路径，保留局部遮蔽和模块级覆盖边界，
  复用既有导入/成员指令及 VM 单次初始化缓存；实际进程测试比较源码和归档输出一致。
  实现提交为 `5b16b1f`。
- **缓存与输出**：请求键包含 IR、源码摘要、13A 优化指纹、锁摘要和调试/工具链身份；
  全局索引在同一跨进程锁内合并。命中仍重新 SHA-256 并解码，所有成员验证后原子写出。
  锁文件经 `LockFile::from_json` 完整校验，根项目条目不会被误当成第三方包。
- **协议与 CLI**：新增协商能力 `build_archive`；TS 只路由及搬运结果，不发现 LLVM。
  默认输出为输入同目录 `<stem>.xar`，`-o` 覆盖；拒绝重复 `-xar` 和 `--emit-llvm`。
  第三方包限制复用 `X11-PROTOCOL-006`，构建阶段为 `archive_build`，不复用运行期 008。
- **运行与调试**：VM 新增已验证模块执行视图；归档内本地对象只从索引加载，
  官方库回调驱动器加载器。普通归档只含紧凑映射；`-debug` 追加独立调试符号及激活位，
  不自动携带源码对象。无法静态解析的动态调用明确拒绝，不漏报能力或打包不完整程序。
- **定向验证**：`18b_build_archive.rs` 当前 12 项通过，覆盖真实核心进程构建/运行、
  官方库、本地模块搬离源码后运行、O0–O3、显式资源、缓存破坏、入口歧义、第三方限制、
  调试对象、锁只读及 REPL 隔离；CLI 路由和默认/显式输出回归通过。
- **完整验证**：自审后全工作区测试 **1236 项通过、41 项环境门控 ignored、0 失败**，
  四入口差分、O-1 和实际协议进程归档回归均已实际执行；最终 Bun **298 项通过、5 项环境门控跳过、0 失败**。
  首轮 Bun 的 AST 适配器在并行 Rust 构建期间触发 60 秒超时，编译结束后单独重跑全部通过。
  最终 `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、
  `bun run check`（含覆盖率、clippy、主工作区及 benchmarks 锁文件检查）、
  `bunx tsc --noEmit` 均退出 0。未执行的 41 项 Rust 与 5 项 Bun 环境门控用例不计为通过，
  不据此声称已完成真实原生工具链或跨平台窗口验收。

## 十三、自审修复（2026-10-19）

### 13.1 可复现问题与修复

| 优先级 | 触发条件与原行为 | 修复 |
| --- | --- | --- |
| P1 | 已知文件函数与数组中取出的进程函数在分支合流，未解析目标被空集合吞掉；真实构建成功但 `required_capabilities=1`，漏报 `process` | 调用目标区分未知值与确定的非函数值；未知值经复制、成员、参数、返回值及合流继续传播；动态调用只要仍含未知目标就拒绝写出，不以部分已知结果推导完整要求 |
| P1 | `config.xiao` 输入配合 `-o app.xiao` 可用 ZIP 覆盖真正的入口；依赖源码、声明资源及缓存索引也缺少输出保护 | 新增 `archive_output.rs`，在创建缓存前和原子写出前检查归档输出；保护入口、所有已解析模块、配置、锁和资源来源文件，并禁止写入受管理缓存；路径使用标准绝对化并解析已有链接及父目录段 |

### 13.2 回归证据

- `review_mixed_known_and_unresolved_targets_cannot_omit_capabilities`：修复前真实复现漏报；修复后未知调用明确拒绝且无归档输出。
- `review_output_cannot_replace_any_archive_input`：入口、依赖、未调用但已解析模块、配置和资源的内容与 mtime 均不变。
- `review_archive_output_cannot_corrupt_its_cache`：缓存索引及包含 `..` 的等价路径均拒绝，缓存内容与 mtime 不变，也不创建缓存内额外归档。
- 真实锁文件回归追加输出覆盖锁的负例；`18b_build_archive.rs` 当前 12 项定向回归通过，驱动器 clippy 通过。
- 修复提交为 `83c77cd`；全工作区 1236 项通过，Bun 298 项通过，`bun run check`、fmt、clippy、类型检查和文档门禁均通过。

## 相关页面

- [18B. 后端调用与可诊断工作流](18b-backend-calls-and-workflow.md)——**本批的直接前置**：§2.6/§2.7/§2.8 与 §八 验收
- [18. 优化与产物 CLI 接入](18-optimization-cli.md)——`build -xar` 的规范入口与产物生命周期
- [17B. 资源与索引接入](17b-resources-and-index-integration.md)/[17C. 入口与运行器](17c-entry-and-runner.md)——归档组装与运行
- [20E. 归档接线](20e-archive-wiring.md)——字段、启动检查与 O-1
- [16A](16a-content-addressed-objects-and-indexes.md)/[16B](16b-cache-maintenance.md)——缓存与 `read_checked`
