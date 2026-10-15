# 20C-第八批. 转换与动态边界的收口

> **背景**：[20C-第七批](20c-seventh-batch-str-to-int-and-ci-coverage.md) §12.1 登记的前置 **F-2**
> （动态值路径的通用守卫），以及 [20C-第四批](20c-fourth-batch-int-to-str.md) §8.1 登记的
> **`lint`/`lfloat` 数值转换债**——**登记最久的一笔**，[02. 类型系统](02-type-system.md):130-133
> 明写它「**阻塞所有依赖 `lint` 数值转换的接口**」。
> 本批收这两笔，并把**欠了两批的「诊断身份」裁定**落定
> （[20C-第五批](20c-fifth-batch-reprobe-and-library.md) §八 第 3 条：与 F-1 的修复同批定）。
>
> **但**：写本批规划时把那笔 `lint` 债**实测了一遍**，发现**登记的表述是错的**——
> 它根本不是「转换」问题：**声明一个 `lint` 形参，就足以让整个产物在 VM 验证阶段被判为损坏**
> （§三，逐条给真实错误与反例）。
>
> **一句话概括**：**先按真实形状重述这笔债**（它比登记的大，而且性质不同），
> 再定「两笔债是不是同一批做」与「诊断身份要不要补档位」。

状态：**已实施并完成 CI 取证（2026-10-16）**：§九 四条**全部裁定**；
F-2 已独立提交并完成双平台维护回归，P-1/P-2 与诊断身份修复也已完成，运行号见 §十一。

## 一、为什么是这一批

1. **F-2 是当前唯一一个真缺陷**：`dynamic` 持 `str` 做 `as str`（**恒等转换**）都报
   `X06-RUNTIME-002`，而静态路径下 `"x" as str` 是合法的。第七批 §12.1 已把它登记为**下一批前置**；
2. **`lint`/`lfloat` 那笔是登记最久的一笔**，且它**阻塞一个类型族的所有转换**；
3. **诊断身份那条裁定欠了两批**：第五批就写明「先只登记，**与 F-1 的修复同批定**」，
   而 F-1 的修复**正是本批**；
4. **三者同属一个子系统**（转换矩阵 / 降低器 / Runtime `convert_to`），可以分笔提交、
   但共用同一套补探与同一组双路径证据。

## 二、并入的返工项（**先做**）

### 2.1 F-2（中）：动态路径只修了整数目标

**出处**：第七批 §12.1；审核侧临时探针实测。

| 探针（第七批实测） | 结果 |
| --- | --- |
| dyn-int → `int()` / `as int` | 成功，`7` |
| dyn-str `"A"` → `int()` | 成功，`65` |
| **dyn-int → `as str`** | 失败，`X06-RUNTIME-002` |
| **dyn-str → `as str`**（值本来就是 `str`，**恒等转换**） | 失败，`X06-RUNTIME-002` |
| **dyn-str `"true"` → `as bool`** | 失败，`X06-RUNTIME-002` |

**恒等转换也失败** ⇒ 堵它的是**通用守卫**，不是转换本身不支持。
该守卫的运行期判定是 `matches!(value, RuntimeValue::Error(_))`
（`xiao-vm/src/semantics/exec.rs:2163`）——**它要求值是个 Error**；
而 `xiao-types/src/control_checker.rs:82` 写明这个档位本来就是给**错误边界**用的。

**已裁定（星崽，2026-10-15）：甲——整体收敛到错误边界。**
去掉 `ConversionKind::RuntimeChecked if is_integer(target) => {}`
（`xiao-types/src/checker/expression.rs:663` 与 `:806`）这个**按目标类型**的分叉，
让 `RuntimeChecked` 的推入只在**错误边界**保留，普通值转换一律不推——动态路径与静态矩阵对齐。

**连带（写下来免得做漏）**：

1. 这不是「再放宽一格」，而是**去掉一个本来就不该在这里的守卫**；
2. 收敛后 `dyn → str`/`bool` 会**从失败变为成功**，属**行为面变化**，
   必须配一组**新的双路径证据**（不是「旧用例还绿」就算）；
