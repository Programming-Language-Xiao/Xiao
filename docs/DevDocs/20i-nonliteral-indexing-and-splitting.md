# 20I. 非字面量下标、数组注解与文本切分

> **20H 之后的语言批次**。20H 把集合方法接到了字面量集合上，但审核实测还留着
> 一处误诊断和一份未推送的最终修复；同时，20C 一直登记着的「子串查找/切分」
> 仍被三个前置挡着：**表达式下标写不出来**、**数组类型注解不存在**、
> **数组无法动态追加**。本批先收 20H，再把这三个前置和文本接口一起接通。
>
> **一句话概括本批**：**让 `s[i + 1]`、`[str]` 与 `split` 第一次能在 Xiao 里写出来。**
>
> 状态：**已实施，待四平台 CI 取证（2026-10-20）**。五道裁定：所有有序容器支持表达式下标、
> 字符串用码点且非负、数组注解用 `[T]`、`find=-1`/`split` 保尾空/`slice` 半开、
> 新增 `array_push`。实现已落在语法、类型、IR、字节码、VM、Runtime、LLVM、标准库和回归夹具。

## 一、Agent 交接上下文

### 1.1 接手前提

按顺序读：

1. [20H 集合方法式 API 与 20G 收口](20h-set-methods-and-20g-closeout.md)
   —— **直接前置**；其 F-1–F-5 是本批的返工输入；
2. [03. 容器、集合与索引路径](03-collections.md) §可索引能力边界、§统一选择器语法、
   §嵌套路径与结果形状——下标的既定边界；
3. [20D-规划](20d-platform-resources-stage-plan.md) §2.2——「非字面量下标」与
   「子串查找/切分」的登记与归属；
4. [20C-第二批](20c-second-batch-probe.md) §3.1 第 4/5 行、§4.3——
   `fifth-substring-split` 与 `fifth-split-array` 的实测挡点；
5. [20F](20f-container-parameters-and-collections.md)——类型参数、`[T]` 注解要复用的
   签名/实例化机制；
6. [20AB intrinsic 契约](20ab-intrinsics-contract-and-minimal-entry.md) 与
   [20G](20g-set-mutation-and-20f-closeout.md) §4.4——`array_push` 的契约与移除验证先例；
7. [18B 收口](18b-build-xar-and-closeout.md) §八——推送 CI 的取证口径。

### 1.2 现状盘点（2026-10-19 实跑）

| 环节 | 现状 | 结论 |
| --- | --- | --- |
| 20H 定向 | `20h_set_methods` 10、`20g_set_mutation` 7、`d17c_archive` 12、`18b_build_archive` 13 通过 | 20H 主干成立 |
| 20I 定向 | `20i_indexing_and_text` 11 通过；`r2a_containers` 17 通过 | VM、数组追加、表达式下标、文本接口和边界诊断已接通 |
| 全工作区 | Rust workspace 全量测试无失败；Bun 单独跑 298 通过 / 5 跳过 / 0 失败 | 本地主体成立 |
| 20H 推送 CI | `66856a2` 已进入当前 `origin/main`；历史运行号覆盖前一版本，新的四平台取证仍待本批维护回归 | F-1 的本地提交状态已核销，CI 证据待刷新 |
| `s[1]` 字面量 | `"小雪"[1]` 输出 `雪` | **码点语义** |
| `s[i]` 变量 | 有序容器按变量表达式解析；字典表/字典列动态键由 20J 按静态键优先分派 | **表达式下标已接通** |
| `s[i + 1]` / `a[f()]` | 选择器语法不接受任意表达式 | 解析前置 |
| 字符串越界 | `X06-RUNTIME-017`（运行时） | 动态下标沿用 |
| 数组越界 | 字面量越界 `X03-TYPE-004`（静态） | 静态/动态边界要分开 |
| 数组注解 | `[str]`、`[T]` 参数/返回可解析并参与类型推断 | **数组注解已接通** |
| 数组追加 | `array_push` 通过 VM/LLVM/Runtime 契约接线，别名顺序回归通过 | **动态追加已接通** |
| 子串/切分 | `find`、`slice`、`split` 由纯 Xiao 实现，`split` 使用 `array_push` | **文本接口已接通** |
| 字典表动态键 | 20J 已接通；静态键优先，无同名键时按 `str` 动态键读取 | 见 [20J](20j-dynamic-dict-keys-and-20i-closeout.md) |

