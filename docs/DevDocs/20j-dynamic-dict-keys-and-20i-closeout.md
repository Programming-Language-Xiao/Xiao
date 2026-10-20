# 20J. 字典动态键与 20I 收口

> **20I 之后的语言批次**。20I 把表达式下标接到了有序容器，但审核实测留下两处
> 会误伤正常代码的问题：**`array_push` 后静态长度不失效**、**动态正越界与字面量
> 越界用了不同错误码**；字典表的动态键也一直明确留在边界外。本批先收 20I，
> 再把字典表/字典列的动态键接通。
>
> **一句话概括本批**：**让 `d[key]` 在键名不冲突时按变量查表，同时把 20I 的
> 数组长度与越界身份收成一套。**
>
> 状态：**已实施、待四平台 CI 取证（2026-10-20）**。四道裁定：字典表与字典列都动态、
> 静态键优先、缺失键报运行时错误、结果取共同值类型/dynamic。实现已落在类型检查、
> VM、Runtime ABI、LLVM、归档路径、门禁脚本和回归夹具。

## 一、Agent 交接上下文

### 1.1 接手前提

按顺序读：

1. [20I 非字面量下标、数组注解与文本切分](20i-nonliteral-indexing-and-splitting.md)
   —— **直接前置**；其 F-1–F-7 是本批的返工输入，§4.1 的表达式下标是动态键的语法基础；
2. [03. 容器、集合与索引路径](03-collections.md) §字典表、§字典列、§可索引能力边界
   —— 键路径与位置路径的既定语义；
3. [20D-规划](20d-platform-resources-stage-plan.md) §2.2——非字面量下标/切分的登记链；
4. [20G](20g-set-mutation-and-20f-closeout.md) §4.4 与 [20I](20i-nonliteral-indexing-and-splitting.md)
   §4.3——intrinsic/`array_push` 的契约与移除验证先例；
5. 错误身份以 `xiao-diagnostics/src/lib.rs` 为准：`X06-RUNTIME-014` 容器索引越界、
   `X06-RUNTIME-015` 字典键不存在、`X06-RUNTIME-017` 选择器边界；
6. [18B 收口](18b-build-xar-and-closeout.md) §八——推送 CI 的取证口径。

### 1.2 现状盘点（2026-10-20 实跑）

| 环节 | 现状 | 结论 |
| --- | --- | --- |
| 20I 定向 | `20i_indexing_and_text` 11、`20h_set_methods` 10、`20g_set_mutation` 7、`d17c_archive` 13、`18b_build_archive` 13、`r2a_containers` 17 通过 | 主干成立 |
| 全工作区 | 本批 `cargo test --workspace --no-fail-fast` 全量通过；Bun 既有基线 298 通过 / 5 跳过 / 0 失败 | Rust 主体成立，Bun 未改动 |
| 20I 推送 | `ecea72f`/`7731189` **未推送**（审核时 ahead 2） | 无 CI |
| 原生/归档 | 手工探针的表达式下标、`find`/`slice`/`split`、`array_push` 追加与迭代通过 | 可用；缺自动化原生执行 |
| `array_push` 后读新位置 | `values = [10,20]; array_push(values,30); values[2]` → 输出 `30`；辅助函数和别名同样通过 | F-1 已修 |
| 动态正越界 | `text[i]` / 数组 / 元组 / 字典列 → `X06-RUNTIME-017` | 与字面量/动态负统一；F-2 已修 |
| 字典列裸名称 | `column[i]` 有同名键时取键，`column[k]` 无同名键按 str 键，`column[i+0]` 取位置 | F-3 已冻结 |
| 字典表动态键 | 无同名静态键时，`str` 裸名称/表达式按动态键读取；缺失报 `X06-RUNTIME-015` | 本批已接通 |
| 运行时字典键 | `X06-RUNTIME-015`（字典中不存在该键）已存在 | 动态缺失键可复用 |
| 20I 门禁 | `run.sh` 现已记录全量 Bun 失败并继续执行两条隔离重试 | F-4 本地修复，待门禁取证 |
| 平台复现 | Windows 脚本现已把 MSVC `link.exe` 放到 PATH 首位 | F-5 本地修复，待四平台取证 |

