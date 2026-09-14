# 03D. C2-B 异构集合与动态成员静态闭环交接记录

> 状态：已完成静态阶段。本阶段把 C2-A 的单一元素类型扩展为“静态成员类型并集 +
> 动态尾标”，但不创建 Runtime 集合对象、不执行集合代数或增删操作。本文是后续代理
> 接手时的实现契约，不能把静态检查结果误写成运行时功能已经可用。

## Agent 交接上下文

### 接手前必须阅读

1. [00. 决策基线](00-decisions.md)：`bool` 独立类型、可哈希规则、国际化字段和集合总边界。
2. [02A. P2-B/S0 静态标量类型](02a-p2-static-types.md)：`Type`、环境、转换矩阵和
   `RuntimeCheckKind`。
3. [03. 容器、集合与索引路径](03-collections.md)：容器分类、无序集合不可索引和路径规则。
4. [03C. C2-A 最小集合静态闭环](03c-c2a-sets.md)：花括号消歧、可哈希诊断、C2-A
   快照和不可索引边界。
5. [11C. 国际化](11c-localization.md)：`code`、`message_id`、`params` 与展示文本分离。
6. [12. 测试与开发里程碑](12-tests-and-milestones.md)：阶段门禁、快照和 UseDocs 同步要求。

### 已有输入

- `xiao-syntax` 已能把非空纯值花括号解析为 `Expression::SetLiteral`，把 `{}` 保留为
  空字典表，并识别 `set()` 调用。
- `xiao-syntax` 的声明节点使用 `DeclaredType`：旧式标量前缀为
  `DeclaredType::Scalar`，C2-B 注解为 `DeclaredType::Set(SetTypeAnnotation)`。
- `xiao-types` 已有结构化容器类型、静态可哈希判定、诊断参数和集合不可索引检查。
- Runtime、IR、LLVM 和 CLI 尚未消费集合对象；本阶段只能输出类型和旁路检查标记。

## 一级工程目标：闭合 C2-B 静态集合语义

### 冻结规则

1. **默认异构**：未显式声明的非空集合按元素的静态类型推导并集，例如
   `{1, "x", true}` 的类型是 `set<bool | int | str>`（显示顺序是稳定规范化顺序，
   不是用户可观察的迭代顺序）。
2. **显式并集**：变量根声明使用 `set<T>` 或 `set<T | U>`，例如：

   ```xiao
   set<int> ids = {1, 2}
   set<int | str> values = {1, "x"}
   ```

   当前类型项只允许标量和 `none`。显式声明保留完整并集，即
   `set<int | str> values = {1}` 仍绑定为 `set<int | str>`。
3. **旧写法兼容**：`int ids = {1, 2}` 仍表示旧式同构容器元素约束；它不能借 C2-B
   的默认异构规则接受字符串或布尔成员。
4. **规范化**：静态成员按确定的 Xiao 类型文本排序并去重。重复类型项不产生额外
   语法错误；它们在类型层表示同一个成员约束。
5. **严格类型隔离**：集合相等性和哈希身份按静态类型隔离。`true` 与 `1`、`1` 与
   `1.0`、不同整数宽度都不是同一个集合元素。静态重复诊断只能在类型相同且常量值
   相等时触发。
6. **动态尾标**：集合中出现 `Dynamic` 或尚未统一的类型变量时，保留已知静态并集，
   并设置 `allows_dynamic = true`。动态成员登记 `SetHashability`；把动态集合赋给
   受限集合或执行动态成员判断时登记 `SetMembership`，不能伪造静态成功。
7. **空集合**：无上下文的 `set()` 仍为 `Type::Set(SetType::Unknown)`；在
   `set<T>` 声明初始化上下文中可接受并绑定为声明者的完整类型。`{}` 永远是空字典表。
8. **成员判断**：`in`/`not in` 返回 `bool`。右侧必须是集合；已知静态左值必须命中
   并集中的同一类型，动态边界只登记 Runtime 检查。不可哈希左值仍立即报错。
9. **声明边界**：C2-B 集合类型注解只允许变量根，不能写成
   `set<int> values[0]`；`const set<...>` 暂不开放。集合仍完全不可索引。
10. **明确不做**：不创建集合 Runtime 对象，不实现增删、`+`/`&`/`-`/`^` 运算、集合
    比较、`frozenset`、元组递归哈希或后端指令。

### 国际化边界

