# 10B. N0-B Runtime ABI 交接文档

> **本批的可执行交接。** N0-A 已交付（`b6df79d`），N0-B 是它的续作，负责把动态值、
> 引用计数、`Weak`、容器、表与释放计划接到 `xiao-runtime-abi` 上。
>
> 上游方向稿见 [10A. LLVM 原生构建闭环](10a-n0-native-closure.md) §1.2 与 §3；
> 本批的**权威定义**是 [10. LLVM 原生后端](10-native-backend.md) `:15`、`:47-49`、
> `:78-79`。本文**不新增要求**，只把它们展开成清单。

## Agent 交接上下文

### 接手前提

1. [10A. LLVM 原生构建闭环](10a-n0-native-closure.md) —— **方向稿**。§1.2 的批次表定义了
   本批范围，§3.2/§3.3 的两处裁定（溢出走 `llvm.trap`、`str` 延后）以本文第二节为准。
2. [10. LLVM 原生后端](10-native-backend.md) —— 主文档。`:15` 是 ABI 的来源，
   `:63` 是它的**禁止条款**，`:78-79` 是本批的核心验收。
3. [06B. Runtime 对象与表生命周期执行闭环](06b-runtime-objects-and-tables.md) ——
   `xiao-runtime` 的对象头、Strong/Weak、表状态机与释放展开**已经在这里实现**。
   本批是**给它加一层 C ABI**，不是重写它。
4. [09-B0-B. 生产 VM 执行闭环](09b0b-production-vm.md) 与 [09-B0-C](09b0c-frontend-to-vm-driver.md)
   —— 本批的**字节码侧对照**。原生侧的语义必须与它们一致（`10:54` 第 1 条）。
5. [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md) —— **开发规定
   主表**，第二章全部沿用。
6. [21A. 单线程 RC、强环与并发模型决策交接](21a-rc-cycles-and-concurrency-handoff.md) ——
   `RefCountStrategy` **只抽象算术、不抽象存储**（§2.4）。本批不得假定它能直接承载原子
   语义，也不得借 ABI 之名改动它。

### 现状盘点（2026-09-21 实测）

| 项 | 现状 |
| --- | --- |
| `xiao-runtime-abi` | **62 行，零依赖**。只有 `ABI_VERSION = 1`、`XiaoOpaqueHandle`/`XiaoHandle`、`XiaoAbiSpan` 和**四个** `extern "C"` 入口 |
| `retain` / `release` | **空实现**：`retain` 原样返回指针，`release` 什么都不做。注释自陈「N0-A 不创建 Runtime 对象」 |
| `xiao_runtime_write_i64` | 唯一的输出入口，注释自陈「**不是 builtin 分派机制**」 |
| `RuntimeValue` | **14 个变体**：7 个标量 + `Str(StringHandle)` + `Table(TableInstance)` + `TableDropView` + `Array`/`Tuple`/`DictTable`/`DictColumn`/`Set` 句柄 + `Error` |
| `xiao-codegen-llvm` | N0-A 已交付；`ir.rs` **1700 行**，占 `A0-SIZE-001` 上限的 68% |
| 生成代码处理的值 | **只有静态标量**（N0-A 的 `assert!(!module.uses_runtime)` 是它的验收） |

### 本批交付与不负责

**交付**：动态值在 ABI 上的表示、真实的引用计数与 `Weak`、容器与表的原生 ABI、
**正常路径**的释放计划降低。

**不负责**：统一错误路径与 `try`/`catch`/`finally` 的展开（N0-C）、源码映射与诊断事件
（N0-C）、三目标固定宽度与裁剪验证（N0-D）、`xiao build` 与 CLI（11/X0）、优化（15）、
诊断窗口（X0）。

---

## 一、先补 N0-A 留下的三处

**用户明确要求把要填补的地方一并写入。** 这三处不是 N0-B 的范围扩大，而是**开工前的清场**：
前两处会让本批的门禁结论不可信，第三处会直接影响本批的代码组织。

### 1.1 `tests/benchmarks/Cargo.lock` 又漏了（**同一盲区，第二次实际漏提交**）

N0-A 加了新 crate，`core/rust/Cargo.lock` 提交了，**独立 crate 的那份没提交**：

```diff
+[[package]] name = "xiao-codegen-llvm"
```

**为什么"跑 check"抓不到它**：`cargo check --manifest-path tests/benchmarks/Cargo.toml`
会**自动补写** lock，所以它**照样成功**。检查通过**证明不了** lock 已提交。

