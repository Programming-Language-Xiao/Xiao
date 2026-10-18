# 20F. 通用容器参数与官方 `set` 模块

> **20E 之后的语言批次**，承接 [20D-规划](20d-platform-resources-stage-plan.md) §6.1
> 与 [03. 容器](03-collections.md) 的「通用容器参数类型」登记。20C 路线稿的
> 「集合」交付物一直缺一个**语言侧**前置：函数形参写不出 `set<T>`，而且函数签名
> 是单态的——同一个函数的所有调用点被统一成同一种元素类型。
>
> **一句话概括本批**：**让公共函数能接受「元素类型由调用方决定」的集合，
> 并用官方 `stdlib/set.xiao` 证明这条能力真的能写库。**
>
> 状态：**实现完成，本地门禁组成项复验通过（2026-10-19，四道裁定均甲）**。语法、量化
> 签名与调用点实例化、刚性统一、生成期擦除、官方 `set` 模块、VM 与归档回归已落地。

## 一、Agent 交接上下文

### 1.1 接手前提

按顺序读：

1. [03. 容器、集合与索引路径](03-collections.md) §集合、§集合运算、
   「通用容器参数类型」登记（`:68`）——**语义与唯一登记落点**；
2. [20D-规划](20d-platform-resources-stage-plan.md) §2.1、§2.2、§6.1——
   为什么「集合」是 20C 的带走项、它的三要素与归属；
3. [20C 官方库](20c-official-xiao-library.md) §3.4.1——**「最小接口」必须真有增量**，
   零增量占位不得入库；
4. [02. 类型系统](02-type-system.md)——HM 变量、统一与泛化的既有边界；
5. [04. 函数与控制流](04-functions-and-control.md)——函数签名、参数种类与返回注解；
6. [20E 归档接线](20e-archive-wiring.md)——`stdlib.*` 模块列表与启动前校验；
7. [18B 收口](18b-build-xar-and-closeout.md) §十二、§十三——本批承接的两项审核发现。

### 1.2 现状盘点（2026-10-19 实测）

| 环节 | 现状 | 结论 |
| --- | --- | --- |
| 函数参数注解 | `FunctionTypeAnnotation` 只有 `Scalar`/`None`/`Resource`（`xiao-syntax/src/ast.rs:523`）；参数解析只识别类型关键字（`xiao-syntax/src/parser/statements.rs:619`） | **写不出 `set<...>`** |
| 集合注解 | 只存在于声明位置：`parse_declared_type`（`xiao-syntax/src/parser.rs:397`）→ `parse_set_type_annotation`（`:440`）；类型项只认标量/`none`/资源（`:508`） | **不能用于函数头，也没有类型变量项** |
| 函数签名 | 预登记与写回都用 `TypeScheme::monomorphic`（`xiao-types/src/function_checker.rs:101`、`:268`、`:349`）；`FunctionSignature` 不携带量化变量（`xiao-types/src/functions.rs`） | **单态：首个调用点钉死元素类型** |
| HM 基础 | `TypeScheme` 有 `quantified`（`xiao-types/src/types.rs:314`）；`generalize`/`instantiate` 已实现（`xiao-types/src/unify.rs:312`、`:321`），且能替换 `Set` 内部变量 | **可复用，但只用于 `let` 绑定** |
| 标准库签名提取 | 只认完整注解，参数未注解即跳过，并明写「不推断跨模块类型变量」（`xiao-driver/src/frontend.rs:203`、`:256`） | **官方集合函数必须写成完整注解** |
| 导入函数调用 | 直接使用具体 `FunctionSignature` 匹配实参（`xiao-types/src/checker/imports.rs:78`；`from` 绑定见 `checker/statement.rs:756`） | **没有调用点实例化** |
| 运行时集合 | `RuntimeValue::Set`（`xiao-runtime/src/value/mod.rs:481`）；`SetHandle` 有 `len`/`contains`/`union`/`intersection`/`difference`/`symmetric_difference` 等（`xiao-runtime/src/containers/set.rs:124`、`:139`、`:170`–`:203`） | **只读能力齐全；没有 `add`/`remove`** |
| 集合静态代数 | `SetType::union`/`intersection`/`difference`/`symmetric_difference`（`xiao-types/src/set_types.rs:169`–`:243`）；运算分派在 `xiao-types/src/set_operations.rs:71` | **可直接复用** |
| 后端对类型变量 | 字节码与 VM 都把 `IrType::Variable` 当作动态值（`xiao-bytecode/src/lower/mod.rs:1004`；`xiao-vm/src/semantics/exec.rs:2386`） | **生成期擦除有现成落点** |
| 官方模块 | `stdlib/` 只有 `text`、`path`、`os`；registry 与 UseDocs 各有三套 | **集合模块不存在** |

