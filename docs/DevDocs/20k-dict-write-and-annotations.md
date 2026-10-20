# 20K. 字典写入、查询接口与字典注解

> **20J 之后的语言批次**。20J 把字典表/字典列的**动态读**接通了，但审核实测发现
> 动态写入路径会在降低阶段报 `X09-BYTECODE-001`；同时，20I 的数组长度失效还有
> 成员接收者与过度失效两处残余。本批先收这些，再把字典写入、`stdlib/dict.xiao`
> 查询接口和 `dict<str, T>` 注解接通。
>
> **一句话概括本批**：**让 `d[key] = value` 真正可写，并让字典第一次能写进函数签名。**
>
> 状态：**已实施、待四平台 CI 取证（2026-10-20）**。四道裁定：只更新已有键、赋值 +
> stdlib 自由函数、`dict<str, T>` 注解、删除缺失报 015、keys/values 返回数组。
> 本地 VM、`.xiaoc` 降低和 LLVM 文本降低已接通；四平台原生执行与推送取证仍留在外部门禁。

## 一、Agent 交接上下文

### 1.1 接手前提

按顺序读：

1. [20J 字典动态键与 20I 收口](20j-dynamic-dict-keys-and-20i-closeout.md)
   —— **直接前置**；其 F-1–F-6 是本批返工输入，§4.1 的静态键优先规则继续有效；
2. [20I 非字面量下标、数组注解与文本切分](20i-nonliteral-indexing-and-splitting.md)
   —— 表达式下标、`[T]` 注解与 `array_push` 的静态长度问题；
3. [03. 容器、集合与索引路径](03-collections.md) §字典表、§字典列——键/位置语义；
4. [20F](20f-container-parameters-and-collections.md) 与 [20I](20i-nonliteral-indexing-and-splitting.md)
   §4.2——类型参数与 `[T]` 注解的实例化机制；
5. [20AB intrinsic 契约](20ab-intrinsics-contract-and-minimal-entry.md) 与
   [20G](20g-set-mutation-and-20f-closeout.md) §4.4——intrinsic/移除验证先例；
6. 错误身份以 `xiao-diagnostics/src/lib.rs` 为准：`X06-RUNTIME-014` 容器索引越界、
   `X06-RUNTIME-015` 字典键不存在、`X06-RUNTIME-017` 选择器边界；
7. [18B 收口](18b-build-xar-and-closeout.md) §八——推送 CI 的取证口径。

### 1.2 现状盘点（2026-10-20 实跑）

| 环节 | 现状 | 结论 |
| --- | --- | --- |
| 20J 定向 | `20j_dynamic_dict_keys` 7、`20i_indexing_and_text` 11、`20h_set_methods` 10、`20g_set_mutation` 7、`d17c_archive` 13、`18b_build_archive` 13、`r2a_containers` 17 通过 | 读路径成立 |
| 全工作区 | **1293 通过 / 0 失败 / 42 ignored**；Bun 298 通过 / 5 跳过 / 0 失败；`run.sh` 退出 0 | 本地主体成立 |
| 静态键写入 | `values["name"] = 2`、字典列 `column["first"] = 2` 可运行 | 已有能力 |
| 动态键写入 | `values[key] = 2` → `X09-BYTECODE-001: selector_bounds 未消费` | **阻断**；见 F-3 |
| 动态位置写入 | `column[i] = 2` → 同一 `X09-BYTECODE-001` | **阻断** |
| 新增键写入 | `values["new"] = 2` → `X03-TYPE-005` | 本批裁定只更新已有键 |
| `array_push` 成员接收者 | 表字段数组 push 后读新位置仍 `X03-TYPE-004` | 20I F-1 残余 |
| 只读调用后的数组 | `first(values); values[2]` 不再静态拒绝，运行时 `X06-RUNTIME-014` | 20I F-1/F-2 残余 |
| 字典注解 | 函数参数/返回没有字典类型语法 | 本批新增 |
| 字典查询接口 | 只有下标读取；无 get/set/remove/contains/keys/values | 本批新增 |
| 20I/20J 推送 | 审核时 **ahead 5**，两者都未推送 | 无 CI；见 F-6 |
| `run.sh` | 隔离重试可达并实测重试成功，但全量 Bun 失败仍直接判红 | F-4 部分修复 |

