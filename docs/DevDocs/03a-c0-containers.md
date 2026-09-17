# 03A. C0 基础容器与精确路径实现交接记录

> 状态：已完成。实现、规格测试、分层 UseDocs、目录 README、模块登记和质量门禁均已通过；
> C1/C2 只可按本文“当前不负责的后续工作”继续接手。

> 本页是第 03 阶段的第一个可执行子工程。它把数组、元组、字典表、字典列和
> 声明路径先闭合在“语法 AST → 静态类型 → 精确读取检查”范围内；集合、运行时
> 容器对象和高级选择器必须留给后续 C1/C2，不能为了看起来完整而提前混入。

## 一级工程目标：建立静态容器闭环

### 目标与上下文

前置交付已经提供：

- `xiao-syntax` 的 Token、源码区间、P1 选择器 AST 和 P2 标量声明 AST；
- `xiao-types` 的标量类型、作用域、转换矩阵、HM 统一和结构化诊断；
- A0 的 workspace、目录 README、模块登记和 UseDocs 门禁。

本子工程的输出是可供后续 Runtime/IR 消费的静态结构，不执行 Xiao 代码，也不创建真实数组或字典对象。实现时必须先读 [03. 容器、集合与索引路径](03-collections.md)、[02A. 静态标量类型交接](02a-p2-static-types.md) 和 [12. 测试与开发里程碑](12-tests-and-milestones.md)。

### 冻结决策

1. 数组用 `[]`，默认保留异构位置类型；空数组只表示未知形状。
2. 元组采用 Python 风格圆括号：`()`、`(value,)` 和 `(a, b)`，无逗号的 `(value)` 仍是分组表达式。
3. 字典表用 `{key = value}`，字典列用 `<key = value>`；C0 的键只能是名称、反引号名称或字符串。
4. 同一字典容器中规范化后重复的键在类型阶段拒绝；无序表不暴露顺序，字典列保留书写顺序。
5. `int name` 没有初始化且没有路径时仍是标量声明；`int name = [...]` 是数组元素约束；`int name[3/2]` 是从根容器开始的精确路径约束。
6. 声明路径数字从零开始，`/` 只表示进入嵌套容器，不表示除法；后出现的相同路径覆盖前一条，更深路径可以追加约束。
7. 空数组路径只产生 `ContainerMaterializationPlan`，默认值填充和自动扩容推迟到 Runtime。
8. C0 的读取选择器只接受一个 `SelectorItem::Exact`，无步长；范围、多选、全选、随机和集合索引都产生稳定拒绝诊断。
9. 集合、可哈希检查、集合运算、实际容器修改、范围结果重建和随机抽取属于 C2/C1，不在本子工程实现。

## 二级工程任务

### C0-A：容器 AST 与解析

交付位置：

- `core/rust/crates/xiao-syntax/src/ast.rs`：四种容器表达式、`DictKey`、`DictEntry` 和声明路径字段；
- `core/rust/crates/xiao-syntax/src/parser.rs`：数组、元组、字典表、字典列和声明路径解析；
- `core/rust/crates/xiao-syntax/tests/c0_containers.rs`、`c0_snapshots.rs`：正反语法和节点索引回归。

验收重点：每个节点保留完整 `SourceSpan`；`{}` 稳定解析为空字典表；非法字典条目使用 `X03-PARSE-001`/`X03-PARSE-002`，而不是泛化成 P0 表达式错误；尚未冻结的 `const name[path]` 明确使用 `X03-PARSE-004` 拒绝。

### C0-B：结构化类型与声明约束

交付位置：

- `core/rust/crates/xiao-types/src/containers.rs`：数组形状、字典条目、路径约束树和物化计划；
- `types.rs`、`unify.rs`、`conversion.rs`：容器递归遍历、统一和结构化赋值兼容性；
- `environment.rs`：绑定上的 `container_constraints`；
- `container_checker.rs`：容器字面量推导、重复键检查、显式数组元素类型和重复路径合并。

