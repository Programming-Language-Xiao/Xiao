# 20C-原生门禁修复专项（维护回归 `38484238759` 红）

> **背景**：`167a03e` 推送后，[维护回归 38484238759](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/38484238759)
> 的两个原生作业**都红**，`main` 当前不绿。这是 **2026-10-11 之后 34 个提交的第一次 CI 曝光**——
> 期间 10R D2 与 20C 的四个批次都没经过维护回归，所以红的不止一处、也不止一批。
>
> **一句话概括**：本文是三处红的**修复规格**——每条给出**真 CI 证据、根因定位到文件:行、最小复现、修法要求与负例**；
> 外加「让下一次能自动发现」的五条执行点。
>
> 状态（2026-10-14）：**三条红已修复，`main` 已转绿**——`fb1f435` 推送后的维护回归
> [38493485483](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/38493485483) **五个作业全绿**
> （含两个原生作业与首次带 `--no-fail-fast` 的 Linux 全量忽略套件）。
> 三条处置已裁定——**N-2 修降低**、**N-3 按故障类别分派**、**`--no-fail-fast` 加进 workspace 门禁**（§10.1）。
> `N-1` / `N-2` 属本批（20C 语言边界），`N-3` 属 10R。
> **第二轮审核并入四笔返工**（§十一，**没有一条是红**）：F-A 释放对账是同义反复、
> F-B/F-C/F-D 是注释与清单的留痕卫生；`c595c82` 已把这四笔改掉。
> **第三轮审核并入四笔**（§十二，同样**没有一条是红**）：`--no-fail-fast` 只治了一半的遮蔽、
> §5.2.1 的落点与判据要改（审核侧的账）、F-A 的新断言只比总数不比位置。
> **返工后的 CI**：`5d32140` 的维护回归 `38522338923` 与 workspace 门禁 `38522338905` **都绿**；
> 本轮残余项随后已完成一次性分支的故意红/还原绿验证（§5.2.1），并补齐释放位置断言及其变异证据（§11.1.1）。
> **跨步骤遮蔽与位置断言现已闭环**：红运行 `38534726488`、还原后的绿运行 `38535786407`。
> 修复前的红色运行仍原样保留在 §一；**回填的数字全部来自 CI 日志，不用本机结果代替**。

## 一、CI 判决（真运行号与作业号）

| 工作流 | 结论 |
| --- | --- |
| [workspace 门禁 38484238745](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/38484238745) | **success** |
| [维护回归 38484238759](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/38484238759) | **failure** |
| └ Deterministic parser and artifact security regression `115593911292` | success |
| └ Deterministic artifact size regression (Windows baseline) `115593911492` | success |
| └ Rust workspace default tests (Linux) `115593911509` | success |
| └ **Native differential regression (Windows)** `115593911617` | **failure** |
| └ **Native differential regression (Linux)** `115593911650` | **failure** |

**三条红（两平台三处）**：

| # | 现象 | 平台 | 归属 | 级别 |
| --- | --- | --- | --- | --- |
| **N-1** | `and`/`or` 的原生短路把 `i1` 当 `i8` 传，**七种形态全部构建失败** | Windows（Linux 未跑到，见 §5.2） | `167a03e`（本批） | **P0** |
| **N-2** | `value = 2 ** 3`（裸赋值）原生 `原生后端不支持 动态整数幂` | Windows（同上） | **既有缺口**，本次被新守卫**首次抓到** | **P0** |
| **N-3** | 硬件故障退出码跨平台不一致：Linux 报 `132`、测试期望 `139` | Linux | `24f3f9b`（10R D2） | **P0** |

**修复后的绿色运行（`fb1f435`，2026-10-14）**：

| 工作流 / 作业 | 结论 |
| --- | --- |
| [维护回归 38493485483](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/38493485483) | **success**（五个作业全绿） |
| └ Native differential regression (Windows) `115623868914` | **success** |
| └ Native differential regression (Linux) `115623869640` | **success** |
| [workspace 门禁 38493485411](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/38493485411) | **success** |

**返工后的绿色运行（`5d32140`，2026-10-14）——含 §十一/§十二 的改动**：

| 工作流 / 作业 | 结论 |
| --- | --- |
| [维护回归 38522338923](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/38522338923) | **success**（五作业全绿） |
| └ Native differential regression (Windows) `115714682969` | **success** |
| └ Native differential regression (Linux) `115714683004` | **success** |
| [workspace 门禁 38522338905](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/38522338905) | **success** |

**这一轮审核在门禁日志里核实过的事实**（不只看绿灯）：`run.sh` 的段落头 `== Rust/TypeScript 门禁 ==` 出现；
日志里 **25 行 `test result:`** ⇒ `cargo test -p xiao-driver` 确实拉起了多个测试二进制并跑完；
`integer_power_result_matches_vm` / `logical_short_circuit_keeps_shared_release_plans` /
`operator_coverage_guard_vm` 三条**都在其中且 ok**；`FAILED` / `panicked at` 命中 **0**。

> ⚠️ **这一轮全绿不算 §5.2.1 的取证**——见该节「实施结果」的口径：fail-fast 只在**有失败**时才显出差异，
> 一次全绿运行证明不了它。**「脚本带上参数仍正常」与「参数治好了遮蔽」是两件事**。

**CI 日志里的关键数字（两平台一致）**：

| 用例集 | Windows | Linux |
| --- | --- | --- |
| `n0_a`（含两个故障类别用例） | `7 passed; 0 failed` | `7 passed; 0 failed` |
| `c20_capability_probe` 原生半边 | `2 passed; 0 failed` | `2 passed; 0 failed` |
| 日志里的 `20C-PROBE` 行数 | **14** | **14** |

**Linux 上的 c20 是第一次真的跑到**：修复前 `n0_a` 先失败而工作流没有 `--no-fail-fast`，
`cargo test` 就停在那个二进制上了（§5.2）。加了 `--no-fail-fast` 之后，
Linux 日志里出现了 `Running tests/c20_capability_probe.rs` 与 **14 行 `20C-PROBE`**——
**这就是 §5.2 那条修补的直接证据**：不是"Linux 也绿了"，而是"Linux 这次终于跑了，且绿"。

**Windows 作业修复前的 CI 原文（与审核本机一字不差）**：

