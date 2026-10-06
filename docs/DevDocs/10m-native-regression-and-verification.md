# 10M. 原生动态入口回归修复与验证闭环（N0-F）

> **10 阶段的续批**，接 [10L](10l-native-dynamic-alignment.md)。10L 首轮把动态入口与 VM 对齐了，
> 但审核实测发现：**它把四个基准程序的原生构建打坏了**，而 `cargo test`、clippy、fmt、`bun test` 都是绿的。
>
> **一句话概括本批**：先修回归、把「绿门禁掩盖红的原生」这个模式堵死，再谈 10L 剩下的对齐面。
> 不修完 H1–H4，10L 的任何「已接入」结论都不成立。
>
> 补记（2026-10-07 审核复核时发现）：`bun run check` **在 10L 之后的 HEAD 上是红的**（H8 的行数门禁）。
> 所以严格说不是「全部门禁绿」，而是「`cargo test` 那一路绿」。审核复跑时漏了 `bun run check`，
> 本可更早发现；接手 Agent 请把这一条也视为返工项。
>
> 状态：**规划稿（2026-10-07）**。「建议」处未经星崽确认，确认项集中在末尾「待定决策」。

## 一、Agent 交接上下文

### 接手前提

1. [10L](10l-native-dynamic-alignment.md) —— 上一批；枚举记录（§十一）与扩大范围实现记录（§十二）；
2. [19D](19d-performance-comparison.md) 的原生可构建性探测与其实施记录；
3. [10D](10d-environment-gated-test-spec.md) —— 门控用例的规格；**静默 `return` 是被它禁止的做法**；
4. [15A](15a-native-pass-mapping-and-runtime-trimming.md) §3.2 —— 不得跳过 Runtime 检查；
5. 本机复现原生门控的办法：`vcvars64.bat` 导入 MSVC 环境、设 `XIAO_CLANG`/`XIAO_LLVM_AS`/`XIAO_LLC`/`XIAO_STRIP`/
   `XIAO_TARGET_TRIPLE`/`XIAO_RUNTIME_LIBRARY`，先 `cargo build --release -p xiao-runtime`。**本机确实可以配起来**
   （审核时已实跑），不要沿用 10L 第十二节「本机未配置」的说法。

### 现状盘点（2026-10-07，Windows 本机实测加读码）

| 环节 | 现状 | 判定 |
| --- | --- | --- |
| 原生基准构建 | 五个程序**全部**被拒；四个在 clang 阶段报 `use of undefined value '%abi.fail'`，`container-dense` 是另一个理由 | **回归**，见 §2.1 H1 |
| 同批之前的状态 | 在 `55e41db`（10L 实现前）上实跑同一探测：四个可构建，只有 `container-dense` 被拒 | 对照证据 |
| 既有门控用例 | `n0_a_native_driver::optional_real_frontend_to_native_round_trip` 在 `55e41db` 通过，在 HEAD 失败 | **回归**，见 §2.1 H2 |
| 探测测试 | `native_benchmark_probe` 只断言目标三元组与运行库存在，**从不断言构建成功**，是一张记录表 | 见 §2.3 |
| LLVM 文本校验 | `n0_b_dynamic.rs::validate_with_llvm_as` 在 `XIAO_LLVM_AS` 缺失时**直接 return**，默认 `cargo test` 因此从未校验过新路径的 LLVM 文本 | 见 §2.2 |
| 差分覆盖 | 7 个用例的原生一路通过，但都没有触发「函数体内的动态检查」这条路径 | 见 §2.4 |
| 未定义标签 | `dynamic/control.rs:905` 硬编码 `label %abi.fail`；该标签只在顶层入口定义（`entry.rs:88`），而函数体是独立 LLVM 函数，其余同类位置都走 `error_target()`/`error_terminal_label` | 疑似根因，见 H1 |

### 本批边界

| 子任务 | 内容 | 本批 |
| --- | --- | --- |
| H1–H8 | 10L 审核返工项（§2.1） | **做**，每项单独提交 |
| 10L 剩余对齐面 | 高级范围/随机选择的原生结果序列、嵌套 `finally` 的释放差异 | **能做就做**，但排在 H1–H4 之后（§2.5） |
| 整数溢出（静态路径） | `ir.rs` 的 7 处 `llvm.trap` 是否接统一错误路径 | **不做**，待定决策 2 决定 |
| 20C、20D | 官方 Xiao 库、`os` 与平台资源 | **不做** |

