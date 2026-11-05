# B2-A1依赖与值身份验收

工程期：20O-B2-A1。入口为e06cc88a，代码快照为`39b2c1ea72939b20ea48d096e03b3a515bf81127`。
职责：保存依赖、诊断、接纳/显示/身份及入口恢复的真实验收，不代替A2～A7或B2-A整批交付。

**A1本批已退出，A2～A7尚未开始。** [closeout.json](closeout.json)列出有效验收；新增教程范围和cmath仍planned，所有1.18/20产物仅内部验证。
A0原始失败result、有效closeout和全部旧归档未改写；B1旧721条F10未运行，22D/22C及19/11/22历史债务没有关闭。

## 实现与证据边界

F-7维护策略使用任务级Cargo PATH代理：Rust设置后安装，不更改测试命令形态、目标、时限、并行或28/28守卫；构建/测试/检查强制locked，所有子进程清理RUST_BIGDECIMAL_*。
代理通过参数/环境/退出码自检和Windows真实Cargo冒烟；英文Windows的cp1252安装日志失败已保留，修正为ASCII安全JSON后重新运行，未改数值Rust实现。
早先一次B1 VM执行恰逢这次工具修正，虽测试退出0，执行器因源码变化明确拒绝；该记录不列入有效验收。


F-7六包精确接入；类型层提供共享接纳与借用规范视图，Runtime临时解析BigInt/BigDecimal并显式输出，不调用默认Display/精度/舍入，不保存跨调用共享缓存。
Lint(String)/Lfloat(String)、XiaoValue标签、C符号/布局/状态、XCON v1及intrinsic表保持边界；033用于接纳，034只建立门面，精确除法留A2/A3。
ABI、VM常量与XCON→Runtime入口先建立合法性；Eq/Hash只借读合法值。裸Rust枚举构造属于受信任内部代码，不能借它绕过输入接纳。
无状态返回数值C构造器仍用现有挂起错误通路，LLVM在构造处检查传播；finally示例验证原错误保留。
VM常量恢复拆入constants.rs，保留2500行门槛，不改释放计划或并发排序/断言。

[A1-DICT实际答复](decision-receipts.json)确认语言字典保留str键。
实际Xiao集合用SetHandle验证去重/查询；Rust HashSet/HashMap只验证RuntimeValue的Eq/Hash，不冒充语言数值字典键。
两处定向Clippy expect只说明测试输入为不可变数值，枚举的其他变体含可变句柄；全局`-D warnings`未改。
[registry登记](registry-path-registration.json)只为既有rust.xiao-driver.tests追加已批准的数值夹具路径，全部状态和其他字段不变，未新增模块身份。

## 源码、依赖和产物

[bootstrap.json](bootstrap.json)记录唯一任务根`D:\project\Xiao\.a1-numeric\20261106-d1fc82faa51d`；源码、空Cargo target、空Xiao缓存、临时输出各自隔离，XIAO_STDLIB_ROOT指向本任务stdlib。
[product-identity.json](product-identity.json)记录工具版本、目标、代码提交、Git对象和构建产物；不复用A0 Runtime或可执行文件。

最终验收绑定[source-manifest](source-manifests/783299e533053a6f4c7fbdc250b488cb8fcc68bbd4a3e8c73d7a2918255277b8.json)中明确列出的964个源码输入，不是全仓库或证据文件总数。
原始树摘要为`783299e533053a6f4c7fbdc250b488cb8fcc68bbd4a3e8c73d7a2918255277b8`，规范文本/二进制树摘要为`ba73d1e5e8ec63f9ed48c2db13d2ec1f4044b0881ec3cd5be886b2fada061f5b`。
[run-step.py](run-step.py)在每条有效命令前后采集同一输入并拒绝期间变化，审计逐文件对照固定提交的Git blob。
Runtime静态库SHA-256为`f4a7fbdeb42d4d272cf9a80028de08ed8f3a3d6e28ebe522509f8dabd41fa2c3`；[六个最终原生输出](native-artifacts-final.json)保存路径/大小/摘要，并核对Runtime/诊断/文档适配器回归后未变，二进制仅留任务目录内部验证。

[锁差异](lock-audit.json)只新增六包及Runtime依赖边，既有包逐对象保留；before.txt为入口Git内容的UTF-8/LF快照，不冒称原始CRLF。
先按冻结闭包预填锁，再由真实Cargo --locked核验，不把预填当解析成功。
[最终metadata](metadata-locked.json)与[实际特性](resolved-features-final.json)确认：

| 包 | 精确版本 | 实际特性 |
| --- | --- | --- |
| bigdecimal | 0.4.11 | std |
| num-bigint | 0.4.8 | std |
| num-integer | 0.1.47 | i128、std |
| num-traits | 0.2.19 | i128、std |
| libm | 0.2.16 | arch、default，无unstable |
| autocfg | 1.5.1 | 无可选特性 |