### 1.3 本批边界

| 项 | 本批 | 说明 |
| --- | --- | --- |
| 20J F-1–F-7 | **做** | 数组长度、越界身份、动态写降低、门禁、平台、推送、文档 |
| `d[key] = value` | **做** | 既有键更新；缺失键运行时 015；不允许新增键 |
| 字典列动态键/位置写入 | **做** | 既有键/既有位置；越界 017 |
| `dict<str, T>` 注解 | **做** | 函数参数与返回位置；跨模块 |
| `stdlib/dict.xiao` | **做** | get/set/remove/contains/keys/values 自由函数 |
| 新增键/upsert | **不做** | 静态形状不变；新增键留后续 |
| 字典方法式 API | **不做** | 不开成员分派；只用赋值与自由函数 |
| 嵌套字典注解、局部字典注解 | **不做** | 保持后置 |

## 二、20J 审核返工项（先做）

### F-1 **`array_push` 长度失效的成员残余与过度失效**

**证据 A（误拒）**：

```xiao
[[Holder]]
    values = [10, 20]

item = new Holder()
array_push(item.values, 30)
print(item.values[2])
→ X03-TYPE-004
```

失效只处理 `Expression::Name` 与名称别名；表字段/容器成员仍保留旧长度。

**证据 B（过度失效）**：

```xiao
def first([int] values) -> int
    return values[0]

values = [10, 20]
first(values)
print(values[2])
→ 不再静态拒绝，运行时 X06-RUNTIME-014
```

任何已知函数调用只要带数组实参都会失效，只读函数也不例外。要求：

1. 成员接收者（表字段、嵌套容器）的数组在 `array_push` 后也要失效，或给出可接受的限制并写明；
2. 失效范围不要覆盖“只读”调用；若无法做变异摘要，至少在文档写明“凡传数组给函数即失效”的取舍，并保留静态负例；
3. 补成员接收者正例、只读调用后的诊断用例。

### F-2 **长度失效后的字面量下标仍走 `X06-RUNTIME-014`**

| 场景 | 结果 |
| --- | --- |
| 无调用的 `values[2]` | 静态 `X03-TYPE-004` |
| `first(values)` 后的 `values[2]` | **`X06-RUNTIME-014`** |
| `i = 1; values[i]` / `text[i]` | `X06-RUNTIME-017` |
| `"x"[1]` | `X06-RUNTIME-017` |

要求：冻结一个运行时越界身份（建议全部 017），并让“字面量下标因失效进入运行时”也走
同一码；补 `first(values); values[2]` 用例。若保留 014/017 分工，必须把路径写进文档并
让测试覆盖两种。

### F-3 **动态键/位置写入在降低阶段报 `X09-BYTECODE-001`（本批主功能的前置）**

**证据**：

```xiao
values = {name = 1}
key = "name"
values[key] = 2
→ X09-BYTECODE-001: 生产字节码含未降低构造：运行时检查未消费：selector_bounds
```

字典列 `column[key] = 2` 与 `column[i] = 2` 同样复现。类型检查为动态写目标登记了
`selector_bounds` 运行时检查，但赋值降低没有消费它。要求：

1. 本批的写入 lowering 必须消费该检查并生成动态键/位置写入；
2. 写入缺失键（动态）报 `X06-RUNTIME-015`；位置越界报 017（按 F-2）；
3. 若本批暂不实现写入，则应在前端给出稳定“未支持”诊断，不能落到 `X09-BYTECODE-001`；
4. 补 VM/原生/归档写入用例。

### F-4 **`run.sh` 的全量抖动仍会判红**

隔离重试已可达，且本次 `Ctrl+D` 首次失败、重试通过；但 `bun_status` 非零仍直接退出 1。
要求：对两条已知抖动，若全量失败但隔离重试通过，则视为通过；或把它们移出全量串行
门禁并单独判定。保留“模式匹配 0 条测试时 Bun 退出 1”的防假绿。

