# FIX-B 维护回归 41310926623 证据

工程期：20O-B1-FIX-B。目录职责：保存提交
`d762ceecb31ef28bc7df4606cc7873f340a5213d` 的实际执行证据，attempt 1。
维护回归六个作业与同提交工作区门禁 41310926525 均成功。

[evidence.json](evidence.json) 记录实际 run/job、head、场景清单、精确汇总行、来源行号，
以及实际下载的完整作业日志字节数和 SHA-256。哈希不针对摘录计算。
摘录去除 ANSI CSI 与行尾空白、统一 LF，保留原始时间戳；完整下载源文件保留原始字节。
远端 [计数守卫摘录](guards.log) 另记录四路径 28/28；其来源摘要与默认作业完整日志相同，
它只证明计数守卫，不代替产品场景。

| 精确成功摘录 | 作业 | 覆盖 |
| --- | --- | --- |
| [默认 VM](default-linux.log) | 124860930687 | B0/FIX-A/FIX-B 各七份，共 21 场景 |
| [Windows 原生](windows.log) | 124860930712 | B0/FIX-A 各七份、FIX-B 七份 O0/O3；另六次清理 |
| [Linux 原生](linux.log) | 124860930671 | 同上 |
| [macOS 原生](macos.log) | 124860930751 | 同上 |

每个精确目标均为恰好 `1 passed; 0 failed; 0 ignored`；原生普通执行 28 次，
成功/getter 失败/复制失败三条清理路径各 O0/O3，共六次。注入程序只链接测试 shim，
与普通生成器产物分开记录。ABI 1.18 三种产物的代表性接受/拒绝与 getter C 状态另由同作业测试覆盖。

此前失败单独保留，不计作通过：

- 41306343104：ABI 夹具的传递 weak 组件漏记，[Linux](first-linux-failure.log) /
  [macOS](first-macos-failure.log)；[Windows](first-windows-failure.log) 为 22D 大字符串释放顺序断言。
- 41308052854：Windows [attempt 1](history-124852437628-failure.log) 的多参数字符串与
  [attempt 2](history-124856538894-failure.log) 的 detached 释放顺序断言。

星崽明确选择“保持 FIX-B，单列 22D 阻塞”。本批未修改 22D 的实现、夹具或比较器；
最终当前运行通过全部原断言，不表示历史不稳定或 22 发布债务已修复。

可重新下载完整 Windows 源日志并按文件字节计算摘要：

```powershell
gh api repos/Programming-Language-Xiao/Xiao/actions/jobs/124860930712/logs > windows.full.log
Get-FileHash -LiteralPath windows.full.log -Algorithm SHA256
```

其余 job ID、原始来源摘要与失败归属见机器证据清单。
本目录验证 FIX-B 前置，不证明七库产品或 CSV 全量扫描性能。
实现边界见 [FIX-B 收口记录](../../../20o-b1-fix-b-closeout.md)。
