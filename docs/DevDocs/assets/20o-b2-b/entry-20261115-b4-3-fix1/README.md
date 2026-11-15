# B4-3-FIX.1 numeric_range 路由证据

工程期：20O-B2-B4-3-FIX.1。2026-11-16开始实施，入口857b0a69，ABI1.18/生成器21/接口1。先在7d8b85e4、c7a45e10、4be30eaf三个固定提交直接复跑既有n0_b单项；分别隔离源码、target、Runtime、Home、TEMP和标准库根。归因符合规划后才修改predicate和必要测试。

证据采集只追加回执，原始日志按SHA-256内容寻址，重复内容只存一个对象。产品与非assets DevDocs在运行前后分别记录raw/规范摘要，工具绑定已提交Git blob。临时输出放任务TEMP，避免污染文档摘要。历史B4-3/B4-4/B4-5证据不改写，功能结果、历史审计和未执行平台分别登记。

FIX.1产品提交为`cfa79381`，只改LLVM路由predicate和新增定向测试；ABI1.18/生成器21/接口1，cmath数学内核、模块加载器、registry/教程不变。最终取证快照`be95d0d5`的受保护产品树与产品提交一致，证据/规划提交不混入产品身份。

三点归因复跑确认`7d8b85e4`（B3-4）和`c7a45e10`（B4-0/1）通过，`4be30eaf`（引入numeric_range豁免）失败。FIX1后n0_b全绿，路由定向3/3、codegen测试、完整workspace、Clippy/fmt/锁、B4-3四组O0/O3、B4-4双路径原生O0/O3、B1 VM/原生精确点、B4-3/B4-4/B4-5固定审计均通过。

路由证据区分四种情况：整数`depth - 1`的numeric_range保留动态weak；Float `**`仅豁免span精确对应幂节点的检查，静态发射共享`cmath_pow`、无`llvm.pow.f64`；混合Float pow和整数范围检查仍走动态；sfloat `**`保持原有`llvm.pow.f32`。pow清单SHA-256 `53b751666f42814f94993ed606e3dfe650941b022392d7d04ea57d9f3d16443d`的1,075项逐项一致分类：1,011相同、40边界、24位模式差异各1ULP；Runtime/cmath与六个VM路径共6,450项参考和冻结规则核对通过。28/28守卫只模拟维护工作流的计数输出。

本批证据与只读审计已固定。B4-5原blocked收口仍作为历史记录保留，FIX.1只恢复B4-5重跑条件，不代替B4-5重新收口；本批不宣称B4整批关闭或内部`language_available=true`。Linux/macOS、远端CI、性能、旧721均未执行。