```text
panicked at crates\xiao-driver\tests\c20_capability_probe.rs:471:49:
已接受算子原生构建失败：binary-starstar 原生后端不支持 动态整数幂（8..14）

panicked at crates\xiao-driver\tests\c20_capability_probe.rs:631:49:
成功代表性原生构建失败：second-case-conversion-logic 工具 clang 失败（状态 Some(1)）：
  …-14.ll:398:69: error: '%t69' defined with type 'i1' but expected 'i8'
```

## 二、N-1 **`and`/`or` 的原生短路：`i1` 当 `i8` 传**

### 2.1 事实（审核本机实跑，7 种形态）

| 形态 | VM | LLVM 原生 |
| --- | --- | --- |
| `return a and b`（两个 `bool` 参数） | 成功 | **clang: `'%t23' … i1 but expected i8`** |
| `return true and false` | 成功 | **同左**（`%t15`） |
| `return a < b and b > 0` | 成功 | **同左**（`%t57`） |
| `return a >= "A" and a <= "Z"` | 成功 | **同左**（`%t69`） |
| `a >= "z" or a <= "Z"` | 成功 | **同左** |
| `a == b and b == a` | 成功 | **同左**（`%t59`） |
| 动态操作数（`false and explode(0)`） | 成功 | **同左**（`%t40`） |

**A/B 对照**（差异只在新代码）：

| 对照 | 原生 |
| --- | --- |
| `return not a` | `status=0 stdout="false\n"` ✓ |
| `return a < b` | `status=0 stdout="true\n"` ✓ |
| `return a and b` | **构建失败** ❌ |

### 2.2 根因定位：**在动态路径，不在静态 `ir.rs`**

审核用 IR 降低文本探针确认（用完即删）：三种源码——包括全静态的
`def f(bool a, bool b) -> bool / return a and b`——**全部产出 `dynamic.logical.*` 标签**，
且 `runtime=true`。⇒ **源码程序走的是动态路径**，静态 `ir.rs::emit_logical_short_circuit`
本次证据**没有覆盖**，不要照着它改。

已定位的一处：

```rust
// core/rust/crates/xiao-codegen-llvm/src/dynamic/expression.rs:171
let right = self.emit_condition(right)?;                       // 返回 i1
let right_value = self.emit_value_call("xiao_runtime_value_bool", &format!("i8 {right}"));
//                                                                 ^^^^^^^^^^^^^^ 把 i1 标成 i8
```

```rust
// core/rust/crates/xiao-codegen-llvm/src/dynamic/control.rs:966 / :976
let output = self.next_temp();
self.emit(format!("  {output} = trunc i64 {payload} to i1"));   // emit_condition 的结果是 i1
// …
self.emit(format!("  {output} = xor i1 {value}, true"));        // 「not」分支也是 i1
```

而 `xiao_runtime_value_bool` 的形参在 ABI 里是 **`i8`**（`dynamic/runtime_abi.rs:214`）。
长短两侧一致：**这就是 clang 报的 `defined with type 'i1' but expected 'i8'`。**

**同时要检查**：同一个新增函数的**左侧**与**短路分支**（`:163` 的 `"i8 1"` / `"i8 0"` 是常量，
大概率没问题，但左侧的 `br i1 {left}` 与它之前对 `left` 的处理要与右侧同口径）。
**静态 `ir.rs` 那条要单独验证**——源码路径测不到它，别用本节的结论替它背书。

### 2.3 最小复现

```xiao
def f(bool a, bool b) -> bool
    return a and b

print(f(true, false))
```

### 2.4 修法与判据（**冻结**）

1. **判据不是「clang 不报错」**，而是「**同一个位置换成既有的布尔生产者也一样**」：
   把 `and` 换成 `==`（`a == a`）或 `not`（`not a`），**两者都必须能构建并运行**——
   新代码必须与既有布尔生产者**共用同一种值表示**，不许自造一套再补一个 `zext`；
2. **两侧同批修**：动态路径与静态 `ir.rs` **同一提交**落地，不许只修一半；
3. **负例**：修完后，把 `and` 与 `or` **各测一遍**，且**至少一例的右侧带副作用/会出错**
   （`false and explode(0)`）——**只测静态结果不算测到了短路**；
4. **正向断言必须过一次 clang**（见 §5.1）——否则同一个缺口还会再漏一次。

## 三、N-2 **`value = 2 ** 3`（裸赋值）原生不支持**

### 3.1 事实（隔离实测）

| 形态 | 原生 |
| --- | --- |
| `value = 2 ** 3`（**裸赋值；正是守卫的探针源码**） | **`原生后端不支持 动态整数幂（8..14）`** ❌ |
| `value = 2 * 3`（同形状对照） | `status=0` ✓ |
| `value = 2 ** 3` **加一句无关的 `print("ok")`** | `status=0` ✓ |
| 函数体内 `2 ** 3` / `a ** b` / `a ** 3` | 全 ✓ |

⇒ 触发与**语句上下文**有关，不是「`**` 不支持」这么简单。

**机制（审核用 IR 降低文本探针查实，用完即删）**：

| 形态 | `entry_mode` | 降低结果 |
| --- | --- | --- |
| `value = 2 ** 3`（裸） | `Script` | **降低阶段报错** |
| `value = 2 ** 3` + `print("ok")` | `Script` | 成功；模块里出现 **`c"pow\00"` 常量**（动态算子名表） |
| `value = 2 ** 3` + `print(value)` | `Script` | 成功；同上 |
| `value = 2 * 3` | `Script` | 成功；**无** `pow` |

⇒ **四条都是 `Script` 入口**，所以不是入口模式切换；**能跑的三条走的是动态算子名表那条路，裸赋值那条在降低阶段就被拒**。

**站点**：`xiao-codegen-llvm/src/ir.rs:1687`（`fn power` 的整数分支）——

```rust
// Dynamic integer exponents need a dedicated loop and are deliberately deferred to the
// Runtime batch. Constant exponents still exercise checked integer multiplication.
let _ = result;
Err(unsupported("动态整数幂", Some(span)))
```

**这里有一处「注释与实现不一致」**：注释说**常量指数走受检整数乘法**，而 `2 ** 3` 的指数**正是常量**，却在裸赋值形态下走到了这条拒绝。⇒ 要么常量指数那条路**没有覆盖这个形态**，要么它在到达 `power()` 之前就被别处拦下了。**修的时候先把这一点查清并在实施记录里写明**——这正是本仓库反复记过的「注释与实现不一致比缺失更难查」。