### 1.3 本批边界

| 项 | 本批 | 说明 |
| --- | --- | --- |
| 20H F-1–F-5 | **做** | 推送取证、默认值误诊断、平台复现 PATH、REPL 抖动、文档 |
| 表达式下标 | **做** | `str`、数组、元组、字典列；单个精确项 |
| 数组注解 `[T]` | **做** | 函数参数与返回位置 |
| `array_push` | **做** | 动态追加，供纯 Xiao `split` 使用 |
| `text.find` / `slice` / `split` | **做** | 纯 Xiao，除 `array_push` 外不新增 intrinsic |
| 字典表动态键 | **不做** | 保持现有键路径语义 |
| 多选/范围/步长表达式端点 | **不做** | 仍要求静态端点 |
| 负索引、字节索引 | **不做** | 见 §三 |
| `frozenset`、泛型表、约束语法 | **不做** | 保持登记 |

## 二、20H 审核返工项（先做）

### F-1 **最终修复未推送、无 CI（证据阻断）**

`main...origin/main [ahead 1]`：`66856a2`（集合方法链接收口）仍在本地；
20H 文档引用的 `39239884610`/`39239884581` 属于 `9e42913`/`09c1a2b`，
不覆盖最终修复。要求：

1. 推送 `66856a2`，核销维护回归与工作区门禁；
2. 在运行号落地前，20H/20G/20F 的“已验收通过”只对上一提交成立；
3. 回填运行号后，再确认 registry/UseDocs 的 `verified` 覆盖最终修复。

### F-2 **参数默认值里的集合方法被误报为方法值**

**证据**：

```xiao
def f(int x = {1}.size()) -> int
    return x

print(f())
→ X03-TYPE-025: 集合方法 size 只能直接调用，不能作为值使用
```

这是直接调用，不是方法值。扫描只遍历函数体，没有遍历
`FunctionParameter.default`；当方法只出现在默认值时，`stdlib.set` 签名没有注入，
调用落到 Set 成员分支，被当成方法值。

**要求**：

1. `expression_uses_set_method`/`statements_use_set_method` 的语法扫描与类型后扫描
   都遍历函数与表方法的参数默认值；
2. 补默认值方法正例（`def f(int x = {1}.size())`）与表方法默认值正例；
3. 诊断码保持不变：真正的 `m = values.add` 仍报 025。

### F-3 **平台复现 Windows 作业红（PATH/目标 Shell）**

运行 `39227168623` 的 `windows-amd64` 作业红：

```text
A0-PARSER-001 Rust AST 适配器启动失败
/usr/bin/link: extra operand ...
```

`C:\Program Files\Git\usr\bin\link.exe` 排在 MSVC `link.exe` 前面，适配器把 Git 的
`link` 当成链接器。要求：

1. 平台复现工作流在构建适配器时使用 cmd/pwsh + vcvars，或把 MSVC 工具链放到 PATH 前；
2. 修完后重跑四平台，确认 Windows 作业与 ordered summary 都绿；
3. 在 20H/20I 文档登记该红与修复运行号，不能只写“其它平台绿”。

### F-4 **`run.sh` 的 REPL 用例抖动**

两次单独跑 `run.sh` 分别红在：

- `确认后调用完整源码并恢复覆盖模式、剪贴板和光标`（剪贴板期望内容，收到 `[]`）；
- `!inLF! 后空缓冲 Ctrl+D 结束整个会话`（5 秒超时）。