类型层必须返回结构化 `Type::Array/Tuple/DictTable/DictColumn`，不能把异构数组降级为 `Array<Dynamic>`。初始化器为空时不得在这里填入字符串或数值默认值。
对已经初始化且结构可知的绑定追加路径约束时，必须立即复用同一棵约束树做边界和元素类型检查；只有未知长度或动态根才可以推迟到 Runtime。

### C0-C：精确索引与稳定诊断

交付位置：

- `path_constraints.rs`：语法路径降低为 `ContainerPathSegment`，以及数组、元组、字典表/列的精确下降；
- `container_checker.rs`：选择器单项检查和诊断映射；
- `xiao-types/src/diagnostics.rs`：`X03-TYPE-001` 至 `X03-TYPE-006`。

静态已知越界、字典键缺失、路径段类型错误必须区分；未知数组边界返回 `Dynamic`，由后续 Runtime 插入检查，不得把未知边界误报为编译错误。C0 不把不支持的范围或随机项静默当作全选。

### C0-D：测试、UseDocs 与交接

交付位置：

- `tests/spec/05-containers/`：可供跨后端复用的规范样例和错误样例；
- `docs/UseDocs/language/collections/`：面向自然人的分层使用文档；
- `docs/module-registry.json`：登记新增测试和 UseDocs 路径；
- 本页、`03-collections.md`、`12-tests-and-milestones.md` 和各目录 README：同步边界和退出条件。

代码和测试完成后必须同一提交补齐 UseDocs；UseDocs 页面状态达到 `verified` 前，C0 不能在总索引中标成已完成。

## 实现接口摘要

### 静态类型形状

```text
Array(Homogeneous { element, length })
Array(Heterogeneous { elements })
Array(Unknown)
Tuple(items)
DictTable(entries)
DictColumn(entries)
```

`DictTable` 的查找按规范化键，`DictColumn` 既可按数字位置也可按键名；两者都允许嵌套路径。

### 物化计划

`ContainerMaterializationPlan` 只记录绑定名、约束树和每层最小长度。例如 `int list[3/2]` 的 `minimum_lengths` 为 `[4, 3]`。它不是运行时值，也不承诺当前阶段已经完成默认值填充。

### 稳定诊断

| 编号 | 含义 |
| --- | --- |
| `X03-TYPE-001` | 容器元素或显式类型不匹配 |
| `X03-TYPE-002` | 字典重复键 |
| `X03-TYPE-003` | 路径段与容器种类不匹配 |
| `X03-TYPE-004` | 静态数字索引越界 |
| `X03-TYPE-005` | 字典键不存在 |
| `X03-TYPE-006` | 路径无法降低为非负整数/键名 |

## 交接清单

接手代理必须在修改前确认：

1. 不把集合语义塞进字典表解析器；`{}` 仍是空字典，集合留给 C2。
2. 不在 `xiao-types` 创建 Runtime 对象，不从类型层依赖 VM、LLVM 或 CLI。
3. 不把新的容器分支继续堆入 `checker.rs`；容器逻辑进入 `container_checker.rs` 或新的单一职责模块。
4. 修改 AST 后同时更新节点索引、解析快照、类型测试和 UseDocs；所有公共 API 保持 100% 文档注释。
5. 先运行定向门禁，再运行仓库总门禁：

```text
cargo test --manifest-path core/rust/Cargo.toml -p xiao-syntax -p xiao-types
cargo clippy --manifest-path core/rust/Cargo.toml -p xiao-syntax -p xiao-types --all-targets -- -D warnings
bun run check
bun run check:coverage
```

## 当前不负责的后续工作

- C1：范围、多选、单边范围、步长、`[=]`、`[?x]`/`[!?x]` 和结果形状重建；
- C2：集合、可哈希能力、`set()`、`+` 并集和 `frozenset` 评估；
- Runtime：自动扩容、默认值、实际读写和运行时边界检查；
- 04/F1（已完成静态阶段）：函数、控制流和入口的 AST/类型约束见 [04](04-functions-and-control.md)；
  闭包、模块和后端执行仍属于后续阶段。