### 3.2 归属：**不是本次引入**

`167a03e` 只动了 `ir.rs` 的 `matches!`（把 `and`/`or` 从拒绝改成短路降低），没碰 `**`。
这是**既有缺口被新守卫首次抓到**——守卫在干活，这一点要写清楚，别记成本次引入的回归。

### 3.3 **审核侧的账（同一类错的第三次）**

[20C-语言边界](20c-language-boundary-and-operators.md) §1.3 写的「其余
`BinaryOperator` / `UnaryOperator` 变体**已逐个实测通过**」——审核当时用的是
**函数内、参数形态**，**没覆盖顶层裸赋值**。**覆盖度小于结论宽度**，
与 [20C-第二批](20c-second-batch-probe.md) 的 G-4（压在 4 行样本上的结论）同型。

**冻结**：§1.3 那张表**加一栏「语句上下文」**（顶层裸赋值 / 顶层带输出 / 函数体内），
或**明确写清它只覆盖函数内形态**。**不许**让一张表同时读成两种覆盖。

### 3.4 处置：**裁定「修降低」**（星崽直接裁定，2026-10-14）

**裁定**：**修降低**——让顶层裸赋值形态也能构建。**不移出接受面。**
**来源**：星崽直接裁定（原话「N-2 修降低」）。

**冻结**：

1. **不动的部分**：`**` **留在接受面**，守卫里继续期望 `Success`；
   **不许**改成「按形态拒绝」——那会让同一个算子在两个语句上下文里有两种身份；
2. **要查清的部分**：`ir.rs:1687` 上方注释所说的「常量指数走受检整数乘法」在裸赋值形态下
   为什么没接上（见 §3.1 的机制表）。**先查清、写进实施记录，再改**；
3. **改完必须两形态都过**：`value = 2 ** 3`（裸）与 `value = 2 ** 3` + `print("ok")`
   **都要能构建、运行**，且**整数幂的结果与 VM 一致**（`2 ** 3 = 8`）；
4. **负例**：改完之后，把 `value = 2 ** 3` 这一条**放回守卫的第一项**（它现在就在第一项）——
   守卫必须**绿**；若仍红，说明只修了另一条形态；
5. **顺带**：`2 ** 3` 是**常量指数**，若最终确认常量指数这条路的实现与注释不符，
   **注释与实现同批对齐**（§2.6 的老教训）。

### 3.5 连带：守卫**一红即停**

`Power` 是 `BinaryOperator::ALL` 的**第 1 项** ⇒ 旧版守卫 panic 后**后面 20 个变体一个都没跑到**。
「穷举」在实践中退化成「只测第一个红的」。

**冻结**：守卫**收集全部失败再一次性报**（跑完全部再断言），否则一次只能暴露一个缺口。

## 四、N-3（**10R 的账**）硬件故障退出码跨平台不一致

### 4.1 事实（Linux 作业 CI 原文）

```text
thread 'optional_external_illegal_instruction_remains_fatal' panicked at n0_a.rs:998:5:
assertion `left == right` failed
  left: Some(132)
 right: Some(139)
stderr: xiao-error class=fatal code=X07-FATAL-005 message_id=fatal.hardware params={} span=<none> exit_code=132
```

### 4.2 根因：`xiao-runtime/src/crash.rs` 两侧口径不同

| 平台 | 行为 | 出处 |
| --- | --- | --- |
| **Unix** | **按信号分派**：`SIGSEGV`/`SIGBUS` → `139`、`SIGFPE` → `136`、**`SIGILL` → `132`**、其他 → `134` | `crash.rs:54-66` |
| **Windows** | **一律 `139`**，不区分故障类别 | `crash.rs:121,132` |

`llvm.trap` → `ud2` → **`SIGILL`** ⇒ **Linux 报 `132` 是对的**。
而测试 `n0_a.rs:998` 写死 `Some(139)` ⇒ **Windows 绿（因为 Windows 硬编码 139）、Linux 红**：
**测试把 Windows 的行为当成了跨平台契约。**

### 4.3 归属与性质

- 引入提交：`24f3f9b fix(10r): 接通静态数值错误 Runtime 通道`（`git log -S` 确认），
  **不是 `167a03e`**；
- 它是 `#[ignore]` 用例，只在原生作业里跑，而**上一次维护回归是 2026-10-11**
  ⇒ **本次是它的第一次 CI 执行**；
- **没有任何文档冻结过这个码**（检索 09B0D / 10E / 10R 无命中）⇒ 这不是「实现违背契约」，
  是**契约本身缺失**。

### 4.4 修法：**裁定「按故障类别分派」**（星崽直接裁定，2026-10-14）

**裁定**：取「**按故障类别分派**」——**Unix 侧不改**（现状即正确），**Windows 侧改成按类别分派**。
**来源**：星崽直接裁定（原话「N-3 按故障类别分派」）。

**冻结**：

1. **Unix 侧原样保留**：`SIGSEGV`/`SIGBUS` → `139`、`SIGFPE` → `136`、`SIGILL` → `132`、
   其他 → `134`（`crash.rs:54-66`）。**不许**为了迁就 Windows 把它压平；
2. **Windows 侧要按异常码分派**——**好消息是管道已经在了**：
   `crash.rs` 的 `unhandled_exception(_: *mut ExceptionPointers)` **把参数整个丢掉了**（参数名是 `_`），
   而它指向的 `ExceptionRecord` **已经带了 `code: u32` 字段**（`crash.rs:85-94`）。
   ⇒ 修法是**把那个 `code` 用起来**，按它分派到与 Unix **相同的四个码**，
   **不是**新造一套 Windows 专有的映射；
3. **映射必须与 Unix 一一对应**（示意，实现时按常量名核对）：
   访问违例 → `139`、非法指令 → `132`、整数/浮点除零 → `136`、其余 → `134`。
   **映射表要写进实现旁的注释**，并被用例覆盖；
4. **测试 `n0_a.rs:998` 的期望要从 `Some(139)` 改成 `Some(132)`**——
   该夹具用 `llvm.trap`（→ `ud2` → **非法指令**），两侧都应是 `132`；
   **这是基线变更**（该路径对外可见的退出码在 Windows 上由 `139` 变 `132`），
   按 [20C](20c-official-xiao-library.md) §2.4 的既定口径**单独登记、不许悄悄替换**；
