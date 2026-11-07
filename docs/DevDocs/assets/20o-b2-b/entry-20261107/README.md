# B2-B B0 基线记录

状态：B0 已完成、B1 未开始。本行记录 B0 退出时点；后续进度见[B0/B1 收口记录](../../../20o-b2-b0-b1-closeout.md)。B0 实际入口为 `d08b8460`；规划提交为 `e596cea8`，产品基线为 A6 的 `e7bf7a49`。

全新任务根、target、Runtime、Xiao 缓存、临时目录和仓库 stdlib 已固定。审计现在强制检查这些路径都位于任务根，`XIAO_RUNTIME_LIBRARY` 必须存在并与 closeout 的字节数和摘要一致，`fresh_target`/`fresh_xiao_home` 必须为 true。B1 契约 9、资源 17、B2-A 资源 3+8、语言 12/1 ignored、Driver 64/1 ignored、B1/A1 双精确、workspace/benchmark/clippy/fmt、docs/usedocs/layout/coverage 和 28/28 全部通过。双精确入口使用 `--exact`，恰好 `1 passed / 0 failed / 0 ignored`。

B1 F10、B2-A 性能开关和旧 721 条均未运行。ABI 1.18、生成器 20、活动 intrinsic 最大 32、33～48 未激活、100 墓碑、`stdlib.cmath` planned、七库 registry verified、Cargo.lock 和产品树相对 A6 未变。

B0 不写产品、不改锁、ABI、intrinsic、registry 或工作流。下一入口是 B1 私有 libm 依赖/共享内核实施；公开入口、C ABI、ABI 1.19/21 和 cmath 仍后置。

只读复核命令（在仓库根目录执行）：

```text
python -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261107/audit.py
```

脚本固定读取 `d08b8460` 与 A7 的 Git 对象，不要求当前 HEAD 等于被测提交，不写 `closeout.json` 或 `evidence.json`。支持 `--root` 指定证据副本、`--source-root` 指定 Git 仓库。成功结果为 18/18；文件数按最终清单实际核对。

修订归档文件后，显式重建清单，再运行只读复核：

```text
python -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261108-b1/rebuild-evidence.py --root docs/DevDocs/assets/20o-b2-b/entry-20261107
```
