# B4-5 FIX.1 后重开证据

2026-11-16按B4-5规划§2.1重开。固定来源为`6e172c1e`，产品为`cfa79381`，两者受保护产品树一致；ABI1.18、生成器21、标准库接口1。旧`entry-20261115-b4-5/`的失败和FIX.1证据保持历史，不覆盖。

本目录只保存B4-5收口工具与运行证据，不修改产品、测试、UseDocs或registry。使用此前不存在的任务根`D:\project\Xiao\.b4-5\20261116\rerun`，target、Runtime、Home、TEMP均独立；清除继承的XIAO_/RUST_BIGDECIMAL_，两个性能开关固定为0。每条回执记录PID、起止时间、受控命令、产品和DevDocs前后摘要、固定Git文档/工具身份以及Runtime摘要。原始日志按SHA-256内容寻址，同一字节只存一个对象。

先验证B4-1～B4-4、FIX.1及旧blocked历史，再在新Runtime执行当前类型/IR/TAC/VM/LLVM、双路径、归档、子VM、22c/边界与路由回归、B1精确入口、O0/O3矩阵和完整workspace。1,075项pow使用固定清单逐项对账，六条VM承载共6,450项；28/28守卫仅模拟Cargo输出，单独记录工作流和脚本摘要。

首次pow采集在依赖选择阶段失败：新target含多份合法驱动器库，旧工具要求只有一份。失败回执与日志保留；新采集按已执行B1原生测试程序的Cargo指纹选择驱动器及其传递依赖，用独立`pow-verified`回执绑定逐项报告。报告保存测试程序和库摘要，不使用文件时间或任意第一份库。

本轮运行结果：完整workspace的187个Cargo/文档目标共1,747项通过、0失败、54项原忽略；两条子进程摘要单列，不重复计入父汇总。release定向9目标76项通过，TAC2/2，30条导入、32条子VM、48条删源归档与四组原生矩阵通过；B1 VM/原生精确点、Clippy、fmt、锁及13项历史审计通过。pow为1,011一致、40项非有限输入顺序边界、24项各1ULP，六条VM承载6,450项通过。Runtime原始摘要为`27b460d2fde00c0e36f2799474e3a21310d5555530cbc8d7694caf6df220702d`，全部回执运行前后相同。

workspace解析按可执行指纹保留跨crate同名目标，各Cargo目标最后一条摘要属于父进程，之前的子进程摘要分别核对；解析器自测验证同名目标、子摘要、失败子摘要及缺摘要拒绝。收口反例包含基线、清单篡改、缺文件、来源替换、阶段提交伪造、Runtime路径逃出任务根、workspace失败与公共planned提前升级；均核对完整拒绝文本和前后树摘要。

全部功能与workspace通过后才写内部`language_available=true`及B5交接。公共cmath、UseDocs、registry保持planned；Windows本机证据不代表Linux/macOS、远端CI、性能或最终发布。B1旧721不运行，22C/22D与19/11/22债务不关闭。最终文档定稿提交后只运行一次`docs-final`，回执绑定全部DevDocs正文及本目录静态工具的raw/规范摘要；生成的报告和日志由最终清单覆盖。

只读复核入口：

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261116-b4-5-rerun/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261116-b4-5-rerun/audit.py --self-negative
```

审计从固定Git对象验证产品、锁和历史身份，不要求当前HEAD一致，不写收口文件。反例在临时副本核对精确拒绝文本及前后树摘要。`collect.py --collect`才显式生成closeout、自反例、交接及evidence；不手改报告或清单。B5仍须在独立批次实施。

内部`b4_closed=true`仅涵盖本页Windows语言收口；最终签收取决于`docs-final`回执、closeout和默认审计全部通过。`b5-handoff.json`记录后续ABI1.19/生成器21的拒绝/缓存矩阵、B6资源性能与B2-C新快照721/三平台发布义务。原生模块值/限定调用继续保留B1边界；本轮不启动B5。