5. **判据是「两平台一致」**；且**要分别用真实的 `ud2`（非法指令）与空指针解引用（`SIGSEGV`）各测一次**——
   只测一条路径等于没测分派；
6. **写死一个码的写法不许再出现**：新用例**至少覆盖两个不同类别**，
   否则「分派」这件事本身还是没被验证。

## 五、让下一次能自动发现（五条执行点）

| # | 缺口 | 冻结 |
| --- | --- | --- |
| **5.1** | 两处新增**正向**断言只看 LLVM **文本**（`n0_a.rs` 的 `contains("logical.merge")`、`n0_b_dynamic_native.rs` 的 `contains("dynamic.logical.right")`），**从不交给 clang** | 正向断言**至少过一次 `llvm-as` 或完整构建**——「文本对」与「能编译」之间的差，正是 N-1 藏身之处 |
| **5.2** | 工作流**没有 `--no-fail-fast`** ⇒ `cargo test` 遇第一个失败的测试二进制即停。Linux 上 `n0_a` 先挂，**c20 的穷举守卫与夹具探测在 Linux 上一条都没跑** | 两处原生作业**已加**（`fb1f435`），**workspace 门禁也已按裁定补上**（§5.2.1）；**否则「某平台绿」可能只是「没跑到」** |
| **5.3** | `ProbeExpectation::Backend` 曾无任何用例构造，靠 `#[allow(dead_code)]` 压住 ⇒ 后端拒绝那段处理（含 VM `X09-BYTECODE-001` 断言）**无执行点** | 已加入 `backend-negative-member-value` 故意负例，撑住 VM/原生后端拒绝分支；**但 `#[allow(dead_code)]` 与过期注释仍在** ⇒ 见 §11 **F-C** |
| **5.4** | 裁定 C（**右侧未求值不产生释放计划**、双路径 `release_plans` 逐字一致）原先**无执行点**：`d19a_differential` 里没有任何逻辑算子 | 已补 `logical_short_circuit_keeps_shared_release_plans`——**但它的 `release_plans` 断言是同义反复，裁定 C 的核心仍无执行点** ⇒ 见 §11 **F-A** |
| **5.5** | `BINARY_OPERATORS` 曾是**手写的 21 项数组**（`match` 有编译器强制，**数组没有**） | 已搬到 `BinaryOperator::ALL`/`UnaryOperator::ALL` 的枚举旁，但仍是手写清单；新增变体必须同时核对中心清单、`as_str` 与穷举匹配 ⇒ 见 §11 **F-D** |

### 5.2.1 **workspace 门禁也要加 `--no-fail-fast`**（星崽直接裁定，2026-10-14）

**裁定**：`--no-fail-fast` **同时加进 workspace 门禁**，不只两处原生作业。
**来源**：星崽直接裁定。

**落点（更正）**：门禁的实际执行体是 **`tools/gates/run.sh`**，
`.github/workflows/workspace-gates.yml` 只是调用它。**参数要落在脚本里，不是 YAML 里。**
（本文上一版把落点写成 YAML 的 `cargo test` 步骤——**审核侧自己的笔误**，已改正。）

**为什么这一条在这里同样成立**：`cargo test` 默认**遇第一个失败的测试二进制即停**。
`run.sh:12` 的 `cargo test -p xiao-driver` 会拉起**多个测试二进制**
（`c20_capability_probe`、`20c_first_modules`、`d19a_differential`、…），
不关掉 fail-fast，**排在失败者后面的那些二进制根本不会跑**。

**覆盖面（必须写清，否则读者会读错）**：这个门禁**不是**整个 workspace——
它跑的是 **`-p xiao-driver` + `tests/benchmarks`** 两个 `cargo test`，
`xiao-types` / `xiao-codegen-llvm` / `xiao-vm` 等包的测试**不在其中**（那些走维护回归的 `--workspace`）。
**「workspace 门禁」是它的历史名字，不是它对覆盖面的承诺。**

**冻结**：

1. 两个 `cargo test` 都加 `--no-fail-fast`（**已落地**，`run.sh:12` 与 `:17`）；
2. **跨步骤的遮蔽还没治——这是本条现在最要紧的一半**：
   脚本第 3 行是 `set -Eeuo pipefail`，所以**第 12 行一红，第 13–21 行全部不跑**。
   被遮住的恰恰是**纯静态门禁**：`bun run check`（文档与清单）、`cargo fmt --check`、`check:coverage`。
   ⇒ **二选一**，并在实施记录里写明选了哪个：
   - **把纯静态步骤前置**（`fmt`、`bun run check` 移到 `cargo test` 之前），
     让它们不被 Rust 测试失败遮住；或
   - 让脚本**收集全部失败再退出**（去掉 `-e` 的提前退出，逐条记结局）。
   **判据**：故意让 `-p xiao-driver` 的一个测试二进制红，
   **同一 `cargo test` 内它后面的二进制必须仍跑**，**且（若选前置）静态门禁也必须仍跑**；
3. **判据不许写成「证明原本排在失败点之后的 crate 也跑了」**——那个形状在**这个脚本里不存在**
   （它不遍历 crate）。这是本文上一版按原生作业的 `--workspace` 写下的措辞，**已按脚本的真实结构改正**；
4. 本条与 §5.2 同源，**两条一起看**：先有「跑到」，才有资格谈「绿」；
5. **一条否定式口径（必须写死）：全绿运行不算本条的取证。**
   fail-fast 只在**有失败**时才显出差异——全绿时两种行为完全一样。
   ⇒ **「脚本带上参数仍然正常」与「参数治好了遮蔽」是两件事**，
   前者可以由任意一次绿跑证明，**后者只能由一次故意的红证明**。
   **不许**用运行号 `38522338905`（或任何绿跑）把本条写成已取证。

**实施结果（2026-10-14）**：

- **第 1 条已落地**：`run.sh:12` 与 `:17` 都带了 `--no-fail-fast`；
- **门禁在带上该参数后仍正常**：`5d32140` 的 workspace 门禁
  [38522338905](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/38522338905) **success**，
  日志 25 行 `test result:`、0 失败；
- **前次记录**：第 2 条（跨步骤遮蔽）与第 5 条（故意的红取证）当时仍待做；
  那次是**第一次真跑**且全绿，按第 5 条不构成取证。残余项已在本轮完成，见下方运行号。

