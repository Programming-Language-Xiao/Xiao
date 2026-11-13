# B4-2B 自审与修复

本次从已收口 `81e45783` 自审，保留 `246bea46` 的 Runtime/VM 数值实现。当前修复只涉及测试、planned拒绝消息、相邻说明和证据审计；不接LLVM/模块执行，不升ABI/生成器，也不跑性能或旧721。

| 问题 | 修复与复验 |
| --- | --- |
| P2：原审计把多个目标的通过数相加，wrapper+1/22c−1仍可通过 | 按Cargo目标逐项核对wrapper8、边界3、VM7、前端3、22c29，并要求无过滤、无新增忽略；Runtime/VM/字节码和数值回归分别精确核对目标计数与3项既有原生忽略的名称/原因 |
| P2：历史结果只看passed，cmath状态改成verified仍可通过 | 从81e45783固定Git blobs读取6个历史完整结果，逐字段比较 |
| P2：删除反例报告后重建清单仍返回passed | 默认审计必须有已验收的完整9项报告，报告的只读标记和拒绝身份继续强制；首次显式collect允许预验，不能作为最终结果 |
| P2：原测试分开验证解码后未捕获009/span和源码catch，缺少解码后catch同一路径 | Leb128与FixedU16解码后执行同一失败调用，观察ArithmeticError处理器、009、两次finally及释放事件；既有精确span与O0～O3完整释放序列保留 |
| P3：planned拒绝消息仍称原语未绑定，说明仍把span适配归到exec.rs | 消息明确VM模块入口未接通、直接原语已接通；说明准确指向numeric_intrinsics.rs。门控分支和012身份不变 |

`before.json` 保存前三项修复前的真实临时副本执行：基线及3项篡改均退出0；`after.json` 保存修复后基线退出0、3项篡改按准确中文原因退出1。每次比较完整目录项及文件字节，原证据目录不被反例改写。`reproduce.py --expect-rejections` 强制核对错误文本，不能以无关异常算成功。原始输出按SHA-256存一个日志对象，回执记录其引用；文件与摘要由本目录audit再次读取。

当前源码执行点绑定 `checked/source.json` 的固定提交，独立任务根为 `D:\project\Xiao\.b4-2b-review\20261113\checked`，使用此前不存在的target、Xiao Home和TEMP/TMP，清理继承的XIAO/RUST_BIGDECIMAL环境，显式绑定仓库stdlib，两个性能开关为0。重新运行驱动完整50项：VM专项7、wrapper8、前端3、22c29和FIX1边界3；同时重新执行Clippy与fmt。新测试日志必须包含两种解码catch标记和30条planned拒绝。Runtime/VM私有数学实现未改，原归档的272/35、字节码65、数值15通过及3项原生忽略均作为各自固定快照证据核对，不声称本次又跑了一遍。

`checked/docs` 是本次静态文档和工具统一提交后的单次最终完整门禁。回执记录PID、时间、命令、环境、产品与DevDocs前后raw/规范摘要及Git身份；本README、原目录的新审计器和清单、反例脚本均纳入静态DevDocs，排除本目录运行日志、回执、review/evidence等运行生成物。源码与HEAD运行期间不变，UTF-8/LF规范摘要对照固定Git树，原始字节前后一致。历史docs回执继续绑定其当时文档。

从仓库根只读复核当前自审与原归档：

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/review-20261113-b4-2b/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/review-20261113-b4-2b/audit.py --live-runtime
```

默认核对固定Git对象、当前50项回归、前后反例、文档门禁和完整字节清单；不要求HEAD等于取证提交。`--live-runtime` 额外核对原归档中保留的三套Runtime实际字节，默认只验证归档身份。显式 `--collect` 生成本目录review.json与evidence.json，拒绝覆盖。

本次只证明Windows本机内部B4-2及自审修复；B4整批未完成、模块语言不可用、cmath仍planned。Linux/macOS、远端CI、性能及旧721没有执行，22C/22D与19/11/22历史债务独立保留。
