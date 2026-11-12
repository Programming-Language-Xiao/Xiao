# B4-2A-FIX1 边界修复与归属

FIX1 在 Windows 本机完成，修复提交 `c46102115c63956466361d83243dea8ba3e4e74f`。B4-2A 的退出结果另记于 `b4-2a-exit.json`；B4-2B 未启动，`language_available=false`，ABI 1.18、生成器 20、cmath planned 保持原值。

| 执行点 | 实际结果 |
| --- | --- |
| B3-4 固定 `7d8b85e4` | 独立工作树/target 的 22c 29/29 |
| B4-0/1 固定 `c7a45e10` | 独立工作树/target 的 22c 27/29，两项失败与原自审一致 |
| 修复 `c4610211` | 新 target 的 22c 29/29，原 22c 文件逐字规范摘要不变 |
| B4-2A TAC/字节码 | 65/65，保留 34～48 的 66 条调用/宽度观测 |
| B4-1 前端 / B3-4 wrapper | 3/3、8/8；30 条导入后调用均拒绝，hex 原生降低与不可达模块隔离通过 |
| 新边界集成测试 | 3/3：32 条子 VM、48 条归档以及同名项目模块/未导入模块隔离 |
| 驱动器单元测试 | 64 passed、0 failed、1 既有 ignored（真实终端模拟器，10D §4.2） |
| 工程门禁 | 驱动器/字节码 Clippy `-D warnings`、fmt、完整 docs/UseDocs/layout 门禁通过 |
| 历史审计 | B1、B2、B3-CABI、B3-4、B4-0/1 均通过，只证明各自固定快照 |

本次自审核对了 FIX1 证据体量：docs 门禁重复运行回执与重复日志确有膨胀，历史目录和日志字节保留不改。FIX1 后的新证据目录改用原始日志 SHA-256 内容寻址：同字节只存一个对象，每次执行仍单独保存 PID、时间、命令、环境、源码身份、退出码及日志对象引用；不会为去重删除运行事实。

F2 的具体规则和实现位置：新日志对象写入 `logs/sha256/<前两位>/<完整摘要>.log`，记录中只放 `log_ref/log_bytes/log_sha256`，每次执行的回执独立保留；采集函数检测到相同摘要时逐字节比较并复用对象，审计拒绝未引用、重复路径或内容与文件名摘要不符的对象。`content_store_self_test` 和紧随其后的同命令复跑由同一采集器执行；审计要求两份回执的日志摘要与对象引用完全相同，并核对存储函数对不同内容及损坏对象的处理。

旧回执 `fixed/self_review_docs`、`fixed/final_self_review_docs` 等只证明各自运行成功；它们的 `source_before` 没有 DevDocs 字节，不能单独证明当时读取了最终 README 或规划文本。`fixed/docs_final.json`、`fixed/docs_final_sealed.json` 和 `fixed/docs_final_closeout.json` 保留为先前提交快照上的成功运行；最终回执为 `fixed/docs_final_verified.json`，绑定当前 README、规划、审计器和反例源码定稿后的 DevDocs 快照，并要求全量通过。历史 `docs` 门禁日志中的 `A0-COVERAGE-002` 警告仍如实保留。UseDocs/layout 的独立通过记录仍见 `fixed/sealed_usedocs`、`fixed/sealed_layout`。

归属由直接运行确定，见 `b34/concurrency.*`、`b401/concurrency.*` 及生成的 `attribution.json`。`c7a45e10` 为唯一接入 cmath 类型/IR 的产品提交；其 checker 让标准库包装能够编译，旧流程所依赖的“包装编译失败”随之消失，于是未使用导入可在子 VM 加载，归档也可生成。B3-4 当时的 29/29 结论仍有效，不能把这项回归归到 B4-2A 新增测试。

checker 必须继续接受 B4-1 已验收的类型/IR，B4-2A 也必须保留直接原语的 TAC 往返。因此修复采用驱动器内的执行前门控：`DriverModuleLoader::compile` 与归档闭包载入共用 planned 模块拒绝；归档另检查入口及实际加载模块的语句，连未调用函数与死分支里的显式导入也在输出/缓存写入前拒绝。不会扫描 `IrProgram.modules` 的全部预载模块，不会把无关标准库或项目模块引入当前闭包。没有改 checker、VM 分派、Runtime、LLVM、格式/版本、registry、教程或 22c 断言。

`require_executable_standard_module` 当前显式登记 `stdlib.cmath`。后续任何 planned 官方模块都必须先在所属批次列出规范身份，并同步更新该门控和 VM/子 VM/归档边界测试；不得依据源码或 registry 中出现模块就推定可执行。

子 VM 测试在 O0/O3 下，分别覆盖 `std`/`stdlib`、from 导入、from 别名、模块别名和无别名模块导入；未使用导入与调用两种情况都为 `X06-RUNTIME-012`，无 cmath 加载事件、无输出、无部分子值或边界副本释放。正常父级资源仍按既有错误退出清理。归档测试以相同导入形态覆盖可达、未调用函数与死分支，O0/O3 均为 `X11-PROTOCOL-007`；完整目录与文件字节前后相同，未留下归档、临时文件或缓存目录。

开发期曾将 `from std import cmath` 当作已支持的导入夹具；该写法原本在 VM/归档走不同的命名空间拒绝，不能用来证明本批门控。新专项使用四种已由前端支持的 cmath 文件模块导入；没有调整原有命名空间语义，也未改 22c 的任何断言。

