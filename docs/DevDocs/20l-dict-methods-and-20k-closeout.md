# 20L. 字典方法式 API 与 20K 收口

> **20K 之后的语言批次**。20K 把字典写入、`dict<str,T>` 注解和
> `stdlib/dict.xiao` 的自由函数接通了，但审核实测发现：仓库文档覆盖率跌破
> 90%、`run.sh` 会把任何 Bun 全量失败当成已知抖动放行、数组失效仍漏传递性
> 修改、越界身份还差一条路径。本批先收这些，再把字典方法式 API 接通。
>
> **一句话概括本批**：**让 `d.get(k)` 与 `get(d, k)` 等价，同时把门禁重新变真。**
>
> 状态：**已实施、f119ed0 维护回归与工作区门禁已绿，待当前本地后继提交的新平台复现（2026-10-21）**。四道裁定：八条方法、
> 字典表与字典列都支持、动态接收者静态拒绝且只直接调用、复用 `stdlib/dict.xiao`。
> 本地 VM、`.xiaoc`/`.xar`、LLVM 文本降低和 Windows 受控原生首批模块回归已通过；
> f119ed0 的维护回归 `39517248193` 三平台原生作业与工作区门禁 `39517248607` 已通过，
> `set-dict-alias`、空日志门禁和新的四平台 ordered summary 属于当前本地后继提交的外部待取证项。

## 一、Agent 交接上下文

### 1.1 接手前提

按顺序读：

1. [20K 字典写入、查询接口与字典注解](20k-dict-write-and-annotations.md)
   —— **直接前置**；其 F-1–F-7 是本批返工输入，§4.3 的自由函数是方法语义来源；
2. [20H 集合方法式 API 与 20G 收口](20h-set-methods-and-20g-closeout.md)
   —— 方法分派、方法值诊断、静态链接/模块调用两条路径的既有形状；
3. [20J 字典动态键与 20I 收口](20j-dynamic-dict-keys-and-20i-closeout.md)
   —— 静态键优先、动态键/位置读取与 `X06-RUNTIME-015/017`；
4. [03. 容器、集合与索引路径](03-collections.md) §字典表、§字典列；
5. [20I](20i-nonliteral-indexing-and-splitting.md) §4.2——`[T]` 注解用于
   `keys`/`values` 返回数组；
6. [20AB intrinsic 契约](20ab-intrinsics-contract-and-minimal-entry.md)——
   私有字典动作的契约/移除验证；
7. 错误身份以 `xiao-diagnostics/src/lib.rs` 为准：`X06-RUNTIME-014/015/017`；
8. [18B 收口](18b-build-xar-and-closeout.md) §八——推送 CI 的取证口径。

### 1.2 立项时现状盘点（2026-10-20 实跑）

| 环节 | 现状 | 结论 |
| --- | --- | --- |
| 20K 定向 | `20k_dict_write` 9、`20i_indexing_and_text` 13、`20j` 7、`20h` 10、`20g` 7、`d17c_archive` 14、`18b` 13、`r2a` 17 通过 | 主干成立 |
| 全工作区 | **1305 通过 / 0 失败 / 42 ignored**；Bun 单独跑 298 通过 / 5 跳过 / 0 失败 | 本地主体成立 |
| 完整门禁 | **红**：`A0-COVERAGE-001` 仓库文档覆盖率 **89.93% < 90%** | 阻断；见 F-1 |
| 写入/注解 | 动态键/列位置写入、别名、缺失 015、越界 017、`dict<str,T>` 参数/返回、stdlib 六接口通过 | 可用 |
| 写入根 | `item.values[key] = 7` / 临时接收者赋写报 `X03-TYPE-012` | 仅直接名称根；见 F-5 |
| 私有字典动作 | `__dict_remove`/`__dict_keys`/`__dict_values` 不在 `intrinsics.json`，无移除验证 | 契约缺口；见 F-5 |
| 数组失效 | 直接/别名/单层 helper/表字段/只读调用已修；`outer → inner → array_push` 仍误拒 | F-1 残余；见 F-3 |
| 越界身份 | 修改调用后的字面量越界仍 `X06-RUNTIME-014` | F-2 残余；见表 §二 |
| `run.sh` | 全量 Bun 失败只要两条隔离重试通过就放行 | **假绿**；见 F-2 |
| 20I/20J/20K 推送 | 审核时 **ahead 7**，均未推送 | 无 CI；见 F-6 |
| 平台复现 | 脚本已把 MSVC `link.exe` 放 PATH 首位；无新运行号 | 待取证 |

