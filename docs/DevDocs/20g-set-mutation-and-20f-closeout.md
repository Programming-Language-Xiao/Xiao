# 20G. 集合增删与 20F 审核收口

> **20F 之后的集合批次**。20F 接通了通用容器参数与官方 `set` 的只读入口
> （`size`/`is_empty`），但审核实测留下三类必须先处理的问题：**原生别名路径是红的**、
> **函数头的 `set<...>` 注解绕过可哈希校验**、**集合多实例用例不可证伪**。
> 本批先把这些返工做完，再补集合增删与只读包装。
>
> **一句话概括本批**：**先把 20F 的验收补成真的，再让集合真正可变。**
>
> 状态：**规划完成、待实施（2026-10-19）**。四道裁定：范围取「add/remove + 只读包装」、
> API 用 `stdlib/set.xiao` 自由函数、`remove` 缺失报可恢复错误、迭代开始时快照。
> 本批未改代码。

## 一、Agent 交接上下文

### 1.1 接手前提

按顺序读：

1. [20F 通用容器参数与官方 `set` 模块](20f-container-parameters-and-collections.md)
   —— **直接前置**；§三–§六 的设计与验收仍然有效，本批只覆盖 §二 的返工项与新增范围；
2. [03. 容器、集合与索引路径](03-collections.md) §集合、§集合运算、§可哈希规则——
   集合语义、元素唯一性与顺序契约的唯一权威；
3. [20C 官方库](20c-official-xiao-library.md) §3.4.1——「最小接口」必须冻结一个
   语义决定；只读包装也要逐条写出它冻结了什么；
4. [20AB intrinsic 契约与最小入口](20ab-intrinsics-contract-and-minimal-entry.md)
   —— 新增 intrinsic 的契约、移除验证与双路径接线的做法；
5. [20D 进程 API](20d-process-api.md) §十二——`os_*` intrinsic 在 VM/LLVM 两条路径
   接线的先例；
6. [20E 归档接线](20e-archive-wiring.md)——`stdlib.*` 模块列表与启动前校验；
7. [18B 收口](18b-build-xar-and-closeout.md) §八、§十二、§十三——本批继续承接
   F-1（推送 CI 取证）与已勘误的 F-2。

### 1.2 现状盘点（2026-10-19 实跑）

| 环节 | 现状 | 结论 |
| --- | --- | --- |
| 20F 定向 | `p2_type_parameters` 4 通过；`20f_container_parameters` **9** 通过 | 基础语法/刚性与 VM 路径成立 |
| 全工作区 | **1250 通过 / 0 失败 / 41 ignored**；Bun 298 通过 / 5 跳过 / 0 失败；`run.sh` 退出 0 | 本地门禁成立 |
| 原生无别名 | 手工探针 `size({1,2})`、`size({"a","b","c"})`、`is_empty(set())`、本地泛型两组均输出正确 | **可用** |
| 归档 | `build -xar` + `run -xar` 输出正确；缺 `set.xiao` 时 `X17-XAR-008` | **可用** |
| 原生别名 | `from stdlib.set import size as count` → `X11-PROTOCOL-007: IR 无效：名称 size 没有动态值槽` | **红**；见 F-1 |
| `set<file>` 参数注解 | `def g(set<file> values) -> int` 可编译并运行 | **类型漏洞**；见 F-2 |
| 集合多实例测试 | 关闭调用点实例化后仍通过；标量用例才变红 | **不可证伪**；见 F-3 |
| 泛型诊断 | 每个泛型函数/`size` 调用打 `X06-LIFETIME-005 ... (DynamicValue)` | 噪声；见 F-5 |
| 类型参数分隔 | `def f<T U>` 与 `<T,,U>` 被接受 | 解析器宽松；见 F-6 |
| 运行时可变性 | `SetHandle` 只有只读/代数接口，没有 `insert`/`remove` | 本批新增 |
| intrinsic 契约 | 现有最高 id 28；arity 没有二参数种类 | 本批扩展 |

