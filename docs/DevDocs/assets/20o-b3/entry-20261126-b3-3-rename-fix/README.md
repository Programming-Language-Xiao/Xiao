# B3-3 排他重命名自审复验

Windows 的 `MoveFileExW` 与 macOS 的 `renamex_np` 对同路径的无覆盖移动可能返回成功。固定产品 `2460e75f` 在这两个平台提交前读取目标末段。目标存在时返回 `FS_EXISTS`，包括同路径、等价路径拼写和悬空链接。遇到缺失的同路径源时返回 `FS_IO_FAILED`。宿主原子排他移动处理检查后出现的目标。三平台专项覆盖这些用例以及原有 VM、原生 O0/O3 教程和权限矩阵。Windows 六份原生样本获得 COFF `Complete`。[原收口](../entry-20261125-b3-3-final/README.md) · [修复专项](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45807842820)

第一轮修复 `cbd7f190` 的 macOS 测试复现了同路径返回成功，原始失败日志与 CI 作业身份在 `runs.zip` 内。修复快照的工作区及维护门禁使用相同源码，Linux 对账覆盖 176 个 Runtime 导出。文档门禁回执绑定定稿 DevDocs 与 UseDocs 的 Git 树摘要。[首轮 macOS 回执](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45806308281/job/139862395380) · [维护门禁](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45807842760) · [工作区门禁](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45808384106)

Windows 维护作业首轮在 `xcon-bounded-string-success` 的对象释放顺序断言失败，第二次尝试在同一源码上通过。失败原始日志和两次作业回执都在归档内，22D 的间歇性释放差分仍待归因。[首轮作业](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45807842760/job/139867799304) · [重跑作业](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45807842760/job/139878903980)

B3-4 从 `2460e75f` 的 ABI 1.20、生成器 25、标准库接口 1 接续，机器可读交接见 `status.json`。

## 复跑

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261126-b3-3-rename-fix/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261126-b3-3-rename-fix/self_negative.py
```

审计核对固定产品与原封存清单、日志摘要、两条新增回归标记、18 份原生样本及 CI 身份。反例在临时副本中删除同路径回执并重建清单，再检查拒绝文本、退出码和审计前后字节。[采集脚本](collect.py)