3. 旧行为（改动前对所有目标都推）**是既有缺陷**，不是第七批引入的——
   文档不得写成「第七批弄坏的」。

### 2.2 G-4 复发（低，**审核侧已在本次改掉**）

第七批 §13.4 记过一处「`README.md` 第七批行粗体未闭合」，并在那里写了
「下一批加接口时记得连同本行一起过一遍粗体配对」。**那句话没有拦住它**：
`77228bb` 重写该行时**又**留下了「已订正`**` |」——同一处、同一个错。

**这说明了什么**：靠「记得检查」防不住这一类，因为它不是知识缺口而是**注意力缺口**。
**本批的处置**：把这行纳入 §八 的通用验收——凡改动 `README.md` 的 20C 行，
**必须跑一遍逐行粗体配对**（该检查只要一行 python，见第七批 §13.4 的做法）。

### 2.3 已在案、**不属于本批主体**的欠账（对账用）

按「先清欠账」列全，但**本批不做**：P-2 的真实 push 运行号、10Z-D 的 Linux/macOS 受控复跑、
T4 受控复跑、B1 成对实验、O-1 受控取数、产物级 D1/D2 静态剖面实测、
O6 条件 5（no hosts）与条件 1。它们按各自文档登记，**不许**因为本批开工就写成已完成。

## 三、`lint`/`lfloat` 这笔债的**真实形状**与登记不符

**本节全部是审核侧 2026-10-15 的实测**（临时探针，用完即删），不是转述旧文档。

### 3.1 登记的说法

[02. 类型系统](02-type-system.md):130-133：

> `lint`/`lfloat` 与其它数值之间的显式转换仍是既有缺口：类型层目前会接受例如
> `lint as int`、`lint as float`，但原生路径可构建而 VM 在生产验证阶段以
> `X09-BYTECODE-002`（Fatal，`fatal.corrupt_artifact`）拒绝，说明降低器的寄存器类别与指令期望不一致。

### 3.2 实测（函数边界 / 顶层 / 标量对照）

| 形态 | VM 结果 |
| --- | --- |
| 顶层 `lint big = 42` → `big as int` | **成功**，`42` |
| 函数内**局部** `lint big = 42` → `big as int` | **成功**，`42` |
| 形参 `def f(lint v) -> int` → `v as int` | **失败**，`X09-BYTECODE-002` |
| 形参 `def same(lint v) -> lint` → `return v` | **失败**，`X09-BYTECODE-002` |
| 形参 `def ignore(lint v) -> str` → `return "ok"`（**v 从不使用**） | **失败**，`X09-BYTECODE-002` |
| 形参 `def ignore(lfloat v) -> str` → `return "ok"`（**从不使用**） | **失败**，`X09-BYTECODE-002` |
| 形参 `def ignore(sint v) -> str` → `return "ok"`（**标量对照**） | **成功**，`ok` |
| 形参 `def f(int v) -> lint` → `v as lint` | 失败，`X06-RUNTIME-012` |

### 3.3 真实错误（**不是转述**）

从 `DriverOutcome` 取到：

```text
phase: Verification
code:  X09-BYTECODE-002
path:  "program"
message: functions[0].blocks[0].instructions[3].arguments[0].value:
         寄存器 1 的类别 int 与期望 obj 不一致
report:  class=Fatal, code=X07-FATAL-002, message_id=fatal.corrupt_artifact
```

### 3.4 三条结论

1. **性质不同**：这不是「转换没实现」，而是**声明一个 `lint`/`lfloat` 形参就让整个产物被判损坏**——
   连**从不使用**那个参数也一样（`sint` 形参的对照通过，所以**不是所有形参**，是 obj 类标量的形参）；
2. **`lint → str` 也坏**：它是**第四批声称接通、且 CI 验过**的那条。原因很清楚——
   第四批的夹具 `tests/fixtures/20c/second-int-to-str.xiao` **全是顶层写法**，
   **从未覆盖「操作数是函数形参（寄存器）」这个形态**。
   ⇒ 那条「已验证」的边界比它写的小；
