# B3-2A 与 B3-3A/B 内部证据

本批按 `5343baac` 的拆分范围实施：共享 Runtime pow 前置及15个cmath C ABI/intrinsic34～48。产品代码为 `89d782e1`，仅13个允许路径变化；**截至本批证据时点** `stdlib/cmath.xiao` 尚未新增，类型/IR/VM/LLVM、教程、registry、ABI/生成器和锁文件保持入口身份。后续B3-4源码包装与802回归另见[2026-11-10修复证据](../entry-20261110-b3-fix-802/README.md)，B4以后仍后置。

代码复用了旧隔离分支中的已验证实现，执行证据全部在本批全新任务根生成。原分支 `codex/b3-runtime-20261108` 的包装/LLVM原生阻塞与失败日志仍保留。本批不带包装，完整原生回归按新范围重新执行，不将旧失败改写为通过。

| 验证 | 本批要求与实际记录 |
| --- | --- |
| Runtime | [完整测试](baseline/runtime.log)：269项单元通过，集成/文档测试通过；原有1项集成ignored另记 |
| C ABI | [10项边界测试](baseline/abi.log)，15个符号、17,243条正式MPFR参考、out释放/保留、挂起槽和类型002/数学009 |
| 外部C | [O0/O3](abi/c-probe-result.json)分别231项；新目录链接本批release Runtime，检查实际可执行文件和Runtime身份 |
| 声明 | [8项单元](baseline/intrinsics.log)，34～48连续、Pure/None；1～33和100墓碑逐项不变 |
| pow | [1,075组对账](baseline/pow-switch-manifest.json)：1,011一致，40非有限输入边界，24组位模式变化均为1 ULP；每条保存新结果与参考距离、规则和可达性 |
| 回归 | [B1/B2-A](abi/regression.log)，B1契约9/资源17及B2-A算术、语言、资源、源码/xiaoc/xar重放；[VM](abi/b1-vm.log)及[原生](abi/b1-native.log)维护精确入口包含完整语料 |
| 精确入口 | B1/B2 VM与原生共4个，均 `--exact`、恰好1 passed；原生分别验证O0/O3 |
| 守卫/工程 | 28/28；workspace check、benchmark `--locked`、Clippy、fmt及文档门禁 |
| 平台 | Windows x86_64-msvc本机；Linux/macOS和远端CI本批未执行 |

正式结论由 `collect.py` 从实际执行记录生成到 `closeout.json`，`--prepare` 明确写 `passed=false`，文档门禁齐备后才正式收口。根清单由 `refresh-evidence.py` 按实际字节生成，包含 `.gitattributes`。`baseline` 名称指本批所选源码的首次构建验证，已含共享pow和C ABI；旧33入口由固定Git对象与历史B3-1审计核验，不称本轮重建旧Runtime。

只读复核：

```text
python -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261109-b3-cabi/audit.py
python -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261109-b3-cabi/audit.py --live-runtime
```

默认审计读取固定Git对象、源码清单、日志、C符号与构建身份，不要求当前HEAD等于旧代码。`--source-root` 只需包含固定Git对象，可指向无工作树的裸仓库；参考从固定代码对象读取。`--root` 支持导出的证据目录。`--live-runtime` 额外核对本机Runtime与独立存档的实际字节，默认通过不包含这项结论。审计不写任何文件、不重建清单。

[自反例工具](negative.py)当前有10项：基线、改清单、缺`.gitattributes`、重建清单后替换精确目标/Runtime身份/pow数值距离，以及改错收口统计、pow规则/距离、C优化参数和单条执行记录源码身份；逐项核对全文拒绝原因、退出码及只读性。`--core-only`只用于反例内部，不能代替最终默认审计。

[自审记录](self-review-3411c506/README.md)保留旧6项结果与修复前真实反例。审计现完整核对收口字段、每条执行的源码身份、pow规则/可达性/距离和外部C完整参数；内部产品、原始运行与黄金值未变。

纯float成功调用不分配数值载荷，所以本批用失败注入证明不会发生字符串分配；没有编造可触发的033或011数学分支。结构错误1/2/3不碰挂起槽，类型/数学错误使用6；Fatal/8沿既有不可恢复边界，未伪造可恢复的Fatal测试。悬空指针不在安全输入承诺内，状态3验证空句柄、存活盒子错类/无有效载荷，绝不解引用已释放地址。

参考向量的生产测试引用仍在DevDocs，按已登记要求在B4前迁移至共享fixture并保留摘要。LLVM静态浮点幂仍用旧路径，B4才完成全语言共核。B5集中切ABI1.19/生成器21；本批仍1.18/20、标准库接口1、cmath planned。旧721未运行，B2-C最终721与状态收口、22C/22D及19/11/22债务保持原批次。

初次差表登记命令用了错误的阶段名，记录在 `abi/pow-annotations.log`，未通过参数解析；修正为实际 `baseline/pow.json` 后重新登记，旧日志保留。该错误不影响产品测试或黄金值。
