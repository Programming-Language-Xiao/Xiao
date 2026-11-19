# B6-2 自审补证

B6-1 阶段状态为 `restricted`，`stage_passed=false`。Windows、Linux、macOS 三平台功能通过。Windows COFF 导出完整性未验证，Linux Clippy 未执行。星崽裁定在 macOS 专项通过后启动 B6-2。[Windows/Linux 重跑](../../entry-20261118-b6-1-rerun/README.md)与[macOS 专项](../../entry-20261118-b6-1-macos/README.md)

## 采样源码对账

性能结果及 `perf_pass=true` 绑定 `79ea0375`。以固定 Git 对象对比采样提交、`58176ff5` 和自审入口 `1c4bb4a7`，四处受测源码变化逐字节对账。两个测量器文件分别增加 22 和 28 行 Rustdoc。VM 的 `with_module_loader` 在相同泛型约束下移入 `module_exec.rs`，实现和属性一致。剔除该方法后，两个 VM 文件分别与移位前剩余字节一致。其余受保护源码、两份锁、清单及采集入口对账一致。

从 `1c4bb4a7` 的固定源码在 Windows 独立目录重建 release 核心和测量器，生成的 183 项目录与冻结清单逐项一致。本次是源码、目录及功能复核，新增性能样本数为 0。B6-3 使用自身源码和 Runtime 身份，B6-5、B7 对账时引用这份差异证明并标明实测提交。后续受保护源码或采集入口有变化时，重新核对差异并补测受影响项。[源码与重建回执](source-boundary.json)

## Rust API 的语言边界

相对 B6-2 开工前的 `8167154e`，产品差异仅为 `Vm::with_module_loader` 与 `xiao_driver::standard_library_module_loader` 两个 Rust 宿主接口。CLI、intrinsic、C ABI、官方标准库、registry 和 UseDocs 的 Git 字节一致。

重建后的 `xiao-core` 通过正常协议接收了八条 Xiao 源码请求。两个官方别名的 `abs(-2.0)` 均输出 `2`。两个 Rust 名称的裸调用均以 `X02-TYPE-001` 拒绝，经 `std.cmath` 和 `stdlib.cmath` 的四条导入均以 `X05-MODULE-005` 拒绝。[语言探测结果](source-boundary.json)

## 审计自反例

原样副本退出 0。删对象、改对象字节、换用移位后源码提交并重建清单、删掉 Windows O3 汇总并重建清单，均以退出 1 和预期异常文本拒绝。原始证据与各临时副本的审计前后摘要一致。[退出码与实际异常](negative-cases.json)

在仓库根执行以下命令可复核源码差异和重跑自反例。附加 `--task` 并指定新任务目录可重建核心和复跑语言探测。

```text
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261118-b6-2/review/verify_boundary.py --revision HEAD
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261118-b6-2/review/self_negative.py
```
