# 静态浮点相等修复的基线复核

本目录已完成 Windows 本机未修复基线的真实探针。产品保持生成器21、ABI1.19；这里证明缺陷和既有语义，不是修复验收。原先的oeq候选已撤回，按位相等/不等和生成器22的修订方案尚待实际裁定。

[probe.rs](probe.rs)使用真实Xiao函数参数，分别记录VM输出、默认降低与加入打印后的动态降低。每条LLVM观察记录Runtime依赖、生成文本、真实llvm-as结果；能够汇编的程序再通过既有NativeBuild接口构建并执行O0/O3。静态程序采用既有测试专用入口观察：布尔false对应退出码0，true对应退出码1，不能把退出码1误报为运行时错误。模块依赖Runtime不等于局部比较已走动态入口，归档文本必须一并核对。

探针还直接记录 Runtime 语言比较入口对 Float/Sfloat 正负零的结果，供与普通有序数值比较区分。包含正负零、同号零、普通有限数值、既有混合提升、整数和布尔对照；不新增 NaN/Inf 源码输入，不改变 Eq/Hash 或语言规则。

实际使用Rust1.96.0、LLVM22.1.2和固定5864cbde的已登记调试库。构建回执按真实Cargo依赖指纹唯一选择rlib，Runtime静态库另记摘要；不是新冷构建。六个位置参数依次为全新观察目录、固定Runtime静态库、clang、llvm-as、llc、编译器路径，完整命令、PID、时间、环境和工具身份见[构建回执](probe-build.json)与[执行回执](probe-run.json)。执行前后借用库及产品工作区检查通过。

[observations.json](observations.json)保留全部16组实际观察：

| 观察 | 结果 |
| --- | --- |
| VM | 16组成功 |
| 打印动态原生O0/O3 | 32次汇编及执行，输出全部与VM一致 |
| 默认降低的浮点相等 | 24次真实llvm-as拒绝，均为非法fcmp eq；未执行原生产物 |
| 默认降低的异号零不等 | 4次执行得到false，VM及动态路径为true，存在语义差异 |
| 默认降低的整数/布尔对照 | 4次执行与VM一致 |

Float/Sfloat语言入口按位相等：异号零相等为false、不等为true；同号负零相等为true。因此直接改oeq会改变现有语义。建议的bitcast加icmp eq/ne仍是待批准方案，没有在本目录执行产品修复。

执行产物共归档64份LLVM文本及40份bitcode，按摘要去重；40份本机原生可执行文件只登记路径、大小及SHA-256，未作为可移植产物归档。日志与LLVM对象的压缩方式、压缩前后摘要均在回执中。探针宿主退出0只表示完成基线采集，不表示64条原生路径通过。

构建命令为 `python -B -X utf8 docs/DevDocs/assets/20o-b2-static-float-equality/entry-20261117-review/build-probe.py`，执行命令将末尾脚本改为 `run-probe.py`。两脚本是本机一次性采集器，拒绝覆盖回执或复用任务目录，且依赖已登记的本机调试库；不能把它们称为任意机器可直接重跑的测试入口。

[audit.py](audit.py)只读核对采集源码、原始stdout、对象摘要、拒绝原因、输出差异和固定Git产品树，不要求当前HEAD等于旧提交，不写closeout或清单。完整命令为 python -B -X utf8 docs/DevDocs/assets/20o-b2-static-float-equality/entry-20261117-review/audit.py。--receipts-only只核对回执，明确不验证最终清单。默认审计依赖仓库中登记的两个Git对象和历史来源回执，不要求本机原生可执行文件仍存在，也不会重新哈希外部Runtime，不能据此声称重新执行过产品。

[collect-docs.py](collect-docs.py)采集最终文档门禁，运行bun的仓库all入口；回执记录实际退出码及前后输入摘要。首次[docs-gate.json](docs-gate.json)退出1，原因是本页提前链接尚未生成的回执和清单，原始失败日志保留。[docs-gate-final.json](docs-gate-final.json)退出0；其后修正审计器的临时归档路径，最终轮以--label final-verified另存docs-gate-final-verified.json，不覆盖前序回执。输入包含DevDocs全部Markdown、本页、规划、UseDocs、标准库和文档检查工具；回执与日志输出不参与输入摘要，避免自引用。

[seal.py](seal.py)在最终门禁后按实际字节一次生成evidence.json，包含.gitattributes，不改写既有回执。规划文本另存内容对象，后续修改当前规划不会改写这次历史文档证据。日志同摘要只保存一个对象。

[rejected-oeq-test.rs](rejected-oeq-test.rs)是已撤回且没有编译执行的测试草案，不在产品测试目录内，不作为通过证据。Linux/macOS、远端CI、修复后的元数据/缓存矩阵均未执行；B5和URL看门狗阻塞保持原状态。