**本轮残余修复（2026-10-14，方案甲）**：`tools/gates/run.sh` 已把
`cargo fmt --check`、`bun run check`、`check:coverage` 前置到所有 Rust 测试之前；
benchmark 检查/测试/自测也排在 `cargo test -p xiao-driver` 之前，两个 cargo 测试仍保留
`--no-fail-fast`。一次性分支从 `origin/main` 起步，提交 A `3dc0d82` 只破坏
`20c_first_modules` 的一条输出断言；workspace 门禁红运行
[38534726488](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/38534726488)
显示该二进制失败后仍出现 `Running tests/artifact_size_regression.rs`，随后继续运行
`c20_capability_probe` 等二进制，且 `bun run check`、覆盖率输出均已在失败前出现。
提交 B `5b693cd` 还原同一文件，绿运行
[38535786407](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/38535786407)
全绿；该运行还通过步骤标签逐字核对了 `cargo fmt --check`、`bun run check`、
`check:coverage` 位于 benchmark 与 `xiao-driver` 测试之前。提交 A 只触碰 Rust 测试文件，
由于 workflow 的 `push.paths` 未包含该路径，红运行按同一分支手动 dispatch。

## 六、审核侧自己的账：§3.2.1 第 6 条**判据是错的**

[20C-语言边界](20c-language-boundary-and-operators.md) §3.2.1 第 6 条冻结的是
「把三处里**任意一处**改回去 ⇒ 拒绝面断言必须变红」。审核变异实测：

| 变异 | 守卫 |
| --- | --- |
| 只关 `xiao-types/src/numeric.rs:213` 的拒绝 | **仍绿** |
| 只关 `xiao-types/src/checker/expression.rs:211` 的提前闸门 | **仍绿** |
| **两处都关** | **红**（`c20_capability_probe.rs:383`「身份算子必须在前端拒绝：binary-is」） |

⇒ 两处是**冗余的纵深防御**（这是好事），**「任意一处」的判据不成立**。

**冻结**：第 6 条改成「关掉**全部**拒绝点 ⇒ 变红」。

**顺带**：这次红也**独立验证了 §5.4 的新身份**——落回后端时报
`code = X07-FATAL-006`、`message_id = fatal.internal`、
文案「生产执行前字节码降低一致性验证失败」✓。

## 七、两处**与事实不符的声明**（同一提交内）

1. `tests/fixtures/20c/README.md:21` 曾写「`second-case-conversion-logic.xiao` …
   **VM 与 LLVM 原生输出 `true`**」——修复前实测原生构建失败（N-1），该声明把「期望」写成了「结果」。
   当前已改为「VM 输出 `true`；LLVM 原生待受控运行号」；
2. [20C-语言边界](20c-language-boundary-and-operators.md) 状态头曾写
   「**`tests/fixtures/20c/README.md:14` 已与夹具语义同步**」但当时 `:14` 未改，仍是宽泛的
   字符串比较描述；当前已同步为「字面量常量折叠的相等与字典序比较」。

**冻结**：两处**改回事实**；并且**未跑过的平台不许在 README 行里写成结果**——
按 [20C](20c-official-xiao-library.md) 的既有口径写「待原生运行号」。

**本地实施回写（2026-10-14）**：N-1 已在动态 LLVM 路径把条件 `i1` 零扩展为 Runtime ABI 所需的
`i8`，静态路径同时保留并加入 `llvm-as` 可选校验；N-2 已将常量非负整数指数展开为带溢出检查的
整数乘法，动态指数仍保留原有 Runtime 路径。守卫清单现在从 `BinaryOperator::ALL`/
`UnaryOperator::ALL` 派生，并附带一条真实的后端拒绝负例；VM 守卫收集全部失败后一次报告。
N-3 已接入 Windows `ExceptionRecord.code` 分派，并补非法指令/访问违例两类测试。该段记录当时
受控 LLVM 门禁尚未复跑；本轮 workspace 遮蔽取证与恢复运行号见 §5.2.1。

## 八、落点

```text
docs/DevDocs/20c-native-gates-red-fix.md                本文
core/rust/crates/xiao-codegen-llvm/src/dynamic/expression.rs   N-1：右侧 bool 表示（`:171` 及其左侧/短路分支）
core/rust/crates/xiao-codegen-llvm/src/ir.rs             N-1：静态短路那条**单独验证**后同批修；
                                                         N-2（裁定修降低）：`fn power` 的整数分支（`:1687`）与其上方注释
core/rust/crates/xiao-codegen-llvm/src/dynamic/control.rs      N-1：`emit_condition` 的 i1 口径（只读，确认表示）
core/rust/crates/xiao-codegen-llvm/src/dynamic/runtime_abi.rs  N-1：`xiao_runtime_value_bool` 的 `i8` 形参（只读）
core/rust/crates/xiao-driver/tests/c20_capability_probe.rs      N-2：`**` 探针的期望；守卫跑完全部再报；§5.1 过 clang；§5.3 后端负例；§5.5 清单派生
core/rust/crates/xiao-codegen-llvm/tests/n0_a.rs         §5.1：正向断言过 clang；N-3：`:998` 期望改 `132` + 至少两个故障类别
core/rust/crates/xiao-driver/tests/n0_b_dynamic_native.rs §5.1：同上
core/rust/crates/xiao-runtime/src/crash.rs               N-3：Windows 侧用起 `record.code`（`:85-94`）按类别分派（§4.4）；Unix 侧**不改**
core/rust/crates/xiao-driver/tests/c20_capability_probe.rs §5.4：双路径释放序列对账用例
.github/workflows/maintenance-regression.yml             §5.2（已落地）：两处原生作业加 `--no-fail-fast`
tools/gates/run.sh                                      §5.2.1：两个 cargo test 加 `--no-fail-fast`（已落地）；
                                                         第 2 条：选择方案甲，纯静态步骤前置（已落地并取证）
core/rust/crates/xiao-codegen-llvm/src/dynamic/expression.rs  §11.2 F-B：注释改成事实
core/rust/crates/xiao-driver/tests/c20_capability_probe.rs    §11.1 F-A：释放序列造可变量；§11.3 F-C：删 `allow(dead_code)` 并改注释
core/rust/crates/xiao-syntax/src/ast.rs                  §11.4 F-D：`ALL` 上方注明它是手写清单、改枚举时必须核对
docs/DevDocs/20c-language-boundary-and-operators.md       §1.3 加「语句上下文」栏；§3.2.1 第 6 条判据；状态头改回事实
docs/DevDocs/10r-static-overflow-implementation.md       N-3 的归属与契约结论回填
docs/DevDocs/09b0-bytecode-closure.md                    §5.2 结论回填（本专项的三个真运行号）
tests/fixtures/20c/README.md                             §七 第 1 条：`:21` 改回事实，`:14` 同步
```

