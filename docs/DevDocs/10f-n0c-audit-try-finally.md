# 10F. N0-C 专项审核：`try`/`catch`/`finally` 的 LLVM 控制流

> **这是一份审核任务书，不是结论。** 目标是把 `31fb944` 里那条被两次重复的判定
> 交给一位**独立的审核者**独立复核——**允许推翻本档的全部观察**。
>
> **审核对象**：`31fb944 feat(10c): 接通原生错误路径与源码映射`（2026-09-29）。
> **审核范围**：只审 `try`/`catch`/`finally` 的原生控制流发射与 `llvm-as` 验收，
> **不审**本批的 ABI 形状、`suppressed` 语义、源码映射（那些另有结论）。

## 一、要判定的核心分歧

同一处缺陷有**两个互相矛盾的定性**，本档要求给出裁决：

| 来源 | 判定 |
| --- | --- |
| **实现者**（`31fb944` 提交信息） | 「清理发射器与 LLVM 基本块终结状态之间仍存在**结构性控制流问题**……后续需**重构 `emit_try_cleanup` 的标签所有权和终结状态契约**」 |
| **上一位审核者**（本次对话，第一轮） | 「**不是结构性问题，是一处取用遗漏**——`success_target` 已经解构在手边，finally 分支忘了用它」 |
| **上一位审核者**（同上，第二轮实测后） | **自己的判断被推翻**：补上那一行后缺陷没有消失，只是**错误点漂移**了（见 §三 试验 A） |

**⚠️ 上一位审核者在本次审核中已经错过一次。** 本档 §三、§四 的观察**同样需要独立验证**，
**不得**因为写在这份文档里就当作已证事实。

---

## 二、可复现的事实（截至 `31fb944`，工作区干净）

### 2.1 门禁状态（实测）

| 项 | 结果 |
| --- | --- |
| `cargo test --workspace` | ❌ **退出码 101** |
| `check:lock` | ❌ 退出码 1（`tests/benchmarks/Cargo.lock` 未随提交更新） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ 0 |
| `bun tools/repo-check/src/cli.ts all` | ✅ 0（461 个 `A0-COVERAGE-002` 为存量） |

### 2.2 `llvm-as` 拒绝带 `finally` 的产物

```bash
# 默认跑：注意 optional_llvm_accepts_error_path_module 会静默跳过
cargo test -p xiao-codegen-llvm --test n0_c_errors

# 打开真正的解析器校验（关键）
XIAO_LLVM_AS="D:/msys64/ucrt64/bin/llvm-as.exe" \
  cargo test -p xiao-codegen-llvm --test n0_c_errors
```

失败样例（`n0_c_errors.rs:172-185` 那个测试）：

```
llvm-as: <tmp>.ll:232:1: error: expected instruction opcode
dynamic.try.catch0.body20:
```

### 2.3 边界：**无 `finally` 的 `try/catch` 是合法的**

实测对照（同一份 `try/catch`，只切换 `finally_body` 的有无）：

| 形态 | 空基本块数 | `llvm-as` |
| --- | --- | --- |
| `try`/`catch`（无 `finally`） | 0 | ✅ 通过 |
| `try`/`catch`/`finally` | 3 | ❌ 拒绝 |

**所以缺陷只在 `finally` 路径上**——这个边界请独立复核。

### 2.4 一个把缺陷藏起来的机制

`n0_c_errors.rs:172-175`：

```rust
let Some(llvm_as) = std::env::var_os("XIAO_LLVM_AS") else {
    return;
};
```

**未设环境变量即静默通过。** 本机 `/d/msys64/ucrt64/bin/llvm-as` 是存在的，只是没人指给它。
**在仓库的默认状态下，这个测试永远是绿的。**

### 2.5 另一个测试是**红的**，且断言了不存在的标记

`n0_c_errors.rs:199-222` 的 `lowers_nonlocal_control_exits_through_cleanup_chain`
断言 `module.text.contains(...)` 包含：

```
"dynamic.return"  /  "dynamic.break"  /  "dynamic.continue"
```

**实现里一个都不存在。** 枚举 `control.rs` 全部 `next_label("dynamic…")` 前缀共 18 个，
`break`/`continue` 用的是 `dynamic.while.end` / `dynamic.while.cond`，`return` 不发标签。

这是 `cargo test --workspace` 变红（退出码 101）的直接原因。
**请独立判定：这是"测试写错"还是"实现缺失"。**

---

## 三、审核者做过的两次试验（**结论待复核**）

两次试验均为**临时改动，已还原**，仓库当前状态与 `31fb944` 一致。

### 试验 A：只补 `br`（`control.rs:448-450`）

