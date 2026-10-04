# 20AB-FIX. 生命周期诊断与移除验证

> **20AB 的审核修复批。** 20AB 主体（`cd08ca0`）与 `abi_io` 拆分（`a161644`）已交付，
> **验收 11 条中 9 条通过**（契约表自洽、未知 id 拒绝、名称分派消除、`print` 可用、
> 返回值不变、依赖方向、生成一致、不回归、门禁全绿），全量测试 0 失败、clippy 绿。
>
> 本批修**两项未达标**，其中一项是审核新发现的、不在原验收清单里的缺陷。

## 一、Agent 交接上下文

### 接手前提

1. [20AB 交接文档](20ab-intrinsics-contract-and-minimal-entry.md) —— **本批的修改对象**；§二 9 条冻结、§八 11 条验收；
2. [20A 研究](20a-intrinsics-contract-research.md) §二 —— 现状实测盘点（5 处名称分派）；
3. [20. 内置函数与标准库](20-builtins-and-standard-library.md) §三 3.3、§六 1 —— **契约消费者清单与测试族**（本批两项都出自这里）。

### 审核实测记录（2026-10-04）

审核用**真实 CLI** 跑了两个程序做对照，这是下面 §2.2 的证据来源：

```text
$ xiao run hello.xiao            # 内容：print("hello world!")
hello world!
X06-LIFETIME-005: 值 expression 的生命周期需要 Runtime 检查 (Unknown)
X06-LIFETIME-005: 值 expression 的生命周期需要 Runtime 检查 (DynamicValue)
状态  退出码
----  ------
成功  0

$ xiao run plain.xiao            # 内容：int a = 1
状态  退出码
----  ------
成功  0                          # ← 干净，无任何诊断
```

**验收 6「`print` 可用」本身是满足的**（确实输出了 `hello world!`、退出码 0）；
本批要修的是它**附带的两条诊断**。

## 二、问题清单

### 2.1 **移除验证缺失**（验收 4，原文档标「关键」）

20AB 交接文档 §八 第 4 条：

> **移除验证**（**关键**）：删除表项、VM 绑定或 ABI 包装中的**任一**环节，
> 专门用例**必须失败**——这是「表真的在被消费」的唯一硬证据。

**实测**：全仓搜索只找到**一句文档注释**提到它——
`xiao-intrinsics/src/lib.rs:174`「对外暴露表一致性校验，供编码器、VM 和移除验证共同调用」——
**没有任何用例实现这个校验**。

**为什么这是缺口**：方向稿 §三 3.3 明写「**表存在但某个消费者仍按字符串特判，视为未完成**」。
验收 3（名称分派消除）已经用 grep 证明了**当前**没有特判，但**无法防止将来回退**——
只有「删掉任一环节则用例失败」才能把「表是被消费的」变成**可持续验证的事实**。

### 2.2 **`print` 产生两条生命周期诊断**（审核新发现）

**现象**：见 §一 的对照实测。`print("hello world!")` 比 `int a = 1` 多出两条
`X06-LIFETIME-005` 警告，`severity` 为 `warning`（所以退出码仍为 0）。

**结构化内容**（`--json` 实测）：

```json
{ "code": "X06-LIFETIME-005", "message_id": "x06.lifetime.dynamic_check",
  "severity": "warning",
  "span": { "start": 0, "end": 5 },                       // "print" 这个名字
  "params": { "reason": { "value": "Unknown" }, "value": { "value": "expression" } } }

{ "code": "X06-LIFETIME-005", "message_id": "x06.lifetime.dynamic_check",
  "severity": "warning",
  "span": { "start": 0, "end": 21 },                      // 整个调用表达式
  "params": { "reason": { "value": "DynamicValue" }, "value": { "value": "expression" } } }
```

**根因（已定位到行）**：`xiao-lifetime/src/escape.rs:1500-1519` 处理 `Expression::Name` 时——
`print` 作为被调用的名字被**当成一个要跟踪所有权的值表达式**来分析：

- 它**不是本地绑定**（无 `binding_type`），
- 也**没有值的类型事实**，

于是走「未知名称」分支，报 `Unknown`；另一条 `span 0..21` 的 `DynamicValue`
来自 `escape.rs:1673` 的调用表达式处理。

**一句话**：**生命周期分析不认识契约表里的 intrinsic，把「被调用的函数名」
当成了「需要跟踪的值」。** intrinsic 既没有所有权也没有生命周期，本不该进入这条分析。

**两个问题叠加**：

