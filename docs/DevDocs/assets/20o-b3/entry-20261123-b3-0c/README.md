# B3-0C fs、env 与 log 冻结证据

B3-0C 按九项实际答复冻结 fs、env、log 的接口、能力、错误、预算和测量方法。固定产品为 `e79fab3c`，ABI 1.19、生成器 22、标准库接口 1。联合候选为 ABI 1.20、生成器 23、标准库接口 1，具体编号和产物验收身份由 B3-0D 定案。[实施规划](../../../20o-b3-0c-fs-env-log-plan.md) · [前批交接](../entry-20261123-b3-0b/closeout.json)

固定入口核对了 B3-0A 三平台原始探测、B1 的 20 个入口、18 项资源、双精确回归及 28 条守卫。守卫包括 4 份实测输出与 24 条派生负例。B3-0B 只读审计和封存态七条负例在本批原样复跑。Windows 产品 COFF 的要求为 `complete_required`，验收状态为 `pending`。[入口复核回执](baseline.json) · [三平台原始证据](../entry-20261122-b3-0/README.md)

## 宿主观察与冻结边界

| 模块 | 可复核事实 | 冻结边界 |
| --- | --- | --- |
| fs | 三平台宿主样例中同目录替换成功。Windows 样例中符号链接后接 `..` 返回路径不存在，Linux 和 macOS 从链接目标父目录读到标记 | `std.os` 管句柄和进程，`std.fs` 管写入、枚举、删除、重命名和元数据。写入按同目录原子提交，覆盖由调用者指定。首批仅采用普通宿主模式，受限/沙箱模式延期未排期 |
| env | 隔离子进程中可区分空值和缺失，也能识别非法原生编码。样例名称在 Windows 大小写等价，在 Linux 和 macOS 不等价 | 专用 `environment` 授权管理运行快照和局部写删。同一运行的 Xiao 并发任务共享视图，`std.os.spawn` 按宿主规则继承环境 |
| log | 三平台样例在捕获流中得到 48 条完整记录。读端关闭后写入报告 BrokenPipe | 使用 `clock` 与 `stdout`，交付带 UTC 毫秒时间戳的单行记录。输出端接纳和刷新失败由调用报告，后续 CLI 转发失败由 CLI 报告 |

环境和日志的宿主观察使用合成输入与独立流。原子提交、运行快照与输出错误仍需在各模块的产品阶段验证。[B3-0A 原始观察](../entry-20261122-b3-0/entry.json) · [既有 os 契约](../../../../../stdlib/os.md) · [九项裁定](decisions.json) · [冻结契约](contract-draft.md)

## 教程候选

三份完整示例、独立黄金和适用的运行输入保存在 `candidates/tests/fixtures/docs/`。候选已通过本机 Xiao 解析器检查，正文代码和输出与夹具逐字一致，黄金按冻结行为独立核对。VM、原生 O0/O3 精确验收在产品阶段执行，当前为 `planned`。[文件操作](tutorials/fs.md) · [环境变量](tutorials/env.md) · [日志](tutorials/log.md) · [入口矩阵](tutorials.json) · [解析回执](tutorial-check.json)

## 复跑

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261123-b3-0c/audit_baseline.py
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261123-b3-0c/audit_closeout.py
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261123-b3-0c/self_negative.py
```

入口复核报告的是开工时点。收口审计从固定 Git 对象校验裁定、候选字节、定稿文档及原始回执，逐字对照 `closeout.json`。定稿的 DevDocs、UseDocs 树摘要和本批 Markdown 的摘要随三项实际门禁保存在 `doc-gate.json`，相同输出按摘要共用。

本次预检含原样基线和七条负例。第二轮增加伪造预检回执，共八条负例。默认审计按固定 Git 字节核对两轮回执，默认负例入口可复跑八条，含伪造封存回执和遗漏 `fs` 延期项。每条负例核对退出码、完整拒绝文本和审计前后字节。

`--document-commit` 和 `--receipt-commit` 用于采集时对应阶段的证据副本。最初的文档定稿 `aa4baccf`、预检回执 `148c43b7` 和七条负例回执 `bcf6ad0a` 随 `304b8a03` 首次封存，可在其独立检出用默认审计核对固定 Git 字节。当前证据目录直接使用上面的封存态命令复跑。

B3-0D 接收六模块契约与合并版本候选。编号、环境能力位、协议载体、C ABI 登记和产品级 COFF 验收由下一批确定。交接字段 `fs_restricted_mode_deferred_unscheduled` 记录 `fs` 首批的已裁定延期项。实际产品和正式登记仍处于 B2-C 基线。