### 1.3 本批边界

| 项 | 本批 | 说明 |
| --- | --- | --- |
| 20I F-1–F-7 | **做** | 数组静态长度、越界身份、字典列规则、门禁重试、平台复现、推送证据、文档 |
| 字典表动态键 | **做** | 表达式/裸名称按规则解析；str 键；共同值类型/dynamic |
| 字典列动态键 | **做** | str 键；int 变量/表达式继续按位置；静态键优先 |
| 字典表/列的静态键 | **保持** | 现有 `d["name"]`、`d[name]`（同名键）不变 |
| 集合/数组的动态键 | **不做** | 集合不可索引；数组只做位置表达式 |
| 字典注解语法、嵌套容器 | **不做** | 保持后置 |
| 字典增删方法 | **不做** | 只做读取 |

## 二、20I 审核返工项（先做）

### F-1 **`array_push` 后静态长度不失效（已修）**

**证据**：

```xiao
values = [10, 20]
array_push(values, 30)
print(values[2])
→ X03-TYPE-004: 选择器索引超出容器长度 2
```

辅助函数追加后同样：

```xiao
def append([int] values, int value) -> none
    array_push(values, value)

values = [10, 20]
append(values, 30)
print(values[2])
→ X03-TYPE-004
```

**要求**：

1. 直接 `array_push` 后，接收者数组的已知长度失效（改为未知长度/同构未定长）；
2. 数组作为参数传给可能修改它的函数后，调用方的已知长度也要保守失效，或在文档中
   明确“只保证直接 `array_push` 的失效”，并给出可接受的替代写法；
3. 补两条正例：直接追加后读新位置、辅助函数追加后读新位置；
4. 原有静态越界负例（`values[2]` 无追加）仍必须拒绝。

### F-2 **动态正越界与字面量/负越界的错误码不一致（已修）**

**证据**：

| 场景 | 错误码 |
| --- | --- |
| `"abc"[5]`（字面量越界） | `X06-RUNTIME-017` |
| `i = -1; text[i]`（动态负） | `X06-RUNTIME-017` |
| `i = 1; text[i]` / 数组 / 元组 / 字典列（动态正越界） | **`X06-RUNTIME-014`** |

20I 计划 §4.1 写“运行时沿用 `X06-RUNTIME-017`”，实现记录只写了动态负数 017；测试
没有覆盖动态正越界。**要求**：冻结一个身份（建议统一为 `X06-RUNTIME-017`，或在
文档把 014/017 的分工写清并让测试覆盖两者），补动态正越界用例；不得留下“同一语义
两个码、调用方按 017 捕获会漏”的状态。

### F-3 **字典列裸名称歧义（已冻结）**

**证据**：

```xiao
column = <i = 10, j = 20>
i = 1
print(column[i])      # 10（键）
print(column[i + 0])  # 20（位置）
```

**要求**：本批的静态键优先规则同时覆盖字典列：
- 裸名称与静态键同名 → **静态键**；
- 无同名键且变量/表达式是 `int` → 位置；
- 无同名键且变量/表达式是 `str` → 动态键；
- 两者都不是 → 稳定类型错误。
补 `column[i]`、`column[k]`、`column[i + 0]` 三条冻结用例。

### F-4 **`run.sh` 的 REPL 隔离重试在失败时不可达（已修）**

`run.sh` 以 `set -Eeuo pipefail` 开头，而新增的隔离重试段在 `bun test` 之后；全量
Bun 一旦因抖动失败，脚本立即退出，重试段永远不执行。要求：

1. 允许 `bun test` 失败并记录，再跑隔离重试用例；两者都通过才判定门禁通过；
2. 或把两条抖动用例移出串行门禁，单独跑并写明结果；
3. 保留“模式匹配到 0 条测试时 Bun 退出 1”的防假绿行为（已实测成立）。