**本批要做的**：

1. **第一个提交**把当前遗漏补上（`fix:` 前缀）；
2. **把判据从"跑得通"改成"跑完工作区仍然干净"**——例如在验证清单里加一条
   `git diff --exit-code tests/benchmarks/Cargo.lock`，或跑完统一看 `git status`。
   **选哪种写法请在本批交接记录里写明**，因为口头提醒已经失败四次。

### 1.2 83 条 `A0-COVERAGE-002`：`xiao-codegen-llvm` 的声明缺文档

两个口径描述的是同一件事，先把它们对上：

```
覆盖率报告：总体 4717/4800（98.27%），公共 API 2482/2482（100.00%）
bun run check：83 条 A0-COVERAGE-002「声明缺少代码文档」
```

**83 = 83**，逐条可定位。按文件分布：

| 文件 | 条数 |
| --- | --- |
| `core/rust/crates/xiao-codegen-llvm/src/ir.rs` | **59** |
| `core/rust/crates/xiao-codegen-llvm/src/toolchain.rs` | 9 |
| `core/rust/crates/xiao-codegen-llvm/tests/n0_a.rs` | 7 |
| `core/rust/crates/xiao-codegen-llvm/src/lib.rs` | 5 |
| `core/rust/crates/xiao-codegen-llvm/src/error.rs` | 2 |
| `core/rust/crates/xiao-driver/src/native.rs` | 1 |

**它不是门禁失败**：公共 API 是 100%（硬门槛），总体 98.27% 也高于 90% 门槛，
而且 **`bun run check` 的退出码是 0**。其余 19 个成员全是 100%，这里是唯一例外。

**本批要做的**：在 N0-B 继续往 `ir.rs` 加东西**之前**补平这 83 条。理由很实际：
本批要加的动态值/容器/表会让这个文件更大，**带病扩张之后更难补**。

> **一条门禁阅读提醒（写给接手者，也是本文作者踩过的）**：`bun run check` 的 warning
> **不影响退出码**，所以 `bun run check | tail -3` 会显示"通过"而**把 83 条 warning
> 全部吞掉**。核对这一项时**必须看完整输出**，或直接 `grep -c A0-COVERAGE-002`。

### 1.3 `ir.rs` 已用掉 `A0-SIZE-001` 上限的 68%

1700 / 2500。N0-B 要加动态值、容器、表、释放计划——**大概率会超**。

**本批要做的**：**开工前先规划拆分**（例如按标量 / 动态值 / 容器 / 调用约定分文件），
而不是超了再拆。`00E` 的旁置 md 豁免是**最后手段**，不是设计目标。

---

## 二、本批的核心裁定（**动手前必须先回答**）

### 2.1 动态值在 ABI 上怎么表示（**最重要的一个**）

`10:63` 禁止「把 Rust 内部类型布局直接暴露为 Xiao 语言契约」，而 `RuntimeValue` 是一个
**14 变体的 Rust enum**（含 `String`、`TableInstance`、多个句柄类型）。所以必须有一个
**C 可表达的表示**。三条路：

| 方案 | 形状 | 代价 |
| --- | --- | --- |
| **A. Tagged union** | `#[repr(C)]` 的 `{ tag, payload }` | 要维护 tag ↔ `RuntimeValue` 的映射 |
| **B. 一切皆句柄** | 连 `int` 也装箱成 `XiaoHandle` | **直接违反 `10:78`**：标量程序也会链接 Runtime |
| **C. 混合** | 静态标量走 LLVM 原生类型，动态位置走句柄/tagged union | 需要判据说明"哪里是动态的" |

**B 必须排除**（它和 N0-A 那条 `assert!(!module.uses_runtime)` 直接冲突）。A 与 C 之间要选。

**选 A 时最需要注意的风险**：tag 表很容易变成**第二份类型系统**——本仓登记过 7 次的
A 类病。若走 A，必须写明 tag 与 `RuntimeValue` 变体的**单一来源**在哪，
以及新增变体时靠什么机制保证两边同步（穷尽性守卫？由 `RuntimeValue` 派生？）。

**选 C 时最需要注意的风险**：判据必须来自 `xiao-ir` 的**静态类型信息**，
**不得**在 LLVM 降低层重新推断（`10:11`、`08` 阶段已冻结的边界）。

**结论要写进本批交接记录，并说明为什么不选另一条。**