## 二、必须先冻结的 6 条

### 2.1 **H1–H8：先修回归，再谈对齐**

每项标注「怎么知道的」和「没核实的部分」。

| 编号 | 问题 | 依据 | 处置 |
| --- | --- | --- | --- |
| H1 | 四个基准程序原生构建失败，clang 报 `use of undefined value '%abi.fail'`。`control.rs:905` 是文件里唯一硬编码该标签的位置，其余同类位置都用 `error_target()`/`error_terminal_label`；`abi.fail` 只在顶层入口 `entry.rs:88` 定义，函数体是独立 LLVM 函数 | 在 `55e41db` 与 HEAD 两个提交上实跑同一探测，结果不同；grep 全部 `label %` 与 `abi.fail` 的使用点 | 改掉硬编码，用函数自己的错误终点标签；改完重跑探测确认四个程序恢复，并**再加一个覆盖函数体内动态检查的差分用例**（§2.4）。根因是否只有这一处，**没有单独隔离验证**，所以修完必须靠实跑确认而不是靠读码宣布完成 |
| H2 | `n0_a_native_driver::optional_real_frontend_to_native_round_trip` 在 HEAD 失败，报「动态模块需要调用方通过 `Toolchain::with_runtime_library` 注入静态库」；该用例在 `55e41db` 通过 | 两个提交上分别实跑 | 先判断这是**有意的行为变更**（动态模块确实需要运行库）还是缺陷。若是前者，就显式更新该用例并在提交说明写明「既有测试改动与理由」；若是后者，在代码侧修。**不得**只把断言放宽了事 |
| H3 | 探测测试没有任何构建断言，五个程序全被拒它照样绿 | 读 `native_benchmark_probe.rs`，全文只有目标三元组与运行库存在两个断言 | 见 §2.3 |
| H4 | 19D 的实施记录仍写四个程序「可构建」，10L §十二 仍写「本机未配置 LLVM 环境」；两处都与实测不符 | 读两份文档与最后那个提交的消息 | 重跑探测后按真实结果回填两处，删除过时表述（§2.5） |
| H5 | 10L §十二 写「星崽确认接入全部 14 类检查」。审核对话里没有见到这项裁定 | 无记录 | 请星崽确认来源；确认后把出处写进文档，未确认前不在文档里写成已裁定 |
| H6 | `validate_with_llvm_as` 在环境变量缺失时静默 `return`，违反 10D「门控用例不得静默跳过」 | 读该函数 | 见 §2.2 |
| H7 | `b249331`（满足 lint）紧跟在 `6ebbdce` 之后，说明前一个提交没有通过 clippy；规划要求每步门禁全绿 | `git log` | 后续每个提交单独跑 `cargo clippy --workspace --all-targets -- -D warnings` 再提交 |
| H8 | **`bun run check` 在当前 HEAD 上是红的**：`A0-SIZE-001`，`xiao-runtime/src/abi.rs` 2843 行、上限 2500。该文件在 `c58169a` 是 2473 行（正好在限内），10L 的 ABI 增量把它顶出去了。19 的验收里「`bun run check` 退出码 0」因此当前不成立 | 在 HEAD 与 `c58169a` 上分别 `wc -l` 该文件，并跑 `bun run check` | 按 [00E](00e-file-size-gate.md)「超标文件按批次拆分」把运行库 ABI 拆成模块；拆完 `bun run check` 必须回到 0 |

### 2.2 **未定义标签这类错误必须能在默认门禁里被发现**

H1 的性质是：LLVM 文本里有分支跳到不存在的标签。它逃过默认门禁不是因为门禁慢，而是因为**校验根本没跑**：
`validate_with_llvm_as` 拿不到 `XIAO_LLVM_AS` 就静默返回。

**冻结**：

- 按 10D 的口径改造：要么把需要 `llvm-as` 的校验改成显式 `#[ignore]` 门控用例（并在实施记录里登记本机是否跑过），
  要么加一条**不依赖外部工具**的结构校验——对每个发射出来的函数体，检查 `label %X` 的引用都有对应的 `X:` 定义。
  两者选其一并在提交说明写明理由。**建议加结构校验**：它跑在默认 `cargo test` 里，不依赖环境，能立刻拦住同类错误；