### F-5 **平台复现 Windows 仍红（脚本已修，待 CI）**

`39227168623` 的 `windows-amd64` 仍因 `C:\Program Files\Git\usr\bin\link.exe`
抢先于 MSVC 的 `link.exe` 失败（`/usr/bin/link: extra operand`）。20I 记录写
“已有 vcvars64.bat 导入”，但这不改变 PATH 顺序。要求把 MSVC 工具链放到前面或
用 cmd/pwsh 跑适配器，并重跑四平台 + ordered summary。

### F-6 **推送与证据**

- 推送 20I 的 `ecea72f`/`7731189`，核销维护回归与工作区门禁；
- 原生 `array_push`/`split`/表达式下标目前只有降低用例；补受控原生执行回归；
- `array_push` 的移除验证（删表项/VM 绑定/ABI 包装任一环节）补用例；
- `.xiaoc` 文本切分路径补一条实际运行用例（`.xar` 已有）。

### F-7 **文档**

- 20I §二 F-1 仍写 `ahead 1`/未推送；`66856a2` 已推送且
  `39262231800`/`39262231787` 全绿，应关闭该条；
- 20I 的 F-3/F-4 状态按本批实际修复回填；
- 20H/README 的运行号与状态随 F-1 更新。

## 三、四道裁定

| # | 裁定 | 结果 |
| --- | --- | --- |
| 1 | 动态键范围 | **字典表 + 字典列都动态** |
| 2 | 裸名称消歧 | **静态键优先**；无同名键时按变量类型（str 动态键 / int 位置），表达式形式总是动态/位置 |
| 3 | 缺失键 | **运行时错误**，沿用 `X06-RUNTIME-015` |
| 4 | 结果类型 | **共同值类型 / dynamic**；异构值退化为 dynamic |

## 四、设计

### 4.1 语法与消歧

**形式**：

```xiao
profile["name"]          # 静态键
profile[name]            # 同名静态键优先；否则 name 为 str 时动态键
profile[name + ""]       # 表达式形式，总是动态键
column[i]                # 同名静态键优先；否则 i 为 int 时位置
column[key]              # 同名静态键优先；否则 key 为 str 时动态键
column[i + 0]            # 表达式形式，按 int 位置
column[key + ""]         # 表达式形式，按 str 动态键
```

**冻结**：

- 静态键优先级适用于字典表和字典列：**同名键存在时，裸名称永远按键**；
- 无同名键时，裸名称按变量类型分派：str → 动态键；int → 位置（仅字典列）；
- 显式表达式（非裸名称）不参与静态键优先，按表达式类型分派；
- 两种容器都**只支持单个精确项**；多选/范围/步长仍要求静态端点；
- 键表达式与索引表达式都只求值一次；接收者只求值一次；
- 字典表动态键必须是 `str`；字典列动态键是 `str`，位置是整数族（int/sint/lint）。

### 4.2 类型检查

- **字典表**：动态键表达式类型必须是 `str`（或 dynamic/变量，运行时检查）；
  结果类型 = 所有值类型的共同类型；异构 → `dynamic`；
  缺失键 → 运行时 `X06-RUNTIME-015`；
- **字典列**：
  - 动态键（str）→ 共同值类型/dynamic；缺失键 → `X06-RUNTIME-015`；
  - 位置（整数族）→ 共同值类型/dynamic；越界 → 运行时 `X06-RUNTIME-014`
    （或按 F-2 冻结的统一身份）；
- 静态键仍走现有 `X03-TYPE-005`；静态位置仍走 `X03-TYPE-004`；
- 动态键/位置登记运行时检查；表达式类型错误用 `X03-TYPE-003`；
- 结果类型擦除与 20F/20I 一致；不新增 `dynamic` 兜底文本。

### 4.3 IR/Runtime