单独跑 `bun test` 为 298 通过 / 5 跳过 / 0 失败，CI 工作区门禁绿。判定为**时序抖动**，
不是 20H 回归。要求：隔离/重试这两条 REPL 用例，或把它们移出 `run.sh` 的串行门禁，
并在文档写明“本地门禁通过”的判定必须包含单独 Bun 复跑。

### F-5 **文档与状态对账**

- 20H/README 的“已验收”在 F-1 推送后补运行号；
- 20H §1.2 的 `20H 定向 9`、`d17c 11`、`18b 13` 与实际一致，保留；
- 20H 的 `.xiaoc`/方法/原生证据保留；
- F-2 的默认值扫描缺口登记到 20H 或本批；
- F-3/F-4 的 CI/门禁状态写入下一批的验收。

## 三、五道裁定

| # | 裁定 | 结果 |
| --- | --- | --- |
| 1 | 表达式下标范围 | **所有有序容器**：`str`、数组、元组、字典列；字典表动态键不做 |
| 2 | 字符串下标 | **码点 + 非负 + 越界报错**；与现有字面量一致 |
| 3 | 数组注解 | **`[T]`**：参数与返回位置；`[str]`、`[T]` |
| 4 | 文本接口 | `find` 未找到返回 **-1**；`split` **保尾空**；`slice(start,end)` **左闭右开**、越界报错 |
| 5 | 数组构造 | 新增 **`array_push<T>([T] values, T value) -> none`**；`split` 用纯 Xiao 实现 |

## 四、设计

### 4.1 表达式下标

**语法**：单个精确项里的表达式，例如 `s[i]`、`a[i + 1]`、`column[f()]`。

**AST/解析**：

- 选择器项需要能保存一个**表达式**，而不只是 `Key`/`Integer` 路径段；
  新增 `SelectorItem::ExactExpression { expression, span }`（名称可调整）；
- 裸名称 `x[i]` 需要同时保留“字典键 `i`”与“变量 `i`”两种解释：
  - 有序容器（`str`/数组/元组/字典列）→ 变量表达式；
  - 字典表 → 现有键语义；
  - 由类型检查器按接收者种类选择，解析阶段不武断；
- `s[i + 1]`、`a[f()]` 这类非路径表达式直接解析为表达式项；
- 现有字面量 `[0]`、多选、范围、步长、随机选择全部保持原语法；
- **只支持单个精确表达式项**；范围/多选/步长的端点仍要求静态，不在本批。

**类型检查**：

- 有序容器 + 单表达式项：
  - 索引表达式必须是整数族（`int`/`sint`/`lint`；`bool` 不放宽）；
  - `str` → `str`（单码点）；数组 → 元素类型（异构数组取共同类型，否则 `dynamic`）；
    元组 → 元素共同类型/`dynamic`；字典列 → 值共同类型/`dynamic`；
  - 非负：静态已知负字面量直接拒绝；动态值登记运行时检查；
  - 越界：静态可判的仍走 `X03-TYPE-004`；动态登记运行时检查，运行时沿用
    `X06-RUNTIME-017`（选择器索引超出运行时长度）；
- 字典表 + 单表达式项：保持现有键路径语义；动态键不做，非名称表达式报现有键错误；
- 表达式只求值一次；接收者只求值一次。

**IR/Runtime**：

- 复用现有 `IndexGetDynamic` TacOp/ABI；增加索引表达式的降低与运行时边界检查；
- 字符串按码点读取，与 `for c in str` 和 `"小雪"[1]` 一致；
- 数组/元组/字典列按运行时长度检查；越界错误身份统一到既有运行时选择器错误；
- 生成期擦除与 20F 一致，不新增 `.xiaoc` 格式或 Runtime ABI。

### 4.2 数组类型注解 `[T]`

**语法**：

