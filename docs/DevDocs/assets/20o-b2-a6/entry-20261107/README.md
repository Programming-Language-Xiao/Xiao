# A6资源与受控性能证据

A6本批验收已完成，固定代码快照为`e7bf7a49ba3ed55679b69c97ac1bd5ce7f5777b6`；有效退出由`closeout.json`和只读审计共同核对。入口为`d4ae7877`，A5产品基线为`f21715d3`。A7和B2-B/C未开始，新增数值教程与cmath继续planned。

`bootstrap.json`固定独立任务目录与干净入口；`environment.json`保存受控子进程的target、Runtime、Xiao缓存、临时目录、仓库stdlib和MSVC环境。Cargo固定1.96.0与--locked，每次清理继承的RUST_BIGDECIMAL_*；代理仅为GitHub取证子进程沿用既有Git HTTP代理，不修改宿主设置。

## 可复核入口

以下命令在Windows仓库根执行，只读取固定Git对象、JSON、原始日志与ZIP；不会运行归档中的二进制。后续HEAD变动不作为A6旧快照失败理由。

```powershell
python -X utf8 docs/DevDocs/assets/20o-b2-a6/entry-20261107/audit.py
```

`validate-audit.py`在临时副本中重建清单，再逐项破坏状态、Cargo参数、精确计数、实际作业ID和平台/产物绑定。它检查实际stderr全文与退出码，避免因任意异常或源码中出现预期文本而假通过。已完成的1正/7反实跑记录在`self-review-20261107/negative-cases.json`，独立报告另有8种反例。复跑必须使用归档之外的新输出目录，避免结果日志改变历史清单：

```powershell
$a6ReviewOutput = Join-Path ([System.IO.Path]::GetTempPath()) ("xiao-a6-audit-" + [guid]::NewGuid().ToString("N"))
python -X utf8 docs/DevDocs/assets/20o-b2-a6/entry-20261107/validate-audit.py --output $a6ReviewOutput
```

归档清单为`evidence.json`，其精确文件集合、字节数与SHA-256不递归包含自身或Python缓存。文件数量以该清单为准，不以提交说明中的估算代替。

## 来源与范围

`run-step.py`保存每条实际argv/cwd/PID、起止、退出码、原始日志、清理的环境变量名、命令前后输入清单及原始/UTF-8 LF摘要。执行期间输入变化直接拒绝。`checks.json`列出验收执行点、实际计数和完整登记范围；Rust格式检查覆盖本批全部变更Rust文件。

登记范围包括core/rust、stdlib、tests/fixtures、tests/benchmarks、工作流、.cargo、tools、cli/ts、UseDocs、registry、package.json、bun.lock和.gitattributes。当前HEAD与被测代码的该范围比较命令为：

```powershell
cmd /c git diff --exit-code e7bf7a49ba3ed55679b69c97ac1bd5ce7f5777b6 HEAD -- core/rust stdlib tests/fixtures tests/benchmarks .github/workflows .cargo tools cli/ts docs/UseDocs docs/module-registry.json package.json bun.lock .gitattributes
```

该命令只核对上述登记范围；历史审计通过固定Git对象恢复输入，不要求当前工作树继续保持旧状态。DevDocs是滚动取证与交接说明，不冒充产品输入。

## 测量与原件

固定27条路径、77个规模；VM77条，原生O0/O3各77条。五批各为新进程、预热一次，短操作保留全部校准；正式样本为最后保留批次，时间先按repeat归一后取中位数，OS驻留增量取五批最大保守峰值。编译/输入准备/验证在时钟外；完整语言调用包含实际解析、运算、分配与结果构造。

原始报告和每批stdout/stderr/process.json、完整源程序、字节码或原生文件保存在ZIP中。`verify-performance.py`重算校准、五批、原始单位、PID、黄金事件、统计和翻倍比；`audit.py`另外绑定实际源码、工具/Runtime身份、CI工作流与平台角色、产物ID/远端摘要、目标平台和日志中的每一条统计行。验证器要求字节码或两份原生可执行文件的完整摘要，逐批核对执行文件、参数、输出路径及启动时间，并用原始时长恢复schema1控制器的实际校准次数。只读复核使用固定代码中的catalog，不把未来夹具当成历史输入。

CI保留原始API响应、每次尝试和实际日志作业ID；重跑保留的绿色执行以最早同次执行ID取日志，最新聚合映射单列。artifact名称包含运行号和尝试号，首轮不会被覆盖。

## 历史与退出边界

`performance-archives`保留首轮Bool宽度错误、gcd看门狗、旧原生超时及中间全量；`ci`保留Rustdoc缺口、macOS头文件可见性和历次翻倍比失败。历史JSON与原件不改成通过，新快照不会引用旧可执行文件作自己的成绩。

`independent-22c-record.json`保留最终源码默认回归首轮Linux作业`128177526738`的排队取消失败；同提交第二次尝试全绿。`independent-22d-record.json`绑定`fb215262`默认回归`42289944485`中Windows作业`128140430506`的释放顺序断言。22D/22C及19/11/22债务独立保留，后续绿色不关闭它们。ABI1.18、生成器20、活动intrinsic最大32、B1原28/28守卫和性能阈值不变；仅A6手动模式40分钟已由`ci-timeout-receipt.json`记录实际裁定。旧721条F10没有运行。

第二轮自审见[独立复核目录](../self-review-20261107/README.md)。原先1正/7反与8种报告反例是首次执行时点；本次新增角色、执行产物和校准反例另存，历史结果不回写。