### 1.2.1 07-D 复核后的当前对账

立项表中的覆盖率红、数组传递失效和 014 残余是历史返工记录；当前本地工作区已完成相应
修复。07-D 复核又发现并修复了原生别名的同名函数污染：LLVM 生成器现在按“标准库模块
身份 + 导出名”保存函数定义，`stdlib.set` 的 `size as count` 不会解析成 `stdlib.dict.size`。
`20c_first_modules::first_modules_match_native_results` 在 Windows 受控 LLVM 下通过，
覆盖 `set-alias` 与 `text-alias`；维护回归 `39517248193` 已证明 f119ed0 在三平台原生作业
全绿，工作区门禁 `39517248607` 也已成功；当前本地后继提交的新增平台证据仍待取证。

`run.sh` 已按真实 Bun 失败名去除 ANSI 后分类，`XIAO_GATE_SELF_TEST=1` 同时覆盖真实
抖动和普通失败拒绝。私有 `__dict_*` 动作继续留在编译器私有 IR/Runtime 边界，20L 定向
回归现断言它们不在 intrinsic 表，并验证 VM 输出和 LLVM 三个私有符号；私有动作保持
编译器内部边界，移除验证不适用，不再登记不存在的外部四路移除承诺。

### 1.3 本批边界

| 项 | 本批 | 说明 |
| --- | --- | --- |
| 20K F-1–F-7 | **做** | 覆盖率、门禁假绿、传递失效、越界身份、私有动作、推送、文档 |
| 字典方法 | **做** | get/set/remove/contains/keys/values/size/is_empty |
| 字典表 + 字典列 | **做** | 方法均按 **str 键**；数值位置仍用下标/赋值 |
| 新增键/upsert | **不做** | 沿用 20K 只更新已有键；缺失 015 |
| 动态接收者 | **静态拒绝** | 不引入运行时分派 |
| 方法值 | **不做** | 只允许直接调用 |
| 嵌套/局部字典注解、方法式 API 扩展到其它容器 | **不做** | 保持后置 |

## 二、20K 审核返工项（先做）

### F-1 **仓库文档覆盖率 89.93% < 90%（门禁红）**

`run.sh` 在 `bun run check` 处失败：

```text
[error] A0-COVERAGE-001 仓库 repository
  全仓库文档覆盖率 89.93% 低于 90%。
error: script "check" exited with code 1
```

新增的 `dict_collect_dynamic`（`xiao-runtime/src/abi_dynamic.rs`）、
`expression_mutates`/`statements_mutate`（`xiao-types/src/checker.rs`）等声明缺文档。
要求：为新增声明补 Rustdoc 或按规则登记生成文件，直到完整 `bun run check` 退出 0；
不能只跑 `check:docs`/`check:usedocs` 就声称门禁通过。

### F-2 **`run.sh` 会把任何 Bun 全量失败放行（假绿）**

当前脚本：

```bash
if [ "$repl_status" -ne 0 ]; then exit 1; fi
if [ "$bun_status" -ne 0 ]; then
  printf 'Bun 全量测试出现已知 REPL 抖动，但隔离重试全部通过；门禁按隔离结果通过。\n'
fi
```

只要两条隔离重试通过，全量 Bun 的**任何**失败都被当作已知抖动放行，普通用例真红也会变绿。
要求：

1. 从全量输出识别失败测试名；只有失败项**全部属于**两条已知抖动时才按隔离结果放行；
2. 其它任何失败必须 exit 1；
3. 隔离重试仍要保留“匹配 0 条测试即失败”的防假绿；
4. 补一条“故意让普通用例失败、门禁必须红”的脚本级自测或文档化验证。

### F-3 **数组失效仍漏传递性修改**

```xiao
def inner([int] values) -> none
    array_push(values, 30)

def outer([int] values) -> none
    inner(values)

values = [10, 20]
outer(values)
print(values[2])
→ X03-TYPE-004
```

直接、别名、单层 helper、表字段、只读调用都已修；`outer → inner` 这种间接修改未被
`mutating_functions` 摘要捕获。要求：计算传递闭包，或采用“凡把数组传给用户函数即失效”
的保守策略，并补传递用例；保留只读函数的静态诊断用例。

### F-4 **修改调用后的字面量越界仍是 014**