每条新增诊断保留稳定 `code`、`message_id` 和 `DiagnosticParam` 参数；中文文本只是
当前预览。类型层不加载语言目录，也不从展示文本反解析类型。快照只比较机器字段，
UseDocs 说明触发条件但不把翻译句子当作接口。

## 二级实现任务（SOP）

### C2-B.1：声明语法与 AST

1. 在 `xiao-syntax` 中增加 `Pipe` Token，并仅在 `set<...>` 声明上下文解析，不把它
   变成集合运行时运算符。
2. 用 `TypeTerm`、`SetTypeAnnotation` 和 `DeclaredType` 保存源码结构；每个注解保留
   `SourceSpan`，类型项暂限标量/`none`。
3. 处理空注解、尾部 `|`、非法容器类型、缺少 `>`、集合路径和 `const` 集合的稳定
   诊断与恢复；`set()` 必须继续解析成普通调用。
4. 在 `xiao-syntax/tests/c2b_sets.rs` 覆盖单行、多行、重复类型项、错误恢复和
   `set < value` 比较表达式消歧。

### C2-B.2：静态类型模型

1. 在 `xiao-types/src/set_types.rs` 将集合描述扩展为 `Unknown`、`Homogeneous` 和
   `Heterogeneous { members, allows_dynamic }`，提供稳定规范化、成员查询、展示和
   赋值兼容辅助。
2. 在 `types.rs`、`unify.rs` 中递归遍历并集成员，统一时合并成员并集并保留动态尾标；
   不把集合元素类型塞回标量转换矩阵。
3. 集合赋值保持静态成员类型不变性；未知集合可在显式声明上下文具体化，动态尾标由
   调用方登记 Runtime 检查。

### C2-B.3：类型检查与兼容路径

1. `set_checker.rs` 推导非显式集合的静态并集，保留动态尾标，执行严格类型隔离的
   静态重复检查和 Python 风格可哈希检查。
2. 显式 `set<T | U>` 初始化器逐成员匹配完整并集；`set()` 接受上下文具体化；非集合
   初始化器使用结构化参数诊断。
3. `in`/`not in` 对并集逐项匹配；动态左值或动态集合登记 `SetMembership`，不生成
   集合选择计划。
4. `container_checker.rs` 继续把旧式 `int name = {...}` 作为同构约束，并在嵌套路径
   进入集合容器时约束所有已知成员；集合路径仍不能定位具体成员。

### C2-B.4：快照、UseDocs 与交接

1. 新增 `core/rust/crates/xiao-types/tests/c2b_heterogeneous_sets.rs` 和
   `c2b_snapshots.rs`，覆盖类型并集、显式完整声明、动态标记、严格成员判断、旧写法
   兼容和诊断参数。
2. 新增 `tests/spec/05-containers/c2b-valid.json` 与 `c2b-errors.json`；快照阶段标识
   固定为 `03D`，不得改写 C2-A 快照的历史语义。
3. 同步更新 `docs/UseDocs/language/collections/sets.md`、`errors.md`、主题 README、
   模块 registry 和受影响源码/测试目录 README。代码、测试与 UseDocs 必须同一提交。
4. 完成 Rustfmt、workspace 检查、测试、Clippy、Rustdoc、Bun 目录/UseDocs/覆盖率门禁，
   再把本阶段状态更新为已完成。

## 验收命令

```text
cargo fmt --manifest-path core/rust/Cargo.toml --all -- --check
cargo check --manifest-path core/rust/Cargo.toml --workspace
cargo test --manifest-path core/rust/Cargo.toml --workspace
cargo clippy --manifest-path core/rust/Cargo.toml --workspace --all-targets --all-features -- -D warnings
cargo doc --manifest-path core/rust/Cargo.toml --workspace --no-deps
bun run check
bun run check:usedocs
bun run check:coverage
bun test
```

## 后续交接边界

### C2-C：集合运算

先另行冻结 `+` 并集、`&` 交集、`-` 差集、`^` 对称差和比较运算的结果类型、显式类型
冲突及动态传播，再实现 Runtime/IR 计划。本阶段没有伪造这些运算的 AST 计划。

### Runtime/IR

后续阶段负责真实集合存储、元素哈希、增删和运行时类型验证；必须消费本阶段的
`SetType` 语义和 `RuntimeCheckKind`，不能建立第二套元素并集规则。元组递归哈希和
`frozenset` 仍须经过独立规格确认。
