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
> 两条处置已裁定——**N-2 修降低**、**N-3 按故障类别分派**（§10.1）。
> `N-1` / `N-2` 属本批（20C 语言边界），`N-3` 属 10R。
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
| **5.2** | 工作流**没有 `--no-fail-fast`** ⇒ `cargo test` 遇第一个失败的测试二进制即停。Linux 上 `n0_a` 先挂，**c20 的穷举守卫与夹具探测在 Linux 上一条都没跑** | 两处原生作业加 `--no-fail-fast`；**否则「某平台绿」可能只是「没跑到」** |
| **5.3** | `ProbeExpectation::Backend` 曾无任何用例构造，靠 `#[allow(dead_code)]` 压住 ⇒ 后端拒绝那段处理（含 VM `X09-BYTECODE-001` 断言）**无执行点** | 已加入 `backend-negative-member-value` 故意负例，撑住 VM/原生后端拒绝分支 |
| **5.4** | 裁定 C（**右侧未求值不产生释放计划**、双路径 `release_plans` 逐字一致）原先**无执行点**：`d19a_differential` 里没有任何逻辑算子 | `c20_capability_probe::logical_short_circuit_keeps_shared_release_plans` 已补双编译路径对账，并确认右侧函数不入栈 |
| **5.5** | `BINARY_OPERATORS` 曾是**手写的 21 项数组**（`match` 有编译器强制，**数组没有**） | 已从 `BinaryOperator::ALL`/`UnaryOperator::ALL` 派生；新增变体必须补中心枚举清单与穷举匹配 |

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
N-3 已接入 Windows `ExceptionRecord.code` 分派，并补非法指令/访问违例两类测试。受控 LLVM
门禁尚未复跑，故运行号与 `main` 绿色状态仍待外部执行。

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
core/rust/crates/xiao-driver/tests/d19a_differential.rs  §5.4：双路径释放对账用例
.github/workflows/maintenance-regression.yml             §5.2：两处原生作业加 `--no-fail-fast`
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
11. **通用**：`cargo test --workspace`、clippy、fmt、`bun run check`、`bunx tsc --noEmit` 全绿；
    提交标题带类型前缀、正文非空。

## 十、裁定结果与仍然待定

### 10.1 已裁定（星崽直接裁定，2026-10-14）

| # | 问题 | 裁定 | 落点 |
| --- | --- | --- | --- |
| 1 | **N-3** 的硬件故障退出码契约形态 | **按故障类别分派**——Unix 侧原样保留，Windows 侧按异常码分派到相同四个码 | §4.4 |
| 2 | **N-2** 的处置 | **修降低**——`**` 留在接受面，让顶层裸赋值形态也能构建 | §3.4 |

**来源**：星崽 **直接裁定**（原话「N-3 按故障类别分派。N-2 修降低」）。
两条都**不是**「与审核倾向一致」的推定——N-2 尤其要注意：审核的倾向原本是**移出接受面**，
**裁定取的是相反的一条**，按 G-2 的口径这不影响其效力，但**不许在后续文档里把它写成「按审核建议处理」**。

### 10.2 后续输入

1. **`main` 转绿的方式**：**已按修前进落地**（`fb1f435`），绿色运行号见 §一；
2. **仍然待定**：**`--no-fail-fast` 是否同时加进 workspace 门禁**？
   （本文 §5.2 只要求了两处原生作业；workspace 门禁跑的是默认测试，红在哪一条不影响其余）
3. **本轮审核留下的四条**（**没有一条是红**，都是留痕与判据卫生）：
   - **F-A（中）**：`logical_short_circuit_keeps_shared_release_plans` 的 `release_plans` 断言
     **是同义反复**（同一份 source 经同一个编译器两次 compile ⇒ 构造性相等），
     与「双路径」和「短路」都无关；**裁定 C 的核心仍无执行点**。真正有效的是它后半段
     「`right` 从未被进入」。见 §5.4 的追加要求；
   - **F-B（低）**：N-1 新注释称该 `zext` 是「every other bool producer」的既有形状，
     而全 crate 只有这一处 `zext i1 … to i8`；建议抽共享 helper 或把注释改成事实；
   - **F-C（低）**：`ProbeExpectation` 上的 `#[allow(dead_code)]` 与其注释（「在册算子没有后端拒绝项」）
     已过期——`Backend` 现有真实用例（`c20_capability_probe.rs:202`）。留着把
     「枚举有变体没人构造」这个信号静音，正是 §5.3 要防的；
   - **F-D（低）**：`BinaryOperator::ALL` 仍是手写 21 项（搬到 `ast.rs` 紧挨枚举是**改进**，
     加变体时 `as_str` 的穷尽匹配就在旁边会编译不过），但仍非强制；§5.5 未回填说明。

## 相关页面

- [20C-语言边界批](20c-language-boundary-and-operators.md) —— 本专项的 §五 与 §2.2/§3.2.1 都是它的冻结
- [20C](20c-official-xiao-library.md) §2.4 —— 验证纪律、遮蔽教训、基线变更的登记口径
- [10R-实现](10r-static-overflow-implementation.md) —— N-3 的归属文档
- [10Z-驱动](10z-performance-driver.md) §2.7 D7 —— 「证据必须有执行点」；本文 §五 的出处
- [09B0](09b0-bytecode-closure.md) §3.3 —— 中心不变量与本专项三个真运行号的回填处