```xiao
values = [10, 20]
alias = values
array_push(alias, 30)
print(values[5])
→ X06-RUNTIME-014
```

只读调用后的静态诊断已保留；但长度失效后字面量下标仍走 014，而动态正/负和字面量字符串
是 017。要求冻结一个身份（建议 017）并覆盖“修改后字面量越界”；若保留 014/017 分工，
写明路径并让测试覆盖两者。

### F-5 **私有字典动作与写入根契约缺口**

- `__dict_remove`/`__dict_keys`/`__dict_values` 不在 `intrinsics.json`，没有 20AB 的
  移除验证/效果/能力元数据；20K 计划写的是“不新增 intrinsic”；
- `contains` 用 `try get(...) catch err as Error`，会吞掉任何 `Error`；
- 写入只支持直接名称根：`item.values[key] = 7` 报 `X03-TYPE-012`。

要求：

1. 把三个私有动作登记为受控契约并补移除验证，或明确写成编译器私有动作并给出等价证据；
2. `contains` 只捕获缺失键身份（015），不吞其它错误；
3. 写入根限制写进 `03`/UseDocs，并补正负例；若本批要支持成员根，则一并实现。

### F-6 **推送与原生/平台证据**

- 推送 20I/20J/20K，核销维护回归与工作区门禁；
- `array_push`、动态字典键/写入目前只有降低用例；补受控原生执行回归；
- `array_push` 与私有字典动作的移除验证；
- 平台复现四平台 + ordered summary 运行号；
- `.xiaoc`/`.xar` 写入/查询已有 `d17c_archive`，保留。

### F-7 **文档**

- 20K §1.2 的 1293/“全量 Bun 失败仍判红”与 §五点一/现状对齐；
- 20K 记录 A0-COVERAGE-001 的本地红与修复运行号；
- 20J §1.2/F-1/F-2 的“已修”按传递失效/014 残余收窄；
- 20I §二 F-1 的 `ahead` 与 d17c 计数更新。

## 三、四道裁定

| # | 裁定 | 结果 |
| --- | --- | --- |
| 1 | 方法面 | **六条 + `size`/`is_empty`**：get/set/remove/contains/keys/values/size/is_empty |
| 2 | 接收者 | **字典表 + 字典列**；方法均按 str 键，数值位置仍用下标/赋值 |
| 3 | 调用边界 | **动态接收者静态拒绝、只直接调用**；新增 `X03-TYPE-027/028/029` |
| 4 | 实现路径 | **复用 `stdlib/dict.xiao` 自由函数**；静态链接/模块调用两条都测 |

## 四、设计

### 4.1 方法表面

```xiao
values.get("name")
values.set("name", 2)
values.remove("name")
values.contains("name")
values.keys()
values.values()
values.size()
values.is_empty()
```

**逐条等价**：

| 方法 | 等价自由函数 |
| --- | --- |
| `values.get(k)` | `get(values, k)` |
| `values.set(k, v)` | `set(values, k, v)` |
| `values.remove(k)` | `remove(values, k)` |
| `values.contains(k)` | `contains(values, k)` |
| `values.keys()` | `keys(values)` |
| `values.values()` | `values(values)` |
| `values.size()` | `size(values)` |
| `values.is_empty()` | `is_empty(values)` |

**冻结**：

- 方法不是第二套实现：语义、错误身份、别名、顺序、只更新已有键都复用
  `stdlib/dict.xiao`；
- 接收者可以是字典表或字典列；**键参数固定接受 `str`**；字典列的数值位置仍用
  `column[i]` / `column[i] = v`，方法不重载位置；
- `set`/`remove` 只更新/删除已有键；缺失 → `X06-RUNTIME-015`；
- `keys`/`values` 返回数组（`[str]`/`[T]`），顺序沿用接收者现有迭代顺序；
- `size` 返回条目数；`is_empty` 等价 `size() == 0`；
- 方法修改对别名可见；接收者、键、值各求值一次；求值顺序接收者 → 键 → 值；
- 只有直接调用；`m = values.get` 报 `X03-TYPE-028`；动态接收者报 `X03-TYPE-027`；
  未知方法报 `X03-TYPE-029`。

### 4.2 字典表与字典列的开放视图

`stdlib/dict.xiao` 的自由函数参数是 `dict<str,T>`（开放键集）。字典列的静态类型是
`DictColumn`，不是 `DictTable`。本批需要一条明确的规则：

- `dict<str,T>` 在方法/自由函数调用中表示**按键读写的开放字典视图**，同时接受
  字典表与字典列；