### 1.3 本批边界

| 项 | 本批 | 说明 |
| --- | --- | --- |
| 函数显式类型参数 | **做** | 只在顶层函数头；参数与返回注解都能写 `set<T>` |
| `stdlib/set.xiao` | **做** | 首版只有 `size`、`is_empty` 两条只读入口 |
| VM 与 LLVM 双路径 | **做** | 同一源码、同一输入结果一致 |
| `.xar` / 20E 接线 | **做** | `stdlib.set` 进入 `stdlib_modules`，缺模块报 008 |
| 集合增删（`add`/`remove`） | **不做** | 需要 `SetHandle` 可变、intrinsic 契约与双路径绑定；另立批次 |
| 集合运算符的库包装 | **不做** | `contains`/`union` 等与现有 `in`/运算符零增量，按 §3.4.1 不写占位 |
| 非字面量下标、子串/切分 | **不做** | 保持各自登记，本批单主题 |
| 泛型表/方法、显式实例化、约束语法 | **不做** | 见 §七 |

## 二、四道裁定（均甲）

### 2.1 **显式类型参数，不做隐式泛化**

**语法**：

```xiao
def size<T>(set<T> values) -> int
    count = 0
    for value in values
        count = count + 1
    return count
```

**冻结**：

- 类型参数写在函数名之后：`def name<T, U>(...)`；参数与返回注解都可以使用
  `set<T>` 与裸的 `T`；
- 类型参数在**函数体内是刚性的**：`T` 不能与 `int`/`str`/另一个类型参数在体内被合并；
  体内若把 `T` 当具体类型使用，报稳定诊断；
- 函数绑定保存**量化方案**；每个调用点实例化出新的类型变量再统一实参；
- **生成期擦除**：不做单态化、不按实例复制函数体；`IrType::Variable` 在字节码与
  LLVM 两侧继续按动态值处理（`xiao-bytecode/src/lower/mod.rs:1004`、
  `xiao-vm/src/semantics/exec.rs:2386`）。

**为什么不是隐式 HM 泛化**：顶层函数目前先按单态预登记
（`function_checker.rs:101`），前向引用会先约束签名；标准库签名提取也只认完整注解
（`frontend.rs:203`）。隐式泛化会把「先看到谁」变成语义的一部分。显式 `<T>` 让
预登记阶段就能建立方案，**顺序无关**。

**为什么不是动态元素通配**：`set<dynamic>`/`set<*>` 无法把元素实参与容器元素类型
关联起来；`contains` 一类接口会退化成运行时猜类型，且还要新增 `dynamic`/`*` 语法。

### 2.2 **模块名取 `stdlib/set.xiao`**

**冻结**：

- 源码 `stdlib/set.xiao`，导入写作 `from stdlib.set import size, is_empty`；
- 扁平命名，与 `text`/`path`/`os` 一致；
- 模块路径段 `set` 不会进入 `starts_set_type`
  （`xiao-syntax/src/parser.rs:422` 的判定只在声明起点、且要求 `set<` 相邻或第三个
  Token 已是类型项），因此 `stdlib.set` 与 `set()`/`set<...>` 不冲突；