```xiao
def split(str text, str separator) -> [str]
def first([int] values) -> int
def head<T>([T] values) -> T
```

**冻结**：

- 参数与返回位置支持 `[T]`；本批只支持**一层**数组，元素可为标量、资源、`none`
  或类型参数；嵌套数组/数组套集合留后续；
- 解析：函数头的 `[` 在类型位置开始数组注解；与数组字面量在表达式位置不冲突；
- 类型：`Type::Array(ArrayType::homogeneous(element))`；数组不可哈希，不能作为集合元素；
- 驱动器标准库签名提取、导入签名实例化、IR lowering、字节码编码都要能表示 `[T]`；
- 局部变量声明 `[T] name = ...` 不在本批；`split` 的局部结果由 `[]` 与
  `array_push` 推断；
- 数组注解不改变现有 `int list = [...]` 的容器约束消歧，两者必须都有测试。

### 4.3 `array_push`

**契约**（沿用 20G 的 `set_add` 形状）：

```json
{"id":31,"name":"array_push","kind":"array_push","arity":"two",
 "parameter_type":"array_element","return_type":"none",
 "vm_binding":"array_push","abi_binding":"array_push",
 "effect":"mutates_container","capability":"none",
 "error":"runtime.invalid_value","status":"active"}
```

**冻结**：

- 第一个参数必须是数组，第二个是元素；返回 `none`；
- 原地追加，保持顺序；对别名可见；数组是可变共享对象；
- 不检查可哈希（数组元素不要求可哈希）；
- `arity` 复用 20G 的 `Two`；`parameter_type` 新增 `array_element`；
- VM 绑定调用 `ArrayHandle` 的可变入口；LLVM 动态路径通过 Runtime ABI 接线；
- 移除验证：删表项、VM 绑定或 ABI 包装任一环节，专用用例必须失败。

### 4.4 文本接口

在 `stdlib/text.xiao` 增加，全部纯 Xiao（`array_push` 除外）：

```xiao
def find(str text, str needle) -> int
def slice(str text, int start, int end) -> str
def split(str text, str separator) -> [str]
```

**逐条冻结**：

| 接口 | 语义 |
| --- | --- |
| `find` | 返回首次出现的码点位置；未找到返回 `-1`；空 needle 返回 `0`；不报错 |
| `slice` | 返回 `[start, end)` 的码点子串；`end` 可取长度；越界抛 `Error(code = "TEXT_SLICE_OUT_OF_RANGE")`；`start > end` 返回空串；非负 |
| `split` | 按分隔符切分；空分隔符抛 `Error(code = "TEXT_SPLIT_EMPTY_SEPARATOR")`；**保留尾部空段**，首/中空段同样保留；未找到分隔符返回 `[text]`；空文本返回 `[""]`；结果用 `array_push` 构造 |

- 三者都用非字面量下标/循环实现，不新增文本 intrinsic；
- 错误码写进 `stdlib/text.md` 与 UseDocs；测试匹配 code，不匹配文案；
- `find`/`slice` 可在无 `array_push` 的路径上先用；`split` 依赖 4.3。

### 4.5 与 20F/20H 的边界

- 表达式下标与 `[T]` 注解复用 20F 的类型参数/实例化机制，不新开一套；
- `array_push` 复用 20G 的 intrinsic 形状与 20H 的方法链接设施；
- 集合方法不受影响；`split` 的 `[str]` 与集合的 `set<T>` 不互相转换；
- 字典表动态键、集合/字典的可变方法、非字面量范围端点仍不在本批。

