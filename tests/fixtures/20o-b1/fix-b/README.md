# B1-FIX-B 指定错误 code 读取

工程期：20O-B1-FIX-B。目录职责：执行真实全局 `error_code`，与七库探测及纯 Xiao 产品分批。
`golden.json` 保存独立 stdout，两个既有精确目标均逐字比较输出；原生还检查退出码零、
空 stderr 和十秒看门狗，各跑 O0/O3。

| 文件 | 断言 |
| --- | --- |
| default-and-custom.xiao | 默认/自定义 Unicode、同类不同码、别名与临时对象；也是 UseDocs 完整示例 |
| nested-rethrow.xiao | 内外 catch、重复读取与重抛不串读 |
| escape-lifetime.xiao | 返回拥有的 str，源销毁并继续分配后仍有效 |
| pending-finally.xiao | finally 中读显式对象不改变原挂起错误 |
| dynamic-types.xiao | 动态非错误值为 002，成员读取仍不开放 |
| long-code.xiao | 2,304 字节/1,024 码点的 code 不截断 |
| abi-requirements.xiao | 借用既有 current_runtime 生成带元数据的真实文件，检查 ABI 1.18 和旧 Runtime 拒绝 |

另外三份源码由 `failure-shim.c` 记录临时引用归还：success-cleanup 为成功复制，
allocation-failure 为 ResourceError / 011，getter-failure 为 MemoryError / 004。
shim 只链接测试可执行文件，保留生成器实际控制流，明确区分受注入测试和普通产品执行；
它不进入 Runtime，不构成生产 ABI。每场景 O0/O3 各一次。

B0/FIX-A/FIX-B 共 21 个普通场景；原生普通运行 28 次，另有清理运行 6 次。
精确 Rust 汇总仍为恰好 `1 passed; 0 failed; 0 ignored`，不把场景数当作测试项数。
默认测试还有静态形态/类型/Fatal、三目标 LLVM 复制顺序、VM 分配失败、三 Carrier
结构化身份和可移植产物版本负例。真实 C getter 的状态/零分配/别名借用期在 Runtime 单测。

范围与证据见 [FIX-B 收口](../../../../docs/DevDocs/20o-b1-fix-b-closeout.md)。