`emit_try_cleanup` 的 `finally` 分支拿到 `success_target` 却从未使用它，
成功路径汇合到本地 `success` 标签后就断了。补一行：

```rust
if continues {
    self.emit_label(&success);
    self.emit(format!("  br label %{success_target}"));   // 新增
    self.terminated = true;                                // 新增
}
```

**结果：空基本块归零，但 `llvm-as` 仍然拒绝**，错误点漂移到：

```
dynamic.try.catch0.fail28:
```

**→ 这一步证伪了"一处遗漏"的判定。**

### 试验 B：再修 `control.rs:351-353`

```rust
if !self.terminated {
    self.emit_label(&catch_body_label);   // ← 标签发射被 terminated 守卫
}
```

试验 A 新增的 `terminated = true` 正好触发这个守卫，导致 **`dynamic.try.catch0.body20:`
标签整个消失**，其内容并入上一块，于是产生"块缺终结指令"。改为无条件发射后：

**结果：`optional_llvm_accepts_error_path_module` 通过**
（该测试文件从 `2 passed / 2 failed` 变为 `3 passed / 1 failed`，
剩余 1 个失败即 §2.5 那个断言不存在标记的测试）。

### 审核者对这两次试验的**自评（须被质疑）**

- 试验 B 的 `:351` 守卫**确实**是「标签所有权与终结状态混用」——
  **这一点支持实现者的"结构性"判定**；
- 但**两处小改**即让 `llvm-as` 接受，**又不足以支持"需要重构"**；
- **最可疑的是**：试验 B 只验证了**一个**用例（单 `catch` + 有 `finally`）。
  **"一个用例通过"不等于"全体通过"**——这正是本批一路翻车的原因。

---

## 四、必须独立判定的问题

1. **定性**：这属于「有限处数的实现缺陷」还是「需要重构的契约问题」？
   **给出边界**，不要只给结论。
2. **完备性**：§三 试验 B 是否已经覆盖**全部**根因？请用**多形态**用例复核，至少包含：
   - 无 `finally` / 有 `finally`
   - **多个 `catch`**
   - **嵌套 `try`**
   - `finally` 内 `return`/`break`/`continue`
   - **循环体内**的 `try`（`alloca` 是否会随迭代增长？）
   - `catch` 体自身再 `raise`
3. **同类第三处**：`dynamic.rs:284-299` 的 `check_status_at` 用
   `self.emit(format!("{label}:"))` 发标签，**而非 `emit_label`**（不重置 `terminated`）。
   这是否是同一契约问题的第三个实例？**请给出判定与证据。**
4. **`control.rs:548`**（`emit_cleanup_context` 里的 `emit_label(&success)`）
   与 `:449` 形态相同。**它是否安全？为什么？**（审核者的观察：它后面调用方会补 `br`，
   故为安全——**此观察未经充分验证**。）
5. **测试策略**：`XIAO_LLVM_AS` 静默跳过是否应当保留？若产物合法性是 N0-C 的验收前提，
   这个开关应当如何处置？
6. **登记**：`docs/module-registry.json` 把 `rust.xiao-runtime-abi` 标为 `status: "verified"`、
   `stage: "10B/10C"`，而提交信息自述「**再宣称 N0-C LLVM 验收完成**」。
   两者是否冲突？**该状态应为何值？**

---

## 五、审核纪律（本次审核的硬要求）

1. **必须用真的 `llvm-as`**。字符串断言（`module.text.contains(...)`）**不构成** IR 合法性证据。
2. **必须自己跑命令**，不接受任何「已验证」的声明——包括本档 §三 的试验记录。
3. **必须覆盖多形态**，不得用一个用例的通过代表整体。
4. **如实报告**：没有验证的推测必须标注为推测。
5. **允许推翻本档的任何观察**，并请把推翻过程一并写出。

---

## 六、本次不负责

本节记录原专项审核任务的范围；下节是后续按审核结论实施修复的记录。

- **不改代码**——本档是审核任务，裁决与修法建议交回后再决定由谁实施。
- **不审** ABI 形状、`suppressed` 语义、源码映射表、错误构造参数解析（另有结论）。
- **不审** `xiao-runtime` 侧的平台异常处理缺口（`§3.1` 红线）与 `xiao_runtime_diagnostic_event`
  空壳——这两项已在别处记录为**未完成**，不在本次范围。
- **不做** N0-D（三目标固定宽度、Runtime 裁剪验证）。

## 七、复核结论与修复记录（2026-09-30）

本次复核实际使用 `D:\msys64\ucrt64\bin\llvm-as.exe`，不是字符串片段断言。修复后执行：

```powershell
$env:XIAO_LLVM_AS = "D:\msys64\ucrt64\bin\llvm-as.exe"
cargo test -p xiao-codegen-llvm --test n0_c_errors -- --ignored --nocapture
```

