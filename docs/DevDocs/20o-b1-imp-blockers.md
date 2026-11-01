# B1-IMP 实测前置与暂停记录

> 状态：七库实现未完成。math/statistics/fmt 共 12 个入口有纯 Xiao 草稿，
> hex/base64/url/csv 尚未实现；按 F10 未达时停止报告的要求，暂停后续产品开发。
> 教程与七库 registry 条目均保持 planned，不把草稿或局部通过写成整批验收完成。

当前三模块的六项契约回归通过（19.79 秒）：VM/导出、std 别名、逐函数移除、真实归档、
普通包隔离与教程一致性。原始记录在 [本机证据目录](assets/20o-b1-imp/local-20261102/README.md)。

## 一、已授权并独立交付的 std 别名前置

星崽已确认独立修复既有 std 别名接线。提交
`4011060f6b4a0a42cc3c55a0e1e485b93cfbac49` 已推送；维护回归
[41331047226](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41331047226) 与工作区门禁
[41331047202](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41331047202) 成功。
VM、LLVM 与归档共用规范身份，ABI 保持 1.18，生成器版本为 16。

## 二、F12 命名参数的原生接线缺口

math 产品在位置调用时已经过本机 VM 与原生 O0/O3；加入已冻结的命名调用后，
VM 继续通过，原生在 `abs(value = ...)` 以 Unsupported 拒绝：

```text
IMP math O0: 原生后端不支持 动态函数 abs 的参数形态
```

实际拒绝来自 `xiao-codegen-llvm/src/dynamic/expression.rs` 的 emit_function_call：
当前只接受 positional/name=None 的实参。F12 契约不变，失败用例保留，
没有删除命名调用换取绿色。是否允许独立补齐固定参数命名调用降低已提交星崽裁定，
答复前不修改后端，不扩展默认值、可变参数或语法、intrinsic、ABI。

## 三、fmt 最大合法输入的 VM 初测

平台：Windows x86_64-pc-windows-msvc，AMD Ryzen 9 9955HX，16 核/32 线程，
rustc 1.96.0 (ac68faa20 2026-05-25)。工作区保持 jobs=4、incremental=false、
RUST_TEST_THREADS=4。

测试读取真实 `stdlib/fmt.xiao` 正文，只在测试编译单元附加 wrapper。
输入为已准备的 65,536 个 ASCII x，调用 `pad_left(text, 0, ".")`，结果应逐字保留原文。
计时从测试 wrapper 的 FunctionEntered 到 FunctionReturned，包含真实产品调用和绑定，
不包含编译、VM 初始化、输入构造、打印和黄金比较；载体与生产 VM 相同，使用 StackCarrier。

debug Rust VM 的首次初测和同源码复核均超过 60 秒测量看门狗；最新测试总耗时 61.39 秒。
这不是完整的五次中位数证据，不能宣称 F10 达标，也没有把总测试耗时当作调用精确耗时。
release 构建的串行复核中，最大输入五次中位数为 1.4593852 秒；4K/8K/16K 的
固定批次翻倍比为 2.0654 / 2.6285，时间初测过线。原始样本、完整日志与源码摘要见
[本机机器清单](assets/20o-b1-imp/local-20261102/evidence.json)。这仍只覆盖该例 VM 时间。

现有字符串遍历需要进一步核对：StringHandle::len 调用 chars().count()，
VM 字符串 index_step 和原生 value_iter_get 用 chars().nth(index)。循环遍历会重复扫描前缀，
单靠产品源码中的 for 不能证明实际成本线性；本批尚未擅自修改这些实现。
是否允许单独优化既有字符串读取也已提交星崽裁定；原生时间、峰值内存和其余入口尚未测全。

复跑初测：

```text
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_imp_vm_performance -- fmt_maximum_input_has_bounded_vm_call_time --exact --nocapture --test-threads=1
cargo test --manifest-path core/rust/Cargo.toml --release -p xiao-driver --test 20o_b1_imp_vm_performance -- --nocapture --test-threads=1
```

不会缩小 65,536 字节上限、放宽 VM 10 秒预算、删除测试或新增 intrinsic 规避问题。
原生 F12 与 F10 之外，剩余四库和整批资源/性能、三平台证据、CLOSE 都未完成。