## 二、20F 审核返工项（先做）

审核对象是 `4fd454d` 与 `52cc596`。以下每一项都写明**复现证据、要求与验收**；
在返工完成前，20F 不得标记为「验收通过」，registry/UseDocs 也不得保持 `verified`。

### F-1 **原生别名/模块调用路径失败（阻断）**

**证据**（受控原生工具链，`x86_64-pc-windows-msvc`）：

```text
from stdlib.set import size as count
print(count({"a", "b"}))
→ X11-PROTOCOL-007: IR 无效：名称 size 没有动态值槽（23..27）
```

`stdlib.text` 的同类写法同样复现：

```text
from stdlib.text import length as len
→ X11-PROTOCOL-007: IR 无效：名称 length 没有动态值槽（24..30）
```

**性质**：别名导入不走静态链接；IR 仍以原名（`size`/`length`）引用被导入函数，
而本地绑定名是别名（`count`/`len`），原生 IR 校验找不到对应槽。VM 路径正常，
所以 20F 的 VM 用例把这条缺口遮住了。

**要求**：

1. 定位并修复原生模块调用路径的别名/原名槽不一致；修复必须同时覆盖
   `stdlib.set` 与既有 `stdlib.text`/`stdlib.path`，不能只给 `set` 打补丁；
2. 增加两条原生回归：`set` 别名与 `text` 别名，构建并实际运行、逐字比较输出；
3. 无别名静态链接路径保持现状；VM 路径不得回退。

**验收**：两条原生别名用例在受控工具链上通过；无别名用例仍通过；VM 别名用例仍通过。

### F-2 **函数头的 `set<...>` 注解绕过可哈希校验**

**证据**：

```xiao
def g(set<file> values) -> int
    return 1

print(g(set()))
```

编译并运行成功，输出 `1`。03 规范明确 `file`/`directory` 不能作为集合元素；
变量声明走 `check_set_annotation_members`，但函数参数/返回注解走
`function_checker::annotation_type`，没有同一个校验。

**要求**：

1. 函数参数与返回位置的 `set<...>` 注解都校验成员可哈希：
   具体资源成员报 `X03-TYPE-016`（与变量声明同一错误身份）；
2. 类型参数项（`set<T>`）不在此处判死，仍由调用点实例化时按 `X03-TYPE-023` 处理；
3. 负例覆盖 `set<file>`、`set<resource>`、`set<file | int>` 与返回位置；
   正例覆盖 `set<T>`、`set<T | int>`。

**验收**：上述负例均在声明处报 `X03-TYPE-016`，不再需要等到调用泛型函数才报错。

### F-3 **集合多实例用例不可证伪**

**证据**：把 `check_known_function_call` 的非递归调用改为直接使用原签名
（跳过 `instantiate_function_signature`）后：

- `local_generic_set_function_has_multiple_instances` **仍然通过**；
- `local_generic_scalar_function_has_multiple_instances` **失败**
  （`cannot unify int with str`）。

原因是集合统一会把 `set<int>` 与 `set<str>` 合成为异构集合，不会报错；该用例
并没有真正观察调用点实例化。

**要求**：

1. 把集合多实例用例改成**能观察到元素标量类型**的形态，例如
   `def take<T>(set<T> values, T fallback) -> T`，分别用 `set<int>`/`int` 与
   `set<str>`/`str` 调用；跳过实例化时第二次调用必须失败；
2. 或增加一个直接断言实例化签名的类型层单测；两条路线至少落一条；
3. 保留现有标量用例，作为第二道保护。

**验收**：重新施加「跳过实例化」变异时，**集合用例本身**必须变红；还原后全绿，
工作树干净。

### F-4 **`verified` 状态缺双路径证据**

