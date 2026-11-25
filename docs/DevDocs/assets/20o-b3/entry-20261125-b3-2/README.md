# B3-2 args/cli 实施证据

本目录封存 B3-2A 开工基线。受测产品为 `8d1c09f0`，ABI 1.20、生成器 23、标准库接口 1。B3-1 的三平台 Runtime、工具、作业身份和 Windows COFF `Complete` 来自封存提交 `84660c11`。本机复跑了 B3-1 的只读审计和自反例。[B3-1 收口](../entry-20261124-b3-1/README.md)

args 与纯 Xiao cli 沿用 B3-0 的接口、预算、错误和能力。53～63 当前均为 `reserved`。五项开工裁定已批准：只激活 53～54、`RunOptions.arguments`+hello、纯 Xiao CLI、正式夹具和 args/cli 产品级 COFF。启动链路仍须在 Bun 转换前捕获原生参数。[实施规划](../../../20o-b3-2-args-cli-implementation-plan.md) · [开工裁定](decisions.json)

Windows 新任务根以 `x86_64-pc-windows-msvc` 为目标执行 Rust 1.96.0 的 `--locked --offline --release` 构建，4 项 time Runtime 测试通过。clang 版本日志报告的默认目标为 `x86_64-w64-windows-gnu`。B3-2 的 Windows args/cli 原生验收要求显式传入 `-target x86_64-pc-windows-msvc`，并记录实际目标及编译、链接命令。[实施规划](../../../20o-b3-2-args-cli-implementation-plan.md)

`closeout.json` 的 `b1_guards.inherited_b3_1` 指向 B3-1 封存回执，登记 4 项实际 Cargo 输出与 24 项派生负例。`b1_guards.b3_2a_windows_replay` 指向本批 Windows 归档，登记 28 项模拟通过、实际 Cargo 输出 0 项。首轮对账误把工作流检出字节与 Git 的 LF 字节比较，原回执保留在首轮任务根。第二轮在新任务根重建，并同时记录实际检出与 Git 字节摘要。日志按内容摘要存入 ZIP，Runtime 原件留在任务根。[基线采集器](baseline.py) · [Windows 复跑包](windows-baseline.zip)

从主仓库运行以下命令复核固定 Git 对象、两个锁文件、冻结夹具、平台报告和 Windows 构建记录。文档门禁回执绑定 DevDocs、UseDocs 和 ABI 说明的 Git 树。守卫回执按脚本及工作流摘要、28 个用例身份和各自退出结果核对。

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261125-b3-2/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261125-b3-2/self_negative.py
```

两条复跑命令均为只读。`self-negative-result.json` 独立归档原样基线与三条反例的实际结果，逐项记录拒绝原因及审计前后摘要。反例覆盖文档树摘要被替换、守卫脚本摘要被替换和守卫用例重复。`seal.py` 在已提交的文档快照上执行三项门禁，在临时副本实跑自反例，审核通过后写入生成件与清单。审计核对自反例回执的定稿文档身份及原样输入摘要，复跑脚本逐字节比对封存回执。`closeout.json` 记录 `b3_2a_entry_and_baseline_verified=true`、`b3_2_started=false` 和 `b3_2_passed=false`。[收口回执](closeout.json) · [只读审计](audit.py) · [自反例](self_negative.py) · [封存脚本](seal.py)
