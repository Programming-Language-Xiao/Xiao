# B2冻结准备的入口资料

工程期：20O-B2-FREEZE资料准备。职责：保存既有基线、交接P3处理及候选依赖来源。
边界：没有写B2产品、intrinsic、ABI、依赖锁或测试实现；以下结果不能当成B2高精度功能验收。

[清单](evidence.json)记录465e213d入口和87bac439代码基线，实际代码仍为ABI1.18、生成器20。

| 本机Windows现有检查 | 通过数 | 原始执行的规范化日志 |
| --- | --- | --- |
| 数值类型 | 2 | [types-numeric.log](types-numeric.log) |
| 转换类型 | 1 | [types-conversion.log](types-conversion.log) |
| Runtime运算/现有拒绝边界 | 15 | [runtime-ops.log](runtime-ops.log) |
| B1契约矩阵，覆盖20入口 | 9 | [b1-contract.log](b1-contract.log) |
| B1 VM精确目标 | 恰好1 passed/0 failed/0 ignored | [b1-vm-exact.log](b1-vm-exact.log) |

新增cmath planned登记后，另通过两个既有精确契约目标：
[实现矩阵](planned-registry-active-matrix.log)与[教程一致性](planned-registry-tutorials.log)各恰好1 passed。
文档、教程、目录和覆盖率门禁均通过，命令、范围及结果见[检查摘要](documentation-checks.json)。

日志统一转换为UTF-8/LF并剥除ANSI控制序列，清单的sha256指归档日志；raw_log_sha256仅标识原始采集日志。
源码另列test_source与test_source_git_sha256，后者按87bac439的Git文件内容计算，不与日志摘要混用。

87bac439的工作区门禁41585868754、实际作业125751076106成功，已[补登原证据链](../../20o-b1-close/review-fix-20261103/ci-87bac439.json)。
它是文档/审计修复节点，B1被测产品快照不变；没有据此宣称B2或新增三平台实现已通过。

此前两次validate-audit.ps1比较是在cmath planned登记前、Git HEAD为465e213d的未提交工作树中采集的；
当时B1被审文件仍等于106b99c7快照。10份产物逐字节一致，两份含随机路径的诊断不同，
见[历史比较清单](replay-comparison.json)。旧结果不证明当前HEAD的默认命令可运行；
两份诊断原文保留，语义判定仍依赖真实Exception.Message、异常类型、两个反例与validation.json。

新增planned登记后，无SourceRoot的两个默认命令均预期拒绝当前registry。
当前复跑必须显式导出106b99c7并传入SourceRoot；完整命令见[B1复跑入口](../../20o-b1-close/README.md#只读复跑)，
新增透传后的两次执行、实际SourceRoot、导出命令、进程输出及原始归档未改证明见[本轮复跑实证](../../20o-b1-close/source-root-replay-20261103/README.md)。
旧[历史审计摘要](historical-source-audit.json)缺少当时完整来源记录，仅保留历史身份；不补造旧命令或把它冒充本轮输出。

[候选依赖资料](candidate-dependencies.json)包含上游发布提交、Cargo清单的内容摘要、注册索引公布的归档校验值和依赖要求。
num-bigint=0.4.8与bigdecimal=0.4.11、仅std和双许可留档已确认；其余候选版本/闭包与构建策略仍待审。
未核验crate归档或三平台构建，Cargo.toml/Cargo.lock未改。
bigdecimal的默认精度/舍入/显示可受构建环境改变，该事实必须纳入冻结，不能照搬为Xiao语义。

裁定与未决项见[冻结草案](../../../20o-b2-freeze-draft.md)。
