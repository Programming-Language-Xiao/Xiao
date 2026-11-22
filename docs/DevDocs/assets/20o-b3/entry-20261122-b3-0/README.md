# B3-0A 入口与宿主探测

固定产品 `e79fab3c`、源码树 `80be41ee`、ABI 1.19、生成器 22、标准库接口 1。两个 Cargo.lock、1,222 个产品路径和继承状态由固定 Git 对象复核。B2-C 只读审计在恢复其原版 README 后通过 19 项。[B3-0 规划](../../../20o-b3-0-probe-freeze-plan.md) · [B2-C 收口](../../20o-b2-c/entry-20261122-b2-c/README.md)

## 实际执行范围

Windows 本机、Linux Runner 与 macOS Runner 使用 Rust 1.96.0，分别在独立任务根锁定构建 release Runtime。原始回执保存命令、源码及 Runtime 前后摘要、LLVM/链接器、目标三元组和输出。三平台均通过 B1 七库 20 入口、18 项资源、VM/原生精确点；28 个计数守卫使用本批 4 份实际 Cargo 输出及 24 项派生负例。[探测运行 45033610157](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45033610157)

| 模块 | 探测证据 | 需要纳入契约的差异 |
| --- | --- | --- |
| time | UTC 纪元采样；10,000 次单调钟取样；0/1/10 毫秒各 5 次；宿主可唤醒等待和超时 | Windows/Linux/macOS 最小观察增量分别为 100/49/41 纳秒。这是主机观察值。macOS 10 毫秒请求最长约 40.4 毫秒。调度误差与 Xiao 取消传播由后续契约和测试处理 |
| args | 子进程检查 argv[0]、空项、双引号、末尾反斜杠、`--` 和合成非法 Unicode | 三平台按项传递均保真。Rust 只在验证前保留原生字符串，Xiao 的输入来源与拒绝边界待裁定 |
| cli | 用固定提交的实际 CLI 解析器解析常规运行和三条分隔符样例 | 三平台的 `run probe.xiao -- --help` 都触发工具帮助，另外两条带分隔符的运行请求报 `X11-CLI-ARG-001`。用户参数透传需要后续接入 |
| fs | 临时目录内排他创建、同目录替换、失败后的源文件、目录符号链接、只读文件 | 三平台同目录替换成功，覆盖目录失败并保留临时源；Windows 的符号链接后接 `..` 返回路径不存在，Linux/macOS 读到链接目标父目录中的标记 |
| env | 子进程内仅测试合成名称和值，核对缺失、空串、删除和非法 Unicode | Windows 对样例名大小写不敏感，Linux/macOS 区分。空串存在与名称缺失可区分 |
| log | 4 个线程共 48 条锁内写入、独立 stdout/stderr、关闭读取端写入 | 三平台记录完整，关闭管道返回 BrokenPipe。缓冲、刷新、权限和用户可见错误依实际裁定 |

这些观察取自独立 Rust 探测程序与现有 CLI 源码。B3-0A 的结果用于准备六模块契约。Windows 公共产物必须由 Xiao 产品验证器报告 COFF `Complete` 且声明组件等于观察组件。[Windows 验收政策](../../../20o-b3-host-libraries-plan.md)

取证分支附带触发的仓库门禁缺该观察用例的 LLVM 环境，`b6_diagnoses_linux_native_cmath_runtime_component_observation` 在读取 `XIAO_CLANG` 时失败，原日志随包封存。[辅助运行 45033610101](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45033610101)

## 复跑与下一步

`observations.zip` 以 SHA-256 引用原始命令、日志与 CI 上传包。审计复核固定工作流、探测配方、作业身份、所有路径和当前入口结论。裁定台账记录本次入口时点，后续答复另行登记。[入口回执](entry.json) · [清单](evidence.json) · [裁定台账](decisions.md)

本提交的 docs、UseDocs 和布局检查为本机检查；最终封存文档门禁在 B3-0F 执行，不由本提交的局部检查代替。

## 自反例

`self_negative.py` 在临时副本上验证四个篡改样例：清单改写、缺 `.gitattributes`、伪造 `b3_0_passed` 和伪造 `decisions_approved`；审计或派生检查必须按对应理由拒绝，且原件只读。结果见 [self-negative-result.json](self-negative-result.json)。

```powershell
cmd /c git fetch origin codex/b3-0-probe-runner
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261122-b3-0/audit_entry.py
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261122-b3-0/self_negative.py
```

B3-0B 的时间单位、参数来源和 cli 范围已列为待答。六模块精确契约、B3-0 收口和 B3-1 入口仍待实际裁定。