### 2.2 `xiao-runtime-abi` 与 `xiao-runtime` 的关系

现状是个**有意的矛盾**：`xiao-runtime-abi` **零依赖**（纯边界声明），
但真实的 `retain`/`release` 必须触碰 `xiao-runtime` 的对象头与计数器。

三条路：

- **A**：ABI crate 只放**类型与函数签名**，`extern "C"` 实现由 `xiao-runtime` 提供
  （ABI crate 仍零依赖，实现侧依赖它来拿类型）；
- **B**：ABI crate 直接依赖 `xiao-runtime`，零依赖被打破；
- **C**：函数指针表，Runtime 启动时注册。

**A 与既有约束最相容**（ABI 的版本与兼容规则仍在单一位置，`10:63` 也好守），
但要把"谁定义符号、谁提供实现"写清楚，否则会出现两份签名。

**无论选哪条**，都要回答：`ABI_VERSION` 的**兼容规则**是什么？
（新增入口算不算 breaking？生成代码与 Runtime 版本不一致时如何拒绝？）

### 2.3 释放计划：N0-B 与 N0-C 的边界

10A §1.2 把「释放计划」划给 N0-B、「错误路径」划给 N0-C，但**两者是耦合的**——
`06B` 的展开驱动是 `finally -> drop -> catch/传播` 一条链。

**本批要澄清并写死**：

- **N0-B 做**：正常作用域退出（以及 `return`/`break`/`continue` 这类**非异常**转移）
  的释放序列，与 `IrOwnership.release_plans` 逐条对齐；
- **N0-C 做**：异常展开路径上的清理，以及 `Fatal` 那条不执行释放计划的终止路径。

**判据**：`06A` 的八类退出边里，哪些属于 B、哪些属于 C，要逐项列出。
**不要留"看起来都做了"的模糊区**——那正是 `09R2G` 释放边界修复的成因。

---

## 三、范围与切分

**权威范围**（`10:47-49` 的二级任务 2、3 + `10:15`）：

1. 动态值与引用计数的 Runtime ABI（`10:48`）；
2. `Weak` 的 ABI（`10:48`）；
3. 容器与表的 ABI（`10:15` 点名「动态值、容器、表生命周期」）；
4. 控制流与**释放计划**降低为基本块（`10:49` 的前半）；
5. 产物的 Runtime 组成可解释（`10:79`）。

**范围很大**。若评估后认为一批做不完，**在本批交接记录里给出拆分方案**（例如
`N0-B1 动态值与引用计数` / `N0-B2 容器与表` / `N0-B3 释放计划`），并说明每批的验收面。
**不要**为了凑一批而把验收做虚。

**单模块**：沿用 B0 与 N0-A 的口径。

---

## 四、最可能翻车的地方

1. **把 Rust 布局当 ABI**（`10:63`）。`RuntimeValue` 的 enum 布局、`TableInstance` 的
   字段、对象头的计数器——**一个都不能出现在 ABI 上**。
2. **tag 表变成第二份类型系统**（§2.1 A 的风险）。
3. **一切皆句柄**（§2.1 B）——直接违反 `10:78`。**好在它有一条现成的探测器**：
   N0-A 留下的 `assert!(!module.uses_runtime)` 会在标量程序上立刻失败。
   **本批不得为了让新测试通过而删掉那条断言**——它正是这条路的防线。
4. **两份签名**（§2.2）：ABI crate 声明一份、`xiao-runtime` 又写一份。
5. **释放边界模糊**（§2.3）：把异常展开顺手做了（越界到 N0-C），或把正常退出漏了。
6. **在 LLVM 层重新推断类型**（§2.1 C 的风险）。
7. **借 ABI 之名改 `RefCountStrategy`**（21A §2.4 已裁：它只抽象算术，不足以承载原子
   语义；本批**不需要**原子，所以**不要顺手改它**）。
8. **动了 B0 的冻结项**：`FORMAT_VERSION = 3`、opcode `0..40`、`tests/spec/` 共享向量、
   `tests/benchmarks/reports/`。
9. **又忘了 `tests/benchmarks/Cargo.lock`**（§1.1）——本批会加代码，连锁更新必然发生。

---

## 五、硬性约束

门禁、区分度验证、工具规定、单一来源原则、解耦约束**全部沿用 09R2D 文档第二章**。

### ★ 两条约束**分别**核对