[六份归档](dependency-inputs.json)重新核对冻结SHA-256，[十一份许可证](licenses-rechecked.json)对照上游原文和既有仓库副本。
[环境挑战](environment-filter-challenge.json)只向子进程注入六个无效RUST_BIGDECIMAL_*值；[真实release构建](build-runtime-release.json)记录全部清理且构建通过，宿主环境不改。
Rust/Cargo固定1.96.0；workspace rust-version=1.85不改，不推断未实测MSRV。

## 有效验证

| 组 | 实际执行记录 |
| --- | --- |
| 四个相关包完整测试 | [a1-package-tests-locked.json](a1-package-tests-locked.json)，含类型/诊断/Runtime/VM单元、集成与文档测试 |
| Eq/Hash真实分配器 | [identity-allocations-exact-locked.json](identity-allocations-exact-locked.json)，最大合法文本的调用区间分配0、释放0 |
| 三份完整VM示例 | [a1-vm-exact-locked.json](a1-vm-exact-locked.json)，精确1项，UseDocs与fixture逐字一致 |
| 原生O0/O3 | [a1-native-exact-locked.json](a1-native-exact-locked.json)，精确1项，三份源码各两档，共6次黄金 |
| B1契约/资源 | [9/9](b1-contract-locked.json)、[17/17](b1-resources-locked.json)，保留导入/导出/移除/真实归档/普通包隔离和边界 |
| B1 VM/原生精确 | [VM](b1-vm-exact-locked.json)、[原生](b1-native-exact-locked.json)均唯一1 passed、0 failed、0 ignored |
| B1守卫 | [28/28](b1-guards-locked-report.json)，仅四路径模拟守卫，不冒充平台产品证据 |
| workspace | [Clippy](workspace-clippy-locked.json)、[workspace](workspace-check-locked.json)、[benchmark](benchmarks-check-locked.json)均通过，原阈值/并行策略不变 |
| 文档 | [最终docs/usedocs/layout/覆盖率](documentation-closeout-locked.json)退出0，原有coverage warning保留 |

“零分配”仅指合法Runtime值的Eq/Hash区间，不把构造、ABI恢复或大数操作临时内存写成零。
本批验证输入接纳与身份；W工作值、算术/转换矩阵和性能的完整验收后置，没有B2性能成绩。

## 三平台CI

代码快照的[workspace 42094736157](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/42094736157)与[维护回归42094736112](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/42094736112)均成功。
[run响应](ci-locked-final-run.json)、[jobs响应](ci-locked-final-jobs.json)绑定39b2c1ea，原始in_progress响应保留。

| 执行点 | 实际作业与日志 |
| --- | --- |
| Windows原生 | 127482140348，[A1执行记录](ci-locked-a1-windows.json) |
| Linux原生 | 127482139990，[A1执行记录](ci-locked-a1-linux.json) |
| macOS原生 | 127482140040，[A1执行记录](ci-locked-a1-macos.json) |
| Linux默认workspace | 127482139640，[VM/分配器/恢复测试](ci-locked-a1-linux-default.json) |

三份原生日志各有六条A1 O0/O3成功标记。这些是代码提交证据，不冒充后续纯文档收口提交CI，也不关闭历史22D/22C债务。
即使本批三平台示例通过，新增教程仍planned，完整算术/转换/性能及B2-C最终文档收口尚未完成。

## 复核与开发历史

原始失败全部保留，不作为最终冻结源码的通过证据：首次锁预填误合并同名syn版本，随后保留全部既有版本并--locked通过；源码/字符串分隔符边界修正；ABI测试按既有首错保留规则修正独立场景；定向Clippy说明、目录登记、VM文件拆分、旧锚点保留和拆分后的unused import修正。
[执行器来源](runner-provenance.json)区分开发期与最终执行器，早期日志不追写源码身份。
[提交适配](commit-hook-adapter.json)只原样临时复制worktree提交消息，调用原commit-msg门禁并清理单文件；仓库hook/规则不改。
[边界检查](boundary.json)比较全部Git差异/未跟踪路径并核对保护路径、状态、锁及签字。
[evidence.json](evidence.json)绑定本目录全部实际字节，清单自身不计入自身摘要，文件数以清单为准。

只读复核归档和固定代码快照，不运行产品或改写历史：

```powershell
python -X utf8 docs/DevDocs/assets/20o-b2-a1/entry-20261106/audit.py
```

完整重跑须另建任务目录、检出39b2c1ea，使用新target/缓存/Runtime，按各JSON的实际argv/环境执行；不能在未来HEAD覆盖本归档。
所有Git调用经cmd；解析/构建/测试/检查均--locked，工具版本探测为单列只读记录。
