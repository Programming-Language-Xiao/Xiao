# `xiao-runtime/src/tables`

## 工程期

06-B，依赖 05-C。

## 职责

这里放由 05-C `TableSignature` 驱动的 `[Table]` 单例和 `[[Table]]` 实例、字段访问、
`new`/`init`/`drop` 钩子以及显式构造状态机。只允许静态签名中的成员，不实现动态
反射、继承或跨文件签名推断。

09R2H 通过 `with_initializer` 和 `with_drop_executor` 把研究执行器接入同一状态机，
原函数指针钩子继续可用。已静态检查的字段入口仍校验运行期类型与状态；`TableDropView`
只持有字段数据的弱引用，只在 `Dropping` 可读、不能写入，回调外读取必定失败。