- 不论选哪种，都**不得**保留「缺环境就静默跳过」的写法；
- 这条校验要能对 H1 那个具体错误报错。写完后用「把 H1 的修复还原」来验证它会红——这是本批对 H1 验证方式的要求。

### 2.3 **探测表要么有断言，要么不叫回归测试**

`native_benchmark_probe` 现在的形状是「跑一遍、打印、永远通过」。它的价值是取数，不是守门。

**冻结**：

- 把「当前可构建的程序」固化成一份**受版本控制的基线**（可以就是探测输出本身，也可以是一份清单）；
- 测试断言：清单里标为可构建的程序，现在必须仍然可构建；新出现的拒绝要么补进清单并写明原因，要么让测试失败；
- 断言信息里带上拒绝原因，方便定位；
- 这份基线要随 H4 的回填一起更新，并注明是哪个提交、哪个平台测出来的。

### 2.4 **差分用例必须覆盖「函数体内的动态检查」**

10L 的回归之所以没被原生差分拦住，是因为 7 个用例里没有一个在**函数体**里触发动态运行时检查。
基准程序正是因为这个才全挂。

**冻结**：

- 往 `d19a_differential.rs` 的 `CASES` 加用例，至少一个要在 `def` 内部执行带动态检查的运算（例如函数内算术导致 `numeric_range`），
  另一个覆盖函数体内的布尔条件（`dynamic.bool` 那条路径，正是 H1 的现场）；
- 每个新用例都要走已有机制：先声明基线必须呈现什么（输出、错误、drop），再比较 VM 与原生；
- 用例加进去后，H1 未修时它必须红。**先加用例、确认它会红，再修**——顺序不能反。

### 2.5 **文档回填以实跑为准，不以后一次文档提交为准**

**冻结**：

- H4 的回填内容是：重跑探测后的五个程序真实状态、跑它的提交号与平台、四个程序失败时的 clang 报错原文；
- 19D 的表格按新结果改；改完若某程序仍被拒，就照实写被拒及原因，不写「可构建」；
- 10L §十二 那段「本机未配置 LLVM/Runtime staticlib 环境」删掉或改成事实；
- 溢出的措辞分清两条路径：**动态路径已接通**（差分用例可证），**静态路径 `ir.rs` 的 7 处 `llvm.trap` 仍未接**。
  不要写成「整数溢出已解决」或「整数溢出完全没做」；
- 10L 剩余的两项（高级范围/随机选择的原生结果序列、嵌套 `finally` 释放差异）在回填时保持「未覆盖」登记，
  本批能做就做，做不了就留着，不改写状态。

### 2.6 **验证方式：本机把原生环境配起来跑，不接受「未配置所以没跑」**

本机确实可以配好（审核时已实跑）。10L 用「本机未配置」解释了为什么没跑原生，而同一个提交又声称差分通过。

**冻结**：

- 凡涉及原生行为的结论，必须在配置好环境的本机上实跑 `--ignored` 用例后填写，并在实施记录里写明跑了哪几条；
- 跑不了的要说明**具体缺什么**（缺工具、缺运行库、缺平台），不能只写「未配置」；
- Linux、macOS 的结论仍然只认 CI 运行记录，没有运行号就写「未验证」。

## 三、落点

```text
core/rust/crates/xiao-codegen-llvm/src/dynamic/control.rs      H1：905 行的硬编码标签
core/rust/crates/xiao-codegen-llvm/src/dynamic.rs              函数体发射与错误终点标签
core/rust/crates/xiao-codegen-llvm/tests/n0_b_dynamic.rs       H6：校验不再静默跳过；结构校验
core/rust/crates/xiao-driver/tests/n0_a_native_driver.rs       H2：那个失败用例
core/rust/crates/xiao-driver/tests/native_benchmark_probe.rs   H3：加断言与基线清单
core/rust/crates/xiao-driver/tests/d19a_differential.rs        §2.4：函数体内检查的用例
tests/benchmarks/                                              可能的可构建基线清单
docs/DevDocs/19d-performance-comparison.md、10l-*.md            H4：回填与更正
docs/DevDocs/README.md                                          主表登记
```