**证据**：`docs/module-registry.json` 的 `stdlib.xiao-standard-library-set` 与
`docs/UseDocs/library/set.md` 标为 `verified`；仓库里没有 20F 的原生、`.xiaoc`
或 `.xar` 运行回归（现有归档用例只构建、不运行）。

**要求**：

1. 在 `verified` 之前补齐：无别名原生、别名原生（依赖 F-1）、`.xiaoc` 入口、
   `.xar` 构建并实际运行；
2. 归档缺模块负例（`X17-XAR-008`）进自动化；
3. 证据未齐前，registry 与 UseDocs 保持 `draft`，文档写明已验证路径与未验证路径；
4. 20F §十 的「7 项」改为实际的 9 项；README 20F 行的「复验中」在证据齐后更新。

**验收**：上述四类用例都有可重放命令与结果；`verified` 只在全绿后翻。

### F-5 **泛型函数触发 `X06-LIFETIME-005` 兜底文本**

**证据**：

```text
from stdlib.set import size
print(size({1, 2}))
→ X06-LIFETIME-005: 值 3 的生命周期需要 Runtime 检查 (DynamicValue)
```

裸泛型 `def f<T>(T) -> T` 还会出现 `(Returned)`。这是警告、退出码为 0，但
`DynamicValue`/`Returned` 是内部 Debug 文本；20AB-FIX 修过同类的
`X06-LIFETIME-005` 兜底文本。

**要求**：把 reason 映射为用户可读文案（至少不出现 `DynamicValue`/`Returned`），
或在文档中明确该警告是擦除后的预期检查并给出稳定 message_id；不得新增错误级诊断。

**验收**：同一程序不再出现 `DynamicValue` 文本；运行时检查记录仍保留。

### F-6 **类型参数列表缺少逗号不报错**

**证据**：`def f<T U>(T value) -> T` 与 `<T,,U>` 都被接受并运行。

**要求**：类型参数之间必须用 `,` 分隔；空项、连续逗号、缺逗号报
`X04-PARSE-007`；尾逗号是否允许在实现前明确并加测试。

**验收**：缺逗号与连续逗号负例均报稳定诊断；合法 `<T, U>` 不受影响。

### F-7 **文档与推送证据**

- 20F §十 的定向测试数量 7 → **9**；
- README 20F 行「完整 workspace 与门禁复验中」→ 实跑结果；
- 20F 状态改为「本地实现完成；原生别名路径未通过，验收未完成」，直到 F-1–F-4 关闭；
- **18B F-1 仍未核销**：分支现领先远端，18B 与 20F 的提交都没有推送 CI；
  本批第 0 步推送后核销维护回归与工作区门禁，未落地前不写「验收通过」。

## 三、本批新增范围与四道裁定

### 3.1 范围

| 项 | 本批 | 说明 |
| --- | --- | --- |
| `add<T>` / `remove<T>` | **做** | 真正的可变入口；补 03 §集合「增加和删除元素」 |
| `contains<T>` | **做** | 只读包装；冻结成员判定语义 |
| `union<T>` / `intersection<T>` / `difference<T>` | **做** | 只读包装；冻结结果顺序与成员类型规则 |
| 方法式 API | **不做** | `values.add(x)` 仍后置；本批只做自由函数 |
| `frozenset` | **不做** | 03 仍待定 |
| 非字面量下标、子串/切分 | **不做** | 保持各自登记 |
| 泛型表/方法、约束语法 | **不做** | 20F 边界不变 |

### 3.2 四道裁定

| # | 裁定 | 结果 |
| --- | --- | --- |
| 1 | 新增范围 | **add/remove + 只读包装**（contains/union/intersection/difference） |
| 2 | API 形态 | **`stdlib/set.xiao` 自由函数**；方法式 API 后置 |
| 3 | `remove` 缺失 | **报可恢复错误**，不返回 bool |
| 4 | 迭代中修改 | **迭代开始时快照**，当前循环不受影响 |

## 四、接口与语义

### 4.1 `stdlib/set.xiao` 首版完整接口