3. **登记的表述会误导实现**：按登记去修，会以为要动 `convert_to` 与转换矩阵；
   实际主战场在**降低器/验证器**这一侧。

**尚未钉死的（不许在本文里写成已定）**：那个 `int` 类**从哪里来**。
两个已核过的**排除项**：

- `class_of_type`（`xiao-bytecode/src/lower/mod.rs:983-998`）**已经**把
  `ScalarType::Lint | ScalarType::Lfloat` 映射成 `RegisterClass::ObjHandle`（`:989-991`）；
- `declare_parameters`（`mod.rs:528-551`）用的就是它（`:531`）。

⇒ **`int` 类不是从这两处直接来的**。真源头是 §五 要钉的**第一问**。
本文**只登记症状、作用域与排除项**，不写根因结论。

## 四、两道待定裁决

### 4.1 裁决一：P-1 与 P-2 是不是同一批做

| | 做法 | 代价 |
| --- | --- | --- |
| **甲** | **两件全做**：P-1 + P-2 的**全部方向**（含浮点/lfloat） | 大；`lint ↔ float/sfloat/lfloat` 的舍入、溢出、非有限值都要现定一套语义 |
| **乙** | **只做 P-1**（先让 `lint`/`lfloat` 形参不再炸），P-2 单独登记下一批 | 小；但 `int as lint` 仍报错，`lint` 族仍不能用 |
| **丙** | **P-1 + P-2 的整数族方向**：`int`/`sint` → `lint`（精确加宽）、`lint` → `int`/`sint`（解析 + 范围检查） | 中；浮点方向留下一批 |

**说明**：P-1 是**硬故障**（程序跑不起来），P-2 是**能力缺失**（报一个明确的
`X06-RUNTIME-012`）。丙 之所以切得开：**整数族方向的语义没有歧义**
（加宽精确，窄化按既有的 `numeric_range` 先例报范围错），
而**浮点方向有舍入与溢出两处要现裁**——那是另一件事，不该混进来。

**裁定（星崽，2026-10-16）：丙。** 连带两条：

1. `lint → int`/`sint` **复用第七批 `str → int` 那套机器**（解析十进制 + 范围检查），
   **不是新写一套**——第七批已把「解析单值 + 失败报 `X06-RUNTIME-012`」这条路铺好；
2. **浮点/lfloat 方向必须显式登记为「本批不做」**，不许因为「顺手」而带进来
   （[20C-第四批](20c-fourth-batch-int-to-str.md) §八 的范围纪律原样有效）。

### 4.2 裁决二：诊断身份——**编译器缺陷被报成「产物损坏」**

第五批 §八 第 3 条把这条挂起，理由是「**只有一个样本**，且它本身还是个未修的缺口；
为一个样本设计新分类，容易先定出一个后来发现不合适的形状」。

**现在样本变了**：它不是「`lint as int` 这一个转换」，而是
「**任何声明了 `lint`/`lfloat` 形参的程序**」——一个**普通源码就能触发**的
**编译器缺陷**，却被报成 `X09-BYTECODE-002` + `X07-FATAL-002`（`fatal.corrupt_artifact`，
「字节码或其他执行产物损坏」）。**那是在说用户的东西坏了，而坏的是编译器。**

| | 做法 |
| --- | --- |
| **甲** | **给编译器缺陷自己的身份**，与「产物真损坏」分开 |
| **乙** | **不补**，仍报 `X09-BYTECODE-002`／`fatal.corrupt_artifact`，只在文档里登记这是误报 |
| **丙** | 本批先修 P-1 让该路径不再可达，**身份问题等有第二个样本再定** |

**要一并摆上的**：不补则会在**修好 P-1 之前**继续把编译器缺陷说成产物损坏；
**修好之后这条路径不可达，反而更难再观察到它**——这正是丙的风险。

### 4.2.1 裁定与落点（**审核侧补记，2026-10-16**）

**裁定（星崽，2026-10-16）：甲。** 但**落点比选项文字写的窄**，两处要更正：