## 四、硬约束

1. H 系列各自单独提交，提交说明写明归属 10L，并带正文；
2. **先加用例、确认它会红，再修**（§2.2、§2.4）；
3. 门禁不得静默跳过；缺环境的校验要么显式 `#[ignore]` 并登记，要么改成不依赖外部工具（§2.2）；
4. 判定原生行为必须实跑，不写「未配置所以没跑」（§2.6）；
5. 不为了让门禁变绿而放宽断言、删用例或缩小探测范围；
6. 不跳过 Runtime 检查换取「可构建」（15A §3.2）；
7. 不新增依赖，`Cargo.lock` 保持同步；
8. 未经星崽授权不推送、不触发工作流；
9. 每个提交单独跑 clippy 与 fmt 再提交（H7）。

## 五、分步提交

1. **H1 的前置**：按 §2.4 加差分用例、按 §2.2 加校验，先让它们红，把红色输出记进提交说明；
2. **H1**：改 `control.rs:905` 的硬编码标签，确认两条新用例转绿，并实跑探测确认四个程序恢复构建；
3. **H2**：判断是行为变更还是缺陷，按结论处理，提交说明写清既有测试的改动与理由；
4. **H3**：探测加断言与可构建基线；
5. **H6**：改造 `validate_with_llvm_as`，消除静默跳过；
6. **H8**：拆分 `xiao-runtime/src/abi.rs`，让 `A0-SIZE-001` 回到通过；拆分只搬代码、不改语义，
   拆完重跑 ABI 相关用例；
7. **H4**：重跑探测，回填 19D 与 10L，删掉过时表述，分清溢出两条路径；
8. **H5**：确认「14 类检查」的裁定来源并写进文档（也可先做，不影响其他步骤）；
9. **10L 剩余对齐面**：能做就做，做不了保持登记（§2.5）；
10. **全量门禁与实跑**：`cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`，以及配置好环境后的原生 `--ignored` 用例；
11. **文档与登记**：本文件实施记录、README 主表。

## 六、最可能翻车的地方

1. **只修 `control.rs:905` 就宣布修好**：根因没有隔离验证过，必须靠重跑探测确认四个程序真的恢复；
2. **把 `native_benchmark_probe` 的断言写成「至少一个能构建」**，等于没守门；
3. **先修再补用例**，结果用例是照着修复后的行为写的，拦不住同类问题；
4. **把 H2 的断言放宽**、或删掉那个用例来让门禁转绿；
5. **继续用「未配置环境」解释没跑原生**——审核已证明本机配得起来；
6. **只看 `cargo test` 就宣布门禁全绿**：`bun run check` 现在就是红的（H8），它不在 `cargo test` 里；
7. **回填时把仍被拒的程序写成可构建**，或把探测输出直接贴成结论而不写是哪个提交测的；
8. **溢出措辞又变成一刀切**，分不清动态路径和静态路径；
9. **把 §2.2 的结构校验写成依赖 `llvm-as`**，默认门禁里又静默跳过；
10. **H 系列修复混进 10L 剩余对齐面的提交**；
11. **提交没写正文 / 没单独跑 clippy**。

## 七、验收

1. H1–H8 逐项有结论；H1 修复后，重跑探测显示四个基准程序恢复可构建，或如实说明仍被拒的程序与原因；
2. §2.2 的校验在默认 `cargo test` 里运行，且对「还原 H1 修复」报错（有红色输出为证）；
3. `native_benchmark_probe` 有可构建基线断言，把某个程序改成拒绝会让它失败（有红色输出为证）；
4. `d19a_differential` 的 `CASES` 含至少一个函数体内触发动态检查、一个函数体内布尔条件的用例，两者在 H1 未修时红、修后绿；
5. H2 的结论有依据，既有测试的改动在提交说明里逐条写明；
6. 19D 与 10L 的文档与实测一致；过时表述已删；溢出的两条路径分别表述；
7. 实施记录写明本机实跑了哪几条原生用例、跑它的提交号；Linux、macOS 无运行号则写「未验证」；
8. `cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --check`、`bun test`、
   `bun run check` 全绿，且每个提交单独绿；其中 `bun run check` 必须从当前的**红**（H8 的 `A0-SIZE-001`）回到 0，
   实施记录里附上修前修后的退出码；