- 视图只承诺 str 键操作；字典列的位置/顺序能力不通过 `dict<str,T>` 视图暴露；
- 类型检查在统一 `dict<str,T>` 与 `DictTable`/`DictColumn` 时按值类型与可哈希键
  兼容；IR 层需要保留“可能是列”的信息或使用统一的开放字典表示；
- 如果实现选择只把 `DictTable` 视为视图，则字典列方法必须另立签名，不能假装复用；
  文档与测试必须明确。

### 4.3 分派与链接

- 方法名表：`get/set/remove/contains/keys/values/size/is_empty`；
- 接收者静态 `DictTable`/`DictColumn` 时，按方法名解析到 `stdlib.dict` 签名，
  去掉第一个接收者参数，统一元素/值类型，再走 `check_function_arguments`；
- 放在表方法分派与成员表达式错误之前，避免先报“字典没有成员”；
- 表方法同名（`get`/`set`/…）仍按表类型走表方法；非字典静态类型保持既有诊断；
- 动态接收者与未知方法在进入动态成员路径前给出 027/029；
- 方法值非调用位置报 028；
- 方法使用必须让 `stdlib.dict` 进入静态链接计划或模块调用路径；两条都测；
  无显式 `from stdlib.dict import ...` 也要可用；
- 降低复用自由函数，不复制 `set/remove` 的错误逻辑；私有 `__dict_*` 动作的契约
  按 F-5 收口。

### 4.4 `size` / `is_empty`

在 `stdlib/dict.xiao` 增加：

```xiao
def size<T>(dict<str, T> values) -> int
def is_empty<T>(dict<str, T> values) -> bool
```

- `size` 通过 `__dict_keys`（或等价只读枚举）计数，不新增公开 intrinsic；
- `is_empty` 返回 `size(values) == 0`；
- 字典列同样适用；顺序不影响计数；
- 若 `size` 在字典列上使用 `[str]` 键数组，必须保持与 keys 一致的顺序语义。

### 4.5 诊断

| 编号 | 含义 |
| --- | --- |
| `X03-TYPE-027` | 字典方法接收者不是静态字典 |
| `X03-TYPE-028` | 字典方法只能直接调用 |
| `X03-TYPE-029` | 静态字典上的未知方法名 |

三条都要有中英文 message_id；不与 20H 的集合方法诊断混用。

### 4.6 测试

- 八方法在字典表/字典列上的结果、错误、别名、顺序、对应；
- 方法 vs 自由函数逐字一致；
- 缺失键 015、列位置仍用下标、越界 017；
- 表方法同名隔离；动态接收者/方法值/未知方法诊断；
- 无显式导入的方法链接；静态链接/模块调用两条；
- 原生、`.xiaoc`、`.xar`；
- F-1–F-7 的回归。

## 五、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 推送 20I/20J/20K；核销 CI；修 F-1/F-2/F-6/F-7 | 运行号/门禁 |
| 1 | F-3/F-4/F-5：传递失效、越界身份、私有动作契约/写入根 | 诊断/VM |
| 2 | `size`/`is_empty` 自由函数；开放视图规则 | VM |
| 3 | 字典方法分派、诊断、表方法隔离 | 类型/VM |
| 4 | 静态链接/模块调用两条路径 | 双路径 |
| 5 | 原生/`.xiaoc`/`.xar`；移除验证 | 四路 |
| 6 | 全量门禁、文档、registry/UseDocs | 见 §六 |

## 五点一、已实施落点

- 八个字典方法已在静态 `DictTable`/`DictColumn` 上直接调用，并复用 `stdlib.dict` 自由函数；方法值、动态接收者和未知方法分别使用 028、027、029。
- `size`/`is_empty` 已加入标准库；`contains` 通过键枚举判断，只对键存在性返回布尔值，不吞掉其它运行时错误。
- 字典列通过 `dict<str,T>` 开放视图参与方法统一，数值位置继续使用下标；成员根 `item.values[key] = value` 已接通。
- 数组变更摘要已计算用户函数调用图传递闭包；修改后未知长度数组的字面量越界统一走 017。
- `run.sh` 只允许两条已知 REPL 失败在隔离重试成功时放行，普通 Bun 失败保持红灯，并提供 `XIAO_GATE_SELF_TEST=1` 分类自测。
- LLVM 动态生成器按标准库模块隔离同名自由函数，并让别名导入按模块绑定；`set-alias`/`text-alias` 原生夹具覆盖了此前 set/dict 静态链接污染。
- 私有 `__dict_*` 动作保持编译器私有 IR/Runtime ABI 形态，不登记公开 intrinsic；方法、`.xiaoc`/`.xar` 和 LLVM 降低回归覆盖其等价语义，并有 intrinsic 缺失断言与三符号移除边界证据。

