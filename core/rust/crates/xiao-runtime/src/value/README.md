# `xiao-runtime/src/value`

这里放 Runtime 标量、`str` 堆对象和统一 `RuntimeValue`。固定宽度标量保持内联，
字符串通过不透明强句柄管理。

`mod.rs` 只定义值的表示和读取；算术、比较、相等和显式转换统一放在 `ops.rs`，
它是 Runtime 唯一实现算子的地方，字节码 VM 和原生后端都经这里取值语义。

对应工程期 06-B 建立值表示，09R2 补齐算子矩阵与容器变体。当前已实现同宽度数值
的 `+ - * / // % **`、四个比较、语言级相等、四个冻结拼写的 `str -> bool`、
`bool -> str`、`int`/`sint`/`lint -> str` 和数值宽度转换；跨宽度运算一律报错，要求后端先插入显式转换。

数组、元组、字典表、字典列和集合作为五个可拥有变体接入，句柄由 `containers`
模块提供；它们按对象身份相等，`Eq`/`Hash` 与相等口径自洽。R2C 另加入
`RuntimeValue::Error`：错误对象按 `XiaoError::error_id` 保留身份相等，明确不可哈希，
并以 `error` 作为稳定类型名，供 VM 的 catch/raise 路由消费。`lint` / `lfloat`
算术仍是登记在案的债项，不得在这里给出近似结果。

20O-B1-FIX-B 的 `error_code` 借读错误 code，经 `StringHandle::new` 复制为拥有的 str。
字符串构造仅在测试配置下检查一次性分配 failpoint，失败沿用 ResourceError / 011。

20O-B1-IMP 的独立字符串成本前置在构造非 ASCII 字符串时建立不可变码点起点索引；
ASCII 无额外数组。`len`、`character_at` 为常量时间，VM 下标与原生迭代/下标共用该读取，
不再逐项扫描前缀；Unicode 索引缓冲按实际容量计入 Runtime 对象内存，分配失败仍为
ResourceError / 011。不增加 C ABI、intrinsic 或语言语义。`string_index_tests.rs` 核对
Unicode 边界、越界、共享生命周期和拥有字节计数。

20D 追加 `RuntimeValue::ExternalResource`：首批种类为 `file`/`directory`，句柄复制共享同一
逻辑资源，显式关闭幂等且关闭后的资源操作使用 `X06-RUNTIME-025`；资源不参与哈希，
`type_name` 与 `display_text` 共用 `resource<kind>` 形式。
