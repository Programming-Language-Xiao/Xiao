# B5四类证据联合收口

本批完成B5 Windows内部阶段收口：`b5_closed=true`、`b6_entry_open=true`、`b6_started=false`。入口提交`606bbe65`，当前受保护产品固定为`67319d61`，ABI1.19/生成器22/标准库接口1。最终结论见[closeout.json](closeout.json)，由实际审计回执和固定Git对象重算；B6仍须建立自己的测量快照。

| 收口输入 | 固定证据提交/产品 | 本批核验 |
| --- | --- | --- |
| B5版本/缓存/C ABI历史矩阵 | `06eee789`证据；`71e611f8`产品、`5864cbde`最终测试，1.19/21 | 36格基础与12格新入口矩阵、两组缓存、136旧C签名及16项加法扩展；历史审计仍返回当时blocked |
| 静态比较与21→22增量 | `cba346fc`证据；`cfab9771`产品 | 62组VM/248项原生、四格元数据及12行缓存；当前生成器22 |
| URL用例优化 | `606bbe65`证据；`67319d61`测试 | 18/18资源目标、Unicode预算单项5.65秒；原60秒窗口和产品预算保留 |
| 当前完整默认workspace | 同一`606bbe65`证据的workspace-final.json；`67319d61`源码 | 191个测试摘要目标，1,754 passed、0 failed、55原忽略；PID70636、625.688秒，默认4线程、未过滤 |

三份既有审计实际复跑，回执`matrix-audit.json`、`fix-audit.json`、`url-audit.json`记录完整命令、PID、起止时间、退出码、来源前后摘要及原始输出。四类输入共用三份证据目录，完整workspace与URL优化分别列为独立组件；`closeout.json`为每份来源固定提交、Git目录树、权威清单SHA-256和文件集合摘要。原始日志留在各自目录，本目录只增加复核输出的内容寻址对象，同摘要只存一份。

产品差异逐段核验：B5最终测试到静态修复仅有已验收LLVM比较/生成器及关联测试；静态修复到当前快照仅有URL资源测试。Runtime、ABI、标准库、锁、工作流及F10负载未变。本次只做证据及文档收口，引用既有受控完整默认运行，不重复执行workspace、原生矩阵或性能，也不修改旧目录。

历史1.19/21矩阵的失败、8次URL计时和右删失样本保留；当前通过不证明旧快照曾全绿，也不改变`causal_attribution_proven=false`。B1产物仍是获批的来源标记重建，历史原件缺口不核销。28项守卫保持4项真实输出回放+24项派生负例口径。

B6必须按[规划§2.1](../../../20o-b2-b6-resource-performance-plan.md)实跑`url.decode_unicode`全转义最大负载：16,384个编码火箭、196,608字节输入、65,536字节输出；三平台release VM及原生O0/O3各五批，沿B1的10秒/2秒、64MiB和适用翻倍比≤3预算。默认256码点复位样本和混合Unicode输入不能替代；本批没有执行该性能测量，也没有运行最终721。Linux/macOS、远端CI、B6/B7/B2-C与公共发布均未完成，cmath仍planned，22C/22D与19/11/22独立保留。

只读复跑：

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261117-b5-closeout/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261117-b5-closeout/self_negative.py
```

审计从固定Git blob读取历史材料及当时产品，不要求当前HEAD等于产品提交，不重新执行旧脚本或依赖本机Runtime/临时目录。归档的复跑回执证明本批实际执行了旧审计；默认复核则验证这些回执与固定来源。`--root`可指向证据副本，`--repository-root`提供含固定对象的仓库。审计不写回任何文件。

`review.json`是封存前六类语义反例（加真实基线），逐项核对完整拒绝文本、退出码和只读性；其中baseline文件数记录采集时点，最终集合以`evidence.json`为准。封存后自反例另验证篡改清单、修改统计后重建清单也会被固定Git字节校验拒绝。`--prepare`仅用于语义预检，不代表已封存。

全部正文及静态脚本定稿后，单次`docs-final`运行完整仓库文档门禁；输入同时绑定九份修改的DevDocs、本README及本目录根层回执/脚本的前后字节。随后`collect.py seal`按实际字节生成清单，包含.gitattributes；清单自身在提交后与其最近提交的Git blob比较。日志按SHA-256去重，较大对象使用确定性gzip，不复制已有产品测试日志。