### F-5 **平台复现仍待四平台取证**

`reproduce.ps1` 已把 MSVC `link.exe` 放到 PATH 首位；20I/20J 未推送，最近一次
`39227168623` 的 `windows-amd64` 仍红。要求推送后重跑四平台 + ordered summary，
回填运行号。

### F-6 **推送与原生/移除证据**

- 推送 20I/20J，核销维护回归与工作区门禁；
- 原生 `array_push`/动态字典键/表达式下标目前只有降低用例；补受控原生执行回归；
- `array_push` 的移除验证未覆盖；
- `.xiaoc` 文本切分与动态键已由 `d17c_archive` 覆盖，保留。

### F-7 **文档**

- 20J §1.2/F-1/F-2 的“已修”按残余情况收窄；
- 20J F-4 改为“重试可达，全量抖动仍红”；
- 20I §二 F-1 的 `ahead 1` 与 d17c 12 更新；
- 20H/README 运行号与状态随推送更新。

## 三、四道裁定

| # | 裁定 | 结果 |
| --- | --- | --- |
| 1 | 动态写新增键 | **只更新已有键**；缺失报 015；不 upsert |
| 2 | API 形态 | **赋值 + `stdlib/dict.xiao` 自由函数**；不开方法分派 |
| 3 | 字典注解 | **`dict<str, T>`**：函数参数/返回；键固定 str |
| 4 | 删除/枚举 | 删除缺失报 **015**；keys 返回 `[str]`，values 返回共同值类型数组/dynamic |

## 四、设计

### 4.1 写入语义

```xiao
values = {name = 1}
values["name"] = 2       # 静态键更新（已有能力）
key = "name"
values[key] = 2          # 动态键更新；缺失 015
column = <first = 1>
column[key] = 2          # 字典列动态键更新
i = 0
column[i] = 2            # 字典列动态位置更新；越界 017
```

**冻结**：

- 只更新既有键/位置；动态缺失键 → `X06-RUNTIME-015`；动态位置越界 → 017；
- 静态缺失键写入仍为 `X03-TYPE-005`；静态越界仍为 `X03-TYPE-004`；
- 新键写入不允许，不改变字典的静态键集；
- 值类型必须符合字典值的共同类型；异构字典按 dynamic 处理；
- 求值顺序：接收者 → 键/位置表达式 → 值；三者各求值一次；
- 写入对别名可见；字典是共享可变对象；不引入新哈希/排序语义。

### 4.2 `dict<str, T>` 注解

```xiao
def get(dict<str, int> values, str key) -> int
def keys(dict<str, int> values) -> [str]
```

**冻结**：

- 只支持函数参数/返回位置；键类型固定 `str`；值类型可为标量、资源、`none`、
  `[T]`、`set<T>` 或类型参数；
- 解析：`dict` 标识符后接 `<str, T>`；与 `set<T>`/`[T]` 的解析风格一致；
- 类型：需要一个**开放键集**的字典类型表示（只有值类型，不枚举键）；现有
  `DictType` 的固定条目不能直接表达，需新增开放变体/标记；
- 跨模块签名提取、IR 类型、字节码编解码都要能表示开放字典；
- 局部变量声明 `dict<str,T> name = ...` 不在本批；
- 固定形状字面量与开放字典参数的统一规则：值类型兼容即可；传入固定形状字典
  后，若经可变函数调用，调用方的静态键集按 F-6 的保守失效处理（或写明限制）。

### 4.3 `stdlib/dict.xiao`

```xiao
def get(dict<str, T> values, str key) -> T
def set(dict<str, T> values, str key, T value) -> none
def remove(dict<str, T> values, str key) -> none
def contains(dict<str, T> values, str key) -> bool
def keys(dict<str, T> values) -> [str]
def values(dict<str, T> values) -> [T]
```

**冻结**：

