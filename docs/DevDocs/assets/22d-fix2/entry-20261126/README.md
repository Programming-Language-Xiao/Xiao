# 22D-FIX.2 释放差分验收证据

22D 于 2026-11-27 收口，`22d_open=false`。修复产品 `3506455e` 使用 ABI 1.20、生成器 26
和标准库接口 1。工作区门禁所用补充提交
`b302084a` 只增加了测试代码的文档注释。FIX.1 的 `ab92df1` 和 B3-3 的 `2460e75f`
是各自的历史验收点。[实施记录](../../../22d-fix2-release-order-stability-plan.md#十二实施记录)

> **审计口径（2026-11-27 补记）**　2026-11-27 已在固定产品 `3506455e` 的干净 worktree
> 重跑 local F1 100 轮和 Windows 完整/单夹具矩阵，两份矩阵均记录 `source_unchanged=true`。
> 严格 `audit.py` 现已通过；`--legacy-source` 只用于替换前的旧归档。重跑和封存记录见下节。

## 三处失败

| 夹具 | 归因 | 原始证据 |
| --- | --- | --- |
| `native-lock-wait-027` | 取消在锁 ABI 调用前到达，生成代码进入 worker 的错误展开时漏还已经复制的 `gate` 实参 | `original/f1/precheck` 的原生进程以 3 退出，报 027 并打印 `lock-finally`。TSV 记录了两个共享对象的释放动作，其中一个缺 `shared_destroy` 和 `weak_release`。`precheck-cleanup` 在同一时点只加一条释放调用，补回该组终态 |
| `xcon-bounded-string-success` | 原生多出的对象别名被另一对象的事件隔开，旧投影比较了别名次数 | `original/aliases` 的两侧轨迹来自历史第 2 轮，三次销毁逐项一致 |
| `spawn-join-str-multiargument` | VM 多出的对象别名被另一对象的事件隔开 | `original/aliases` 的两侧轨迹来自历史第 4 轮，三次销毁逐项一致 |

F1 的历史远端作业只保留了断言投影。受控证据使用相同源码夹具和入口产品的 Runtime，
通过调整测试生成的 LLVM 控制流指定取消时点。两个共享对象分别来自 `gate` 和 `ready`。
旧版入口单跑 100 轮，两侧每轮都观察到两组终态。取消在进入锁 ABI 后返回时，旧版已有
实参清理路径。修复提交把两条失败路径接入共同的逆序释放块，完整差分同时保留原夹具自然
调度和受控取消回归。Runtime 专项在真实锁阻塞后执行取消，确认 `join` 返回时观测弱句柄
无法升级，最后的弱引用归还恰好一次。[原始远端断言](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/44464422497/job/135358661620)

## 重复执行

Windows 本机修复前和修复后各单跑 F1 100 轮。修复产品另在本机通过完整差分 10 轮以及
三个单夹具各 50 轮。远端按下表执行完整 12/16/1 黄金矩阵及三处单夹具，每轮保留实际
退出码、`22D-CASE`、`22D-TRACE` 和两侧原始轨迹。受控预检查也随每轮完整差分运行。

| 平台 | 完整 | 每个单夹具 | 22D 内部产物验证 | 差分作业 | B3-3 fs 作业 |
| --- | ---: | ---: | --- | --- | --- |
| Windows | 10/10 | 50/50 | `UnverifiedCoffExports` | [140062176193](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45864052545/job/140062176193) | [140062176088](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45864052545/job/140062176088) |
| Linux | 10/10 | 30/30 | `Complete` | [140062175703](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45864052545/job/140062175703) | [140062176596](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45864052545/job/140062176596) |
| macOS | 10/10 | 30/30 | `Complete` | [140062176401](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45864052545/job/140062176401) | [140062176216](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45864052545/job/140062176216) |

B3-3 的排他重命名、VM 教程、原生 O0/O3 教程与权限边界在三平台通过。Windows 六份
fs 样本与对应链接图均达到产品级 COFF `Complete`。各平台均验证了生成器 24、25 的旧
产物，旧版验证器按既有诊断拒绝生成器 26 的新产物。B3-4 的入口产品更新为 `3506455e`。

## 工程门禁

三平台原生维护均在第 1 次执行通过。各平台精确执行 LLVM 汇编夹具、C 布局探针及 29 个
差分场景。22C 的 26 项原回归及 3 项新增回归全部通过，B1 七库的 O0/O3 精确入口均通过。
Windows 本机通过 Runtime 289 项单测、全部 ABI 测试及工作区 195 个测试组。格式、Clippy、
DevDocs、UseDocs、仓库布局与 `git diff --check` 均通过。[三平台原生维护](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45864052346)

初次工作区门禁在新增测试模块的文档覆盖率处失败。补齐代码注释的 `b302084a` 通过新执行，
本机覆盖率为 90.10%，公共 API 覆盖率为 100%。归档保留首次失败和修正后的完整日志。
[初次工作区门禁](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45864052411/job/140062177047) · [修正后工作区门禁](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45864694853/job/140064391789)

## 复核

归档封存了修复前后轨迹、测试日志、三平台作业回执和 fs 验证产物。
状态回执记录固定提交、轮数、实际作业 ID、镜像版本及归档摘要。[原始归档](evidence.zip) · [状态回执](status.json)

```powershell
python -B -X utf8 docs/DevDocs/assets/22d-fix2/entry-20261126/audit.py
python -B -X utf8 docs/DevDocs/assets/22d-fix2/entry-20261126/audit.py --legacy-source
python -B -X utf8 docs/DevDocs/assets/22d-fix2/entry-20261126/self_negative.py
```

严格审计要求每个 local 矩阵记录 `source_unchanged=true`，并把矩阵 `tree` 与固定产品提交对账。
当前 `local/f1` 和 `local/matrix` 已由固定产品 `3506455e` 的干净 worktree 重跑，严格模式通过；
`--legacy-source` 只用于替换前的历史归档。

审计逐轮从原始 TSV 重算生命周期和终态数量，对照差分进程日志、轮次回执和 Runner 镜像。
fs 的六个用例逐项对照原生验证日志、产物摘要、黄金输出和 Windows 链接图；黄金从
固定产品提交读取，不读当前工作树。
反例在临时副本中删除销毁动作、清空一轮 F1 基线的轨迹清单、跨轮引用原始文件、重复
记录 fs 产物或调换 O0/O3 产物及链接图，重算摘要后仍须被拒绝。

Rust 回归分别核对两个打印对象和取消夹具所需的共享对象数量。阻塞锁回归在持锁方收到
worker 的 027 确认后才归还主线程守卫，随后核对 join 后的共享对象状态。
