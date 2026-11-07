# B2-B B1 私有内核证据

本目录记录 `35872782` 的 B1 独立实施：锁定 `libm 0.2.16`，并在 Runtime `arithmetic` 私有模块中验证 15 项 `cmath` 内核和显式精度除法的商余/一次舍入算法。它没有公开函数、intrinsic、C ABI、ABI 版本、VM/LLVM 接线或标准库变更。

受控执行器 `run-step.py` 为每个命令记录完整参数、工作目录、进程号、环境引用、源码前后摘要、日志摘要和退出码。`b1-runtime-tests` 是 Runtime 全测试，`b1-runtime-clippy`、`b1-runtime-fmt`、`b1-benchmark-check` 和 `b1-docs-all` 是门禁，`b1-dependency-tree` 固定依赖闭包与特性。所有执行都使用 Rust 1.96.0、`--locked`，并保持 `XIAO_B1_F10=0`、`XIAO_B2_A_PERF=0`。

审计命令：

```text
python docs/DevDocs/assets/20o-b2-b/entry-20261108-b1/audit.py
```

审计是只读的，固定使用 Git 对象 `d08b8460`（B0 实际入口）与 `35872782`（B1 代码提交）校验产品树、锁文件和源码清单，不读取或要求当前 HEAD。输出固定为 18 项通过，并校验 `.gitattributes`、28/28 守卫、`closeout.json` 和清单篡改反例。`self-negative.py` 会在临时副本中改写清单并删除 `.gitattributes`，两次都必须被拒绝。最终文件由 `rebuild-closeout.py` 与 `rebuild-evidence.py` 按实际字节生成；生成后再运行只读审计。`cmath` 继续 `planned`，旧 721 条 F10 不运行，B2-B 公开接线与 B2-C 最终回归后置。
