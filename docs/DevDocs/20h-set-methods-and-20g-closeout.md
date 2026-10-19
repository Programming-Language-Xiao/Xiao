# 20H. 集合方法式 API 与 20G 收口

> **20G 之后的集合批次**。20G 把 `add`/`remove`、四个只读包装和迭代快照接到了
> VM/原生/归档，但审核实测还留着一个**阻断验收**的缺口：`.xiaoc` 入口没有标准库
> 模块加载器，`stdlib.set` 与既有的 `stdlib.text` 都会在执行前报
> `X06-RUNTIME-012`。本批先把 20G 收口，再补集合的方法式 API。
>
> **一句话概括本批**：**先让 `.xiaoc` 真的能加载标准库，再让 `values.add(x)` 与
> `add(values, x)` 等价。**
>
> 状态：**已实施并验收通过（2026-10-19）**。三道裁定均已落地：
> 方法面与自由函数一致、动态接收者静态拒绝、方法值只支持直接调用。B-1 的 `.xiaoc`
> 标准库 loader、E-1/E-2 原生夹具与方法式 API 已接线；三平台推送 CI 和 workspace 门禁
> 已全绿，registry/UseDocs 已同步为 `verified`。

## 一、Agent 交接上下文

### 1.1 接手前提

按顺序读：

1. [20G 集合增删与 20F 审核收口](20g-set-mutation-and-20f-closeout.md)
   —— **直接前置**；§四 的接口、§六 的验收仍然有效，本批覆盖其未关闭项；
2. [20F 通用容器参数与官方 `set` 模块](20f-container-parameters-and-collections.md)
   —— 类型参数、调用点实例化与生成期擦除；
3. [03. 容器、集合与索引路径](03-collections.md) §集合、§集合运算、§可哈希规则——
   集合语义与顺序契约；
4. [20AB intrinsic 契约与最小入口](20ab-intrinsics-contract-and-minimal-entry.md)
   —— `set_add`/`set_remove` 的契约、移除验证与双路径接线；
5. [20E 归档接线](20e-archive-wiring.md) 与 [17C](17c-entry-and-runner.md)
   —— `.xiaoc`/`.xar` 的启动校验与标准库模块边界；
6. [04. 函数与控制流](04-functions-and-control.md) §表方法相关段落——
   现有表方法调用的形状；集合方法不能复制一套独立的调用栈；
7. [18B 收口](18b-build-xar-and-closeout.md) §八——推送 CI 的取证口径。

### 1.2 现状盘点（2026-10-19 实跑）

| 环节 | 现状 | 结论 |
| --- | --- | --- |
| 20G 定向 | `20g_set_mutation` **7** 通过；`20f_container_parameters` 10 通过；`p2_type_parameters` 4 通过 | 返工项 F-2/F-3/F-6 与 E-2 已有效 |
| 20H 定向 | `20h_set_methods` **10** 通过；`d17c_archive` **12** 通过；`18b_build_archive` **13** 通过 | 八个方法、默认值、单次求值、冲突隔离、`.xiaoc`/`.xar` 的 `set`/`text` loader 已覆盖 |
| 全工作区 | 本地 workspace、Bun、`run.sh` 均已复验；workspace gates `39239884581` 成功 | 本地与推送门禁成立 |
| 推送 CI | `ebf34ed` 维护回归 **39202984158 六作业全绿**；`6919d81` 工作区门禁 `39201428785` 成功 | 18B/20F/20G 的推送证据已落地 |
| 原生别名 | 本地 `set`/`text` 别名构建+运行通过；CI 三平台 `first_modules_match_native_results` 通过 | F-1 已关闭 |
| 原生增删/快照 | `20c_first_modules::first_modules_match_native_results` 已加入 set 增删、包装、快照、非局部返回和方法式 API；`set_remove_missing_native_matches_vm` 钉住未捕获错误码 | maintenance `39239884610`：Windows `118017847526`、Linux `118017847726`、macOS `118017847631` 全部通过 |
| `.xar` | 构建并实际运行通过；缺 `set.xiao` 仍报 `X17-XAR-008` | 可用 |
| **`.xiaoc`** | `d17c_archive::direct_xiaoc_run_loads_standard_library_modules` 对 `stdlib.set`/`stdlib.text` 逐字比较输出；loader 复用项目根、包注册表和官方标准库根 | B-1 已关闭；workspace gates `39239884581` 与 maintenance 默认作业通过 |
| 方法语法 | `values.add(x)` 已在表方法路径前由集合分派接管，静态集合复用 `stdlib.set` 签名 | 本批已完成 |
| 表方法 | 表方法由 `check_table_method_call` 处理；集合没有对应分派 | 可复用形状 |
| 方法值 | 表方法只支持直接调用；集合方法同样按直接调用设计 | 见 §三 |