- 不采用 `collections` 伞形命名：本批只含集合接口，名字不能大于内容。

### 2.3 **首版接口只做 `size` + `is_empty`**

**接口**：

```xiao
def size<T>(set<T> values) -> int
def is_empty<T>(set<T> values) -> bool
```

**冻结的语义决定**（按 [20C §3.4.1](20c-official-xiao-library.md) §3.4.1 逐条写）：

| 接口 | 冻结的决定 | 实现 |
| --- | --- | --- |
| `size` | 集合基数与遍历顺序、元素去重无关；空集合为 0；按元素计数，不依赖宿主的 `SetHandle::len` 暴露面 | 纯 Xiao `for` 计数 |
| `is_empty` | 「空集合」就是没有任何元素；`{}` 是空字典表，不属于本接口 | `return size(values) == 0`，同时覆盖「泛型调泛型」 |

- **不包装** `contains`/`union`/`intersection`/`difference`：这些与 `in`、`+`、`&`、`-`
  等价，写进库就是零增量占位（§3.4.1 的 `format_int` 教训）；
- **不做** `add`/`remove`：`SetHandle` 目前没有可变入口
  （`xiao-runtime/src/containers/set.rs:112`–`:282`），要补它必须同时决定
  `SetHandle` 内部可变性、intrinsic 契约、VM/LLVM 绑定与别名可见性；这是另一批的量，
  在本批登记为后续缺口（落点见 §九）。

### 2.4 **只做 20F 主题，不同批并入其它语言缺口**

- 非字面量下标 `s[i]`（`X03-TYPE-003`）继续按 [03](03-collections.md) 登记；
- 子串查找/切分继续按 [20C-第二批](20c-second-batch-probe.md) §3.3 登记；
- 本批不借「反正都在前端」把范围扩大。

## 三、设计细节

### 3.1 语法与 AST

**新增**：

- `FunctionTypeAnnotation::Set(SetTypeAnnotation)`——参数与返回注解共用；
- `TypeTerm` 增加**类型参数名**项：`set<T>`、`set<T | str>` 都能解析；
- `Function` 语句增加 `type_parameters: Vec<Name>`（有序、去重后保存）。

**解析边界**：

- `def name<T>(` 中的 `<` 只在函数名之后进入类型参数解析，不与比较表达式混淆；
- 类型参数名必须是普通/反引号名称；与标量关键字同名（如 `<int>`）、重复名、
  空列表报 `X04-PARSE-007`；
- 类型参数列表只允许出现在**顶层函数**；表方法、局部函数、`config.xiao` 不接受。

### 3.2 类型表示与刚性

**取值方式**（最小改动）：

- 保留 `Type::Variable(TypeVarId)`，新增一个**刚性集合**（`BTreeSet<TypeVarId>`）
  挂在 `TypeChecker` 的函数检查上下文里；
- 函数头登记时，为每个类型参数分配一个新 `TypeVarId` 并加入刚性集合；函数体检查
  期间生效，退出时弹出（支持嵌套局部函数的作用域）；
- 统一规则：刚性变量只能与**同一个**刚性变量统一；与具体类型、动态值或另一刚性
  变量统一都报 `X04-TYPE-008`（消息区分「类型参数被当作具体类型」与「两个类型参数
  被合并」）；
- 函数体检查结束后，把刚性变量作为 `TypeScheme` 的量化变量写回绑定；**递归调用**
  在函数体内使用刚性签名（不允许多态递归），其它函数的调用点实例化。

**跨模块签名的表示**：

- 标准库签名是在**不运行类型检查器**的情况下从源码提取的
  （`xiao-driver/src/frontend.rs:203`），不能与调用方共享 `TypeVarId` 分配器；
- `FunctionSignature` 增加 `type_parameters: Vec<String>`，注解里的类型参数以
  **名字**保存；
