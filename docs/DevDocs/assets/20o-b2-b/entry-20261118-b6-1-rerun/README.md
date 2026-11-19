# B6-1独立任务根重跑与受限结论

本批已在Windows/Linux独立任务根重跑FIX.2产品`43f18274175ba09ac84e759999c40cde72eae1c5`，ABI1.19/生成器22/接口1。Linux官方cmath包装原阻断已解除；Windows同矩阵功能通过、COFF导出完整性未验证；macOS未验证。`result.json`分别记录`rerun_closed=true`、`b6_1_status=restricted`、`stage_passed=false`、`three_platform_passed=false`，不把重跑完成等同于三平台B6-1验收通过。

| 项目 | Windows | Linux |
| --- | --- | --- |
| 任务根 | `D:/project/Xiao/.b6/b6-1-rerun-20261118-01/windows` | `/home/xiaoczx/.b6/b6-1-rerun-20261118-01` |
| 官方包装 | std.cmath/stdlib.cmath各O0/O3，黄金与009捕获通过 | 同四格通过 |
| 组件验证 | COFF `UnverifiedCoffExports`，功能通过、导出完整性未验证 | ELF `Complete`，声明=观察=`containers/rc/value/weak` |
| Runtime release | 305 passed / 0 failed / 1既有ignored | 同左 |
| 正式Cmath参考 | 15个C ABI入口、17,243项MPFR/GMP参考 | 同左 |
| B6语言、A6资源、VM/归档及数值路由 | 8目标42 passed / 0 failed / 3既有ignored；A6资源8/8 | 8目标43 passed / 0 failed / 3既有ignored；多一项独立组件诊断 |
| codegen回归 | 109 passed / 0 failed / 15既有ignored | 同左 |
| driver额外回归 | 17 passed / 0 failed / 3既有ignored | 同左 |
| 原生精确目标 | Cmath核心/共享pow、A5错误与011原子性、显式除法各O0/O3通过 | 同左 |
| fmt / Clippy | fmt通过；补齐固定编译夹具后同命令Clippy通过 | fmt通过；Rust1.96未安装Clippy，实际组件清单已记录 |

本批只执行固定资源/错误/原子性及相关回归，不以编译或测试耗时作F-3性能成绩。Linux额外诊断直接使用工具链链接，不能替代驱动器产品路径；Linux产品路径通过由`linux-wrappers.json`四格`Complete`单独证明。既有ignored保持原状；本批显式执行四类原生精确目标，没有加ignore或改预算。

`prepare.py`在命令执行前生成`commands.json`及`source.json`，从固定Git对象按受保护根导出1,076个文件；两平台初始源码逐字相同。各任务根、target和Runtime均为全新，只在本批内部复用构建缓存。标准库根指向本任务源码下的`stdlib`。Rust/Cargo1.96.0、Linux Ubuntu26.04 WSL2/LLVM21.1.8；Windows LLVM工具及目标x86_64-pc-windows-msvc的实际版本、工具摘要、OS、锁和Runtime摘要列于命令回执。命令PID为Cargo/工具进程PID；包装四格附实际执行文件路径及摘要，不宣称记录了每个原生子进程PID。

Windows第一次`--all-targets` Clippy因源码导出遗漏`tests/spec/11x0-protocol`而退出101，`windows-clippy.json`及日志原样保留。`retry_clippy.py`从同一固定Git提交补齐`tests/spec`原字节，另存`supplemental-source.json`及补验前后摘要；`windows-clippy-r2.json`记录完全相同命令退出0。没有改测试或缩减Clippy目标。Linux的Clippy缺项由`linux-tool-availability.json`实际工具链组件输出证明；WSL既有配置警告原文保留，不计产品失败。

`entry-gates.json`包含FIX.2主审计、8项反例、B5联合收口、FIX.1、静态比较和URL优化六项只读回放，全部通过。`historical.json`绑定原B6-1、FIX.2/B5/URL/静态比较/FIX.1历史目录每个文件的实际字节，定稿前后复核不变；原B6-1的`blocked/stage_passed=false`不改写。

日志和重复源码快照按原文SHA-256寻址，同一原文只落一个对象。每条命令回执保存完整命令、实际PID/UTC时间、前后源码、工具、Runtime、隔离环境和实际产物摘要。`audit.py`默认只读复核封存的这些身份与固定Git对象，不依赖当前HEAD等于测试提交，也不要求后续DevDocs保持原文；它不重跑产品、不检查本机Runtime存档是否仍存在。

本README、规划/索引、脚本和结果定稿后，单次执行完整docs门禁`docs-final.json`。门禁输入显式包含本批DevDocs及补充反例目录的README/属性文件，保存前后摘要；`finish.py seal`按最终实际字节生成包含`.gitattributes`的清单。先提交主归档，再执行后置反例报告，避免清单与自身摘要递归。

复核命令：

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261118-b6-1-rerun/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261118-b6-1-rerun/self_negative.py
```

反例的记录方式和结果见[补充目录](../entry-20261118-b6-1-review/README.md)。审计支持`--root <临时证据副本> --repository-root <固定对象仓库>`；`--prepare`只做封存前只读语义核对，不作为最终清单通过证据。

B6-2仅具备Windows/Linux受限入口，后续仍须按B6冻结负载、五批独立进程、时间/OS峰值与最大全转义URL协议执行。本批没有启动B6-2、性能/OS峰值、远端CI或旧721；macOS及Windows完整导出表校验仍未完成，B6/B7/B2-C未收口。