### 1.3 本批边界

| 项 | 本批 | 说明 |
| --- | --- | --- |
| `.xiaoc` 标准库加载 | **做** | B-1；源码/归档已有的 loader 口径复用 |
| 原生增删/快照运行回归 | **做** | E-1；受控工具链忽略用例，CI 原生作业执行 |
| 包装顺序/边界断言 | **做** | E-2；顺序、重复插入、未捕获 remove |
| 集合方法式 API | **做** | 与 8 个自由函数一一对应 |
| 动态接收者 | **静态拒绝** | 不引入运行时分派 |
| 方法值 | **不做** | 只允许直接调用 |
| `frozenset`、非字面量下标、子串/切分、泛型表 | **不做** | 保持各自登记 |

## 二、20G 审核返工项（先做）

### B-1 **`.xiaoc` 入口无法加载标准库模块（已关闭）**

**证据**：

```text
from stdlib.set import size
print(size({1}))
→ xiao run <缓存 .xiaoc>
→ X06-RUNTIME-012: 官方标准库模块 set 加载失败: 未注册模块编译器
```

`stdlib.text` 同样复现。原因：`.xiaoc` 专用执行路径
（`xiao-driver/src/protocol/run.rs` 的 `run_xiaoc_request_response`）调用
`run_xiaoc_production_with_capabilities` 时没有传入 `DriverModuleLoader`；
源码运行与 `.xar` 运行都有 loader，只有这条路径的 `module_loader` 是 `None`。

**要求**：

1. 给 `.xiaoc` 执行路径装上与源码/归档相同的标准库 loader（项目根与包注册表按
   现有上下文传入；标准库根沿用 `XIAO_STDLIB_ROOT`）；
2. 覆盖 `stdlib.set` 与既有 `stdlib.text` 两条 `.xiaoc` 运行；
3. `.xiaoc` 的格式/摘要/入口校验顺序不变；无标准库导入的自包含 `.xiaoc` 不回归；
4. 不把标准库对象复制进 `.xiaoc`；仍由宿主提供。

**验收**：上述两条 `.xiaoc` 实际运行并逐字比较输出；缺模块时仍走 20E 的稳定诊断。

### E-1 **原生增删/快照缺自动化运行回归（已关闭）**

**历史现状**：`20g_set_mutation` 的“native”用例曾只做 LLVM 降低，真正执行
`add`/`remove`/包装/快照/非局部返回的原生证据只有审核时的手工探针。现已由
`20c_first_modules::first_modules_match_native_results` 已接入受控原生门禁；Windows/Linux/macOS
分别由 `118017847526`、`118017847726`、`118017847631` 实际执行并通过。

**要求**：

1. 在既有的受控原生门控用例（`first_modules_match_native_results` 或独立的
   10D 忽略用例）加入集合增删、包装、快照与非局部返回程序；
2. 原生输出与 VM/归档逐字比较；工具链缺失时如实 ignored，不得静默少跑；
3. CI 的 Windows/Linux/macOS 原生作业必须实际执行该用例。

**验收**：三个平台原生日志出现该用例且通过；本机受控工具链可复现。

### E-2 **包装顺序与边界断言缺失（已关闭）**

**要求**：

1. `union`/`intersection`/`difference` 的结果**顺序**逐元素断言，不只比较 `size`；
2. `add` 重复元素无操作、`remove` 存在/不存在两种路径；
3. `remove` 未捕获时的稳定错误码 `SET_ELEMENT_MISSING` 与退出码；
4. 别名可见性同时覆盖“通过别名插入”和“通过别名删除”。

