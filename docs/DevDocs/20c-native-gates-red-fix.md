# 20C-原生门禁修复专项（维护回归 `38484238759` 红）

> **背景**：`167a03e` 推送后，[维护回归 38484238759](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/38484238759)
> 的两个原生作业**都红**，`main` 当前不绿。这是 **2026-10-11 之后 34 个提交的第一次 CI 曝光**——
> 期间 10R D2 与 20C 的四个批次都没经过维护回归，所以红的不止一处、也不止一批。
>
> **一句话概括**：本文是三处红的**修复规格**——每条给出**真 CI 证据、根因定位到文件:行、最小复现、修法要求与负例**；
> 外加「让下一次能自动发现」的五条执行点。
>
> 状态（2026-10-14）：**待修**。`N-1` / `N-2` 属本批（20C 语言边界），`N-3` 属 10R。
> **N-1 与 N-2 未修之前 `main` 一直是红的**，与 [20C-语言边界](20c-language-boundary-and-operators.md) §5.3
> 「main 不许红」的既有纪律冲突。

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

**Windows 作业的 CI 原文（与审核本机一字不差）**：

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

⇒ 触发与**语句上下文/类型推断**有关，不是「`**` 不支持」这么简单。

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

### 3.4 处置（二选一，**不许两项都占**）

1. **修降低**：让顶层裸赋值形态也能构建，并补进 §5.1 的守卫；
2. **移出接受面**：按 [20C-语言边界](20c-language-boundary-and-operators.md) §3.2 乙的先例，
   在**指名的层**以稳定码拒绝，并登记为缺口。

**不许**既把它算进「已接受面」（守卫里期望 `Success`）、又让它构建不出来——
那正是本次红的原因，也是「接受面只保留语义已冻结的构造」这条裁定要消掉的东西。

### 3.5 连带：守卫**一红即停**

`Power` 是 `BINARY_OPERATORS` 的**第 1 项** ⇒ 守卫 panic 后**后面 20 个变体一个都没跑到**。
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

### 4.4 修法与验收（**冻结**）

1. **先定契约形态**（见 §十，需星崽裁定）：是「**按故障类别分派**」（Unix 现状，信息量更大）
   还是「**平台统一码**」（Windows 现状）——**定了再改一侧，不许两侧各改一半**；
2. **判据是「两平台一致」**；且**要分别用真实的 `ud2`（非法指令）与空指针解引用（`SIGSEGV`）各测一次**——
   只测一条路径等于没测分派；
3. 若判为基线变更（退出码是对外可见的），按 [20C](20c-official-xiao-library.md) §2.4
   的既定口径**单独登记、不许悄悄替换**；
4. **Windows 侧的 `139` 硬编码要单独评估**：对非法指令也报 `139`，等于**把故障类别说成段错误**，
   与「错误身份必须稳定且可区分」的既有要求冲突。

## 五、让下一次能自动发现（五条执行点）

| # | 缺口 | 冻结 |
| --- | --- | --- |
| **5.1** | 两处新增**正向**断言只看 LLVM **文本**（`n0_a.rs` 的 `contains("logical.merge")`、`n0_b_dynamic_native.rs` 的 `contains("dynamic.logical.right")`），**从不交给 clang** | 正向断言**至少过一次 `llvm-as` 或完整构建**——「文本对」与「能编译」之间的差，正是 N-1 藏身之处 |
| **5.2** | 工作流**没有 `--no-fail-fast`** ⇒ `cargo test` 遇第一个失败的测试二进制即停。Linux 上 `n0_a` 先挂，**c20 的穷举守卫与夹具探测在 Linux 上一条都没跑** | 两处原生作业加 `--no-fail-fast`；**否则「某平台绿」可能只是「没跑到」** |
| **5.3** | `ProbeExpectation::Backend` 已无任何用例构造，靠 `#[allow(dead_code)]` 压住 ⇒ 后端拒绝那段处理（含 VM `X09-BYTECODE-001` 断言）**无执行点** | 要么连变体一起删，要么留一条**故意的后端负例**撑住它（§4.2 自己写着「不许留成死代码」） |
| **5.4** | 裁定 C（**右侧未求值不产生释放计划**、双路径 `release_plans` 逐字一致）**无执行点**：`d19a_differential` 里没有任何逻辑算子 | 补双路径释放对账用例，或明确登记「本批未覆盖」——**不许让文档里的承诺一直悬空** |
| **5.5** | `BINARY_OPERATORS` 是**手写的 21 项数组**（`match` 有编译器强制，**数组没有**） | 清单从同一处派生，或加覆盖性断言——新增变体时「给了探针」与「真的会跑」是两件事 |

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

1. `tests/fixtures/20c/README.md:21` 写「`second-case-conversion-logic.xiao` …
   **VM 与 LLVM 原生输出 `true`**；右侧按需短路」——**实测原生构建失败**（N-1），
   这条声明把「期望」写成了「结果」；
2. [20C-语言边界](20c-language-boundary-and-operators.md) 状态头写
   「**`tests/fixtures/20c/README.md:14` 已与夹具语义同步**」——**`:14` 根本没改**，
   仍是「字符串相等与字典序比较」，而夹具只剩字面量常量折叠（本次提交只改了 `:21`）。

