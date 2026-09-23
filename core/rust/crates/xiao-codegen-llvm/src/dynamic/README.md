# `xiao-codegen-llvm/src/dynamic`

## 工程期

本目录承载 N0-B 动态值、容器、表和正常退出释放计划的 LLVM 降低实现。它只消费已经
验证的 `xiao-ir`，通过 `xiao-runtime-abi` 的固定 `%xiao.value` 形状发射 Runtime 调用；
不改变类型规则、所有权计划或 Runtime ABI，也不负责 N0-C 的统一异常展开。

## 模块职责

```text
dynamic.rs          门面：生成器状态、生成流程和底层发射原语
predicate.rs        纯 IR 谓词：Runtime/容器使用判断、字段标签和名称键
text.rs             动态路径文本解析、字符串转义和 Cast 安全判断
runtime_abi.rs      Runtime 声明、COFF 间接返回和 ABI 调用适配
entry.rs            xiao_entry、C main 适配器和入口观察值
slot.rs             槽收集、表字段初始化、作用域边界和槽读写
release.rs          临时值释放、所有权释放计划和退出边
control.rs          顶层语句、条件、if/elif/else、while、break/continue
expression.rs       表达式、字面量、字符串和表字段读写
container.rs        数组/元组、字典、集合、表构造和表描述符
```

## 依赖边界

`dynamic.rs` 是稳定门面，保存 `DynamicGenerator` 的状态和生成顺序；各职责模块通过
`impl DynamicGenerator` 扩展实现，不能把实现重新塞回门面。`predicate.rs` 和 `text.rs`
保持无状态，容器/表达式模块只能消费它们的纯辅助；其他子模块可以访问门面提供的内部
状态、常量和发射原语，但不得互相形成控制流、表达式、容器、释放或 ABI 的反向依赖。

静态降低器位于同级 `ir.rs`，只依赖 crate 级 `text.rs` 的公共纯函数，禁止反向依赖
`dynamic.rs` 或本目录。crate 级 `text.rs` 是 `escape_llvm` 和 `stable_hash` 的唯一来源，
动态文本解析只保留在本目录的 `text.rs`。

## 验收

`dynamic_architecture_tests.rs` 在测试构建时锁定门面模块登记、静态/动态依赖方向和
禁止的兄弟模块导入；`tests/n0_a.rs` 与 `tests/n0_b_dynamic.rs` 继续作为 LLVM 文本、
Runtime ABI 和静态/动态边界的行为证据。拆分只移动实现，不升 `CODEGEN_VERSION`，生成的
LLVM 文本和 Runtime 组件清单必须保持不变。