**验收**：上述断言在 VM 与原生两侧都成立；缺失任一条时对应用例失败。

### D-1 **文档与证据对账**

- 20G §1.2 的 `20f_container_parameters` **9** → **10**；
- 20G §九/README 20G 行/20F 状态：原生与推送 CI 已由
  `39239884610`/`39239884581` 覆盖；本批另补 `.xiaoc` loader 和方法式 API，三平台运行号
  已分别回填；
- 18B F-1：分支已推送、维护回归全绿，回填运行号，删除“仍未核销”；
- `stdlib/set.xiao` 顶部注释已改成八个入口、两个 intrinsic；
- 20F `00-decisions` 的“集合增删留待后续批次”后加一句指向 20G 的已完成状态；
- registry/UseDocs 已在 B-1/E-1/E-2 的三平台证据齐全后翻为 `verified`。

## 三、三道裁定

| # | 裁定 | 结果 |
| --- | --- | --- |
| 1 | 方法面 | **与自由函数一致**：`size`/`is_empty`/`add`/`remove`/`contains`/`union`/`intersection`/`difference` |
| 2 | 动态接收者 | **静态拒绝**，不引入运行时分派 |
| 3 | 方法值 | **只支持直接调用**；`m = values.add` 报稳定诊断 |

## 四、方法式 API 语义

### 4.1 表面

```xiao
values.size()
values.is_empty()
values.add(value)
values.remove(value)
values.contains(value)
values.union(other)
values.intersection(other)
values.difference(other)
```

**逐条等价**：

| 方法 | 等价自由函数 |
| --- | --- |
| `values.size()` | `size(values)` |
| `values.is_empty()` | `is_empty(values)` |
| `values.add(v)` | `add(values, v)` |
| `values.remove(v)` | `remove(values, v)` |
| `values.contains(v)` | `contains(values, v)` |
| `values.union(other)` | `union(values, other)` |
| `values.intersection(other)` | `intersection(values, other)` |
| `values.difference(other)` | `difference(values, other)` |

方法不是第二套实现：**语义、顺序、错误身份、哈希检查和别名行为必须复用
`stdlib.set` 的自由函数**。实现可以选择 AST 重写或导入调用降低，但不得在
checker/lowerer 里复制 `remove` 的错误逻辑。

### 4.2 调用规则

- **直接调用**：`values.method(...)`；方法值 `m = values.add` 报
  `X03-TYPE-025`（集合方法只能直接调用）；
- **接收者**：静态 `Type::Set(_)` 正常分派；**动态值**调用已知集合方法报
  `X03-TYPE-024`（集合方法接收者必须是静态集合）。数组、字典、标量保持既有成员诊断，
  不改写成集合方法错误，避免劫持其它容器将来的同名方法；
- **未知方法**：静态集合上的未知方法名报 `X03-TYPE-026`；反引号名称不特殊放行；
- **参数**：与自由函数相同的位置/关键字参数规则；`remove` 缺失抛
  `Error(code = "SET_ELEMENT_MISSING")`；`add` 重复无操作；
- **求值顺序**：接收者先求值一次，再按参数顺序求值；`make_set().add(x)` 的
  `make_set()` 只能执行一次；
- **别名**：方法修改共享句柄，对所有别名可见；
- **链式**：`a.union(b).add(x)` 合法，但修改的是 `union` 返回的新集合，不回写 `a`；
- **集合字面量接收者**：`{1, 2}.add(3)` 合法，修改临时集合；结果不可观察，不特殊拒绝。

### 4.3 类型检查与实现

- 新增集合方法分派，识别
  `Expression::Call { callee: Member { object, member } }` 且 `object` 的静态类型为
  `Type::Set`；必须在 `check_table_method_call` 的表类型判定与
  `check_table_member_expression` 的 `member_not_scalar` 之前拦截，否则方法调用会先
  被报成“集合没有成员”；
- 方法签名取 `module_functions["stdlib.set"][method]` 的 `FunctionSignature`：
  去掉第一个 `set<T>` 参数，把接收者的元素类型与 `T` 统一，再走既有
  `check_function_arguments`；