**其一：不需要新增诊断码。** `X07-FATAL-006`（`fatal.internal`）**已经存在**
（`xiao-diagnostics/src/lib.rs:78`，`message_id = "fatal.internal"`）。所以这是**改报**，不是加码。

**其二：决定报哪个码的地方只有一处，而且它已经在做同类的区分。**

```text
xiao-vm/src/run.rs:465   fn verification_fatal(error: &TacVerificationError, ...) -> FatalError
```

规则是：`error.code == TAC_INTERNAL_CONSISTENCY_CODE`（`X09-BYTECODE-001`）⇒ `FatalError::internal`；
**否则**（含本批的 `X09-BYTECODE-002`）⇒ `FatalError::corrupt_artifact`。
而该函数生成的**消息字面就写着「生产执行前」**——即这条路径上的产物**就是本进程刚降低出来的**。

**现存的两个验证调用点也都在进程内**：`xiao-driver/src/module_loader.rs:143`
与生产执行入口。**我没找到**「从磁盘读入 `.xiaoc` 再走同一验证」的现成链路
（`xiao-bytecode` 里**有** `decode`／`decode_xiaoc`，但没找到它们接到 `verification_fatal`）——
**「没找到」不等于「不存在」**，见下方连带。

⇒ **甲落地 = 这条路径上的判断不再只按「哪类检查失败」，而按「产物是谁产的」**：
本进程刚降低的产物上，结构/ABI 不符**同样是编译器缺陷**。

**这推翻一条既有决定，必须写明原因**：
`xiao-driver/src/run.rs:1048` 的测试
`structurally_invalid_tac_keeps_corrupt_artifact_diagnostic`，其注释写着
「TAC 结构/ABI 错误**仍保持损坏产物身份**，与降低器内部缺口区分」——
**那条规则要改，期望也要改**；**不许**只改期望不改注释。

**连带后果（显式登记，不许含糊过去）**：在现存的进程内路径上，
`X07-FATAL-002`（`fatal.corrupt_artifact`）落地后**不可达**。
处置：**保留该码**，并在 `02-type-system.md` 或诊断表处登记为
「**预留给外部产物校验**——当前没有这条链路」。**不许**顺手删掉一个稳定码。

## 五、补探（**先做，成表**）

**口径**：逐字沿用 [20C-第二批](20c-second-batch-probe.md) §3.2 的六条。

| # | 问题 | 为什么必须问 |
| --- | --- | --- |
| 1 | 那个 `int` 类**从哪来**：是 IR 的形参类型丢了 `lint`，还是别的路径写死了类别 | §3.4 已排除 `class_of_type` 与 `declare_parameters`；钉不住它就只能靠猜着改 |
| 2 | `lfloat` 是不是与 `lint` **完全同形**（含 `lfloat` 形参 + `→ lfloat` 目标） | 本批已实测「`lfloat` 形参不使用」同样炸；其余形态待补 |
| 3 | **原生路径**对同一批形态是什么行为 | 登记的旧说法是「原生可构建」——**未经复核**，不许沿用；两条路径若不一致，处置与只坏一条**不同** |
| 4 | 经 `lint`/`lfloat` 形参的所有**转换方向**（`→ int`/`sint`/`float`/`str`）是否同码 | 决定 P-1 修完后的验收断言集合 |
| 5 | `→ lint`/`lfloat` 的 `X06-RUNTIME-012` 是否**只在形参场景**出现，顶层是否也不同 | 区分 P-1 与 P-2 的边界 |
| 6 | 既有夹具 `second-int-to-str.xiao` 的 `lint` 部分**是否应当补一个函数形态** | 「已验证」的边界要写准；补夹具是**登记该缺口**，不是改历史结论 |

**判据**：第 1 问要落到**具体文件与行**（不许写「降低器某处」）；
第 3 问必须**双路径**，且**以 CI 或受控环境为准**（本机单路径不算）。

### 5.1 改动前补探真值（2026-10-16）

