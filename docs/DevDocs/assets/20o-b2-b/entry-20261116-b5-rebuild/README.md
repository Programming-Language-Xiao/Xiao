# B5 重建基线与版本矩阵实施

星崽于2026-11-16明确批准方案2：从固定B1源码106b99c7和固定工具链重建1.18产物，标为重建基线。历史二进制原件未归档的缺口保留；本目录不把重建解释为找回原件，原B5-0预检目录、B4-5/FIX.1证据均保持只读。

产品入口cfa79381，ABI1.18/生成器21/接口1；目标ABI1.19、生成器仍21、接口1。先建立B1的1.18/20重建基线和B4的1.18/21内部基线，再执行新版本、双向拒绝、两组缓存重生成与C调用方兼容。所有产物只作内部验证，公共cmath/UseDocs/registry保持planned；B6、三平台最终发布、性能和721后置。

工具固定Rust1.96.0，Cargo使用原锁及--locked。每个来源独立任务根、source/target/Home/TEMP/缓存，受控环境清除XIAO_/RUST_BIGDECIMAL_。回执绑定命令、PID、时间、源码/锁、Runtime及工具摘要；所有原始字节以SHA-256单对象存储，用途/媒体类型单独记录，同内容的日志与ZIP不重复落盘。最终文档定稿后再单次采集docs门禁。

本次基线字段固定为baseline_kind=reconstructed_from_B1_source、historical_originals_available=false；旧源码产生后再冻结的xiaoc/xar/原生元数据不手改头或放宽版本。签署冻结正文和历史成绩不改写。

产品切换提交71e611f8，最终测试快照5864cbde；后者只有TAC当前ABI、精确归档体积（1168→1167字节）和并发矩阵格数（9→12）的测试修订。两类旧基线、新Runtime均冷构建；最终测试继承同批生产Runtime的来源和测试专属路径明确登记，不把证据或测试提交混成生产来源。当前Runtime原始SHA-256为3bcc6081c6a85db867987c9d42ad1abeb0bc3801a6654e4442ef65e1af0697c5。

36格基础版本矩阵、含新33/34入口的12格矩阵、两组实际缓存失配/重生成、136个旧C签名保留及16项既有加法扩展均已核对。C调用方同一对象分别链接三份Runtime运行；数值范围20格包含值、009及span的VM/原生O0/O3差分。缓存采用生产指纹与生产对象/索引接口的显式探针编排，每步索引/对象/可执行字节均冻结；不代表CLI已经新增自动原生缓存路径。

新1.19的B1/B4回归及原生矩阵通过；完整串行workspace188目标、1749通过、0失败、54原忽略，Clippy/fmt/锁通过。默认并发workspace的URL Unicode预算用例触发60秒`X09-DRIVER-002`，失败回执保留；独立资源17/17和完整串行复核通过。B5保持`b5_closed=false`，B6不开工；本轮未改看门狗、输入或预算。守卫28/28中4项回放真实B1输出，24项是显式派生负例，仅在Windows执行，不当三平台CI证据。

该URL用例单独以修正参数分隔符重跑，通过原60秒看门狗；同目标17项串行也通过。默认workspace仍在并发调度下出现X09-DRIVER-002，因而默认门禁失败继续保留，不能用隔离/串行成绩取代全workspace结果。一次Cargo参数写错和覆盖保护拒绝均作为采集过程回执保留，不混入测试结论。

失败记录均保留，包括采集器依赖目录/宿主编译/运算名与观察方式修正、旧版本断言、体积基线、矩阵格数和默认并发超时。独立后端问题是静态Float比较生成非法fcmp eq：已在cfa79381旧快照直接复现，known-issues.json关联旧/新失败；首次旧宿主忽略额外参数的尝试不采纳为复现。B5未修改该降低逻辑，静态pow值采用C位模式观察，问题交接保留。

只读复核（从仓库根运行）：

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261116-b5-rebuild/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261116-b5-rebuild/audit.py --self-negative
```

默认审计使用固定Git对象验证允许路径、来源/锁、Runtime、各条命令、完整回执、矩阵和字节快照，不要求当前HEAD等于产品提交。全部文件以实际字节建清单，包含.gitattributes；相同摘要只存一个.bin对象，媒体类型与用途单列。文档定稿后单次docs-final绑定全部DevDocs正文与本目录静态工具，随后collect.py --collect显式生成closeout、自反例、B6交接与清单，默认审计只读。

当前交接状态为B5证据有效但阶段未封存：默认并发workspace失败仍是退出阻塞，B6未开始。三平台、远端CI、受控性能和721未执行；公共cmath/UseDocs/registry保持planned。原B5-0来源阻塞和B4-5/FIX.1历史目录均不改写。已生成的B6移交记录保持blocked，不授权或启动B6；需先处理B5默认门禁失败。