该测试把以下十六种形态逐一交给真实 `llvm-as`：无 `finally` 的 `try/catch`、单 `catch/finally`、多
`catch`、嵌套 `try`、`catch` 再 `raise`、`finally` 内 `raise`、`return`、`break`、`continue`（含三种
退出各自在 `finally` 作用域持有字符串的情形）、`try` 主体内的 `return`/`break`/`continue`，以及循环体内的
`try/finally`；结果为 `1 passed, 0 failed`。默认不提供工具链时，同一测试显示为 `ignored`；显式使用
`--ignored` 且缺少 `XIAO_LLVM_AS` 时会因配置错误失败。

本次复核推翻了 §2.3 的边界结论：无 `finally` 的 `try/catch` 同样会因 catch body 标签被
`terminated` 守卫跳过而生成非法 LLVM。缺陷因此不只在 `finally` 路径。

结论是：缺陷不是需要重写整个降低器的无限范围问题，但确实共享一个基本块所有权契约。标签发射和
`terminated` 状态不能互相代替；凡是已经发射终结指令的路径，后续仍要显式建立自己的基本块标签。
本次提交完成了以下修复：

1. `emit_try_cleanup` 的 `finally` 成功块现在跳转到传入的 `success_target`，并在跳转后更新
   `terminated`；catch body 标签无条件发射，避免无 `finally` 或终结型 `finally` 把指令追加到旧基本块。
2. `emit_cleanup_context` 在 `finally` 主体发生 `return`、`break` 或 `continue` 时，先释放
   `finally_scope`，再释放受保护作用域；原有的失败边仍按 `finally -> protected -> outer error`
   顺序处理。默认回归测试还构造了带所有权计划的 `finally` 字符串绑定，并检查其 ABI 槽释放
   出现在 `ret void` 之前。
3. `finally` 主体前后使用 `llvm.stacksave`/`llvm.stackrestore`。正常边、错误边和非局部退出边都
   恢复保存点，按 LLVM 栈恢复语义避免循环迭代累积 finally 内的临时 `alloca`。测试检查保存/恢复
   指令存在，并由真实 `llvm-as` 解析；本次没有执行长循环原生程序测量栈用量。Runtime 声明集中
   登记这两个 LLVM intrinsic。
4. `check_status_at` 以及同类的挂起错误检查改用统一的 `emit_label` 和终结状态更新，避免第三类
   隐式标签破坏同一契约。
5. `n0_c_errors` 的真实 LLVM 测试改为 `#[ignore]` 环境门控；显式运行时用 `expect` 检查
   `XIAO_LLVM_AS`。非局部退出断言改为实际生成的 `ret void`、`dynamic.while.end` 和
   `dynamic.while.cond` 目标。

复核还确认 `emit_cleanup_context` 中的成功标签与 `emit_try_cleanup` 不能使用同一种终结策略。
前者由 `emit_nonlocal_exit_from_depth` 的调用方继续处理：成功标签发射后必须保持
`terminated = false`，调用方才会继续弹出外层清理区域，最后发射真实的 `ret` 或循环分支；把它
提前标成已终结反而会截断外层清理。后者没有这样的后续目标处理，因此由清理器自己从成功块跳到
`success_target` 并标记终结。这个边界已通过 `try` 主体内的 `return`、`break`、`continue` 三个
真实 LLVM 用例验证。

模块登记继续把 `rust.xiao-runtime-abi` 保持为 `10B/verified`。本审核只证明上述控制流产物能被
LLVM 解析器接受，不能据此把完整 N0-C（运行时错误语义、源码映射和端到端原生回环）登记为
`verified`；这些部分仍按 10E 的范围单独验收。

后续建议：在每个新增清理发射器测试中同时保留真实 `llvm-as` 门禁和缺环境失败检查；若以后扩展
`finally` 的异步或跨函数语义，应继续沿用“标签由调用方拥有、清理器只终结当前块”的契约，并为
每条非局部边记录释放计划的实际调用序列。

---

## 相关页面

- [10E. N0-C 错误路径与源码映射](10e-n0c-error-paths-and-mapping.md) —— 本批的规划与验收标准
- [10A. LLVM 原生构建闭环](10a-n0-native-closure.md) `:79` —— N0-C 范围来源
- [10B. N0 Runtime ABI](10b-n0-runtime-abi.md) —— 上游 ABI
- [07. 错误模型与并发安全边界](07-concurrency-and-errors.md) `:158-165` —— `finally → drop → catch` 展开顺序
- [12. 测试与开发里程碑](12-tests-and-milestones.md) `:687` —— 字节码与原生一致
