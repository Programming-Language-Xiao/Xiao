# B1 测量与诊断工具

工程期：20O-B1-IMP-REM-C。职责：保存显式调用的开发期验证程序，使用实际前端、VM 与原生构建器。
边界：不进入 xiao-core、标准库导出、intrinsic 或生产 ABI；不创建无人维护的 ignored 测试。

`b1_library_performance.rs` 对七库 20 个入口的 31 种负载测量最大输入、三档翻倍、原生 O0/O3
与进程内存保守上界。运行前先编译 release 工具；VM 与原生计时串行执行，不与 workspace 编译混跑。
短调用迭代校准到 100ms，随后一次预热和五次采样，每批至少 50ms；所有校准/原始样本均写入 JSON。
原生使用已有受控工具链和真实 std 导入，VM 使用整个未改产品源码与生产 StackCarrier。
黄金验证、输入准备、编译和终端 I/O 在窗口外；少量标记格式化/清理计入时间上界。

```text
cargo run --manifest-path core/rust/Cargo.toml --release -p xiao-driver --example b1_library_performance -- --backend vm --case all --phase all --output b1-f10-vm.json
cargo run --manifest-path core/rust/Cargo.toml --release -p xiao-driver --example b1_library_performance -- --backend native --case all --phase all --output b1-f10-native.json
```

`--list` 列出负载，`--case` 可选单项，`--phase` 可选 maximum/scaling/all。标量无规模翻倍。
报告包含产品摘要、实际规模、校准批次、预热和五次原始样本；`complete=false` 是未完成队列，
任一预算/黄金/看门狗失败不能登记为通过。维护回归手动输入 b1_f10=true 才执行完整测量，
不会把显式 F10 混入默认测试；既有 fmt 默认性能测试继续保留。

`b1_release_diagnostic.rs` 用同一字符串完整/成功源分别记录 VM 和原生 O0/O3 的 stdout、
退出码、对象销毁数与原始 trace。差异存在时进程返回 1，保留全部产物；不放宽既有断言。
运行需要与原有原生精确测试相同的 XIAO_CLANG/LLVM_AS/LLC/TARGET_TRIPLE/RUNTIME_LIBRARY。
