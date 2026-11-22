# B2-C 公共候选与发布证据

被测产品为 `e79fab3c`，树为 `80be41ee`，ABI 1.19、生成器 22、标准库接口 1。本轮 `cmath` 教程源码与独立黄金取自同一份候选，C4 将原字节登记到共享测试夹具。[B2-C 规划](../../../20o-b2-c-final-closeout-plan.md) · [B7 交接](../../20o-b2-b/entry-20261122-b7/README.md)

| 验收项 | 回执 |
| --- | --- |
| B1 七库 F10 | Linux VM 103 条，Windows、Linux、macOS 原生 O0/O3 各 206 条，共 721 条 |
| B2 F-3 | 每平台的 VM、O0、O3 各 183 项，合计 1,647 项与 8,235 个独立样本进程 |
| 语言与数值 | B1 20 入口、18 项资源用例、VM/原生精确回归、4 项实测输出与 24 项派生守卫。三平台核对 A1/A5、cmath、显式除法、浮点比较与 17,243 条 MPFR/GMP 向量 |
| 候选物 | 每平台 5 份程序、10 份 O0/O3 原生可执行文件及 10 份可移植文件。删除应用源码后重放 `.xiaoc` 与 `.xar` |
| 版本与缓存 | 当前验证器的 17 项接受/拒绝和 12 项缓存观察。生成器 22 接纳同 ABI 的 21 原生元数据，21 拒绝 22，缓存按生成器身份失配重生成 |

macOS 性能首轮保留三项失败。F10 的 `statistics.sum` O0 翻倍比 3.25487，F-3 的 VM `lfloat.to-integer` 为 3.12184，O3 `lint.power` 为 5.04523。只重跑三个失败作业，沿用相同源码与阈值。重跑原生 F10 的最大适用翻倍比为 2.96554。归档包含实际作业的 attempt、原始日志和上传对象。[性能运行](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45013071077) · [语言与候选运行](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45016234630)

Windows 的 10 份公共候选以同一次链接映射、PE 地址和导出表核对组件。Linux ELF 与 macOS Mach-O 使用产品的完整验证。Windows 产品观察为 `unverified-coff-exports`，B6 的 `windows_coff_verified=false` 按历史字段保留。B5 的 1.19/21 矩阵与 B1 的 1.18/20 重建来源分开引用。22D 本轮 Windows 通过，历史间歇性释放差分仍为开放。[B5 收口](../../20o-b2-b/entry-20261117-b5-closeout/README.md) · [B6 交接](../../20o-b2-b/entry-20261120-b6-5/README.md)

`execution.zip` 用 SHA-256 保存 916 个路径引用和 653 个唯一对象。C0～C3 的只读审计运行如下。复跑需要取证分支中的固定 Git 对象。

```powershell
cmd /c git fetch origin codex/b2-c-perf-runner codex/b2-c-candidate-runner
python -B -X utf8 docs/DevDocs/assets/20o-b2-c/entry-20261122-b2-c/entry_audit.py
```