```xiao
def size<T>(set<T> values) -> int
def is_empty<T>(set<T> values) -> bool

def add<T>(set<T> values, T value) -> none
def remove<T>(set<T> values, T value) -> none

def contains<T>(set<T> values, T value) -> bool
def union<T>(set<T> left, set<T> right) -> set<T>
def intersection<T>(set<T> left, set<T> right) -> set<T>
def difference<T>(set<T> left, set<T> right) -> set<T>
```

**逐条冻结的决定**（按 [20C §3.4.1](20c-official-xiao-library.md) §3.4.1）：

| 接口 | 冻结的决定 |
| --- | --- |
| `add` | 集合是共享引用；插入对别名可见；已存在时无操作；元素必须可哈希；保持首次出现顺序，不引入哈希索引 |
| `remove` | 集合是共享引用；删除对别名可见；不存在时抛 `Error(code = "SET_ELEMENT_MISSING")`；成功时返回 `none` |
| `contains` | 成员判定使用集合的相等/哈希语义，不依赖遍历顺序；不承诺复杂度 |
| `union` | 结果顺序：左侧原序，随后右侧中不在左侧者按右侧原序；成员类型取两侧已知并集 |
| `intersection` | 结果顺序：左侧原序过滤；成员类型取静态交集 |
| `difference` | 结果顺序：左侧原序过滤；成员类型保留左侧 |

这些顺序不是新的运行时算法，而是把 `SetObject`
（`xiao-runtime/src/containers/set.rs`）已冻结的运算顺序提升为库契约；
**不得**为了实现包装而改 `SetHandle` 的既有顺序或引入哈希索引。

### 4.2 `remove` 的错误身份

- 构造：`Error(code = "SET_ELEMENT_MISSING")`（可恢复错误；`XiaoError` 也能捕获）；
- 触发：元素不在集合中；成功删除不报错；
- 稳定码写进 `stdlib/set.md` 与 UseDocs；测试匹配 `code`，不匹配文案；
- 不新增 `X06-RUNTIME-*` 身份，避免把库级契约混进运行时内部错误；
- 未捕获时按既有错误传播路径退出，不引入新的崩溃路径。

### 4.3 运行时可变性

- `SetObject.elements: Vec<RuntimeValue>` 保持现有结构；`SetHandle` 增加
  `insert`/`remove`（或 `with_elements_mut` 风格的受控入口）；
- 插入前用 `is_hashable` 检查，不可哈希报既有 `RuntimeError::unhashable_element`；
- 插入去重按 `RuntimeValue::PartialEq`，保持首次出现顺序；
- 删除保持剩余元素顺序；
- 不引入哈希索引——模块文档已固定「有序 Vec + 线性去重」，顺序是可观察契约；
- 别名：句柄共享同一对象，`add`/`remove` 修改对所有别名可见；不得复制后重绑。

### 4.4 intrinsic 契约

新增两个 intrinsic，按 20AB 的单一来源表登记：

```json
{"id":29,"name":"set_add","kind":"set_add","arity":"two",
 "parameter_type":"set_element","return_type":"none",
 "vm_binding":"set_add","abi_binding":"set_add",
 "effect":"mutates_container","capability":"none",
 "error":"runtime.unhashable_element","status":"active"}
{"id":30,"name":"set_remove","kind":"set_remove","arity":"two",
 "parameter_type":"set_element","return_type":"bool",
 "vm_binding":"set_remove","abi_binding":"set_remove",
 "effect":"mutates_container","capability":"none",
 "error":"runtime.invalid_value","status":"active"}
```

**冻结**：

- `runtime.unhashable_element` 对应的稳定码是 `X06-RUNTIME-016`
  （`CONTAINER_HASHABILITY_CODE`）；错误键的准确拼写在实施时按现有 `os_*` 条目核对；