| 接口 | 语义 |
| --- | --- |
| `get` | 缺失键 → `X06-RUNTIME-015`；与下标读取一致 |
| `set` | 只更新已有键；缺失 → 015；返回 none |
| `remove` | 删除已有键；缺失 → 015；返回 none |
| `contains` | 键存在返回 true；不报错 |
| `keys` | 返回键数组；顺序沿用字典现有迭代顺序，不额外承诺 |
| `values` | 返回值数组，元素共同类型/dynamic；顺序与 keys 对应；不额外承诺 |

- 只使用赋值/既有读取与 `array_push` 构造结果数组，不新增 intrinsic；
- 错误码写进 `stdlib/dict.md` 与 UseDocs；测试匹配 code；
- 模块身份与 `.xar` 的 `stdlib_modules` 按 20E 规则接入。

### 4.4 IR/Runtime

- 动态键/位置写入需要新的写入动作/ABI（名称由实现定），参数为接收者、键/位置、值；
- 消费 20J 已登记的 `selector_bounds` 检查，不能残留到 `X09-BYTECODE-001`；
- 运行时按键写入使用字典句柄的可变入口；缺失键 → 015；位置越界 → 017；
- `keys`/`values` 枚举返回新数组（`ArrayHandle`），按现有迭代顺序；
- VM 与 LLVM 动态路径共用同一 ABI；`.xiaoc` 若新增 opcode，沿用 20G/20I 的扩展实践；
- 不新增能力位、不改 Runtime ABI 主版本、不把字典复制进归档对象。

### 4.5 失效与静态检查

- 动态键写入不改变静态键集；动态删除（`dict_remove`/赋值删除？本批只有 remove 函数）
  会使静态键集失效——`dict_remove` 对固定形状字面量的调用按保守规则把键集视为开放，
  或限制为开放字典参数；
- 数组 F-1/F-2 的失效逻辑与本批写入共用同一“可变操作”登记，避免再写一套；
- 静态可判的缺失键/越界仍在类型层拒绝；动态路径在运行时检查。

### 4.6 测试

- 静态/动态键写入、字典列键/位置写入、缺失/越界、别名、求值顺序；
- `dict<str,T>` 参数/返回、跨模块、原生/归档；
- get/set/remove/contains/keys/values 的正负例、缺失 015、顺序/对应；
- 20I F-1/F-2 的成员接收者与只读调用用例；
- 原生执行与移除验证；
- `.xiaoc`/`.xar` 写入与枚举。

## 五、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 推送 20I/20J；核销 CI；修 F-4/F-5/F-7 | 运行号/门禁 |
| 1 | F-1/F-2：数组失效范围与越界身份 | 诊断/VM |
| 2 | F-3：动态写入 lowering，消费 selector_bounds | VM |
| 3 | `dict<str,T>` AST/解析/开放字典类型/签名提取 | 类型/跨模块 |
| 4 | 动态键/位置写入：类型/运行时/别名/求值顺序 | VM |
| 5 | `stdlib/dict.xiao` 六接口与错误码 | VM |
| 6 | IR/ABI/VM/LLVM/`.xiaoc`/`.xar`/原生 | 四路 |
| 7 | 原生执行/移除验证/全量门禁/文档 | 见 §六 |

## 五点一、已实施落点

- F-1/F-2：`array_push` 会擦除直接名称、名称别名、表字段数组的固定长度；已知只读函数调用保留静态长度诊断。动态失效后的字面量越界统一沿运行时 017 路径处理。
- F-3：新增 `IndexSetDynamic`，VM、`.xiaoc` 编解码、Runtime ABI 和 LLVM 动态降低均消费动态写入；字典缺失键为 015，字典列/数组位置越界为 017。
- `dict<str, T>`：函数参数与返回注解已解析为开放键集字典类型，值类型支持标量、资源、`none`、数组、集合和类型参数；固定形状字典按共同值类型兼容。
- `stdlib/dict.xiao`：已接通 `get`、`set`、`remove`、`contains`、`keys`、`values`；键和值枚举返回新数组，删除缺失键保持 015。
- F-4：`run.sh` 保留两条 REPL 隔离重试；全量 Bun 仅在隔离重试失败时判红，已知全量抖动而隔离成功时输出明确通过说明。

**最小命令**：

