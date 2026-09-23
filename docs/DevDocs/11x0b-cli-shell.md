# 11X0-B. TypeScript CLI 骨架交接文档

> **本批的可执行交接。** X0-A 已交付协议与 Rust 侧入口（`7b201e1`），本批把 TypeScript
> 侧接起来，让用户第一次真正拿到 `xiao` 命令。
>
> 上游方向稿见 [11X0](11x0-cli-protocol-and-toolchain.md)（§1 的四条决策、§2 的协议契约）；
> 本批的**权威定义**是 [12. 测试与开发里程碑](12-tests-and-milestones.md) X0 退出条件的
> 第 1、4、7 条与 [11. CLI、项目配置与平台](11-cli-config-and-platform.md)。**不新增要求。**

## Agent 交接上下文

### 接手前提

1. [11X0. 跨平台工具链](11x0-cli-protocol-and-toolchain.md) —— **方向稿**。§1 的四条决策
   全部有效；§2.4 的协议单一来源已定为方案 C。
2. [11. CLI、项目配置与平台](11-cli-config-and-platform.md) —— 主文档。`:15` 的核心命令、
   `:43` 的配置修改命令、`:229` 的命令解析与帮助。
3. [00A. 工程框架与目录布局](00a-project-layout.md) `:126` 与 `:135` ——
   `cli/ts/src/*` 的**职责边界**与跨语言边界的字段清单。
4. [11C. 国际化、系统提示与语言包插件](11c-localization.md) —— `[language].locale` 的
   优先级与中英目录；**本批的呈现层要按它设计，但不在本批实现**（见 §3.5）。
5. [09-B0-D. 退出码冻结](09b0d-exit-codes-and-linux-verification.md) —— **11 只负责把
   `as_process_code()` 映射到进程**，不得重新定义那五个值。
6. [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md) —— 开发规定主表。

### 现状盘点（2026-09-22 实测）

| 项 | 现状 |
| --- | --- |
| 协议与 Rust 入口 | ✅ X0-A 已交付：8 字节大端长度 + JSON、`hello`/`run`/`build`/`cancel`/`shutdown`、`xiao-core` 二进制 |
| 共享 fixture | ✅ `tests/spec/11x0-protocol/`，Rust 与 TS **读同一批文件** |
| `cli/ts/src/` | `commands/`、`config/`、`diagnostics/`、`platform/`、`ui/` 已完成 X0-B 骨架；REPL、环境和包目录仍为后续占位 |
| 命令入口 | ✅ `cli/ts/src/main.ts` 与 `bin.xiao` 已接入；独立打包仍归 X0-C |
| `print` | ❌ 内置函数不存在，`print(...)` 报 `X06-RUNTIME-012`（见 §2.2） |
| `xiao test` | ✅ 语义与协议由 X0-T 接入（见 §2.1） |

### 本批交付与不负责

**交付**：`xiao` 命令入口、命令解析与帮助、`xiao run`、`xiao config`、呈现层（颜色/宽度/
非 TTY 降级）、以及它与 X0-A 协议的接线。

**不负责**：独立可执行打包与三平台（X0-C）、`-debug` 与诊断窗口（X0-D）、`xiao build`
（依赖 X0-E 的主机工具链发现）、REPL（11B）、i18n 目录本身（11C）、包管理（11A）、
内置函数（20）。

---

## 一、范围与退出条件映射

| X0 退出条件 | 本批覆盖 |
| --- | --- |
| 第 1 条 `xiao run`、源码快捷运行、`xiao config` | ✅ 本批；`xiao test` 由 X0-T 接入；`xiao build` 归 X0-E |
| 第 4 条 TS 构建 + 静态检查、三平台行为一致 | ⚠️ **构建与静态检查在本批**；三平台一致性归 X0-C |
| 第 7 条 `xiao config` 的布尔写入 | ✅ 本批 |
| 第 2、5、6 条（三平台矩阵、独立可执行、核心发现） | ❌ X0-C |
| 第 8 条（`-debug` 与诊断窗口） | ❌ X0-D |

**落点**：`00a:126` 已为每个子目录定好职责——`commands/`（命令路由）、`config/`、
`diagnostics/`（结构化错误转终端显示、颜色和**退出码**）、`ui/`（提示符、分隔线、颜色、
进度、**无色降级**）、`platform/`、`protocol/`。**按这份职责表填，不要另起结构。**

