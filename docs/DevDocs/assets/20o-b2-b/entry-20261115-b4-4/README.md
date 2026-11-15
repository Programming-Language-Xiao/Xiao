# B4-4 双路径、归档与承载证据

本批入口为`c93f7465`，产品仍为ABI1.18/生成器21。B4-4实现固定官方加载器下的cmath双路径、真实导出/移除、归档重放和float值承载；B4-5及B5后置，教程与registry继续planned。

采集使用全新baseline/fixed任务根，分开target、Xiao Home、TEMP和Runtime静态库；清理继承的`XIAO_*`和`RUST_BIGDECIMAL_*`，标准库显式绑定仓库`stdlib`。日志按SHA-256内容寻址，重复输出只保存一个对象，命令/时间/PID/前后源码与DevDocs摘要分条记录。生成的回执和日志不纳入静态文档树；本批工具及本README纳入，定稿后只跑一次最终完整文档门禁，失败修复例外均单列保存。

`xiao-xar`作为归档格式/VM边界继续通过`run_archive_with_module_loader`接收固定官方加载器，驱动器负责源码编译及注册；不让归档层反向依赖驱动器。缺显式加载器的低层接口继续拒绝官方模块请求。协议归档入口与本批驱动器公开归档入口必须实际重放。

产品接线`e2f24222`、最终测试快照`eb65a3b6`已完成Windows内部功能验收。B4-4内部退出，B4整批尚未关闭；`b4_closed=false`、`language_available=false`，整批状态留B4-5。Linux/macOS、远端CI、性能预算和旧721均未执行；历史B4-3/FIX1证据保持原样。

| 执行点 | 实际结果 |
| --- | --- |
| baseline `c93f7465` | wrapper8/8、旧planned边界3/3、22c29/29 |
| fixed-final `eb65a3b6` | 11目标83 passed / 0 failed / 5环境门控；30条双路径均真实执行 |
| 驱动器 / 六核心crate | 66/0/1；646/0/16（含集成与文档测试），22c仍29/29 |
| 双路径 / 移除 | 15项位置与命名、函数/模块别名、类型负例、真实源码及LLVM函数体逐项移除、保留导出调用通过 |
| 重放 / 值承载 | 两路径的完整语料及float负零子VM经源码、协议xiaoc/xar与驱动器归档执行；应用源码已删除 |
| 原生选择导入 | 两路径×O0/O3逐字匹配黄金，逆序命名实参保持求值顺序；009错误可捕获 |
| 既有回归 | B4-3四组18,318项矩阵、B1/B2 VM与原生精确点、10项固定历史审计通过 |
| 工程门禁 | workspace Clippy `-D warnings`、fmt通过；定稿后的完整docs/UseDocs/layout由唯一`final-docs`回执核对 |

原生模块值/限定调用沿用B1后置边界。首轮完整模块别名语料在`fixed/native-library`报“名称abs没有动态值槽”，失败不删；最终原生把同语料的限定调用改为已声明的选择导入别名，15项调用和黄金不变，VM/xiaoc/xar仍执行原始模块别名语料。未为本批改LLVM文本或提高生成器版本。

首轮产物采集子命令成功，但产物清单和回执同名导致回执独占写入失败。原清单字节移名为`fixed-final/artifact-manifest.json`，缺PID/时间的原始输出由`capture-interruption.json`锚定，明确不计有效运行；`a1124934`分离名称后，`fixed-final/artifacts`重新逐项核对实际文件。保存4份xiaoc、4份xar、4份LLVM文本，另记录4个原生可执行文件实际摘要；二进制可执行文件不复制入证据目录。

从仓库根复核（默认只读、不要求当前HEAD/旧任务目录存在）：

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261115-b4-4/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261115-b4-4/audit.py --self-negative
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261115-b4-4/audit.py --live-runtime
```

默认审计读取固定Git blob，复核完整受保护产品树与22个允许路径、锁/ABI/生成器/intrinsic/registry、命令与测试目标、Runtime冷构建前不存在及之后每次运行相同的字节摘要、日志与真实产物闭包。`--live-runtime`额外读取当前本机任务中的静态库；不加该参数仍校验采集时的身份。`--root`可指向证据副本，`--repository-root`可指向保留固定Git对象的仓库。

反例仅改临时副本，覆盖清单后改写、删除属性并重建清单、伪造产品/环境/文档/Runtime/运行状态、孤立日志、损坏归档；每项核对退出码、实际标准错误全文和完整目录前后摘要。`self-negative.json`由真实子进程生成，审计强制每项`read_only=true`。`--collect`显式独占生成收口/反例/清单；默认审计不写这些文件。最终docs回执绑定所有非assets的DevDocs正文和本批全部`.py/.md`，并与运行时固定Git快照逐项核对；生成JSON/日志排除，避免证据递归。

自审修正见[记录](../../../20o-b2-b4-4-self-review.md)。B4-5未开始，B5的numeric_range静态/动态专项、20/21接受/拒绝及缓存重生成继续后置；三平台、最终教程/registry升级与721不得引用本机成绩冒充。