```bash
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20j_dynamic_dict_keys
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20k_dict_write
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20i_indexing_and_text
cargo test --manifest-path core/rust/Cargo.toml --workspace --no-fail-fast
bun test
bash tools/gates/run.sh
```

## 六、验收

1. **F-1/F-2 关闭**：成员接收者 `array_push` 后可读新位置；只读调用后的静态/运行时越界
   与同语义使用同一身份；相关正负例齐全；
2. **F-3 关闭**：动态键/位置写入不再出现 `X09-BYTECODE-001`；缺失 015、越界 017；
3. **F-4/F-5 关闭**：门禁抖动处理与四平台运行号落地；
4. **F-6 关闭**：20I/20J 推送 CI 全绿；原生执行、`.xiaoc`/`.xar`、移除验证齐全；
5. **写入**：静态/动态键、字典列键/位置、别名、求值顺序、缺失/越界、值类型错误；
6. **注解**：`dict<str,T>` 参数/返回可解析、可跨模块、可在原生/归档使用；
   新键写入仍拒绝；
7. **接口**：get/set/remove/contains/keys/values 的结果、错误码、顺序/对应、共同类型；
8. **四路**：VM、`.xiaoc`、`.xar`、原生逐字一致；
9. **不回归**：workspace、Bun、`run.sh` 全绿；20F–20J/20E/18B 用例不受影响；
10. **文档**：00/02/03/04、20 路线稿、README、registry、UseDocs 同步；
    新能力在四路证据齐全后才翻 `verified`。

## 七、不负责

- **不做**新增键/upsert、字典方法式 API、嵌套/局部字典注解；
- **不做**字典字面量的开放键语法、字典列增删列；
- **不做**新的能力位、Runtime ABI 主版本或哈希/排序变更；
- **不把**写入实现成第二套选择器语义；
- **不在**本批扩集合/数组/文本接口。

## 八、落点

```text
docs/DevDocs/20k-dict-write-and-annotations.md   本文
docs/DevDocs/README.md                            20K 索引行；20J 状态
docs/DevDocs/20-builtins-and-standard-library.md  §七 阶段表补 20K
docs/DevDocs/03-collections.md                    字典写入/注解契约
docs/DevDocs/00-decisions.md                      四道裁定
docs/DevDocs/20j-dynamic-dict-keys-and-20i-closeout.md F-1–F-7 对账
core/rust/crates/xiao-syntax/src/ast.rs           FunctionTypeAnnotation::Dict
core/rust/crates/xiao-syntax/src/parser*           dict<str,T> 解析
core/rust/crates/xiao-types/src                    开放字典类型/写入检查
core/rust/crates/xiao-types/src/diagnostics.rs     稳定诊断
core/rust/crates/xiao-ir/src/model.rs              动态写入动作
core/rust/crates/xiao-ir/src/lower.rs              写入 lowering
core/rust/crates/xiao-bytecode/src                 编解码/验证/liveness
core/rust/crates/xiao-runtime/src/containers/dict.rs 可变写入/枚举
core/rust/crates/xiao-runtime/src/abi_dynamic.rs   写入/枚举 ABI
core/rust/crates/xiao-vm/src/semantics              VM 写入
core/rust/crates/xiao-codegen-llvm/src/dynamic      LLVM 写入
stdlib/dict.xiao / stdlib/dict.md / docs/UseDocs/library/dict.md
core/rust/crates/xiao-driver/tests/20k_dict_write.rs
core/rust/crates/xiao-driver/tests/20i_indexing_and_text.rs F-1/F-2 回归
```

## 相关页面

- [20J 字典动态键与 20I 收口](20j-dynamic-dict-keys-and-20i-closeout.md)
- [20I 非字面量下标、数组注解与文本切分](20i-nonliteral-indexing-and-splitting.md)
- [03. 容器、集合与索引路径](03-collections.md)
- [20F 通用容器参数与官方 `set` 模块](20f-container-parameters-and-collections.md)
- [20AB intrinsic 契约与最小入口](20ab-intrinsics-contract-and-minimal-entry.md)
- [18B `build -xar` 与验收对账](18b-build-xar-and-closeout.md) §八
