# A5 语言全链路实施证据

当前实施代码已固定为`f21715d38069923653438fce8a7f3f8445f49164`，本地功能与门禁通过；macOS首轮被既有22C断言挡住，同提交重跑中，A5尚未声明收口。

## 已取得的实际证据

[checks.json](checks.json)列出17个有效执行点；当前16项已经实际通过，最终文档门禁随收口执行。两个B1精确入口保留全部旧场景，实际--exact且各恰好1 passed/0 failed/0 ignored。A5追加27条VM标记与34条原生标记，和原有28/28计数守卫分别统计。

七份完整语言语料含六类型36个有序提升组合、构造式/as两种36项转换、复合赋值、绑定标签、可恢复错误、线程/任务池与finally。矩阵独立黄金518行，经三个既有VM载体一致；六份.xiaoc及七份真实.xar删除应用源码后重放。两份完整教程与fixture/黄金逐字匹配，新增范围仍planned。

[native-artifacts.json](native-artifacts.json)逐项记录34个实际Windows原生产物的路径/大小/摘要：七份完整语料各O0/O3共14个；六类错误位置共12个；三类临时值清理共6个；定向011共2个。观察器只在测试链接阶段替换既有报告/转换入口，不是新生产ABI或发布产物，也不把受控011当作真实OOM恢复。

[commands.json](commands.json)保留每条实际argv/cwd/PID/时间/退出码、原始日志与执行前后源码身份。失败日志未覆盖；临时值清理、普通赋值/const标签、原生复合赋值与错误位置均有真实反例及修复后回归。[编译期边界回执](constant-error-boundary-receipt.json)保留星崽的实际答复。

## 来源与复核范围

[bootstrap.json](bootstrap.json)固定唯一任务根、入口`bb7a856e`及干净工作树；[environment.json](environment.json)固定任务级target/Runtime/缓存/临时输出，所有Cargo构建/测试/检查均为Rust1.96.0及--locked，子进程清理RUST_BIGDECIMAL_*。TypeScript5.9.3实体复制在源码根内。

[product-identity.json](product-identity.json)固定实际工具、最终release Runtime与仓库stdlib加载根；Runtime内容SHA-256为`027fe22e5636d1447a7b173e7daefd99fa054b17203898cc94737f9fa1394558`。不存在旧B1/A4可执行文件复用。

最终产品输入清单为[source-manifest](source-manifests/2f58be0f0043a3c5c3c5c390bba3b560c82c4c178c6cdc779bbdd8a14712ece9.json)，明列1011项；这是指定输入范围的文件数，不是全库文件数。原始树`2f58be0f0043a3c5c3c5c390bba3b560c82c4c178c6cdc779bbdd8a14712ece9`，UTF-8/LF规范树`9140e710d94af85638df126a1552fc1d35b2b75b328dedf67d0647bb9c006068`。审计用固定Git对象校验，未来HEAD变化不会拿来替代本次被测源码。

完整工作树检查不省略产品、UseDocs、stdlib、.cargo、benchmarks或工具；[boundary.json](boundary.json)另固定实施代码快照的实际差异：

```powershell
cmd /c git diff --name-only bb7a856e --
cmd /c git ls-files --others --exclude-standard
```

源码输入范围包括core/rust、stdlib、tests/fixtures、tests/benchmarks、.github/workflows、.cargo、tools、cli/ts、docs/UseDocs、registry及锁文件。DevDocs滚动证据另列目录清单，不递归计清单自身。

## CI与独立债务

代码工作区门禁[42232824459](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/42232824459)已经通过。维护[42232824476](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/42232824476)首轮其余作业通过，macOS在既有22C队列取消清理断言失败，记录见[independent-22c-record.json](independent-22c-record.json)。首轮的B1原生精确点尚未到达，不能拿standalone的A5执行冒充精确点。

[原始重跑请求](ci-rerun-macos.json)只重跑同一提交失败的macOS作业；[API重新排程](ci-attempt2-scheduling.json)表明其余绿色执行被保留并生成新映射ID。取证同时保存原始执行ID和最新汇总映射ID，以时间、runner与完整步骤证明两者关系；日志从原始ID下载，不能将映射ID或运行ID冒充实际日志来源。重跑即使变绿也不关闭22C/22D及19/11/22债务。

## 最终收口约束

ABI1.18、生成器20、intrinsic最大32、既有C符号/状态/布局、XCON及依赖/锁/stdlib/registry均不变。cmath与新增数值教程范围保持planned，旧721条F10未运行；A6/A7与B2-B/C后置。

最终closeout、evidence清单与审计正反例在三平台证据收齐后生成；当前文件不将等待中的作业写成通过。后续文档提交的CI另登记，不冒称被代码运行覆盖。