- 调用点把这些名字映射到新的 `TypeVarId`（`FunctionTypeAnnotation` →
  `Type::Variable` 的实例化表），再走既有 `check_function_arguments`；
- 这样既避免 ID 碰撞，又让 `stdlib` 的 `def size<T>` 与用户本地函数共用同一套
  实例化逻辑。

**IR 与生成**：

- `lower_type(Type::Variable)` 已产出 `IrType::Variable { id }`
  （`xiao-ir/src/lower.rs:651`）；
- `lower_set_type` 已把集合成员逐个 lower（`:807`）；类型变量成员会落成
  `IrType::Variable`，两个后端都按动态值处理；
- 若 `Type::Rigid` 用独立变体实现，`lower_type` 必须映射为 `IrType::Dynamic`
  （擦除），并在 `xiao-bytecode` 编解码里保持可往返；
- 不新增 `TacOp`、不改 `.xiaoc` 格式、不改 Runtime ABI。

### 3.3 可哈希与实例化校验

- 类型参数可以出现在函数的参数/返回注解中；**只有出现在 `set<...>` 元素位置的
  类型参数**在实例化时要求可哈希。本批不为其它位置的 `T` 增加额外语义
  （例如 `def identity<T>(T value)` 合法，但不构成集合约束）；
- 函数体里 `set<T>` 的元素 `T` 走既有 `hashability` 路径：
  `Type::Variable` 当前返回 `Dynamic`（`xiao-types/src/set_types.rs:314`），
  因此运行时仍会登记 `SetHashability` 检查；
- **调用点实例化后**再做一次静态校验：凡是在 `set<...>` 中出现的类型参数，
  其实参必须落到 `Hashable`（通过）或 `Dynamic`（保留运行时检查）；落到
  `Unhashable` 报 `X03-TYPE-023`；仍未被推断出来的，沿用
  `X04-TYPE-004`（无法推断）而不是默认成 `int`；
- 不引入约束语法（`T: hashable`）——本批的类型参数全部按「可哈希元素」解释，
  写不出别的用法；以后要放宽时再讨论约束。

### 3.4 标准库签名与静态链接

- `standard_module_function_signatures` 增加 `Set` 与类型参数名提取；
  参数仍要求完整注解（`frontend.rs:256` 的跳过规则不变）；
- `plan_standard_link` 对 `stdlib.set` 与其它模块一视同仁：它只看导入符号与
  本地重名，不看类型；追加源码后，`def size<T>` 作为本地泛型函数重新登记，
  调用点实例化；
- 若静态链接与泛型签名在实现中发生冲突，**允许对 `stdlib.set` 关闭静态链接**，
  退回 VM/原生模块调用路径；但两条路径都必须有测试，不能只在一条路径上「看起来能跑」；
- `from stdlib.set import size as size2` 这类别名走导入签名路径，
  必须同样支持实例化（`checker/imports.rs:78`）。

### 3.5 归档与 20E

- `collect_archive_closure` 已把 `stdlib.*` 模块记入 `stdlib_modules`，
  且不把标准库源码复制进 `.xar`；
- 本批不新增索引字段：`stdlib.set` 只是模块列表的一项；
- 验收要求：用 `from stdlib.set import size` 构建 `.xar`，索引出现
  `stdlib.set`；宿主缺少该模块时启动前报 `X17-XAR-008`，版本不符仍报 `007`；
- `stdlib_interface_version` 保持 **1**（兼容扩展，不提升接口主版本）。

## 四、实施步骤