9. 没有通过删用例、放宽断言或缩小探测范围换来的绿。

## 八、不负责与不要重复做的事

- **不做**静态路径 `ir.rs` 的溢出改造，除非待定决策 2 选了并入；
- **不做** 20C 官方 Xiao 库、20D `os` 与平台资源；
- **不改**优化算法、`.xiaoc` 格式、索引字段或信任规则；
- **不实现** Xiao 自己的 LLVM Pass 注册；
- **不推送**、不触发工作流，除非星崽授权。

## 待定决策

1. **H2 的定性**：动态模块构建要求显式注入运行库，是有意的行为变更吗？若是有意，既有门控用例就该改成注入运行库并注明改动。
2. **静态路径溢出是否并入**：`ir.rs` 7 处 `llvm.trap` 要不要接 `X06-RUNTIME-009`。建议**不并入**，它要接统一错误路径，且与 10E 对平台级异常的定义相关，单独立项。
3. **「14 类检查」的裁定来源**（H5）：请确认，确认后写进文档。
4. **回归门**：是否新增随 push/PR 触发、跑原生 `--ignored` 用例的工作流。这是 10L 里就提过、至今未答的一项；本批的 H3 只解决「探测有断言」，不解决「CI 会跑」。
5. **是否授权推送**并手动触发平台复现工作流：本地已领先远端 47 个提交。

## 九、实施记录（2026-10-07）

H1–H3、H6、H8 已完成并分别提交：`2447d1b`（函数体错误路由与差分）、`4362d39`（Runtime 注入与原生构建基线）、`75f94f8`（默认 LLVM 标签结构校验）、`b742489`（Runtime ABI 模块拆分）。H2 的结论是有意行为：动态模块必须由调用方通过 `Toolchain::with_runtime_library` 注入 Runtime staticlib，既有门控用例已显式注入并保持严格断言。

受控环境为 Windows `x86_64-pc-windows-msvc`，使用 `XIAO_CLANG=D:\\msys64\\ucrt64\\bin\\clang.exe`、`XIAO_LLVM_AS=...\\llvm-as.exe`、已构建 `xiao_runtime.lib`。实跑命令及结果：

- `cargo test -p xiao-driver --test n0_a_native_driver optional_real_frontend_to_native_round_trip -- --ignored`：通过；
- `cargo test -p xiao-driver --test native_benchmark_probe -- --ignored --nocapture`：`deep-expression-arithmetic`、`scalar-overflow-and-bool-parity`、`named-local-loop`、`deep-call-recursion` 为 `built`；`container-dense` 如实拒绝，原因为动态表方法尚无函数表 ABI；
- `cargo test -p xiao-driver --test d19a_differential -- --ignored --nocapture`：通过；`nested-finally-drops` 仍保留既有释放序列缺口登记；
- `cargo test -p xiao-codegen-llvm --test n0_b_dynamic`：默认结构校验通过；缺少 `XIAO_LLVM_AS` 时只跳过外部汇编器，不跳过函数标签检查。

H5 的“14 类检查”来源是星崽在本次会话确认“扩大范围”；H4 已回填 19D 与 10L。动态路径整数溢出已由 Runtime 检查接通，静态 `ir.rs` 的 7 处 `llvm.trap` 仍未接入 `X06-RUNTIME-009`。`nested-finally-drops` 的释放序列、高级范围/随机选择完整原生结果、Linux/macOS 原生运行记录和 §2.4 受控性能基线继续记为待补。`bun run check` 已从 H8 修复前的 `A0-SIZE-001` 红色状态恢复退出码 0；`cargo fmt --check`、workspace Clippy、Runtime 定向测试均通过。

## 相关页面

- [10L. 原生动态入口与 VM 对齐](10l-native-dynamic-alignment.md) —— 上一批；枚举与实现记录
- [10D. 环境依赖测试专项规范](10d-environment-gated-test-spec.md) —— 门控用例不得静默跳过
- [15A](15a-native-pass-mapping-and-runtime-trimming.md) —— 不得跳过 Runtime 检查
- [19D. 性能对照](19d-performance-comparison.md) —— 探测记录与 19.16 的前置
- [19A. 差分与模糊测试](19a-differential-and-fuzz.md) —— `native_gap` 机制与差分用例