- 字典表动态键无法复用纯数值的 `IndexGetDynamic`；需要新增“按键取值”的
  IR/TAC 动作或运行时入口（名称由实现定，例如 `DictGetDynamic`），
  参数为接收者与键值；字典列动态键走同一入口，位置继续走数值索引；
- 运行时使用 `DictHandle::value(key)`（或等价只读入口）：
  - 键不存在 → `XiaoError::key_not_found`（`X06-RUNTIME-015`）；
  - 位置越界 → 现有容器索引错误（`X06-RUNTIME-014`，按 F-2 统一）；
- VM 与 LLVM 动态路径共用同一 ABI；`.xiaoc` 若新增 opcode，沿用 20G
  `IterSnapshot` 的 opcode 扩展实践并补编解码/验证用例；
- 不新增能力位、不改 Runtime ABI 主版本、不把字典复制进归档对象；
- 结果值按现有引用语义返回；不引入哈希索引/排序变化。

### 4.4 与 20I 的边界

- 20I 的表达式下标选择项直接复用；字典表/列从“排除”改为“按键/位置分派”；
- F-1 的数组长度失效与 F-2 的越界身份是本批前置，不与动态键混为一谈；
- F-3 的字典列规则在本批冻结，不再留“裸名称歧义”；
- 字典注解语法、动态键的写操作、字典增删方法仍不在本批。

## 五、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 推送 20I；核销 CI；修 F-4/F-5 | 运行号/门禁 |
| 1 | F-1：数组长度失效（直接/辅助函数）；正负例 | 类型/诊断 |
| 2 | F-2：冻结越界身份；动态正/负/字面量三组 | 诊断 |
| 3 | F-3：字典列静态键优先 + 类型分派；三组冻结用例 | 类型/运行 |
| 4 | 字典表动态键：解析/类型/共同值类型/缺失错误 | VM |
| 5 | 字典列动态键与位置：str/int 分派、缺失/越界 | VM |
| 6 | IR/TAC/ABI/VM/LLVM：按键取值；`.xiaoc`/`.xar`/原生 | 四路 |
| 7 | 原生执行/移除验证/全量门禁/文档 | 见 §六 |

**最小命令**：

```bash
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20i_indexing_and_text
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20j_dynamic_dict_keys
cargo test --manifest-path core/rust/Cargo.toml -p xiao-runtime --test r2a_containers
cargo test --manifest-path core/rust/Cargo.toml --workspace --no-fail-fast
bun test
bash tools/gates/run.sh
```

## 六、验收

1. **F-1 关闭**：直接/辅助函数 `array_push` 后读新位置都通过；无追加的静态越界仍拒绝；
2. **F-2 关闭**：动态正/负/字面量越界按冻结身份一致；测试覆盖三者；
3. **F-3 关闭**：字典列 `column[i]`（同名键）、`column[k]`（str 动态键）、
   `column[i + 0]`（位置）三条规则都有用例；
4. **F-4 关闭**：`run.sh` 在 Bun 抖动失败后仍会执行隔离重试，并据结果判定；
5. **F-5 关闭**：平台复现 Windows 四平台 + summary 全绿；
6. **F-6 关闭**：20I 推送 CI 全绿；原生执行、`.xiaoc` 文本切分、`array_push`
   移除验证齐全；
7. **字典表动态键**：静态键优先、str 变量动态键、显式表达式、缺失键
   `X06-RUNTIME-015`、异构值 dynamic、单次求值；
8. **字典列动态键/位置**：str/int 分派、静态键优先、缺失/越界身份、共同值类型；
9. **四路**：VM、`.xiaoc`、`.xar`、原生结果逐字一致；
10. **不回归**：workspace、Bun、`run.sh` 全绿；20F–20I/20E/18B 用例不受影响；
11. **文档**：00/02/03/04、20 路线稿、README、registry、UseDocs 同步；
    新能力在四路证据齐全后才翻 `verified`。

## 七、不负责