| 步 | 内容 | 交付/验证 |
| --- | --- | --- |
| 0 | 18B 承接项：推送 6 个提交、核销 CI 运行号；索引计数勘误 | 见 §八；推送前不宣称 18B 的 CI 证据 |
| 1 | AST/解析：`Function.parameters` 旁加类型参数、`FunctionTypeAnnotation::Set`、`TypeTerm` 名称项、函数头解析 | 解析快照与负例：`<T>`、`<T, U>`、重复/空/关键字名、`set<T>` 参数与返回 |
| 2 | 类型检查：刚性集合、函数方案、预登记、实例化、诊断（`X04-TYPE-008`/`X03-TYPE-023`/`X04-PARSE-007`） | 检查器单测：多实例、刚性违规、不可哈希实例、前向引用、递归 |
| 3 | 驱动器：标准库签名提取保真、`FunctionSignature` 类型参数名、导入调用实例化、静态链接两条路径 | `frontend` 定向测试；别名导入；本地重名 |
| 4 | 官方模块：`stdlib/set.xiao`、`stdlib/set.md`、UseDocs、registry | VM/LLVM 双路径结果一致；`size`/`is_empty` 语义 |
| 5 | 归档与 20E：`.xar` 索引模块列表、缺模块/版本不匹配负例 | `18b_build_archive` 风格的归档测试 + 20E 启动前检查 |
| 6 | 移除验证与总门禁：变异验证、全量测试、Bun、`run.sh` | 见 §六 |

**最小验证命令**（实施时逐条落成）：

```bash
cargo test -p xiao-syntax --test p2_type_parameters
cargo test -p xiao-types --test f04_functions
cargo test -p xiao-driver --test 20f_container_parameters
cargo test -p xiao-driver --test 18b_build_archive
cargo test --workspace --no-fail-fast
bun test
bash tools/gates/run.sh
```

## 五、接口与语义清单

```xiao
# stdlib/set.xiao（首版）
def size<T>(set<T> values) -> int
def is_empty<T>(set<T> values) -> bool
```

- `size`：逐元素计数；空集合 `set()` 返回 `0`；异构集合同样按实际元素计数；
  不承诺遍历顺序；
- `is_empty`：`size(values) == 0`；不把 `{}`（空字典表）当成空集合；
- 两条都不新增 intrinsic，不需要 Runtime ABI 变更；
- 错误面：非集合实参由 `set<T>` 统一失败并走既有类型诊断；不可哈希实参在
  实例化时由 `X03-TYPE-023` 拒绝；动态边界保留既有运行时检查。

## 六、验收

1. **语法**：`def f<T>(set<T> values, T value) -> bool` 可解析；
   `def f<T, U>(...)` 可解析；空/重复/关键字类型参数与缺失 `>` 各自有稳定诊断；
2. **多实例**：同一程序内 `size({1, 2})` 与 `size({"a", "b"})` 都通过，
   VM 与 LLVM 原生输出一致——这是本批防止退回单态的关键用例；
3. **跨模块**：`from stdlib.set import size, is_empty` 在源码、`.xiaoc`、`.xar`
   与原生四条路径都可用；无别名与 `as` 别名两条导入路径都覆盖；
4. **刚性**：函数体内把 `T` 当具体类型（例如与 `int` 合并）必须报
   `X04-TYPE-008`，不能静默特化；
5. **可哈希**：`set<T>` 的类型参数实例化为不可哈希类型时报 `X03-TYPE-023`；
   未推断出的实例沿用 `X04-TYPE-004`；动态边界保留运行时检查；
6. **模块语义**：`size` 对空集、异构集合、单元素集合的结果正确；
   `is_empty` 与 `size == 0` 一致；两组在 VM/LLVM 一致；
7. **归档**：`.xar` 的 `stdlib_modules` 含 `stdlib.set`；宿主缺失时报
   `X17-XAR-008`，接口版本不符仍报 `007`；required-feature 位不变；
8. **移除验证**：删掉调用点实例化（强制回单态）或删掉标准库函数体，
   多实例/模块用例必须失败；恢复后全绿；
9. **不回归**：`cargo test --workspace`、`cargo clippy --workspace --all-targets --
   -D warnings`、`bun test`、`bash tools/gates/run.sh` 全绿；既有 `text`/`path`/`os`
   与 20E 用例不受影响；
10. **18B 承接**：F-1 推送后核销维护回归与工作区门禁运行号；
    F-2 的索引计数与 §十二 一致（`12` 项定向、`1236` 项 workspace）；