---

## 二、必须先裁定的两件事

### 2.1 `xiao test` 的语义（X0-T 已关闭本项）

`12-tests` 的 X0 第 1 条点名了 `xiao test`，但**全仓没有它的语义定义**：它跑什么、
输出什么、怎么和「第 01 到本阶段所有已确定规则的自动化规格测试」（第 3 条）区分开，
都没有写。

**本批要给出结论并写进 `11-cli-config-and-platform.md`**。三条候选方向：

- **A**：`xiao test` 是**项目测试运行器**——发现并运行项目里的 Xiao 测试文件；
- **B**：它是**规格测试的入口**——跑工具链自带的 `.xiao` 规格用例（与第 3 条同一件事）；
- **C**：本批**只登记命令名与诊断**，行为留到有测试框架的批次。

**无论选哪条**，都要说明它与 `cargo test`/`bun test` 的分工，并**明确说出它在 X0 退出
条件第 1 条里是"已接"还是"已登记未实现"**。**不允许**悄悄实现一个名字对但语义含糊的命令。

**X0-B 当时裁定：选择 C。** `xiao test [project]` 先只完成参数登记、帮助和稳定诊断；
随后 X0-T 裁定并落地项目测试运行器：递归发现 `tests/**/*.xiao`，按项目相对路径稳定排序，
逐文件通过核心协议执行并返回结构化结果。`cargo test` 仍运行 Rust workspace，`bun test`
仍运行 TypeScript/CLI 工具链；两者没有被偷偷包装成 `xiao test`。

### 2.2 `xiao run` 的输出（**`print` 不在本阶段**）

**既定事实**（11X0 §1.3）：内置函数不存在，`print("hello")` 报 `X06-RUNTIME-012`；
脚本模式的 `RunOutcome.value` 也恒为 `None`。**用户跑完程序看不到任何输出。**

**本批要做的两件事**（11X0 §1.3 已定，这里落成动作）：

1. **在 UseDocs 里写明这是已知限制**——`docs/UseDocs/tooling/cli/` 下要有这一条。
   用户第一次跑 `xiao run hello.xiao` 什么都没看到时，必须能查到"这是预期行为、
   内置函数属 20 阶段"；
2. **`xiao run` 靠退出码与结构化诊断表达结果**——不靠输出。B0-D 冻结的五个值已经在
   协议里（X0-A 的 `exit_code`/`exit_name`），本批把它们接到进程退出码上。

> **不要**为了让 `xiao run` "看起来有用"而顺手实现 `print`。那是 20 阶段的事，
> 而且它会绕过 `xiao-intrinsics` 的声明式契约（见 [20](20-builtins-and-standard-library.md)）。

**X0-B 落地结果：** `xiao run` 已通过 TypeScript CLI、长度前缀协议和 `xiao-core` 的真实
源码回环。成功或失败由响应中的 `exit_code`/`exit_name` 和结构化 `diagnostics`、`report`
表达；CLI 不读取本地化文本推断结果。`print` 仍未实现，因此脚本即使成功也不会凭空产生
用户程序输出，详细限制写在 [CLI 运行说明](../UseDocs/tooling/cli/shell.md)。

---

## 三、CLI 呈现规范（本批的实质新增）

**这一节是从社区 CLI 设计实践中提炼的**，并且已经**按 Xiao 的具体约束筛过**——
不是通用清单，只留下与本项目相关的。

### 3.1 中文宽度：**这是 Xiao 必然命中的一条**

**症状**：表格、框线、对齐在多字节字符下全部错位。

**根因**：用 `str.length` 算列宽。JS 的 `.length` 数的是 **UTF-16 码元**，而终端里
一个 CJK 字符占**两列**，emoji 更复杂。

**为什么 Xiao 必然命中**：Xiao 的诊断、`[language].locale` 的中文文案、用户的
中文标识符——**输出里天然有 CJK**。

**要求**：**所有对齐/列宽计算必须走 Unicode 感知的宽度库**（如 `string-width`），
不得用 `.length`、不得用 `Buffer.byteLength` 替代。**并且要有一条含中文的回归用例**
（纯 ASCII 的用例抓不到这个）。

### 3.2 颜色降级：四层，顺序不能乱

```
NO_COLOR 环境变量存在     → 完全去色
stdout 不是 TTY           → 纯文本模式（管道、重定向）
COLORTERM 表明 truecolor  → 24-bit
否则                      → 降级到 256 色 / 16 色
```