- 结果类型与自由函数一致（`size`/`is_empty`/`contains` 为标量，
  `union`/`intersection`/`difference` 为 `set<T>`，`add`/`remove` 为 `none`）；
- 动态接收者与未知方法的诊断在进入动态成员路径前给出，不能落到
  `member_not_scalar` 或静默 `Dynamic`；
- **静态链接**：方法使用必须让 `stdlib.set` 进入静态链接计划，或明确走模块调用
  路径；两条路径都要测试，不能只在一条路径上通过；
- **IR/生成**：最终降低为与自由函数相同的调用（最好直接复用同一函数定义）；
  不新增 Runtime ABI、不改 `.xiaoc` 格式、不新增能力位；
- **方法值**：`Member` 出现在非调用位置时，如果对象是静态集合且成员名在方法表内，
  报 `X03-TYPE-025`；不要退化成 `Dynamic`。

### 4.4 诊断编号

| 编号 | 含义 |
| --- | --- |
| `X03-TYPE-024` | 集合方法接收者不是静态 `set<T>`（含 dynamic/非集合） |
| `X03-TYPE-025` | 集合方法只能直接调用（方法值） |
| `X03-TYPE-026` | 静态集合上的未知方法名 |

三条都要有中文/英文 message_id 与 i18n 条目；现有集合诊断 023 已占用。

## 五、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 推送本规划；核销 18B/20G 的运行号（已完成的部分回填） | CI |
| 1 | B-1：`.xiaoc` 路径装 loader；`set`/`text` 两条运行用例 | `.xiaoc` 输出 |
| 2 | E-1：原生增删/快照运行用例；CI 三平台执行 | 原生日志 |
| 3 | E-2：顺序、重复插入、未捕获 remove、别名断言 | VM/原生 |
| 4 | D-1：文档计数、状态、运行号、注释对账 | 门禁 |
| 5 | 方法分派：静态集合识别、自由函数签名复用、三条诊断 | 类型单测 |
| 6 | 降低与静态链接/模块调用两条路径；接收者单次求值 | VM/原生/归档 |
| 7 | 全量门禁、registry/UseDocs 状态收口 | 见 §六 |

**最小命令**：

```bash
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20g_set_mutation
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20h_set_methods
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20f_container_parameters
cargo test --manifest-path core/rust/Cargo.toml --workspace --no-fail-fast
bun test
bash tools/gates/run.sh
```

## 六、验收

1. **B-1 关闭**：`stdlib.set` 与 `stdlib.text` 的 `.xiaoc` 实际运行通过；无标准库
   导入的 `.xiaoc` 不回归；
2. **E-1 关闭**：原生增删/包装/快照/非局部返回用例在 CI 三平台实际执行并通过；
3. **E-2 关闭**：包装顺序逐元素断言；重复插入、remove 两种路径、未捕获错误码、
   两种别名方向均有断言；
4. **方法面**：8 个方法与自由函数在同一输入上结果、顺序、错误身份逐字一致；
5. **别名/求值**：方法修改对别名可见；接收者只求值一次；
6. **错误面**：动态接收者报 `X03-TYPE-024`；方法值报 `X03-TYPE-025`；未知方法报
   `X03-TYPE-026`；`remove` 缺失抛 `Error(code = "SET_ELEMENT_MISSING")`；
7. **路径**：源码、`.xiaoc`、`.xar`、原生四路都可用；静态链接与模块调用两条导入
   路径都覆盖；
8. **不回归**：workspace、Bun、`run.sh` 全绿；20F/20G/20E/18B 既有用例不受影响；
9. **文档**：D-1 全部对账；registry/UseDocs 已在 B-1/E-1/E-2 全绿后翻为 `verified`。

## 七、不负责

- **不做**方法值、隐式接收者绑定、闭包对象；
- **不做**动态接收者的运行时分派；
- **不做**数组/字典/字符串的方法式 API；
- **不做** `frozenset`、非字面量下标、子串/切分、泛型表/约束语法；
- **不做**新的 Runtime ABI、能力位、`.xiaoc` 格式或标准库对象内嵌；
- **不把**方法实现成第二套集合语义。

## 八、落点

