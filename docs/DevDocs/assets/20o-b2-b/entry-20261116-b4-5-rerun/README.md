# B4-5 FIX.1 后重开证据

2026-11-16按B4-5规划§2.1重开。固定来源为`6e172c1e`，产品为`cfa79381`，两者受保护产品树一致；ABI1.18、生成器21、标准库接口1。旧`entry-20261115-b4-5/`的失败和FIX.1证据保持历史，不覆盖。

本目录只保存B4-5收口工具与运行证据，不修改产品、测试、UseDocs或registry。使用此前不存在的任务根`D:\project\Xiao\.b4-5\20261116\rerun`，target、Runtime、Home、TEMP均独立；清除继承的XIAO_/RUST_BIGDECIMAL_，两个性能开关固定为0。每条回执记录PID、起止时间、受控命令、产品和DevDocs前后摘要、固定Git文档/工具身份以及Runtime摘要。原始日志按SHA-256内容寻址，同一字节只存一个对象。

先验证B4-1～B4-4、FIX.1及旧blocked历史，再在新Runtime执行当前类型/IR/TAC/VM/LLVM、双路径、归档、子VM、22c/边界与路由回归、B1精确入口、O0/O3矩阵和完整workspace。1,075项pow使用固定清单逐项对账，六条VM承载共6,450项；28/28守卫仅模拟Cargo输出，单独记录工作流和脚本摘要。

全部功能与workspace通过后才写内部`language_available=true`及B5交接。公共cmath、UseDocs、registry保持planned；Windows本机证据不代表Linux/macOS、远端CI、性能或最终发布。B1旧721不运行，22C/22D与19/11/22债务不关闭。最终文档定稿提交后只运行一次`docs-final`，回执绑定全部DevDocs正文及本目录静态工具的raw/规范摘要；生成的报告和日志由最终清单覆盖。

只读复核入口：

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261116-b4-5-rerun/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261116-b4-5-rerun/audit.py --self-negative
```

审计从固定Git对象验证产品、锁和历史身份，不要求当前HEAD一致，不写收口文件。反例在临时副本核对精确拒绝文本及前后树摘要。`collect.py --collect`才显式生成closeout、自反例、交接及evidence；不手改报告或清单。B5仍须在独立批次实施。