1. **提交标题带规范前缀**；2. **正文说明为什么**。两者**独立**，复发历史：

```text
261d88e / 66f0cec / B0-A 两个提交   →  标题缺前缀
B0-B / B0-C / B0-D                  →  两条都守住了 ✓
2a33680（21A 收束）                 →  标题守住了，正文掉了
b6df79d（N0-A）                     →  两条都守住了 ✓
```

### ★ 新 crate 的文档注释要一次到位

§1.2 那 82 个缺失项集中在 `ir.rs`。本批新增的 ABI 项（每个 `extern "C"` 入口、
每个 `#[repr(C)]` 类型）**都要有文档注释**，且要写明**安全契约**（谁能传空指针、
谁负责释放、指针在什么条件下失效）。公共 API 100% 是硬门槛。

### 其他

- `xiao-runtime-abi` 的每次改动都要复核它的 README 与模块登记。
- 新增 `extern "C"` 入口时，**同步登记到 ABI 版本策略**（§2.2），不要只加函数不改版本规则。
- `A0-SIZE-001`：见 §1.3，**先规划拆分**。

---

## 六、验收

沿用 09R2D 的「撤掉实现 → 用例必须失败 → 还原 → 通过」。**关键验收不是「测试通过」**：

1. **动态值端到端**：一份含字符串/数组/表的源码经原生路径跑出的结果，与**字节码模式一致**
   （`10:54` 第 1 条、`12-tests:687`）。
2. **引用计数真的生效**：有断言证明 `retain`/`release` 改变了计数（N0-A 的空实现会失败）。
3. **`Weak` 语义一致**：`Weak` 不拥有目标、最后强引用立即释放——与 `06B` 的既有契约一致。
4. **标量程序仍然不链接 Runtime**：`10:78` 那条断言在 N0-B 之后**仍然成立**
   （这是 §2.1 排除方案 B 的判据）。
5. **Runtime 组成可解释**（`10:79`）：能通过诊断选项说明产物里有哪些 Runtime 组件。
6. **§2.3 的释放边界逐项列明**，且 B 侧的部分有断言。
7. **§1.1 / §1.2 / §1.3 都已处理**。
8. **门禁全绿**，**包含** `tests/benchmarks` 独立 crate，且**工作区在跑完后仍然干净**。

---

## 七、不负责与不要重复做的事

- **不要做错误路径与异常展开**（N0-C），**不要做源码映射/诊断事件**（N0-C）。
- **不要做优化**（15）、**不要接 `xiao build`/CLI**（11/X0）、**不要做诊断窗口**（X0）。
- **不要重写 `xiao-runtime` 的对象头、表状态机或释放展开驱动**（`06B` 已交付并锁定）。
- **不要改 `RefCountStrategy`**（§4 第 7 条）。
- **不要发明 builtin 分派机制**（20 阶段的事；`xiao_runtime_write_i64` 的注释已自陈它不是）。
- **不要在 LLVM 层重新推断类型或生命周期**。
- **不要删掉 `tests/benchmarks/` 或三个载体**。

## 相关页面

- [10A. LLVM 原生构建闭环](10a-n0-native-closure.md) —— 方向稿，§1.2 定义本批范围
- [10. LLVM 原生后端](10-native-backend.md) —— 主文档，`:15`/`:63`/`:78-79`
- [06B. Runtime 对象与表生命周期执行闭环](06b-runtime-objects-and-tables.md) —— 既有实现
- [06A. 生命周期静态闭环交接记录](06a-lifetime-static-closure.md) —— 八类退出边与释放计划
- [09-B0-B. 生产 VM 执行闭环](09b0b-production-vm.md) —— 字节码侧对照
- [21A. 单线程 RC、强环与并发模型决策交接](21a-rc-cycles-and-concurrency-handoff.md) —— `RefCountStrategy` 的边界
- [09R2D. 两种机型与指令编码器交接文档](09r2d-machines-and-encoder.md) —— 开发规定主表
- [00E. 单文件行数门禁交接](00e-file-size-gate.md) —— `A0-SIZE-001` 与旁置 md 豁免

---

## 八、落地记录（2026-09-21）

本批已按上述边界落地，代码分为三层：`xiao-runtime-abi` 只保留固定 C 布局、版本常量和
外部声明；`xiao-runtime/src/abi.rs` 提供唯一符号实现；`xiao-codegen-llvm/src/dynamic.rs`
消费同一份类型化 IR 并生成 `%xiao.value` 调用。选择 tagged union 是因为标量仍必须保留
N0-A 的原生 LLVM 类型，不能把所有值装箱；标签使用透明的 `u32` 新类型而不是 Rust
`enum`，所以 C/LLVM 传入未知值时可以返回 `InvalidArgument`，不会触发非法判别值的未定义行为。

