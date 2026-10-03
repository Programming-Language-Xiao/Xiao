# `xiao-intrinsics`

## 目录职责

保存 Xiao 内置函数的后端无关契约：稳定 `IntrinsicId`、来源名称、类别、签名、能力、效果和
VM/Runtime ABI 绑定键。声明数据位于 `intrinsics.json`，`build.rs` 在构建期校验并生成只读表。

## 依赖边界

该 crate 位于类型层、IR、字节码、VM、LLVM 和 Runtime 的共同下层，不依赖任何实现层，也不
暴露 Rust 函数指针、Runtime 对象或 LLVM 类型。机器分派只消费 `IntrinsicId`；名称仅供前端
适配和诊断。

## 工程期

20AB：建立 intrinsic 契约、稳定 ID 和声明生成；后续批次扩展标准库和平台资源入口。

## 稳定性

`IntrinsicId` 固定为 `u32`，0 永远无效；已发布 ID 不复用，删除项保留 tombstone。新增声明
必须同时满足生成期和运行期契约校验，并由各消费者的移除验证覆盖。