| # | 形态 | VM | Windows 受控原生 | 结论 |
| --- | --- | --- | --- | --- |
| 1 | `lint`/`lfloat` 形参寄存器类别 | `X09-BYTECODE-002`，错误路径为 `xiao-bytecode/src/lower/mod.rs:965-979` 的 `class_for_value` | 同源旧代码可构建并运行 | 根因是按 `storage=stack` 映射为 `Int`；`class_of_type` 与 `declare_parameters` 已排除 |
| 2 | `def ignore(lfloat v) -> str`，`v` 不使用 | `X09-BYTECODE-002` | 成功，输出 `ok` | `lfloat` 与 `lint` 形参同形，属于 P-1 |
| 3 | `def f(lint v) -> int`、`def f(lint v) -> str` | `X09-BYTECODE-002` | 成功构建 | 形参方向与第四批顶层夹具不同，旧证据不能外推 |
| 4 | `lint`/`lfloat` 形参的其它方向 | `lint → int/str` 到验证阶段；`lfloat → str` 先在类型层报 `X02-TYPE-006` | 已测的 `lint`/`lfloat` 形参成功构建 | 浮点方向仍按裁定留在下一批 |
| 5 | 顶层 `lint big = 42` → `big as int` | 成功，输出 `42` | — | 顶层形态本来正常，作为 P-1 对照 |
| 6 | `second-int-to-str.xiao` 的 `lint` 部分 | 只有顶层写法 | — | 第四批历史证据只覆盖顶层，不追溯改写 |

第 1 问的具体排除链已落到代码：`class_of_type`（`mod.rs:985-997`）和
`declare_parameters`（`mod.rs:520-540`）均按 `IrType` 给出 `ObjHandle`；真正错误的是
`class_for_value` 原先只读取 ownership storage。临时探针已删除，永久断言见
`core/rust/crates/xiao-driver/tests/eighth_batch.rs`。

## 六、落点（**待两道裁决后最终确定**）

已核的现有事实（行号按 2026-10-15 的 `main`）：

```text
xiao-bytecode/src/lower/mod.rs:983-998    class_of_type（已把 Lint|Lfloat 映射为 ObjHandle）
xiao-bytecode/src/lower/mod.rs:528-551    declare_parameters（用 class_of_type）
xiao-types/src/checker/expression.rs:663   F-2 的分叉点（`is_integer(target)` → 不推）
xiao-types/src/checker/expression.rs:806   同上，第二处
xiao-types/src/checker/expression.rs:667   DynamicConversion 推入点（第一处）
xiao-types/src/checker/expression.rs:808   同上，第二处
xiao-vm/src/semantics/exec.rs:2163         dynamic_conversion 的运行期判定
xiao-types/src/control_checker.rs:82       该档位本是给错误边界用的（裁定依据）
xiao-runtime/src/value/ops.rs              convert_to 的 Lint|Lfloat 兜底与新增臂（P-2）
tests/fixtures/20c/second-int-to-str.xiao  第四批的夹具（全顶层写法，未覆盖形参）
```

**其余落点**：

```text
docs/DevDocs/02-type-system.md             重述这笔债的真实形状（**与代码同笔**）
docs/DevDocs/20c-fourth-batch-int-to-str.md  §8.1/§8.3 加指向：登记表述已被实测修正
stdlib/                                    本批**不动**（除非补探证明必须）
docs/DevDocs/README.md                     阶段索引加本文一行
```

## 七、执行顺序（**顺序本身是约束**）

1. **R-1 先做**（F-2 甲）并**推一次**——它是一个**行为面变化**，
   要在别的改动混进来之前先拿到它自己的双路径证据；
2. **§五 补探**成表，**结论先汇报**；
3. **两道裁决**落定后再动 `lint`/`lfloat`；
4. 实现 + `02-type-system.md` 同笔；
5. **推一次**取证。

**冻结**：

1. **不许**把 F-2 的收敛与 `lint`/`lfloat` 的修复放进**同一笔提交**——
   一笔里同时有「动态守卫」与「降低器/形参类别」两件事，出问题时**分不清是哪件造成的**；
2. **不许**在没钉住 §五 第 1 问之前改类别映射——**猜着改会掩盖真因**；
3. **不许**用第四批那条 `lint → str` 的旧证据当本批的验收依据——它**从未覆盖形参形态**。

