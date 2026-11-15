# B4-3-FIX.1 numeric_range 路由证据

工程期：20O-B2-B4-3-FIX.1。2026-11-16开始实施，入口857b0a69，ABI1.18/生成器21/接口1。先在7d8b85e4、c7a45e10、4be30eaf三个固定提交直接复跑既有n0_b单项；分别隔离源码、target、Runtime、Home、TEMP和标准库根。归因符合规划后才修改predicate和必要测试。

证据采集只追加回执，原始日志按SHA-256内容寻址，重复内容只存一个对象。产品与非assets DevDocs在运行前后分别记录raw/规范摘要，工具绑定已提交Git blob。临时输出放任务TEMP，避免污染文档摘要。历史B4-3/B4-4/B4-5证据不改写，功能结果、历史审计和未执行平台分别登记。

最终`docs-final2`回执须在本页、规划和索引定稿提交后采集；它绑定该提交的DevDocs路径集合及raw/规范树摘要。较早的`docs-final`通过记录保留为历史，未覆盖本轮审计修正。收口生成器随后重建`closeout.json`、六类只读反例和`evidence.json`，清单覆盖`.gitattributes`及全部实际字节；默认审计只读，并逐项核对反例拒绝原因与副本未变化。

FIX.1产品提交为`cfa79381`，只改LLVM路由predicate和新增定向测试；ABI1.18/生成器21/接口1，cmath数学内核、模块加载器、registry/教程不变。最终取证快照`be95d0d5`的受保护产品树与产品提交一致，证据/规划提交不混入产品身份。

三点归因复跑确认`7d8b85e4`（B3-4）和`c7a45e10`（B4-0/1）通过，`4be30eaf`（引入numeric_range豁免）失败。FIX1后n0_b全绿，路由定向3/3、codegen测试、完整workspace、Clippy/fmt/锁、B4-3四组O0/O3、B4-4双路径原生O0/O3、B1 VM/原生精确点及B4-3/B4-4/B4-5固定审计均通过。

路由证据区分四种情况：整数`depth - 1`的numeric_range保留动态weak；Float `**`仅豁免span精确对应幂节点的检查，静态发射共享`cmath_pow`、无`llvm.pow.f64`；混合Float pow和整数范围检查仍走动态；sfloat `**`保持原有`llvm.pow.f32`。pow清单SHA-256 `53b751666f42814f94993ed606e3dfe650941b022392d7d04ea57d9f3d16443d`的1,075项逐项一致分类：1,011相同、40边界、24位模式差异各1ULP；Runtime/cmath与六个VM路径共6,450项参考和冻结规则核对通过。28/28守卫只模拟维护工作流的计数输出。

pow逐项报告以独立`pow-review-sealed.json`保存，SHA由同次成功`capture-pow-sealed`回执绑定；同名覆盖样本保留但不采纳。B4-5原blocked closeout仅作历史记录，FIX.1已恢复门槛，不代替B4-5自身C/D重新签收；B4整批仍未关闭，未声明`language_available=true`。Linux/macOS、远端CI、性能、旧721均未执行。

从仓库根目录只读复核：

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261115-b4-3-fix1/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261115-b4-3-fix1/audit.py --self-negative
```

第一条应报告FIX.1通过，同时保持`b4_closed=false`和`language_available=false`；第二条会在临时副本验证清单篡改、登记文件缺失、来源漂移、workspace失败及公共状态冒进，并核对拒绝原因、前后树摘要与只读性。收口文件只由`collect.py --collect`显式生成；本批不执行B4-5自身收口。