## 九、验收

1. **main 转绿**：两个原生作业**都**绿——**维护回归 `38493485483`**（Windows `115623868914` / Linux `115623869640`）+
   **workspace 门禁 `38493485411`**，数字已回填 §一；
2. **N-1**：`and`/`or` 在**两条路径**上都能构建、运行、输出一致；
   且**「同一位置换成 `==`／`not` 也一样能构建」**这条判据成立；
   **至少一例的右侧带副作用**（证明短路，而不只是静态结果对）；
3. **N-2**（裁定「修降低」）：`**` **仍在接受面**；`value = 2 ** 3` 的两种形态（裸赋值 / 加 `print`）
   **都必须能构建运行**且整数幂结果与 VM 一致；`ir.rs:1687` 注释与实现已对齐；
   守卫**跑完全部再报**（不再一红即停）；
4. **N-3**（裁定「按故障类别分派」）：**Unix 侧未改**；Windows 侧按 `record.code` 分派到**相同**四个码；
   `n0_a.rs:998` 的期望为 `132`；**至少两个不同故障类别**各有用例；
   退出码的基线变更**已单独登记**；
5. **§5.1**：至少一条正向断言**真的过了 clang**（不是只看文本）；
6. **§5.2**：两处原生作业加 `--no-fail-fast`，并**用一次真实运行证明 Linux 上 c20 也跑了**；
7. **§5.3/§5.4**：`Backend` 变体要么删、要么有负例撑住；`release_plans` 对账有用例或已登记未覆盖；
8. **§5.5**：清单派生或加覆盖断言；
9. **§六**：§3.2.1 第 6 条已改为「关掉全部拒绝点」；
10. **§七**：两处声明已改回事实；`README:14` 已同步；
11. **§5.2.1**：两个 `cargo test` 已加 `--no-fail-fast`（已落地）；
    **且跨步骤遮蔽已治**——故意让 `-p xiao-driver` 的一个测试二进制红时，
    **同一 `cargo test` 内它后面的二进制必须仍跑**，**纯静态门禁（`fmt` / `bun run check`）也必须仍跑**；
    落点写的是 `tools/gates/run.sh`（**不是** workflow YAML）；判据**不许**写成「crate 也跑了」（该形状不存在）；
    **且不许用任意一次绿跑当作本条的取证**——fail-fast 只在有失败时才显出差异（§5.2.1 第 5 条）；
12. **§十一 四笔返工**：**F-A** 释放对账**造出可变量**；
    **F-A 追加（§11.1.1）**：**短路块的 `Release` 数必须为 0**——把右侧释放挪到分支前必须让它变红；
    **F-B** 抽成共享 helper 或注释改成事实；**F-C** `#[allow(dead_code)]` 已删、
    注释已改、且删后若报 dead_code 是补用例而不是把 allow 加回来；
    **F-D** `ALL` 上方注明手写性质，§5.5 的措辞已从"已派生"改成事实；
13. **通用**：`cargo test --workspace`、clippy、fmt、`bun run check`、`bunx tsc --noEmit` 全绿；
    提交标题带类型前缀、正文非空。

## 十、裁定结果与仍然待定

### 10.1 已裁定（星崽直接裁定，2026-10-14）

| # | 问题 | 裁定 | 落点 |
| --- | --- | --- | --- |
| 1 | **N-3** 的硬件故障退出码契约形态 | **按故障类别分派**——Unix 侧原样保留，Windows 侧按异常码分派到相同四个码 | §4.4 |
| 2 | **N-2** 的处置 | **修降低**——`**` 留在接受面，让顶层裸赋值形态也能构建 | §3.4 |
| 3 | **`--no-fail-fast` 的适用范围** | **workspace 门禁也加**——不只两处原生作业 | §5.2.1 |

**来源**：星崽 **直接裁定**（原话「N-3 按故障类别分派。N-2 修降低」与「`--no-fail-fast` 加进 workspace」）。
两条都**不是**「与审核倾向一致」的推定——N-2 尤其要注意：审核的倾向原本是**移出接受面**，
**裁定取的是相反的一条**，按 G-2 的口径这不影响其效力，但**不许在后续文档里把它写成「按审核建议处理」**。

### 10.2 后续输入

1. **`main` 转绿的方式**：**已按修前进落地**（`fb1f435`），绿色运行号见 §一；
2. **`--no-fail-fast` 加进 workspace 门禁**：**已裁定**（星崽直接裁定，2026-10-14），
   落点与判据见 **§5.2.1**；
3. **第二轮审核并入的四笔返工**（**没有一条是红**）**已由 `c595c82` 完成**，
   实施记录见 **§十一**：**F-A**（中，释放序列改为可变量对照）、
   **F-B**（低，注释改为事实）、**F-C**（低，删除 `allow(dead_code)`）、
   **F-D**（低，`ALL` 手写性质和维护代价已注明）；
4. **第三轮审核并入的四笔**（同样**没有一条是红**）见 **§十二**：
   **第三轮-1** 跨步骤遮蔽未治（`set -Eeuo pipefail`，被遮的是纯静态门禁）；
   **第三轮-2** §5.2.1 的落点与论证前提写歪（**审核侧自己的账**）；
   **第三轮-3** 该节判据在脚本里不可达；**第三轮-4** F-A 的新断言只比总数不比位置。

## 十一、第二轮审核（审 `fb1f435`）并入的四笔返工

出处：2026-10-14 审核 `fb1f435`（三条红的修复）。**四笔没有一笔是红**——
它们不阻塞 `main`，但**都让某条判据名不副实**，所以按返工项走、不按"留痕"走。

### 11.1 **F-A（中）**：`release_plans` 对账是**同义反复**，裁定 C 的核心仍无执行点

**修复前事实**（`c20_capability_probe.rs::logical_short_circuit_keeps_shared_release_plans`）：

```rust
let source = "…result = false and right()\n";
let vm_artifact     = compiler.compile(&FrontendRequest::from_text(source)).expect(…);
let native_artifact = compiler.compile(&FrontendRequest::from_text(source)).expect(…);
assert_eq!(vm_artifact.ir.ownership.release_plans, native_artifact.ir.ownership.release_plans,
           "两条路径必须消费同一份 release_plans");
```

