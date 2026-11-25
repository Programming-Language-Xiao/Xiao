# B3-2 args/cli 实施证据

本目录封存 B3-2A 开工基线。受测产品为 `8d1c09f0`，ABI 1.20、生成器 23、标准库接口 1。B3-1 的三平台 Runtime、工具、作业身份和 Windows COFF `Complete` 来自封存提交 `84660c11`。本机复跑了 B3-1 的只读审计和自反例。[B3-1 收口](../entry-20261124-b3-1/README.md)

args 与纯 Xiao cli 沿用 B3-0 的接口、预算、错误和能力。53～63 当前均为 `reserved`。§3 的实际开工答复待登记，其中启动链路需要在 Bun 转换前捕获原生参数。[实施规划](../../../20o-b3-2-args-cli-implementation-plan.md) · [继承裁定与待答项](decisions.json)

Windows 新任务根执行 Rust 1.96.0 的 `--locked --offline --release` 构建，4 项 time Runtime 测试通过。当前 28 项 B1 复跑使用模拟 Cargo 输出，前批的 4 项实际输出和 24 项派生负例由 B3-1 回执继承。首轮对账误把工作流检出字节与 Git 的 LF 字节比较，原回执保留在首轮任务根。第二轮在新任务根重建，并同时记录实际检出与 Git 字节摘要。日志按内容摘要存入 ZIP，Runtime 原件留在任务根。[基线采集器](baseline.py) · [Windows 复跑包](windows-baseline.zip)

从主仓库运行以下命令复核固定 Git 对象、两个锁文件、冻结夹具、平台报告、Windows 构建记录和定稿文档门禁。

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261125-b3-2/audit.py
```

`closeout.json` 记录 `b3_2a_entry_and_baseline_verified=true`、`b3_2_started=false` 和 `b3_2_passed=false`。[只读审计](audit.py) · [封存脚本](seal.py)