- `arity` 增加 `Two`（现有表只有 zero/one/optional_one/variadic/error）；
- `parameter_type` 增加 `set_element` 描述（第一个参数必须是集合，第二个是元素）；
- `effect` 增加 `mutates_container`；能力位仍是 `none`，不申请新能力；
- `set_add` 重复元素返回 `none`；`set_remove` 返回是否真的删除（bool），
  **库层**负责把 `false` 转成 `SET_ELEMENT_MISSING` 错误；
- VM 绑定调用 `SetHandle` 的新方法；LLVM 动态路径通过 Runtime ABI 同名函数接线；
- 移除验证：删表项、VM 绑定或 ABI 包装任一环节，专用用例必须失败。

### 4.5 迭代快照

**冻结**：`for value in set_value` 在**循环开始时**对集合元素做一次快照；
循环体内的 `add`/`remove` 不影响本次迭代；循环结束后再遍历才能看到新状态。

**实现要求**：

- IR 层提供共享的快照动作（新 `TacOp` 或等价运行时快照），VM 与 LLVM 使用同一语义；
- 快照对象在循环出口按既有生命周期计划释放，不泄漏；
- 只对集合迭代生效；数组/字典/字符串的迭代语义不在本批改动；
- 快照顺序沿用 `SetObject.elements` 的首次出现顺序；不得引入排序；
- 测试：循环内删除当前元素、添加新元素、删除后续元素三种情况，VM/原生输出一致，
  且下一轮循环看到修改后的状态。

## 五、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 推送 18B/20F/本规划提交，核销维护回归与工作区门禁；未落地前状态只写「本地」 | CI 运行号 |
| 1 | F-1：定位原生别名槽不一致，修模块调用路径；补 `set`/`text` 原生别名用例 | 原生构建+运行 |
| 2 | F-2：函数头 `set<...>` 注解可哈希校验；正负例 | 类型诊断 |
| 3 | F-3：加强集合多实例用例；重做「跳过实例化」变异 | 变异红/绿 |
| 4 | F-4：原生、`.xiaoc`、`.xar` 运行与缺模块负例；状态仍 `draft` | 四类证据 |
| 5 | F-5/F-6：诊断文案与类型参数分隔；F-7 文档对账 | 门禁 |
| 6 | 运行时：`SetHandle::insert/remove`、不可哈希与别名测试 | 运行时单测 |
| 7 | 契约：`arity=two`、`set_element`、`mutates_container`、id 29/30；VM/LLVM 接线 | 契约校验 + 双路径 |
| 8 | 模块：`add`/`remove`/`contains`/`union`/`intersection`/`difference`；错误身份 | VM/原生差分 |
| 9 | 迭代快照：IR/VM/LLVM + 三种循环内修改场景 | 双路径一致 |
| 10 | 移除验证、全量门禁、registry/UseDocs 翻状态 | 见 §六 |

**最小命令**：

```bash
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20f_container_parameters
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20g_set_mutation
cargo test --manifest-path core/rust/Cargo.toml -p xiao-intrinsics
cargo test --manifest-path core/rust/Cargo.toml --workspace --no-fail-fast
bun test
bash tools/gates/run.sh
```

## 六、验收

1. **F-1 关闭**：`set` 与 `text` 的原生别名用例构建并运行通过；无别名路径不回归；
2. **F-2 关闭**：`set<file>`/`set<resource>`/`set<file | int>` 在参数与返回位置
   均报 `X03-TYPE-016`；`set<T>` 仍可用；
3. **F-3 关闭**：重做「跳过实例化」变异时集合用例本身变红；还原后全绿；
4. **F-4 关闭**：无别名原生、别名原生、`.xiaoc`、`.xar` 运行四类用例与缺模块负例
   都有记录；registry/UseDocs 才可翻 `verified`；
5. **`add`**：插入新元素、重复插入无操作、别名可见、不可哈希元素被拒；
   VM 与原生一致；
