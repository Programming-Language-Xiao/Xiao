# B2-B B1 私有内核证据

本目录记录 `35872782` 的 B1 独立实施：锁定 `libm 0.2.16`，并在 Runtime `arithmetic` 私有模块中验证 15 项 `cmath` 内核和显式精度除法的商余/一次舍入算法。它没有公开函数、intrinsic、C ABI、ABI 版本、VM/LLVM 接线或标准库变更。

受控执行器 `run-step.py` 为每个命令记录完整参数、工作目录、进程号、环境引用、源码前后摘要、日志摘要和退出码。`b1-runtime-tests` 是 Runtime 全测试，`b1-runtime-clippy`、`b1-runtime-fmt`、`b1-benchmark-check` 和 `b1-docs-all` 是门禁，`b1-dependency-tree` 固定依赖闭包与特性。所有执行都使用 Rust 1.96.0、`--locked`，并保持 `XIAO_B1_F10=0`、`XIAO_B2_A_PERF=0`。

审计命令：

```text
python docs/DevDocs/assets/20o-b2-b/entry-20261108-b1/audit.py
```

审计是只读的，固定使用 Git 对象 `d08b8460`（B0 实际入口）与 `35872782`（B1 代码提交）校验完整产品树范围：`core/rust`、`stdlib`、`tests/fixtures`、`tests/benchmarks`、`.github/workflows`、`.cargo`、`tools`、`cli/ts`、`docs/UseDocs`、registry、`package.json`、`bun.lock` 和 `.gitattributes`。它不读取或要求当前 HEAD。全部检查成功时输出 18/18；同时校验 `.gitattributes`、28/28 守卫、`closeout.json` 和清单篡改反例。`cmath` 继续 `planned`，旧 721 条 F10 不运行，B2-B 公开接线与 B2-C 最终回归后置。

`self-negative.py` 默认只读，在临时副本中分别改写清单、删除 `.gitattributes`。它显式使用 UTF-8 解码子进程输出，并在子进程内设置 UTF-8 模式；退出码必须为 1，完整输出必须等于 `negative-cases.json` 中对应的原因，任何无关异常均视为失败。每次审计前后还比较副本的全部文件摘要，确认成功和拒绝路径均未改写文件。

```text
python -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261108-b1/self-negative.py
```

只有显式传 `--write-result` 才写入真实运行回执；生成后使用 `rebuild-evidence.py` 重建清单，再运行只读审计。`rebuild-closeout.py` 是独立的收口生成命令，审计和自反例都不会调用它。两个审计与自反例均支持 `--root`、`--source-root`，可从主仓库检查历史证据副本。

`cmath.rs` 中的 ULP 向量是 B1 私有底座初检，不构成 F-9 正式误差证据；B3 需要补登记 MPFR 4.2.2/GMP 6.3.0 版本、生成器和摘要的离线参考向量。