ABI 主版本仍为 1，新增动态入口把次版本推进为 1。强/弱 ABI 句柄使用带魔数和种类标记
的盒子；`retain`/`weak_retain` 返回原 ABI 盒子地址并在盒子内部增加一份 Runtime 引用，
从而与 N0-A 的地址契约一致。拥有句柄的 `XiaoValue` 不再实现 Rust 的隐式 `Copy`，跨
边界复制必须调用 `xiao_runtime_value_copy`。动态入口在启动时检查主/次版本，构造和复制
调用检查状态码，失败边进入 `llvm.trap`；这不等同于 N0-C 的可恢复异常展开。由于 ABI
长度字段采用 `usize`、LLVM 描述符当前按 `i64` 发射，N0-B 动态降低明确只接受 64 位
目标；32 位目标留给 N0-D 的固定宽度验证。

所有 `out` 句柄槽都采用显式替换协议：调用方先写入空指针（或仍然 live 的 ABI 强句柄），
Runtime 成功时先归还旧句柄再写入新句柄，失败时保持旧槽位；`XiaoValue` 输出槽同样必须
先初始化为 `none` 或有效值。句柄只能传回 Runtime 返回且尚未 `release` 的地址，释放后
不得重用。盒子魔数和种类标记只能拦截仍可读的明显类型错配，不能把任意外部地址或释放
后的悬空指针变成安全输入；这条边界由调用方的生命周期契约负责。

LLVM 表描述符完整写入表名、形态、字段名、字段类型和公开标记；真实表声明中的字段默认值
会在每次 `new`/singleton 构造后按源码顺序写入，含方法或非 ABI 字段类型的表、带初始化
参数的 `new` 以及其他表体语句仍结构化拒绝，避免静默丢失语义。
动态 `Cast` 只允许类型层已经证明的 identity 透传；跨类型转换（包括 `str -> bool`）
和前端登记但原生侧尚未消费的任意 `runtime_checks` 都会带源码区间拒绝，直到对应失败边
接入后续批次。数组、元组、字典和集合构造
遵循“Runtime 先复制输入值，生成代码再释放临时拥有值”的协议；表字段读写同样经过 ABI
类型和句柄校验。正常结束和 `return` 会
优先消费根程序的 `IrOwnership.release_plans`，按 `order` 发出 strong/weak 释放；没有
所有权元数据的手工 IR 才使用逆声明序兜底。已类型检查的布尔 `if`/`elif`/`else` 与无
异常转移的 `while`（包括没有块级拥有值时的 `break`/`continue`）会降低为基本块。若
分支或循环声明了动态拥有局部值，降低器会在生成 LLVM 前返回带作用域和源码区间的结构化
拒绝，避免把它静默漏到根计划；块级计划发射和作用域展开留给后续 N0-B 批次。`for`、
动态局部的 `break`/`continue` 当前也会结构化拒绝，待同一块级计划机制接入后再实现；异常、`raise`、
`try/catch/finally` 和致命终止路径属于 N0-C。后者需要异常展开上下文，不能在 N0-B 里
用根槽位释放代替。

`CodegenOptions::entry_observation(ExitCode)` 在动态模块中也会显式生成 `i64`
入口和 `main` 适配器；它只记录类型层明确为 `int`/`sint`/`bool` 的最后一次值，避免
把动态标签猜测成退出码。未启用该选项时动态入口保持 `void` 与零退出码。

验证覆盖 ABI 布局、版本、强/弱计数、未知标签、数组往返、动态表描述符、真实前端的字符串/
数组/表同源降低、Cast/运行时检查拒绝边界和真实 `llvm-as` 解析；`xiao-driver` 另有真实
`FrontendCompiler` 动态构建测试。原生链接测试只在
显式提供与目标三元组匹配的 `XIAO_RUNTIME_LIBRARY`、`XIAO_CLANG` 和 `XIAO_LLVM_AS` 时
运行。本机当前仅有 MSYS clang、缺少可用的 MSVC 链接环境，因此未把该次链接结果宣称为
Windows 原生验证；Linux/macOS 与 WSL/容器仍列为待复现。
