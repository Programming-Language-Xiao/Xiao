# `xiao-runtime-abi`

## 目录职责

本 crate 保存 Xiao 原生程序与 Rust Runtime 之间的稳定 C ABI 契约。公开层只使用固定宽度
标量、长度参数和不透明句柄，不暴露 `xiao-runtime` 的 Rust 枚举、引用计数对象或容器布局。

## 工程期

10B 已接入 tagged value、强/弱句柄、字符串、数组、元组、字典、集合和表描述符入口。
20AB 新增 `xiao_runtime_print_values` 与 `xiao_runtime_input` 输出/输入入口，ABI 主版本保持为 1，次版本为 8；改变布局或所有权契约必须升主版本。
异常对象与异常展开仍属于 N0-C。

## 允许与禁止依赖

当前不依赖任何工作区 crate，也不反向依赖 LLVM 后端。后续 Runtime 实现可以实现这些 ABI
入口，但不得把内部布局写入本 crate 的公开类型。

10T 新增独立 `XiaoTableDescriptorV2`、注册式方法回调与值指针字段入口，旧描述符仍为 40 字节。
新描述符 88 字节、方法项 56 字节；回调返回 i32 状态，receiver/参数借用、结果指针转移所有权。