**必须提供 `--color=always` 覆盖**——否则在 CI 里抓不到彩色输出做快照测试。

**与仓内的关系**：`00a:126` 已经把「**无色降级**」写进 `cli/ts/src/ui` 的职责，
本批是落实它。

### 3.3 非 TTY 与管道：不能崩、不能漏 ANSI

`xiao run foo.xiao | head -5` 这种用法下：

- **不能 panic**（管道提前关闭会让写 stdout 抛 `EPIPE`）；
- **不能把 ANSI 转义码混进管道内容**——检测到非 TTY 就要去色。

**这同时是 X0 第 6 条的一部分**：CLI 的输出要被机器消费时必须是干净的。

### 3.4 语义色上限：**最多 3 个语义色 + 1 个强调色**

超过 5 种颜色的一屏等于没有重点。用**灰度做层次**，用颜色只做语义
（成功/失败/信息）。

**与仓内的关系**：`11C` 的 i18n 会给出文案，但它**不该决定颜色**——
颜色是呈现层的事，`11C` 只管文本。

### 3.5 locale 的边界（**只设计，不实现**）

`07:175`、`11:276` 都要求 **locale 只改变显示文本，不改变错误身份、堆栈、退出码或
用户输出**。

**本批要做的**：呈现层**按"文本可替换、其余不可变"的形状设计**——
即颜色、布局、退出码、机器字段都不从文案里推。**不要**在本批实现语言目录（那是 11C）。

---

## 四、与 X0-A 协议的关系

**X0-A 已经交付的，本批只消费**：

- 帧编解码与 `MAX_FRAME_BYTES`（`cli/ts/src/protocol/codec.ts` 已在）；
- `hello` → `run` / `build` / `cancel` / `shutdown` 的消息形状；
- `exit_code` / `exit_name` 字段（B0-D 的五个值）；
- 共享 fixture（`tests/spec/11x0-protocol/`）。

**本批要接的**：把 `commands/` 的命令路由到协议调用上，并把响应渲染成终端输出。
**不得重新定义协议**——协议已经有单一来源（方案 C），新增字段要走两侧同时改 + fixture
同批更新。

---

## 五、最可能翻车的地方

1. **用 `.length` 算列宽**（§3.1）。这是 JS CLI 最常见的错误，而 Xiao 的输出**必然**
   含中文，所以一定会被用户看到。
2. **忘了非 TTY 降级**（§3.3）——管道下崩掉或漏 ANSI。
3. **偷偷实现 `print`**（§2.2）。
4. **`xiao test` 语义含糊地实现了**（§2.1）。
5. **在 CLI 里重新实现编译器语义**——`11:9` 说死了：CLI 只发送规范化请求、
   接收结构化结果并渲染。
6. **通过解析本地化文本判断成败**（`00a:135` 末句、X0 第 6 条）。
7. **重新定义退出码**（它已由 B0-D 冻结，CLI 只做映射）。
8. **把 `xiao build` 顺手做了**——它需要 X0-E 的主机工具链发现（`10:21`）。

---

## 六、硬性约束

门禁、区分度验证、工具规定、单一来源原则、解耦约束**全部沿用 09R2D 文档第二章**。

### ★ 锁文件门禁已固化（**2026-09-22 起**）

`tests/benchmarks` 是独立 crate，它的 `Cargo.lock` 不在 `core/rust` workspace 覆盖内。
该盲区造成过**三次**漏提交，写在文档里提醒三次都没生效，现已固化为 `check:lock` 并
**并入 `bun run check`**（判据与来由见 [00A.1](00a-a0-workspace-and-checkers.md)）。
**本批如果给任何 Rust crate 增删依赖，跑完 `cargo check` 后如果锁文件有 diff 就是漏提交。**

### ★ 两条提交约束分别核对

1. **标题带规范前缀**；2. **正文说明为什么**。复发历史：`261d88e`/`66f0cec`/B0-A 两个
   （标题缺）→ `2a33680`（正文缺）→ B0-B/C/D、N0-A、10C/10D、X0-A **都守住了** ✓

### ★ 门禁看完整输出

`bun run check` 的 warning **不影响退出码**，`| tail -3` 会吞掉它们（10B 的 83 条就是这么
漏掉的）。用 `grep -c` 计数。

### 其他