6. **`remove`**：存在时删除并返回 `none`；不存在时抛
   `Error(code = "SET_ELEMENT_MISSING")` 且可被 `Error`/`XiaoError` 捕获；
   别名可见；
7. **只读包装**：`contains`/`union`/`intersection`/`difference` 的结果、顺序与
   成员类型符合 §4.1；VM/原生逐字一致；
8. **迭代快照**：循环内增删不影响本次迭代；下一轮看到新状态；VM/原生一致；
9. **契约**：id 29/30、arity two、效果与能力位正确；移除验证三项均红；
10. **不回归**：workspace、Bun、`run.sh` 全绿；20E/18B 既有用例不受影响；
11. **18B F-1**：推送后核销运行号；在此之前 18B/20F 状态只写「本地验收」；
12. **文档**：20F 的两处计数更正、README 状态、registry/UseDocs、`stdlib/set.md`、
    UseDocs 的稳定错误码全部同步。

## 七、不负责

- **不做**方法式 API（`values.add(x)`）、`frozenset`、泛型表/方法、约束语法；
- **不做**非字面量下标、子串/切分；
- **不做**哈希索引或改变集合运算顺序；
- **不做**新的能力位、新的 Runtime ABI 主版本或把集合写进 `.xar` 对象；
- **不做**数组/字典/字符串的迭代语义调整；
- **不在**本批顺手扩 20F 之外的类型参数能力。

## 八、落点

```text
docs/DevDocs/20g-set-mutation-and-20f-closeout.md   本文
docs/DevDocs/README.md                              20G 索引行；20F 状态与计数
docs/DevDocs/20-builtins-and-standard-library.md    §七 阶段表补 20G
docs/DevDocs/03-collections.md                      集合增删/迭代快照的契约回填
docs/DevDocs/20f-container-parameters-and-collections.md  §十 计数与状态更正
docs/DevDocs/00-decisions.md                        集合可变/remove 错误/快照决策
docs/module-registry.json                           set 条目状态与代码/测试落点
stdlib/set.xiao                                     add/remove/contains/union/intersection/difference
stdlib/set.md                                       接口、错误码、顺序与快照语义
docs/UseDocs/library/set.md                         用户文档与稳定错误码
core/rust/crates/xiao-runtime/src/containers/set.rs insert/remove、不可哈希、别名
core/rust/crates/xiao-intrinsics/intrinsics.json    id 29/30、arity two、set_element、mutates_container
core/rust/crates/xiao-intrinsics/src/lib.rs         Arity::Two 与 set_element 校验
core/rust/crates/xiao-vm/src/semantics/exec.rs      set_add/set_remove 绑定
core/rust/crates/xiao-codegen-llvm/src/dynamic      原生 intrinsic 与迭代快照
core/rust/crates/xiao-codegen-llvm/src/artifact.rs  Runtime ABI 符号
core/rust/crates/xiao-types/src/function_checker.rs 函数头 set 注解可哈希校验（F-2）
core/rust/crates/xiao-syntax/src/parser/statements.rs 类型参数逗号（F-6）
core/rust/crates/xiao-lifetime/src                   快照对象释放
core/rust/crates/xiao-driver/tests/20g_set_mutation.rs 本批主回归
core/rust/crates/xiao-driver/tests/20f_container_parameters.rs F-1/F-3 回归
```

## 相关页面

- [20F 通用容器参数与官方 `set` 模块](20f-container-parameters-and-collections.md)
- [03. 容器、集合与索引路径](03-collections.md) §集合、§集合运算、§可哈希规则
- [20C 官方标准库](20c-official-xiao-library.md) §3.4.1
- [20AB intrinsic 契约与最小入口](20ab-intrinsics-contract-and-minimal-entry.md)
- [20D 进程 API](20d-process-api.md) §十二（VM/LLVM intrinsic 接线先例）
- [20E 归档接线](20e-archive-wiring.md)
- [18B `build -xar` 与验收对账](18b-build-xar-and-closeout.md) §八、§十二、§十三