- **不做**字典注解语法、嵌套容器注解、字典增删方法；
- **不做**集合索引、数组键（只做位置）；
- **不做**多选/范围/步长的动态端点；
- **不做**新能力位、Runtime ABI 主版本或哈希索引/排序变更；
- **不把**动态键实现成第二套选择器语义；
- **不在**本批顺手扩 20I 之外的数组/文本能力。

## 八、落点

```text
docs/DevDocs/20j-dynamic-dict-keys-and-20i-closeout.md   本文
docs/DevDocs/README.md                                   20J 索引行；20I 状态
docs/DevDocs/20-builtins-and-standard-library.md         §七 阶段表补 20J
docs/DevDocs/03-collections.md                           字典动态键/字典列规则
docs/DevDocs/00-decisions.md                             四道裁定
docs/DevDocs/20i-nonliteral-indexing-and-splitting.md    F-1–F-7 对账
core/rust/crates/xiao-syntax/src/selectors.rs            动态键选择项
core/rust/crates/xiao-syntax/src/parser.rs               字典键表达式解析
core/rust/crates/xiao-types/src/selector_checker.rs      静态键优先/类型分派
core/rust/crates/xiao-types/src/container_checker.rs     结果类型/错误身份
core/rust/crates/xiao-ir/src/model.rs                    按键取值动作
core/rust/crates/xiao-ir/src/lower.rs                    动态键 lowering
core/rust/crates/xiao-bytecode/src                       编解码/验证/liveness
core/rust/crates/xiao-runtime/src/containers/dict.rs     只读按键取值
core/rust/crates/xiao-runtime/src/abi_dynamic.rs         动态键 ABI
core/rust/crates/xiao-vm/src/semantics                    VM 动态键
core/rust/crates/xiao-codegen-llvm/src/dynamic            LLVM 动态键
core/rust/crates/xiao-driver/tests/20j_dynamic_dict_keys.rs
core/rust/crates/xiao-driver/tests/20i_indexing_and_text.rs  F-1/F-2/F-3 回归
```

## 相关页面

## 九、实施记录（2026-10-20）

- **F-1**：`array_push` 及数组实参函数调用会擦除绑定上的固定长度并保留元素类型；直接追加和辅助函数追加后读取新位置通过，无追加的静态越界仍报 `X03-TYPE-004`。
- **F-2**：动态整数/字符串选择统一走 `X06-RUNTIME-017`；动态字典缺失键走 `X06-RUNTIME-015`。
- **F-3**：字典表与字典列支持动态键；静态键优先，字典列表达式按 `str` 键或整数位置分派，异构值退化为 `dynamic`。
- **F-4/F-5**：`run.sh` 在全量 Bun 失败后继续执行隔离重试；Windows 平台复现脚本导入 MSVC 后把其 `link.exe` 放到 PATH 首位。四平台运行号仍待推送取证。
- **字节码/Runtime/LLVM**：复用 `IndexGetDynamic`，新增 `xiao_runtime_value_index_get_dynamic` 处理动态整数位置和字典字符串键；`.xiaoc`/`.xar` 沿用现有编解码与宿主 Runtime ABI。
- **本地验证**：`20j_dynamic_dict_keys` 7 项、`20i_indexing_and_text` 11 项、`d17c_archive` 13 项、`r2a_containers` 17 项通过；`.xiaoc` 动态键与 `.xar` 动态键回归已加入。原生四平台、推送 CI 与 `array_push` 移除验证待外部取证。

- [20I 非字面量下标、数组注解与文本切分](20i-nonliteral-indexing-and-splitting.md)
- [03. 容器、集合与索引路径](03-collections.md)
- [20H 集合方法式 API 与 20G 收口](20h-set-methods-and-20g-closeout.md)
- [20D 平台资源阶段规划](20d-platform-resources-stage-plan.md) §2.2
- [20AB intrinsic 契约与最小入口](20ab-intrinsics-contract-and-minimal-entry.md)
- [18B `build -xar` 与验收对账](18b-build-xar-and-closeout.md) §八
