# B4 入口交接

本次只关闭 planned 源码包装与隔离回归。进入 B4 时仍需接类型检查、IR/TAC、VM、动态/静态 LLVM、双路径执行、归档和完整可执行教程；B3-4 的 `closeout.json` 中 `language_available=false`，不可被解释为 cmath 可运行。

15 个包装与冻结原语映射：

| 导出 | 包装参数 | intrinsic |
| --- | --- | --- |
| sqrt | value | 34 |
| floor | value | 35 |
| ceil | value | 36 |
| round | value | 37 |
| trunc | value | 38 |
| abs | value | 39 |
| min | left, right | 40 |
| max | left, right | 41 |
| pow | base, exponent | 42 |
| sin | value | 43 |
| cos | value | 44 |
| tan | value | 45 |
| exp | value | 46 |
| ln | value | 47 |
| log10 | value | 48 |

B4 的向量迁移清单：

- 将正式 MPFR 4.2.2/GMP 6.3.0 17,243 条向量从 `docs/DevDocs/assets/20o-b2-b/entry-20261108-b3/b1/reference-vectors.tsv` 迁至 `tests/fixtures/docs/cmath/reference-vectors.tsv`，迁移前后逐字节同摘要；B3-4 不搬移产品 fixture。
- 建立完整 Xiao 教程 fixture 与 stdout 黄金，覆盖两条导入路径、15 项调用、位置/命名参数、错误/负零和资源边界；加入 B4 精确 VM/native O0/O3 门禁。
- 将 B3 的 30 条 planned 拒绝断言改为与可执行 B4 语料分开的预期结果；不能删除拒绝边界而不加入对应执行证据。
- 继续保持 ABI 1.18/生成器 21 的 B4 内部范围，B5 再切 ABI；B3-4 不升级 registry 或教程状态。
