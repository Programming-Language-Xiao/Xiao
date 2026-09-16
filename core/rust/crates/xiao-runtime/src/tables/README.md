# `xiao-runtime/src/tables`

## 工程期

06-B，依赖 05-C。

## 职责

这里放由 05-C `TableSignature` 驱动的 `[Table]` 单例和 `[[Table]]` 实例、字段访问、
`new`/`init`/`drop` 钩子以及显式构造状态机。只允许静态签名中的成员，不实现动态
反射、继承或跨文件签名推断。
