# B3-0B time、args 与 cli 冻结记录

B3-0B 按十二项实际答复冻结 time、args、cli，登记三份教程候选。基线为 `e79fab3c`、ABI 1.19、生成器 22、标准库接口 1。B3-0A 已采集三平台宿主观察和 B1 回归，本批复跑其只读审计及四项负例。[实施规划](../../../20o-b3-0b-time-args-cli-plan.md) · [裁定回执](decisions.json) · [前置复跑](preflight.json)

规划提交后，工作分支先纳入独立的推送门禁修复 `9a8f4285`。本批按该父提交对账产品边界。接口候选与补充探测分别固定在 `2ec11203` 和 `06e20efd`。

| 模块 | 冻结范围 |
| --- | --- |
| time | `utc_now`、`monotonic`、`format_utc` 和 `sleep` 首批接受显式 `ms`、`ns`。sleep 最多 60 秒，取消、截止和清理沿冻结通道 |
| args | `program_name` 和 `argv` 读取本次运行的快照，检查 `process` 位。校验原始编码后才进入运行，保留空项、顺序与重复值 |
| cli | `split_terminator` 在首个 `--` 处分组，返回两个新的字符串数组。源码使用现有返回类型推导 |

Windows 本机的 Bun 1.4.0 观察记录了孤立 UTF-16 单元 `D800` 与合法 `FFFD` 进入 TypeScript 后发生同值转换。已冻结的严格校验边界位于这一步之前。Linux/macOS 的 Rust 原始参数观察继续引用 B3-0A。[本机原始回执](preflight.json) · [三平台观察](../entry-20261122-b3-0/README.md)

cli 等价样例复用固定 B2-C 核心和 Runtime。在临时标准库根中通过 `std.text` 和 `stdlib.text` 导入分组函数，两条路径的 Windows VM、原生 O0/O3 以及应用删源后的 `.xiaoc`、`.xar` 都匹配候选教程的独立黄金。证据包含协议帧、实际产物字节及来源。原生探测的 COFF 状态为 `unverified-coff-exports`。[注解与推导探测](cli-shape.zip) · [模块导入与归档探测](cli-import.zip)

错误顺序、稳定身份、资源预算和测量协议已登记。ABI 1.20、生成器 23 和标准库接口 1 是 B3 联合候选。原语编号、协议版本、原生入口上下文传递和 COFF 实现交 B3-0D。Windows 公共候选要求产品验证器报告 `Complete`，声明组件与观察组件相同。[冻结正文](contract-draft.md)

## 教程与验收入口

三份完整示例和独立黄金保持 `planned`。首轮布局门禁要求正式示例目录同时登记与提供 README，本阶段登记表只读。七份文件逐字保存在本目录的 `candidates/tests/fixtures/docs/`，产品接入时按相同相对路径进入正式目录。VM 与受控原生 O0/O3 精确入口已提出。[时间](tutorials/time.md) · [参数](tutorials/args.md) · [分隔](tutorials/cli.md) · [入口矩阵](tutorials.json) · [首次布局回执](layout-first.json)

## 复跑

收口审计核对固定 Git 对象、受检提交的 DevDocs/UseDocs 树摘要和原始回执。自反例覆盖清单改写、缺文件、伪造状态与观察、方向冒充冻结、替换受检 Markdown，以及改写负例回执。脚本逐字核对拒绝文本与审计前后摘要。[收口回执](closeout.json) · [文件清单](evidence.json) · [文档门禁](doc-gate.json) · [自反例原始回执](self-negative-result.json)

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261123-b3-0b/audit_closeout.py
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261123-b3-0b/self_negative.py
```

B3-0C 入口就绪，下一批接收 `fs`、`env`、`log` 的契约工作。