1. **`print(...)` 作为语句使用时返回值被丢弃，本就不该产生生命周期诊断**；
2. **`reason` 的取值是 `DynamicValue`**——这**正好命中方向稿 §六 1 的明文禁令**：
   「类型名和错误参数**不出现 `dynamic` 等兜底文本**」。

**为什么必须修**（而不只是「不好看」）：

- `print` 是**最常用入口**，每次调用吐两条诊断；
- [17. `.xar`](17-xar-archive.md) 的端到端验收**要用 `print("hello world!")`**
  （见 17 规范 `:21`），这批噪声会直接进入 17 的验收输出；
- 它**违反方向稿自己立的规矩**（§六 1 的兜底文本禁令）。

## 三、必须先冻结的 4 条

### 3.1 **`xiao-lifetime` 接契约表，而不是识别名称**

**冻结**：修复 §2.2 时，**不得**在 `xiao-lifetime` 里硬编码 `"print"` / `"input"` 之类的名称判断——
那正是本批要消灭的形态，会把刚清掉的名称分派从 `xiao-types` 搬到 `xiao-lifetime`。

**做法**：让生命周期分析**经契约表**识别 intrinsic（`xiao-intrinsics` 位于共同下层，
`xiao-lifetime` 依赖它在方向上是正确的，符合方向稿 §三 3.3「从一份声明派生的消费者」）。

### 3.2 **intrinsic 不参与所有权/逃逸分析**

**冻结**：对**已登记的 intrinsic 调用**：

- **不产生** `DynamicCheck`（它的返回类型来自契约表，是已知的）；
- **不把被调用名登记为待跟踪的值**；
- 返回值仍是 `None` 的（如 `print`）**继续按 `None` 处理**，不引入新的值身份。

**注意 `input`**：它**返回 `str`**，与 `print` 不同——`str` 是有生命周期的值，
所以 `input` 的返回**可能需要**正常跟踪。**两者不能一刀切**，按契约表的 `return_type` 区分。

### 3.3 **`DynamicValue` 兜底文本不得出现在用户可见诊断里**

方向稿 §六 1 的禁令适用于**本批新增或修改的诊断**：

- 若某条 `X06-LIFETIME-005` 在修复后**仍然合理存在**（例如真的来自用户代码的动态值），
  其 `reason` **必须给出具体原因**，不得是 `DynamicValue` 这种内部枚举名；
- 但**不得**为此改动其他既有的、与 intrinsic 无关的诊断行为——
  先确认它是否真的仍会触发，**不做无差别重写**。

### 3.4 **移除验证不得靠改测试实现**

**冻结**：§2.1 的移除验证**必须是真删**（删表项 / 删 VM 绑定 / 删 ABI 包装三者之一），
观察专门用例**真的失败**，再恢复。

**不得**写成「断言某个函数存在」或「断言常量等于某值」这类**同义反复**——
那不是移除验证，只是把当前实现抄了一遍。

## 四、修复要求

### 4.1 补移除验证（对应 §2.1）

至少覆盖**三个环节各一条**（方向稿 §六 1 最后一条要求「任一环节」）：

| 环节 | 删什么 | 期望 |
| --- | --- | --- |
| 契约表项 | `intrinsics.json` 里删掉 `print` 那条 | 相关用例失败（编码器校验或 VM 分派找不到 id） |
| VM 绑定 | 删 `print` 的 VM 分派分支 | `print_intrinsic_reaches_vm_output_event` 失败 |
| ABI 包装 | 删 `xiao_runtime_print_values` 接线 | 原生侧用例失败 |

**实施方式由实施者定**（条件编译开关、测试专用剔除、或独立校验脚本均可），
但**必须能说明「删了会红、恢复了会绿」**，并把验证过程写进实施记录。

### 4.2 修生命周期诊断（对应 §2.2）

按 §3.1/§3.2 的冻结做：

- `xiao-lifetime` 的 `Expression::Name` 与调用表达式处理**经契约表识别 intrinsic**；
- `print` 的返回值（`None`）**不再产生** `DynamicCheck`；
- **`input` 单独判断**——它返回 `str`，按 §3.2 的注记处理，**不得**因为和 `print` 同类就一并跳过。

**回归测试（必须补）**：

```text
print("hello world!")   → 0 条诊断（当前 2 条）
int a = 1               → 仍 0 条诊断（防回归）
input()                 → 按 §3.2 的裁定产出应有的结果
```

**验收口径**：以 **`--json` 的 `diagnostics` 数组为空**为准，
不要用人类可读输出里有没有那行字来判断。