- `cli/ts` 的每个子目录填充时要按 `00a:126` 的职责表，并保持 README 与实现同步。
- TypeScript 侧的 `export` 项**文档覆盖率必须 100%**（`00a` 的硬门槛，两处都写了）。
- 新增命令要有 `--help`（`11:229`），且帮助文本本身也是呈现层的一部分。

---

## 七、验收

沿用 09R2D 的「撤掉实现 → 用例必须失败 → 还原 → 通过」。**关键验收不是「测试通过」**：

1. **`xiao run` 端到端**：真实源码经 CLI → 协议 → Rust 核心 → 结果回传，
   **进程退出码等于 B0-D 的五个值之一**；
2. **中文对齐有回归用例**（§3.1）——含 CJK 的表格列宽正确，**纯 ASCII 用例不算数**；
3. **非 TTY 有断言**：`| head -5` 形状不崩且输出无 ANSI；
4. **`--color=always` / `NO_COLOR` 都被覆盖**（§3.2）；
5. **`xiao config` 的布尔写入**：`xiao config CLI.git.summary true` 与
   `--global` 变体按 `12-tests` 第 7 条写入，非法值/未知路径/写入中断有稳定诊断，
   **且无关配置与注释不被破坏**；
6. **§2.1 与 §2.2 都有明确结论**并已写进对应文档；
7. **CLI 不复制语义**：grep 不到类型检查、生命周期推断或 TAC 构造；
8. **门禁全绿**（含 `check:lock` 与 `bunx tsc`），`ignored` 数量可见。

---

## 八、不负责与不要重复做的事

- **不要做 `xiao build`**（X0-E）、**不要做 `-debug` / 诊断窗口**（X0-D）。
- **不要实现 `print` 或任何内置函数**（20 阶段）。
- **不要做 REPL**（11B）、**不要做 i18n 目录**（11C）、**不要做包管理**（11A）。
- **不要重新定义协议**（X0-A 已冻结，方案 C 的单一来源要守住）。
- **不要重新定义退出码**（B0-D 已冻结）。
- **不要在 CLI 里实现编译器语义**（`11:9`）。

## 九、X0-B 已落地记录（2026-09-22）

1. `commands/` 提供全局颜色/JSON 选项、帮助、版本、源码快捷运行、`run`、`config`、
   `test`、`build` 和 REPL 未实现分支；`package.json` 暴露 `xiao` bin。
2. `config/editor.ts` 发现祖先目录的小写 `config.xiao`，诊断非规范大小写，支持
   `CLI.git.summary` 与 `language.locale`，保留注释/无关字段并用同目录临时文件原子替换。
3. `platform/core.ts` 提供宿主目标描述、`XIAO_CORE_PATH` 覆盖和开发树核心发现；独立
   安装包已在 X0-C 接入，Linux/macOS 原生矩阵仍按 X0-C 清单待复现。
4. `protocol/client.ts` 先协商版本，再发送真实 Xiao 源码；核心崩溃、坏帧、版本失配和
   取消都保留稳定机器码，退出状态不从人类文案推导。
5. `ui/` 与 `diagnostics/` 实现 Unicode 显示宽度、TTY/`NO_COLOR`/`--color=always`
   降级、JSON 输出和 EPIPE 安全写入；中文对齐和非 TTY 回归测试已加入。

本记录只覆盖 X0-B；独立可执行打包已由 X0-C 接入，Linux/macOS 原生矩阵、`-debug` 窗口、REPL、i18n 和
`print` 不在本批交付范围内。

## 相关页面

- [11X0. 跨平台工具链](11x0-cli-protocol-and-toolchain.md) —— 方向稿与四条决策
- [11. CLI、项目配置与平台](11-cli-config-and-platform.md) —— 主文档
- [00A. 工程框架与目录布局](00a-project-layout.md) —— `cli/ts/src/*` 的职责表
- [00A.1 工作区与质量门禁](00a-a0-workspace-and-checkers.md) —— `check:lock` 的判据
- [09-B0-D. 退出码冻结](09b0d-exit-codes-and-linux-verification.md) —— 五个退出码
- [11C. 国际化、系统提示与语言包插件](11c-localization.md) —— locale 边界
- [20. 内置函数与标准库](20-builtins-and-standard-library.md) —— `print` 的归属
- [12. 测试与开发里程碑](12-tests-and-milestones.md) —— X0 八条退出条件