## 八、验收

1. **F-2**：`dyn → str`/`bool` 与 `dyn → 整数` 在**双路径**上行为一致；
   **恒等转换**（dyn 持 `str` 做 `as str`）不再报 `X06-RUNTIME-002`；
   `dynamic_conversion` 仍在**错误边界**生效（要有一条专门的正向用例，不能只测「不见了」）；
2. **P-1**：**形参形态**（含从不使用的形参、`lint` 与 `lfloat` 两种）在**双路径**上都不再
   产生 `X09-BYTECODE-002`——**夹具必须用形参形状**，这正是第四批漏掉的；
   同时补一条**顶层**形态的对照，证明改动没有把本来好的形状弄坏；
3. **P-2 的整数族方向**（§4.1 丙）：`int`/`sint` → `lint` 与 `lint` → `int`/`sint`
   在双路径上结果一致；窄化越界报**既有**的范围码（`numeric_range` 先例），
   **不许**为它新开码；**浮点/lfloat 方向显式不做**，并在文档里写明；
4. **诊断身份**（§4.2 甲）：验证失败在**进程内路径**上报 `fatal.internal`；
   `xiao-driver/src/run.rs:1048` 那条既有测试的**注释与期望同笔改**
   （注释里写的是被推翻的旧规则）；`X07-FATAL-002` **保留**并登记为「预留给外部产物校验」，
   **不许删**；
5. **诚实**：`02-type-system.md` 的旧表述**要改**（它是错的），但**不许**把旧说法悄悄抹掉——
   按本仓库惯例，**旧批次文档保留其当时的结论**，本批以**指向**订正；
   第四批那条「`lint → str` 已接通」的边界也要收窄到它**实际覆盖过的形状**（顶层）；
6. **通用**：`cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、`bunx tsc --noEmit` 全绿；
   提交标题带类型前缀、正文非空；
7. **粗体配对**（§2.2 的防复发）：凡改动 `README.md` 的 20C 行或本批文档，
   **必须逐行跑一次粗体配对检查**（行内 `**` 计数为奇数即失配）。
   **不许**把它当成「记得看」——它已经复发过一次。

## 九、裁定结果（星崽直接裁定）

| # | 问题 | 裁定 | 落在哪 |
| --- | --- | --- | --- |
| 1 | **本批主体** | **甲：转换与动态边界的收口** | §一 |
| 2 | **F-2 的方向** | **甲：整体收敛到错误边界** | §2.1 |
| 3 | **P-1 与 P-2 做多少** | **丙：P-1 + 整数族方向**（浮点/lfloat 留下一批） | §4.1 |
| 4 | **诊断身份** | **甲：给编译器缺陷自己的身份**——落地是**改报** `fatal.internal`，不是新增码 | §4.2、§4.2.1 |

**来源**：四条均为星崽直接裁定（AskUserQuestion，2026-10-15 两条、2026-10-16 两条）；
四条与审核倾向**全部一致**——但按 [20C](20c-official-xiao-library.md) §2.4 **G-2**，
一致**不等于**推定，本文以**直接裁定**记。

**第 4 条的连带（这一条要留在纸面上）**：裁定是按**我的选项文字**下的，而那段文字有两处不准——
它暗示「要补一个档位」（其实 `fatal.internal` **已经存在**），也暗示「外部读入的产物」是一条**现成**路径
（**我没找到**这条链路）。落点因此比选项窄，且**推翻一条既有决定**
（`xiao-driver/src/run.rs:1048` 那个测试的规则）。逐条见 §4.2.1。
**不许**拿「第 4 条已经裁过」当理由跳过 §4.2.1 里的连带。

## 十、实施与自审核（2026-10-16）

实现按裁定分为两笔，保持 F-2 与高精度数值路径的提交边界：

| 项目 | 落点 | 自审核结论 |
| --- | --- | --- |
| F-2 | `xiao-types` 的 `check_scalar_call`/`check_cast` 不再把普通 `RuntimeChecked` 转成 `dynamic_conversion` | 动态 `str`/`bool`/整数转换统一进入 Runtime；错误边界的 `dynamic_conversion` 保留 |
| P-1 | `xiao-bytecode` 的 `class_for_value` 优先使用 `IrValue.ty`；所有源码形参登记到 `parameter_names` | `lint`/`lfloat` 形参和从不使用的形参不再被错误分配为 `Int` |
| P-2 | `RuntimeValue::convert_to` 新增 `int`/`sint` ↔ `lint` 精确加宽与解析/范围检查 | 窄化越界报既有 `X06-RUNTIME-009`；浮点/lfloat 方向未实现 |
| 诊断身份 | `xiao-vm/src/run.rs:verification_fatal` 对进程内验证失败统一使用 `FatalError::internal` | 既有 `X07-FATAL-006`；`X07-FATAL-002` 保留给未来外部产物校验 |

第 1 问的根因不是 `class_of_type` 或 `declare_parameters`，而是 `class_for_value` 只看
ownership 的 storage 字段：`stack` 被写死映射为 `RegisterClass::Int`。因此普通 `lint`/`lfloat`
绑定在函数调用参数位置与签名的 `ObjHandle` 不一致；修复改用同一 `IrValue.ty` 类型映射。

永久夹具位于 `core/rust/crates/xiao-driver/tests/eighth_batch.rs`，覆盖动态普通值、错误边界、
高精度形参、整数族方向和窄化越界；原生用例使用 `#[ignore]` 由维护回归执行。