三个来源分别使用 `D:\project\Xiao\.b4-2a-fix1\20261112-2f862f51` 下的 `b34`、`b401`、`fixed` 执行目录。前两者从固定提交建立 detached worktree；三者各有独立 target、Xiao Home、TEMP/TMP，显式绑定各自源码的 stdlib，清理继承的 `XIAO_*`/`RUST_BIGDECIMAL_*`，关闭两个性能开关。本批不构建原生 Runtime staticlib；Runtime rlib 随新 target 重新编译，`XIAO_RUNTIME_LIBRARY` 指向各自任务内未构建的位置。`fixed/toolchain.*`、`fixed/targets.*` 记录本机 Rust 1.96.0 和仅安装的 Windows 目标。Linux/macOS、远端 CI、性能及旧 721 F10 未执行；22C/22D、19/11/22 历史债务不在本批关闭。

F3 摘要口径核实：`22c_vm_concurrency.rs` 在 B3-4/B4-1 捕获的原始字节长度为 35,708、SHA-256 为 `1ebb2305cd41dfeabdc36102c172350d80fcfbf8c46b7c31caf258b496ec6c79`；当前本机检出长度为 34,713、raw SHA 为 `e4fd05aaf70d2e410e2f5b487cfe294b56f09a885decd022f2c9e38b3e7c25a1`。三份来源的 UTF-8/LF 规范 SHA 都是 `e4fd05aaf70d2e410e2f5b487cfe294b56f09a885decd022f2c9e38b3e7c25a1`，因此这是检出换行差异；审计同时保留 raw 与 canonical 摘要，没有把二者混用。

F1 的绑定机制：`record.py run` 在终稿 docs 门禁回执内保存 `docs_commit`、`docs_commit_after`、工作树 raw `docs_tree_before/after` 与 `docs_tree_sha256`，并保存规范摘要及 Git tree 的 raw/规范摘要。摘要按 `docs/DevDocs` 下的相对路径计算，包含本批 README、规划、审计器、采集器、反例脚本及其余静态 DevDocs 文件，只排除内容寻址日志对象、运行回执、收口清单和自反例输出这些运行时生成物。审计要求运行前后 raw/规范摘要相同、运行前后 HEAD 不变，并要求工作树 UTF-8/LF 规范摘要等于回执绑定 Git commit 的规范摘要，同时逐字节核对 Git blob 原始摘要。这样既证明门禁实际读取的工作树字节未漂移，也容纳 Windows 检出的 CRLF 与 Git LF blob 差异；未提交文档或门禁期间文档变化都会拒绝。这补足了 `source_before` 未包含 DevDocs 字节的边界。

提交 `cb2444dc0a2afd6d2bf456fe3b1805454b4825c9` 固定原始 `record.py`，`13e7670f0ceb5d6ab4ca181effb07da0a8059284` 加固全新任务根，`2e656b123a99bda33c881606fe01898863a2f122` 加固父路径 junction 检查，`6ed66039ae13fe841ebe60d86f2458b957f6d191` 固定内容寻址采集器，`4ca4b12c976ad971f7ca5241cea784d6b0c05f74` 固定 raw/规范双摘要采集器。审计从这些固定 Git blob 复核回执执行器摘要；当前同名审计脚本是后续自审版。每条回执记录 PID、时间、完整参数、cwd、隔离环境、原始日志摘要和源码/锁前后摘要。`source.json` 同时保留原始字节和 UTF-8/LF 规范摘要，历史审计逐字段校验 B1/B2/B3/B3-4/B4-0/1 的计数、快照、公开状态和版本；日志保持原始字节，既有历史日志不改写。

从仓库根只读复核：

```powershell
python -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261112-b4-2a-fix1/audit.py
python -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261112-b4-2a-fix1/audit.py --self-negative
```

默认审计不读当前工作树、不要求当前 HEAD，也不要求旧 worktree、target 或 Runtime 仍存在；只需仓库保留固定 Git 对象。它支持 `--root` 和 `--repository-root`，不写证据、不生成 Python 缓存。自反例有 13 个用例（1 个基线、12 个拒绝反例），只改临时副本，覆盖属性删除及重建清单、运行记录/收口/退出状态篡改、伪造来源/日志、DevDocs 摘要篡改、孤立内容日志对象、嵌套清单、`..` 逃逸和复用空任务根；每项核对准确拒绝原因及整个目录文件/目录项的前后摘要。当前 13 项报告保存在 `self-negative-findings-content-addressed.json`；旧 `self-negative-findings.json` 是 11 项历史结果并保留不改。修复前三个缺口被漏过的复现日志见 `self-review-20261112/before-negative.json`。

`audit.py --collect` 是显式生成命令，在所有运行与文档完成后从真实记录生成 `attribution.json`、`closeout.json`、`b4-2a-exit.json` 和 `evidence.json`，拒绝覆盖。新增门禁记录后使用 `--refresh-results`，只有归属不变、原运行映射完全匹配时才用全部真实记录重建 closeout 与清单；该模式不改写运行日志。工具或说明更新而没有新运行时，用 `--refresh-inventory`，它先验证原结论再刷新实际清单。旧的 blocked 自审仍保留，后续实现入口以本目录退出记录为准。

自审核额外修复审计与取证缺口：不能通过 `--refresh-results` 篡改状态；嵌套同名清单必须纳入；拒绝 `..`、点路径、符号链接/junction 越界；重建清单不能掩盖缺失 `.gitattributes`；采集器必须拒绝已有任务根，三个变体根互相隔离且路径固定；执行器摘要绑定 `cb2444dc`、`13e7670f` 和 `2e656b12` 三个固定 Git blob；历史审计逐字段核对精确计数、边界状态与既有 ignored 项。修复前旁路复现见 `self-review-20261112/before-negative.json`，十项只读拒绝反例均在临时副本运行。