```text
docs/DevDocs/20h-set-methods-and-20g-closeout.md   本文
docs/DevDocs/README.md                              20H 索引行；20G/20F 状态与计数
docs/DevDocs/20-builtins-and-standard-library.md    §七 阶段表补 20H
docs/DevDocs/03-collections.md                      方法式 API 的契约回填
docs/DevDocs/20g-set-mutation-and-20f-closeout.md   §九 证据对账
docs/DevDocs/20f-container-parameters-and-collections.md 状态与计数
docs/DevDocs/00-decisions.md                        方法式 API/诊断决策
stdlib/set.xiao / stdlib/set.md / docs/UseDocs/library/set.md 方法文档
docs/module-registry.json                           set 条目与测试落点
core/rust/crates/xiao-driver/src/protocol/run.rs    B-1：.xiaoc 标准库 loader
core/rust/crates/xiao-vm/src/run.rs                 B-1：loader 接线（如需）
core/rust/crates/xiao-types/src/checker             方法分派与三条诊断
core/rust/crates/xiao-types/src/diagnostics.rs      X03-TYPE-024/025/026
core/rust/crates/xiao-i18n/src/lib.rs               新 message_id
core/rust/crates/xiao-ir/src/lower.rs               方法调用降低（复用自由函数）
core/rust/crates/xiao-driver/src/frontend.rs        静态链接计划/导入签名
core/rust/crates/xiao-driver/tests/20h_set_methods.rs 本批主回归
core/rust/crates/xiao-driver/tests/20g_set_mutation.rs E-1/E-2 回归
core/rust/crates/xiao-driver/tests/20c_first_modules.rs 原生用例扩展
```

## 九、实施与验证记录（2026-10-19）

- **B-1**：`.xiaoc` 协议入口改用 `DriverModuleLoader`，项目根沿用 `config.xiao` 向上查找
  规则，包注册表沿用 `XIAO_ACTIVE_ENV`，官方标准库根沿用工具链 loader；自包含 `.xiaoc`
  仍不携带标准库对象。`d17c_archive` 的 `stdlib.set` 与 `stdlib.text` 用例各逐字比较输出。
- **方法分派**：类型检查器复用 `module_functions["stdlib.set"]` 的 `FunctionSignature`，
  接收者作为首参数交给同名自由函数；静态链接和 LLVM 动态路径都只保留这一套实现。顶层
  用户函数与方法同名时，标准库链接函数使用内部前缀隔离，避免重复定义或误调用。
- **诊断**：静态集合未知方法、方法值、动态/类型变量接收者分别稳定报
  `X03-TYPE-026`、`X03-TYPE-025`、`X03-TYPE-024`；反引号名称仍走普通成员路径。
- **E-1/E-2**：`20c_first_modules` 的受控原生夹具覆盖 set/text 别名、增删、四种包装、
  快照、非局部返回、方法式 API 与未捕获 `SET_ELEMENT_MISSING`；`20g_set_mutation` 的
  VM 回归为 7 项，`20h_set_methods` 为 10 项；`18b_build_archive` 验证真实
  `build -xar` 通过宿主标准库 loader 运行方法且不复制官方函数。三平台原生作业已实际执行并通过，运行号见上表。
- **本地命令**：`cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20g_set_mutation`、
  `20h_set_methods`、`d17c_archive`、`18b_build_archive`、`cargo test -p xiao-codegen-llvm --tests`、格式检查以及
  `bun run check:docs`/`bun run check:usedocs` 均通过；workspace gates `39239884581`、
  maintenance regression `39239884610` 六作业全绿。

## 相关页面

- [20G 集合增删与 20F 审核收口](20g-set-mutation-and-20f-closeout.md)
- [20F 通用容器参数与官方 `set` 模块](20f-container-parameters-and-collections.md)
- [03. 容器、集合与索引路径](03-collections.md) §集合、§集合运算
- [20AB intrinsic 契约与最小入口](20ab-intrinsics-contract-and-minimal-entry.md)
- [20E 归档接线](20e-archive-wiring.md)
- [04. 函数与控制流](04-functions-and-control.md)
- [18B `build -xar` 与验收对账](18b-build-xar-and-closeout.md) §八