11. **文档**：`00-decisions`、`02-type-system`、`03-collections`、
    `04-functions-and-control`、20 路线稿、README、registry、UseDocs 同步；
    新模块的 UseDocs 达到 `verified` 才能把 registry 条目翻成 `verified`。

## 七、不负责与不要重复做的事

- **不做** `add`/`remove`、集合方法式 API、`frozenset`；
- **不做** `contains`/`union`/`intersection`/`difference` 等零增量包装；
- **不做**泛型表/表方法、显式实例化 `f<int>(...)`、约束语法、多态递归；
- **不做**非字面量下标、子串/切分、常量折叠；
- **不做**单态化、`.xiaoc` 格式变更、Runtime ABI 变更、新 intrinsic；
- **不做**把官方标准库注册成普通包或写进依赖锁文件。

## 八、承接 18B 审核（F-1 / F-2）

审核对象：`ee732db` → `98d9128`（18B 收口批，共 6 个提交）。已实测：

| 项 | 结果 |
| --- | --- |
| `18b_build_archive` | 12 通过 / 0 失败 |
| `20e_archive_wiring` | 11 通过 / 0 失败 |
| `d19a_differential` 四路 | 1 通过 |
| workspace | 1236 通过 / 0 失败 / 41 ignored |
| `bash tools/gates/run.sh` | 退出码 0；Bun 298 通过 / 5 跳过 / 0 失败 |
| 变异验证 | 关闭 unresolved 拒绝后 `review_mixed_known_and_unresolved_targets_cannot_omit_capabilities` 如期变红；已还原，工作树干净 |

**F-1（证据缺口）**：这 6 个提交尚未推送（审核时 `main...origin/main [ahead 6]`），
18B 文档与索引已写「已完成并验收通过」，但没有任何 CI 运行号。

- 本批第 0 步：推送后核销 `maintenance-regression` 与 `workspace-gates`；
- 在运行号落地前，18B 的状态只能表述为「本地验收通过，CI 待推送」；
- 本批完成时必须回填运行号，不能把本地门禁当作远端证据。

**F-2（索引计数漂移）**：README 的 18B-收口行原写「9 项 Rust 定向回归 / workspace 1233」，
与文档 §十二 的「12 项 / 1236 项」不一致。

- 规划时已按下表勘误（这是索引事实，不是行为变化）；
- 实施时复查 README 行与 §十二 仍一致。

| 项 | 原索引 | 实际（§十二 与实测） |
| --- | --- | --- |
| Rust 定向回归 | 9 | **12** |
| workspace | 1233 | **1236** |
| Bun | 298 | **298**（一致） |

## 九、落点

```text
docs/DevDocs/20f-container-parameters-and-collections.md   本文（规划）
docs/DevDocs/README.md                                     20F 索引行；18B 计数勘误
docs/DevDocs/03-collections.md                             「通用容器参数」登记指向本批
docs/DevDocs/20-builtins-and-standard-library.md           §七 阶段表补 20F 一行
docs/DevDocs/00-decisions.md                               显式类型参数的语言决策
docs/DevDocs/02-type-system.md                             刚性变量、方案与实例化边界
docs/DevDocs/04-functions-and-control.md                   函数头类型参数语法
core/rust/crates/xiao-syntax/src/ast.rs                    类型参数、Set 注解、名称项
core/rust/crates/xiao-syntax/src/parser/statements.rs      函数头 <T> 与参数注解
core/rust/crates/xiao-syntax/src/parser.rs                 set<T> 名称项与错误恢复
core/rust/crates/xiao-types/src/types.rs                   类型表示（变量/量化）
core/rust/crates/xiao-types/src/functions.rs               FunctionSignature 类型参数名
core/rust/crates/xiao-types/src/function_checker.rs        刚性、预登记、方案写回
core/rust/crates/xiao-types/src/unify.rs                   刚性统一规则（若独立实现）
core/rust/crates/xiao-types/src/checker/imports.rs         导入调用实例化
core/rust/crates/xiao-types/src/checker/statement.rs       from 导入绑定
core/rust/crates/xiao-types/src/diagnostics.rs             X04-TYPE-008 / X03-TYPE-023
core/rust/crates/xiao-driver/src/frontend.rs               标准库签名提取与静态链接
core/rust/crates/xiao-ir/src/lower.rs                      类型擦除检查（若需）
stdlib/set.xiao                                            size / is_empty
stdlib/set.md                                              模块接口与兼容身份
docs/UseDocs/library/set.md                                用户文档
docs/module-registry.json                                  stdlib.xiao-standard-library-set
core/rust/crates/xiao-driver/tests/20f_container_parameters.rs
tests/fixtures/20f/                                        多实例/负例夹具
```

