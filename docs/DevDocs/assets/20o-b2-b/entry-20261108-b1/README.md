# B2-B B1 私有内核证据

本目录记录 `35872782` 的 B1 独立实施：锁定 `libm 0.2.16`，并在 Runtime `arithmetic` 私有模块中验证 15 项 `cmath` 内核和显式精度除法的商余/一次舍入算法。它没有公开函数、intrinsic、C ABI、ABI 版本、VM/LLVM 接线或标准库变更。

受控执行器 `run-step.py` 为每个命令记录完整参数、工作目录、进程号、环境引用、源码前后摘要、日志摘要和退出码。`b1-runtime-tests` 是 Runtime 全测试，`b1-runtime-clippy` 和 `b1-runtime-fmt` 是代码门禁，`b1-dependency-tree` 固定依赖闭包与特性。所有执行都使用 Rust 1.96.0、`--locked`，并保持 `XIAO_B1_F10=0`、`XIAO_B2_A_PERF=0`。

审计命令：

```text
python docs/DevDocs/assets/20o-b2-b/entry-20261108-b1/audit.py
```

审计只接受 B0 提交到 `35872782` 的七个允许路径（含基准 Cargo.lock 同步），且要求其后产品树没有变化；文档证据本身可在后续收口提交中追加。B1 仍是私有内核可用性证据，`cmath` 继续 `planned`，旧 721 条 F10 不运行，B2-B 公开接线与 B2-C 最终回归后置。