## 十一、补探与 CI 取证

改动前补探已确认：VM 的 `lint`/`lfloat` 形参在生产验证阶段报 `X09-BYTECODE-002`，
原生旧路径可构建；顶层 `lint` 形态在 VM 成功。根因落在 `xiao-bytecode/src/lower/mod.rs`
的 `class_for_value`，而 `class_of_type` 与 `declare_parameters` 均已正确使用对象类别。

F-2 独立提交 `acb0f90` 的维护回归 `38773875689` 已取双平台证据；完整实现提交
`3bb7605` 的维护回归 `38779535750` 与 workspace gates `38779535811` 均成功。Linux/Windows
原生作业分别为 `116575834999`/`116575834732`，两边都执行
`native_matches_eighth_batch_boundaries ... ok`；浮点/lfloat 方向继续登记为下一批范围。

| 工作流/作业 | 运行号 | 实测执行点 |
| --- | --- | --- |
| `Xiao maintenance regression` | `38779535750` | 五个作业全成功；默认测试、Linux/Windows 原生和安全回归均通过 |
| Linux 原生差分 | `116575834999` | `eighth_batch::native_matches_eighth_batch_boundaries ... ok` |
| Windows 原生差分 | `116575834732` | `eighth_batch::native_matches_eighth_batch_boundaries ... ok` |
| Linux 默认测试 | `116575834521` | Rust workspace default tests 与 benchmark self-test 成功 |
| `Xiao workspace gates` | `38779535811` | workspace gates 成功，包含 `bun run check`、clippy、lock 和覆盖率门禁 |

第八批 VM 夹具在维护回归默认测试作业中通过；受控 Windows 本机也独立执行了同一原生夹具。
进程内结构验证错误现在报告 `fatal.internal`；`X07-FATAL-002` 未删除，只保留为未来外部
产物校验路径的预留身份。

## 相关页面

- [20C-第七批](20c-seventh-batch-str-to-int-and-ci-coverage.md) §12.1 —— **F-2 的登记出处**（本批前置）
- [20C-第四批](20c-fourth-batch-int-to-str.md) §8.1/§8.3 —— **`lint` 转换债的登记出处**（本批 §三 修正其表述）
- [20C-第五批](20c-fifth-batch-reprobe-and-library.md) §八 第 3 条 —— 「诊断身份与 F-1 的修复同批定」的出处
- [20C-第二批](20c-second-batch-probe.md) §3.2 —— **复探口径的唯一出处**
- [02. 类型系统](02-type-system.md) §显式转换 —— 本批要重述的那段
- [20C-门禁残余项](20c-native-gate-residuals-fix.md) §5.2.1 —— 「全绿运行不构成取证」「本机不算数」