## 五、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 推送 `66856a2` 并核销 CI；修 F-3/F-4/F-5 | 运行号/门禁 |
| 1 | F-2：参数默认值扫描；正负例 | 前端诊断 |
| 2 | `[T]` AST/解析/类型/签名提取；`[str]` 正例与错误 | 语法/类型 |
| 3 | 表达式下标 AST/解析/类型；有序容器 + 字典表键语义隔离 | 解析/类型 |
| 4 | IR/Runtime/VM/LLVM：动态下标与边界检查；字符串码点；原生/归档 | 双路径 |
| 5 | `array_push` 契约/VM/LLVM/Runtime；别名与移除验证 | 双路径 |
| 6 | `text.find`/`slice`/`split` 纯 Xiao；错误码与边界 | VM/原生 |
| 7 | 全量门禁、UseDocs/registry、文档对账 | 见 §六 |

**最小命令**：

```bash
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20h_set_methods
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20i_indexing_and_text
cargo test --manifest-path core/rust/Cargo.toml -p xiao-runtime --test r2a_containers
cargo test --manifest-path core/rust/Cargo.toml -p xiao-intrinsics
cargo test --manifest-path core/rust/Cargo.toml --workspace --no-fail-fast
bun test
bash tools/gates/run.sh
```

## 六、验收

1. **F-1 关闭**：`66856a2` 推送后维护回归与工作区门禁全绿；文档回填运行号；
2. **F-2 关闭**：参数默认值中的集合方法是直接调用，不再报 025；真正的方法值仍报 025；
3. **F-3 关闭**：平台复现 Windows 作业不再因 Git `link.exe` 失败；四平台与 summary 绿；
4. **F-4 关闭**：`run.sh` 的 REPL 用例稳定，或已移出串行门禁并写明单独 Bun 复跑口径；
5. **表达式下标**：`str`/数组/元组/字典列各有一组变量/表达式下标正例；
   非整数、负字面量、越界分别有稳定诊断；字符串按码点；
6. **字典表隔离**：`d[key]` 仍是键语义；`d[i + 1]` 不被当成动态键放行；
7. **`[T]` 注解**：`[str]`、`[int]`、`[T]` 参数/返回可解析、可跨模块、可在原生/归档使用；
   嵌套数组按边界拒绝；
8. **`array_push`**：追加、顺序、别名、不可哈希元素（允许）、移除验证；
9. **文本接口**：`find` 的 -1/0/重复；`slice` 半开/端点/越界/start>end；
   `split` 空分隔符、首中尾空段、未找到、空文本；VM 与原生逐字一致；
10. **路径**：源码、`.xiaoc`、`.xar`、原生四路；标准库签名与静态链接/模块调用两条；
11. **不回归**：workspace、Bun、`run.sh` 全绿；20F/20G/20H/20E/18B 既有用例不受影响；
12. **文档**：`03`、`04`、`02`、`00-decisions`、20 路线稿、README、registry、UseDocs 同步；
    registry/UseDocs 在四路证据齐全后才翻 `verified`。

## 七、不负责

- **不做**字典表动态键、集合/字典的方法式 API 扩展；
- **不做**负索引、字节索引、范围/多选/步长的表达式端点；
- **不做**嵌套数组注解、数组套集合、泛型表或约束语法；
- **不做**新的文本 intrinsic（`split` 用 `array_push` + 纯 Xiao）；
- **不做** `.xiaoc` 格式或 Runtime ABI 主版本变更；
- **不把**表达式下标实现成第二套选择器语义。

## 八、落点