**最小命令**：

```bash
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20k_dict_write
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20l_dict_methods
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20i_indexing_and_text
cargo test --manifest-path core/rust/Cargo.toml --workspace --no-fail-fast
bun test
bash tools/gates/run.sh
```

## 六、验收

1. **F-1 关闭**：完整 `bun run check` 退出 0，覆盖率 ≥90%；
2. **F-2 关闭**：只有两条已知抖动能按隔离结果放行；普通用例失败必须红；
3. **F-3/F-4 关闭**：传递性修改与修改后字面量越界都有正/负例且身份一致；
4. **F-5 关闭**：私有字典动作有编译器私有边界、intrinsic 缺失断言、VM/LLVM 等价证据；
   移除验证不适用；`contains` 不吞其它错误；写入根限制写明；
5. **F-6/F-7 部分关闭**：f119ed0 的维护回归 `39517248193` 与工作区门禁 `39517248607` 已对账；
   当前本地后继提交的新平台复现和 ordered summary 仍待外部取证；
6. **方法面**：八方法在字典表/列上与自由函数结果、错误、别名、顺序逐字一致；
7. **边界**：动态接收者 027、方法值 028、未知方法 029；表方法同名仍可执行；
8. **链接**：无显式导入可用；静态链接与模块调用两条路径都通过；
9. **四路**：VM、`.xiaoc`、`.xar`、原生一致；
10. **不回归**：workspace、Bun、`run.sh` 全绿；20F–20K/20E/18B 用例不受影响；
11. **文档**：00/02/03/04、20 路线稿、README、registry、UseDocs 同步；
    新能力在四路证据齐全后才翻 `verified`。

## 七、不负责

- **不做**新增键/upsert、字典方法值、动态接收者分派；
- **不做**字典列的数值位置方法（仍用下标/赋值）；
- **不做**嵌套/局部字典注解、字典列增删列；
- **不做**新的能力位、Runtime ABI 主版本或哈希/排序变更；
- **不把**方法实现成第二套字典语义；
- **不在**本批扩集合/数组/文本接口。

## 八、落点

```text
docs/DevDocs/20l-dict-methods-and-20k-closeout.md   本文
docs/DevDocs/README.md                               20L 索引行；20K 状态
docs/DevDocs/20-builtins-and-standard-library.md     §七 阶段表补 20L
docs/DevDocs/03-collections.md                       字典方法/开放视图契约
docs/DevDocs/00-decisions.md                         四道裁定
docs/DevDocs/20k-dict-write-and-annotations.md       F-1–F-7 对账
core/rust/crates/xiao-types/src/checker             方法分派/诊断
core/rust/crates/xiao-types/src/diagnostics.rs       027/028/029
core/rust/crates/xiao-i18n/src/lib.rs                新 message_id
core/rust/crates/xiao-ir/src/lower.rs                方法 lowering
core/rust/crates/xiao-bytecode/src/lower             方法链接
core/rust/crates/xiao-driver/src/frontend.rs         静态链接计划
stdlib/dict.xiao / stdlib/dict.md / docs/UseDocs/library/dict.md size/is_empty
core/rust/crates/xiao-driver/tests/20l_dict_methods.rs
tools/gates/run.sh                                   F-2 假绿修复
core/rust/crates/xiao-runtime/src/abi_dynamic.rs     F-5 私有动作契约
```

## 相关页面

- [20K 字典写入、查询接口与字典注解](20k-dict-write-and-annotations.md)
- [20H 集合方法式 API 与 20G 收口](20h-set-methods-and-20g-closeout.md)
- [20J 字典动态键与 20I 收口](20j-dynamic-dict-keys-and-20i-closeout.md)
- [20I 非字面量下标、数组注解与文本切分](20i-nonliteral-indexing-and-splitting.md)
- [03. 容器、集合与索引路径](03-collections.md)
- [20AB intrinsic 契约与最小入口](20ab-intrinsics-contract-and-minimal-entry.md)
- [18B `build -xar` 与验收对账](18b-build-xar-and-closeout.md) §八