**同一份 `source`、同一个编译器、两次 `compile`** ⇒ 相等是**构造性必然**：
这条断言与「双路径」无关、与「短路」也无关，**它永远不会红**。
⇒ §5.4 名义上有了执行点，**裁定 C 的核心（右侧未求值 ⇒ 不产生释放计划）仍然没被测到**。

**该用例真正有效的部分是后半段**——断言 `VmEvent::FunctionEntered { function: "right" }`
**从未出现**，那是真的短路验证，保留。

**冻结**：二选一，并在实施记录里写明选了哪个——

1. **造出可变量**：比**同语义下「带右侧」与「不带右侧」**两份程序的 `release_plans`
   （例如 `result = false and right()` 对 `result = false`），差异必须能用一句话解释；
2. **比降低后的释放序列**：在 TAC / LLVM 侧比，而不是比前端 IR。

**判据**：改完之后，**把右侧的求值去掉**（让短路不再发生），断言**必须变红**。
**只断言「两次编译相等」不算对账**——那两条路径压根还没分叉。

**实施结果（2026-10-14）**：已选择方案 2（并用方案 1 的对照臂）。测试现在用
`false and ("a" + "b" == right_text)` 与去掉右侧的顶层对照，分别比较 TAC `Release` 指令数和
LLVM Runtime release 调用数；两条后端都证明去掉右侧会减少释放序列，且短路运行仍通过。
后半段的「`right` 从未被进入」保留。

**实现是对的——审核实测（2026-10-14）**：审核 dump 了该程序的降低后 TAC 块结构（临时探针，用完即删）：

```text
block[0]  EnterScope, LoadConst, Move, Release, LoadConst, BranchIf      ← 左侧；分支前归还左侧
block[1]  LoadConst, LoadConst, Arith, Compare, Copy, Release×3, Jump    ← 右侧块：3 条释放全在这里
block[2]  LoadConst(ConstId(1)), Copy, Jump                              ← 短路块：releases = 0 ✓
block[3]  Copy, RunReleasePlan, ExitScope, Return                        ← 合流块：releases = 0 ✓
```

> 这是原始 dump 的**重排摘录**（原输出带 `VReg`/`BlockId` 等寄存器号与操作数）；
> **指令名与「每块 releases 数」逐字来自 dump**，箭头的解释是审核加的。

⇒ **裁定 C 在实现上成立**：右侧的释放**全在右侧块内**，**短路块一条都没有**。
（审核原本怀疑「释放被提到分支前」，**实测不成立**。）

### 11.1.1 **F-4（低，第三轮审核）：新断言只比总数、不比位置**

**事实**：新断言是 `count(with) > count(without)`（TAC 与 LLVM 各一条）。
**把 `block[1]` 的 3 条 `Release` 挪到 `block[0]`（分支前）**，总数仍是 4，`4 > 1` **照样绿**——
而**短路路径从此会释放从未创建的值**，正是裁定 C 要禁的那一类。

> 口径：**「实现是对的」来自上面的 TAC dump（实跑）**；
> **「断言对位置不敏感」是计数不变性的论证**（同一程序内指令重排不改变同类指令总数），**不是实跑**。
> 两者分开标，免得把论证当成证据。

**判定**：**实现不必改**；要补的是**判据的强度**。当前断言覆盖了
「右侧完全没释放」（会红）与「右侧被提前求值」（由后半段那条红），
**覆盖不到「释放被挪出右侧块」**。

**冻结**：补一条**位置**断言——上面 dump 出的形状是可判定的：

1. **TAC 侧**：**短路块的 `Release` 数必须为 0**（即 `logical.short` 对应的块内不得有 `Release`）；
2. **LLVM 侧**：右侧的 `xiao_runtime_value_release_` 必须出现在 `logical.right` 标签**之后**。

**判据**：加上这条后，把右侧的释放人为挪到分支前，断言**必须变红**。
**只比总数不算覆盖位置**——那正是 F-4 记的这一条。

**本轮实施结果（2026-10-14）**：`c20_capability_probe.rs` 已追加 TAC 短路块
`Release` 数为 0 的断言，以及 LLVM `xiao_runtime_value_release_` 位于
`dynamic.logical.right` 至 `dynamic.logical.merge` 区间内的断言；原有 TAC/LLVM 总数断言保留。
在 `xiao-bytecode/src/lower/expr.rs::lower_logical` 的 `short_block` 临时插入一条
`TacOp::Release` 后，测试在短路块位置断言处失败；还原后完整探测测试恢复通过。

### 11.2 **F-B（低）**：N-1 的注释把新引入的转换写成了既有约定

**修复前事实**：`dynamic/expression.rs` 新增的注释称该 `zext i1 → i8` 是
「the same zero-extension shape as **every other bool producer**」，
而**全 crate 里 `zext i1 … to i8` 只有这一处**——既有的布尔生产者要么直接发 `i8` 常量
（`constructor_call("xiao_runtime_value_bool", "i8", "1")`），要么根本不跨这道 ABI 边界。

**判定**：**转换本身是对的**（计算得到的 `i1` 跨到 `i8` 形参，只能 `zext`），
问题在注释把「新加的一处」说成了「已有惯例」。

**冻结**：

1. 二选一：**抽成共享 helper**（例如 `emit_bool_to_abi`，让下一处"计算得到的 bool 跨 ABI"
   直接复用，不再各处手写 `zext`）**或**把注释**改成事实**
   （「这是唯一一处把计算得到的 `i1` 跨到 bool ABI 的地方」）；
2. 若选 helper：**下一次同类转换出现时，必须复用它**——否则就是补丁叠补丁。

**实施结果（2026-10-14）**：选择改注释为事实，明确这是当前唯一把计算得到的 `i1` 跨到 bool ABI
`i8` 的位置；转换本身保持不变。

### 11.3 **F-C（低）**：`#[allow(dead_code)]` 与它的注释已过期

**修复前事实**：`ProbeExpectation` 上仍挂着

```rust
// 当前裁定后的在册算子没有后端拒绝项，但三态仍需保留给后端负例探针。
#[allow(dead_code)]
enum ProbeExpectation { … }
```