```text
docs/DevDocs/20i-nonliteral-indexing-and-splitting.md   本文
docs/DevDocs/README.md                                 20I 索引行；20H 状态
docs/DevDocs/20-builtins-and-standard-library.md       §七 阶段表补 20I
docs/DevDocs/03-collections.md                         表达式下标/数组注解契约
docs/DevDocs/04-functions-and-control.md               `[T]` 函数注解
docs/DevDocs/00-decisions.md                           五道裁定
docs/DevDocs/20h-set-methods-and-20g-closeout.md       F-1–F-5 返工对账
core/rust/crates/xiao-syntax/src/selectors.rs          表达式选择项
core/rust/crates/xiao-syntax/src/parser.rs             选择器/数组注解解析
core/rust/crates/xiao-syntax/src/ast.rs                FunctionTypeAnnotation::Array
core/rust/crates/xiao-types/src/checker                  下标与数组注解
core/rust/crates/xiao-types/src/diagnostics.rs           稳定诊断
core/rust/crates/xiao-i18n/src/lib.rs                    新 message_id
core/rust/crates/xiao-ir/src/lower.rs                    表达式下标/数组注解 lowering
core/rust/crates/xiao-bytecode/src/lower                  下标 lowering
core/rust/crates/xiao-runtime/src/containers/array.rs    push/边界
core/rust/crates/xiao-runtime/src/abi_dynamic.rs         动态下标/数组 ABI
core/rust/crates/xiao-intrinsics/intrinsics.json         id 31、array_element
core/rust/crates/xiao-vm/src/semantics                     array_push 绑定
core/rust/crates/xiao-codegen-llvm/src/dynamic            array_push 与下标
core/rust/crates/xiao-driver/src/protocol/run.rs         F-1 .xiaoc loader（已由 20H 接入）
stdlib/text.xiao / stdlib/text.md / docs/UseDocs/library/text.md  find/slice/split
core/rust/crates/xiao-driver/tests/20i_indexing_and_text.rs
tests/fixtures/20i/
```

## 相关页面

## 九、实施记录（2026-10-20）

- **F-2**：前端静态链接扫描同时遍历普通函数和表方法的参数默认值；默认值中的集合方法按直接调用处理，方法值仍保持 `X03-TYPE-025`。
- **数组注解**：函数参数/返回支持一层 `[T]`，支持标量、资源、`none` 和类型参数；嵌套数组及数组套集合在解析阶段拒绝。
- **表达式下标**：`str`、数组、元组和字典列支持 `s[i]`、`s[i + 1]`、`a[f()]`；字典表动态键由 20J 接通并保留静态键优先。表达式下标的静态负数/可判越界在类型层拒绝，动态正负越界统一为 `X06-RUNTIME-017`，字符串按 Unicode 码点读取。
- **`array_push`**：新增稳定 intrinsic ID 31，VM、Runtime ABI 和 LLVM 动态路径共享数组句柄追加语义；`split` 通过该入口保留首/中/尾空段。
- **文本接口**：`stdlib.text` 新增 `find`、`slice`、`split`；错误码固定为 `TEXT_SLICE_OUT_OF_RANGE` 与 `TEXT_SPLIT_EMPTY_SEPARATOR`。
- **本地验证**：`20i_indexing_and_text` 11 项、`20h_set_methods` 10 项、`d17c_archive` 12 项、`20g_set_mutation` 7 项、`20f_container_parameters` 10 项通过；`xiao-types`、`xiao-runtime`、`xiao-intrinsics` 定向测试通过。受控原生四平台与 workspace/Bun 全量证据待提交后的维护回归刷新。
- **F-3/F-4**：Windows 复现脚本已有 `vcvars64.bat` 环境导入；workspace gates 增加两条 REPL 抖动用例的隔离复跑，单独 Bun 复跑仍作为判定依据。

- [20H 集合方法式 API 与 20G 收口](20h-set-methods-and-20g-closeout.md)
- [20J 字典动态键与 20I 收口](20j-dynamic-dict-keys-and-20i-closeout.md)
- [20F 通用容器参数与官方 `set` 模块](20f-container-parameters-and-collections.md)
- [20G 集合增删与 20F 审核收口](20g-set-mutation-and-20f-closeout.md)
- [03. 容器、集合与索引路径](03-collections.md)
- [20D 平台资源阶段规划](20d-platform-resources-stage-plan.md) §2.2
- [20C-第二批](20c-second-batch-probe.md) §3.1
- [20AB intrinsic 契约与最小入口](20ab-intrinsics-contract-and-minimal-entry.md)