**后续缺口登记**（不属本批，写清归属）：

- `add`/`remove`：需要 `SetHandle` 可变、intrinsic 契约、VM/LLVM 绑定；
  归属下一个集合批次；
- 集合运算符的库包装：只有能写出新的语义决定时才立项；
- 非字面量下标、子串/切分：保持 [03](03-collections.md) 与
  [20C-第二批](20c-second-batch-probe.md) §3.3 的登记。

## 十、实施与验证记录（2026-10-19）

- **语法/AST**：`Function` 增加有序类型参数，函数头支持 `def f<T, U>(...)`；
  `FunctionTypeAnnotation` 支持裸类型参数与 `set<T | U>`，集合类型项支持类型参数。
  空列表、重复名、标量关键字名和非顶层类型参数使用 `X04-PARSE-007`。
- **类型与签名**：`FunctionSignature` 同时保存类型参数名称和类型变量身份；函数绑定写入
  `TypeScheme::quantified`，每个本地或跨模块调用点通过独立实例变量统一实参。集合异构
  实参会退化为动态边界，函数体内刚性变量与具体类型合并报 `X04-TYPE-008`，集合元素
  的不可哈希实例报 `X03-TYPE-023`。`IrType::Variable` 在生成期继续擦除为动态值，
  不做单态化，也不改 `.xiaoc` 与 Runtime ABI。
- **官方模块**：新增 `stdlib/set.xiao` 与 `size<T>`、`is_empty<T>`，通过纯 Xiao
  `for` 计数，不新增 intrinsic；导入和别名由标准库签名提取路径接入。
- **归档**：真实 `build_archive` 用例确认 `stdlib_modules` 包含 `stdlib.set`，归档不携带
  官方对象；宿主标准库版本与缺模块启动检查继续复用 20E。
- **定向验证**：`p2_type_parameters` 4 项通过；`20f_container_parameters` 7 项通过，
  覆盖解析、多实例 VM、官方模块别名、空集合、标量泛型、递归刚性诊断和归档索引。
- **总门禁**：workspace Rust 测试、严格 Clippy、格式检查、Bun 298 项测试、文档/UseDocs
  检查、覆盖率门禁、TypeScript 类型检查和 benchmark 自测均通过；原生相关用例按既有
  工具链环境门控保持忽略。
- **文档登记**：同步 `00-decisions`、`02-type-system`、`03-collections`、
  `04-functions-and-control`、20 路线稿、UseDocs、registry；`add/remove`、泛型表、
  约束语法和非字面量下标保持后续缺口。

## 相关页面

- [03. 容器、集合与索引路径](03-collections.md) §集合、「通用容器参数类型」
- [20D 平台资源阶段规划](20d-platform-resources-stage-plan.md) §6.1
- [20C 官方标准库](20c-official-xiao-library.md) §3.4.1
- [20E 归档接线](20e-archive-wiring.md)
- [18B `build -xar` 与验收对账](18b-build-xar-and-closeout.md) §十二、§十三
- [02. 类型系统](02-type-system.md) / [04. 函数与控制流](04-functions-and-control.md)
