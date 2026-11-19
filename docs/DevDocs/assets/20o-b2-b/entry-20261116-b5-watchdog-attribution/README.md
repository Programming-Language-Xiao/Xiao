# B5 默认 workspace 看门狗归因复核

本目录追加核验 F1 到 F5，原 B5 重建证据目录及失败回执保持只读。产品、测试、60 秒看门狗、资源输入与预算均未修改。两次完整 workspace 使用同一 Windows 主机、Rust 1.96.0、原锁及相同默认命令，先旧后新执行，没有使用测试串行参数。

| 固定产品 | 退出码 | 总耗时 | URL Unicode 与资源目标 |
| --- | ---: | ---: | --- |
| B5 前 cfa79381，ABI 1.18/生成器 21 | 0 | 986.828 秒 | URL 通过；资源 17 通过、0 失败 |
| B5 最终测试 5864cbde，ABI 1.19/生成器 21 | 101 | 839.625 秒 | URL 报 X09-DRIVER-002；资源 16 通过、1 失败 |

两次的完整命令：

    cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml --workspace --no-fail-fast

完整计数分别为旧快照 187 个目标、1,747 通过、0 失败、54 忽略；B5 为 188 个目标、1,748 通过、1 失败、54 忽略。

旧快照本次默认全量通过，B5 最终快照的空闲复跑仍失败；不能关闭 B5，也不能凭这一组先后对照认定 ABI 导致性能回退。原隔离 56.97 秒及串行通过均不覆盖默认门禁失败。B6 仍未开始。

“空闲”仅指采集器在每次启动前未发现 cargo、rustc、xiao-driver 进程，并顺序执行两次 workspace。没有连续 CPU、内存、温度、调度或其他后台活动测量；两个任务根和构建缓存独立，B4 本次还编译了此前未构建的 debug 测试目标。两份回执分别绑定旧/新源码、锁、受控环境、各自此前冷构建的 Runtime 及日志；本次没有重新冷构建 release Runtime。

run.py 是实际采集脚本，runner 摘要写入两份原始回执。默认重复执行会拒绝覆盖；新采集须另建同级空目录。相同字节只存一个 SHA-256 对象，日志不另存重复副本。audit.py 只读固定 Git 对象和归档字节，不据一次对照推断因果；自反例核对清单、来源、命令和摘要篡改，并确认审计前后字节不变。

## 审核问题处理

- F1：已按 cfa79381 跑默认 workspace，并在同一主机顺序复跑 B5。两份完整输出保留，B5 失败继续阻塞；根因尚未证明，不调整看门狗。
- F2：已建立独立的[静态浮点相等修复规划](../../../20o-b2-static-float-equality-fix-plan.md)，记录最小复现、Float/Sfloat、真实 LLVM 验证、VM 与原生 O0/O3 及生成器边界。产品修复尚未开始。
- F3：B5 开工单、测试条目、证据矩阵与退出条件统一为 4 项真实 B1 Cargo 输出回放加 24 项派生负例，全部是 Windows 回放；没有扩大为三平台作业。
- F4：B4/B4-5 索引及 B4-5 头部明确记录各自退出时点，并链接 B5 当前状态。
- F5：本会话可见问题卡 call_b83bf8422de64f64a9f1e1a8bb88e5b0 第 0 项的实际答复为“批准方案 2：重建并标明来源（推荐）”。沿用该裁定，不补造外部会话链接；B1 历史原件缺口继续保留。

## 复核命令

    python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261116-b5-watchdog-attribution/audit.py
    python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261116-b5-watchdog-attribution/audit.py --self-negative

审计默认只读；清单由定稿后实际字节生成，不手改。seal.py --seal 只允许首次采集；它先要求工具与 DevDocs 已提交定稿，再运行一次完整文档门禁。docs-final.json 绑定全部 DevDocs 正文、本目录稳定输入和工具的前后摘要，随后写入自反例、review.json 和最终文件清单。其实际门禁结果以归档回执为准。