**冻结**：两处**改回事实**；并且**未跑过的平台不许在 README 行里写成结果**——
按 [20C](20c-official-xiao-library.md) 的既有口径写「待原生运行号」。

## 八、落点

```text
docs/DevDocs/20c-native-gates-red-fix.md                本文
core/rust/crates/xiao-codegen-llvm/src/dynamic/expression.rs   N-1：右侧 bool 表示（`:171` 及其左侧/短路分支）
core/rust/crates/xiao-codegen-llvm/src/ir.rs             N-1：静态短路那条**单独验证**后同批修
core/rust/crates/xiao-codegen-llvm/src/dynamic/control.rs      N-1：`emit_condition` 的 i1 口径（只读，确认表示）
core/rust/crates/xiao-codegen-llvm/src/dynamic/runtime_abi.rs  N-1：`xiao_runtime_value_bool` 的 `i8` 形参（只读）
core/rust/crates/xiao-driver/tests/c20_capability_probe.rs      N-2：`**` 探针的期望；§5.1 过 clang；§5.3 后端负例；§5.5 清单派生
core/rust/crates/xiao-codegen-llvm/tests/n0_a.rs         §5.1：正向断言过 clang；N-3：`:998` 的期望码
core/rust/crates/xiao-driver/tests/n0_b_dynamic_native.rs §5.1：同上
core/rust/crates/xiao-runtime/src/crash.rs               N-3：两侧退出码口径（`seed §4.4`）
core/rust/crates/xiao-driver/tests/d19a_differential.rs  §5.4：双路径释放对账用例
.github/workflows/maintenance-regression.yml             §5.2：两处原生作业加 `--no-fail-fast`
docs/DevDocs/20c-language-boundary-and-operators.md       §1.3 加「语句上下文」栏；§3.2.1 第 6 条判据；状态头改回事实
docs/DevDocs/10r-static-overflow-implementation.md       N-3 的归属与契约结论回填
docs/DevDocs/09b0-bytecode-closure.md                    §5.2 结论回填（本专项的三个真运行号）
tests/fixtures/20c/README.md                             §七 第 1 条：`:21` 改回事实，`:14` 同步
```

## 九、验收

1. **main 转绿**：两个原生作业**都**绿；`N-1`/`N-2`/`N-3` 各自的运行号回填本文；
2. **N-1**：`and`/`or` 在**两条路径**上都能构建、运行、输出一致；
   且**「同一位置换成 `==`／`not` 也一样能构建」**这条判据成立；
   **至少一例的右侧带副作用**（证明短路，而不只是静态结果对）；
3. **N-2**：二选一落地，且**接受面与可构建性不再互相矛盾**；
   守卫**跑完全部再报**（不再一红即停）；
4. **N-3**：两平台退出码**一致**，且**非法指令与段错误各测一次**；
   若为基线变更已单独登记；
5. **§5.1**：至少一条正向断言**真的过了 clang**（不是只看文本）；
6. **§5.2**：两处原生作业加 `--no-fail-fast`，并**用一次真实运行证明 Linux 上 c20 也跑了**；
7. **§5.3/§5.4**：`Backend` 变体要么删、要么有负例撑住；`release_plans` 对账有用例或已登记未覆盖；
8. **§5.5**：清单派生或加覆盖断言；
9. **§六**：§3.2.1 第 6 条已改为「关掉全部拒绝点」；
10. **§七**：两处声明已改回事实；`README:14` 已同步；
11. **通用**：`cargo test --workspace`、clippy、fmt、`bun run check`、`bunx tsc --noEmit` 全绿；
    提交标题带类型前缀、正文非空。

## 十、待裁定

1. **N-3 的契约形态**：硬件故障退出码取「**按故障类别分派**」（Unix 现状，信息量大、
   但码值不稳定）还是「**平台统一码**」（Windows 现状，简单、但把故障类别压平）？
   审核倾向**按类别分派**，并把 Windows 侧的硬编码 `139` 一并改成按类别；
2. **N-2 取「修降低」还是「移出接受面」**？审核倾向**移出接受面**——
   在类型层以稳定码拒绝「顶层裸赋值」这一形态过于别扭，**更可能是修降低**；
   若排查后确认是类型推断的边界问题，则按 §3.2 乙的先例移出并登记；
3. **`--no-fail-fast` 是否同时加进 workspace 门禁**？（本文只要求两处原生作业）

## 相关页面

- [20C-语言边界批](20c-language-boundary-and-operators.md) —— 本专项的 §五 与 §2.2/§3.2.1 都是它的冻结
- [20C](20c-official-xiao-library.md) §2.4 —— 验证纪律、遮蔽教训、基线变更的登记口径
- [10R-实现](10r-static-overflow-implementation.md) —— N-3 的归属文档
- [10Z-驱动](10z-performance-driver.md) §2.7 D7 —— 「证据必须有执行点」；本文 §五 的出处
- [09B0](09b0-bytecode-closure.md) §3.3 —— 中心不变量与本专项三个真运行号的回填处