而 **`Backend` 现在有真实用例**（`c20_capability_probe.rs:202` 的 `backend-negative-member-value`）。
⇒ 这个 `allow` 已经**什么都没在压**，但它的**副作用还在**：
把「枚举有变体没人构造」这个信号**永久静音**——而那正是 §5.3 要防的东西。

**冻结**：

1. **删掉 `#[allow(dead_code)]`**；若删掉后编译报 dead_code，说明确有变体没人构造，
   **那就去补用例，不是把 allow 加回来**；
2. **同步改掉注释**：它现在描述的是「在册算子没有后端拒绝项」，与事实相反；
3. **加一条纪律**：`allow(dead_code)` **不许**用来"先让它编过"——
   本仓库已经有 §5.3 这个先例，同一个手法第二次出现就该进纪律。

**实施结果（2026-10-14）**：已删除 `#[allow(dead_code)]`，并把注释改为当前三态守卫事实；
`backend-negative-member-value` 保证 `Backend` 分支继续有真实执行点。

### 11.4 **F-D（低）**：`ALL` 清单仍是手写，只是搬了家

**事实**：`BINARY_OPERATORS` 从测试挪到了 `xiao-syntax/src/ast.rs` 的 `BinaryOperator::ALL`
（`UnaryOperator::ALL` 同理），紧挨枚举。**这是改进**——作者加变体时，
`as_str` 的**穷尽匹配就在旁边**，会先编译不过，位置对。

**但 completeness 仍非强制**：`ALL` 是手写的 21 项；加变体时「`as_str` 报错 → 补上」
未必等于「顺手也补了 `ALL`」。漏了的话，守卫**不报错、只是不跑那一条**。

**判定**：**纯 Rust 里没有廉价的全自动写法**（不引宏/derive 就数不出变体数），
所以**不要求做到强制**，但要求把代价写清。

**冻结**：在 `ALL` 上方补一句注释，写明它是**手写**清单、`as_str` 与守卫的穷尽匹配
**都不能保证它完整**，改枚举时**必须同时核对这里**。**并把 §5.5 从「已派生」改成事实措辞**
（「已搬到枚举旁，仍为手写」）。

**实施结果（2026-10-14）**：`UnaryOperator::ALL` 与 `BinaryOperator::ALL` 上方已写明手写性质、
完整性边界和维护纪律，守卫继续从两张清单读取。

## 十二、第三轮审核（审 `c595c82`）并入的四笔

出处：2026-10-14 审核 `c595c82`（第二轮四笔返工）。**四笔没有一笔是红**：
`c595c82` 把 F-A/F-B/F-C/F-D 都真改了，四道门禁与 `cargo test --workspace`（1136 passed / 0 failed）全绿。
以下四条是**判据与覆盖面**的修正，不撤销那四笔。

| # | 项 | 落在哪 |
| --- | --- | --- |
| **第三轮-1** | `--no-fail-fast` **只治了一半的遮蔽**：`tools/gates/run.sh:3` 的 `set -Eeuo pipefail` 让**跨步骤**仍是失败即停，被遮住的恰是 `bun run check` / `fmt --check` / `check:coverage` 这些**纯静态门禁** | §5.2.1 第 2 条——**已改成冻结项** |
| **第三轮-2** | §5.2.1 的**落点写错**（写成 workflow YAML，实际在 `tools/gates/run.sh`）、**论证前提与该脚本不符**（它只跑 `-p xiao-driver` + benchmarks，不是整个 workspace，也没有「crate 排队」这种形态） | §5.2.1 的落点段与覆盖面段——**已改正**（**审核侧自己的账**） |
| **第三轮-3** | §5.2.1 的判据「证明原本排在失败点之后的 crate 也跑了」在**这个脚本里不可达** | §5.2.1 第 3 条——**已按脚本真实结构重写** |
| **第三轮-4** | F-A 的新断言**只比总数、对位置不敏感**（实现是对的，见 §11.1 的 TAC dump） | **§11.1.1**——**已写成追加判据** |

**审核侧的口径纪律（本轮再记一次）**：§11.1.1 把
**「实现是对的」（实跑 TAC dump）** 与 **「断言不敏感」（计数不变性的论证）**
**分开标注**——同一篇里既不能拿论证当证据，也不能因为论证顺手就省掉实跑。

**返工后的 CI 取证（2026-10-14）**：`5d32140` 的维护回归
[38522338923](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/38522338923)
五作业全绿、workspace 门禁 [38522338905](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/38522338905) 绿——
**§十一 的四笔与 §十二 的措辞更正都经受了 CI**。
本轮已闭环：**§5.2.1 第 2 条**（跨步骤遮蔽）以故意红运行
[38534726488](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/38534726488)
取证，并由还原后的绿运行
[38535786407](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/38535786407)
确认恢复；**§11.1.1** 的位置断言以临时 `TacOp::Release` 变异取证并已还原。
§5.2.1 第 5 条仍有效：全绿运行本身不构成遮蔽修复的取证。

**「绿跑」在本专项里能证明什么、不能证明什么（写死，免得后人误用）**：

| 绿跑能证明 | 绿跑**不能**证明 |
| --- | --- |
| 改动没把已通过的东西弄坏 | 某条修补**治好了它要治的遮蔽**（fail-fast 只在有失败时才显出差异） |
| 脚本带上新参数仍能跑完 | 某个断言**真的能红**（要故意破坏一次才算） |
| 两平台都跑到了同一批用例（可从日志的行数核） | 「未覆盖」的东西已经覆盖（那是 §5.3/§5.4 与 §11.1.1 的事） |

## 相关页面

- [20C-门禁残余项修复](20c-native-gate-residuals-fix.md) —— **本专项 §5.2.1 第 2 条与 §11.1.1 的执行清单**：
  两条残余项各自的最小实施/验证步骤与负例，单独成文
- [20C-语言边界批](20c-language-boundary-and-operators.md) —— 本专项的 §五 与 §2.2/§3.2.1 都是它的冻结
- [20C](20c-official-xiao-library.md) §2.4 —— 验证纪律、遮蔽教训、基线变更的登记口径
- [10R-实现](10r-static-overflow-implementation.md) —— N-3 的归属文档
- [10Z-驱动](10z-performance-driver.md) §2.7 D7 —— 「证据必须有执行点」；本文 §五 的出处
- [09B0](09b0-bytecode-closure.md) §3.3 —— 中心不变量与本专项三个真运行号的回填处
