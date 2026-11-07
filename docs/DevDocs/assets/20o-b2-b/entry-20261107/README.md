# B2-B B0 基线记录

B0 只读基线已完成。实际被测 HEAD 为 `d08b8460`；规划页原写 `e596cea8`，该提交只是前一版 DevDocs 规划，产品输入没有漂移。产品基线为 A6 的 `e7bf7a49`。

全新任务根、target、Runtime、Xiao 缓存、临时目录和仓库 stdlib 已固定。B1 契约 9、资源 17、B2-A 资源 3+8、语言 12/1 ignored、Driver 64/1 ignored、B1/A1 双精确、workspace/benchmark/clippy/fmt、docs/usedocs/layout/coverage 和 28/28 全部通过。双精确入口使用 `--exact`，恰好 `1 passed / 0 failed / 0 ignored`。

B1 F10、B2-A 性能开关和旧 721 条均未运行。ABI 1.18、生成器 20、活动 intrinsic 最大 32、33～48 未激活、100 墓碑、`stdlib.cmath` planned、七库 registry verified、Cargo.lock 和产品树相对 A6 未变。

B0 不写产品、不改锁、ABI、intrinsic、registry 或工作流。下一入口是 B1 私有 libm 依赖/共享内核实施；公开入口、C ABI、ABI 1.19/21 和 cmath 仍后置。

B0实际收口：18项执行点与28/28守卫全部通过；实际HEAD为d08b8460，规划页旧e596cea8只作历史规划提交。B1私有依赖/内核尚未开始，不改公开入口、ABI、intrinsic、registry或锁文件。