## 五、验收

1. **`print` 干净**：`print("hello world!")` 经 `xiao run` 的 `diagnostics` 为**空数组**，
   仍输出 `hello world!`、退出码 0；
2. **无名称硬编码**：`xiao-lifetime` 中不出现 `"print"`/`"input"` 字面量判断（grep 可证）；
3. **`input` 未被误伤**：按 §3.2 的区分处理，有其自己的用例；
4. **移除验证可证伪**：三个环节各有一条用例，**真删会红、恢复会绿**，过程记入实施记录；
5. **不回归**：`int a = 1` 等既有程序诊断数不变；`cargo test --workspace` 全绿；
6. **门禁全绿**：`cargo clippy --workspace --all-targets -- -D warnings` 绿；
   `bun run check` 退出码 0（含 `check:lock`）；
7. **不重开**：契约表格式、`IntrinsicId` 宽度、`xiao-lock`、16A/16A-FIX/16B 的既有结论。

## 六、不负责与不要重复做的事

- **不做** 20AB §九 已排除的范围（`os`、官方库、`.xar` 兼容检查、实施 17）；
- **不改**契约表的数据文件格式与 `IntrinsicId` 编码（20AB §2.4 已冻结为 `u32`、`0` 保留）；
- **不为** `print` 增加名称特判——见 §3.1；
- **不无差别重写**既有的生命周期诊断（§3.3）——只处理 intrinsic 相关的那两条；
- **不实施 17**——本批只负责让 17 的端到端验收输出干净。

## 七、实施记录

### 7.1 生命周期修复

- xiao-lifetime 新增对 TypeCheckResult::intrinsic_id_at 的消费，按契约 ID 识别调用；调用分支不再分析
  intrinsic 被调名称，因此不会把入口名称当作未知用户值。
- 契约声明返回 none 的入口不创建生命周期值；返回 str 的入口仍创建正常堆值。print 和 input
  的回归分别验证 dynamic_checks 为空，且 input() 仍有字符串值对象。
- xiao-lifetime 源码没有新增任何按 print/input 名称判断；名称只在类型层前端适配契约表。

### 7.2 三项真删移除验证

以下验证均在当前实现上先临时删除目标环节，观察专门用例失败，再恢复并重新运行用例：

| 环节 | 临时删除 | 失败证据 | 恢复证据 |
| --- | --- | --- | --- |
| 契约表项 | intrinsics.json 的 id=19 print 行 | cargo test -p xiao-driver removal_verification_print_contract_and_vm_binding 失败于“print 移除验证应成功执行” | 恢复该行后同命令通过 |
| VM 绑定 | xiao-vm/src/semantics/exec.rs 的 VmBinding::Print 分派 arm | 同一用例失败于 execution.outcome.result.is_success() | 恢复 arm 后同命令通过 |
| ABI 包装 | xiao-runtime/src/abi_io.rs 的 xiao_runtime_print_values 实现 | cargo test -p xiao-runtime removal_verification_print_wrapper_rejects_null_nonempty_input --lib 编译失败，报告 unresolved import | 恢复包装后 cargo test -p xiao-runtime removal_verification --lib 两项通过 |

这三项不是对符号或常量存在性的重复断言：第一项走真实前端到 VM，第二项走真实 intrinsic 分派，第三项
调用 ABI 包装的参数边界行为。任一消费者被删除，专门用例都会先失败。

### 7.3 遗留项（已转 17A）

§7.2 的第三项（ABI 包装）失败证据是**编译失败**而非运行期断言。编译失败确实证明了符号被消费
——不删不影响编译、删了就红——但它比前两项**弱**：编译失败可能由别的原因引起。该加强**已并入
[17A §2.9](17a-archive-format-and-codec.md) 一并处理**，本批不再追补。

## 相关页面

- [20AB. intrinsic 契约与最小生产入口](20ab-intrinsics-contract-and-minimal-entry.md) —— **本批的修改对象**；§八 第 4 条即 §2.1 的来源
- [20. 内置函数与标准库](20-builtins-and-standard-library.md) —— §三 3.3 消费者清单、§六 1 测试族与兜底文本禁令
- [20A. intrinsic 契约与 20 的批次切分](20a-intrinsics-contract-research.md) —— 现状盘点与四项决策
- [17. `.xar` 字节码归档与启动](17-xar-archive.md) —— `:21` 的端到端验收依赖 `print`，本批解除其输出噪声
- [06. 内存与运行时语义](06-memory-and-runtime.md) —— 生命周期与逃逸分析的权威定义
