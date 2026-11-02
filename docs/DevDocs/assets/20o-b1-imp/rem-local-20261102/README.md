# REM本机功能、门禁与探索记录

工程期：20O-B1-IMP-REM。职责：保存逐模块执行、完整门禁和原始性能失败数据。
边界：每份日志保持其实际阶段，不能把本机、生成器19/20或探索工具版本混作完整三平台F10结论。

[清单](evidence.json)记录机器、工具链、原始字节摘要和统一LF、清理行尾空白/多余末尾空行后的副本摘要。模块产品分批提交：
08074f6e（hex）、8f9f9047（base64）、381a1a2e（url）、dab4f289（csv）；桥接修复b585eeb0。

- final-vm-exact.log：七产品及全部历史/辅助样例，恰好1 passed，0 failed，0 ignored。
- csv-contract-final.log：七模块九项契约，20入口、真实导出/移除/归档/普通包和教程对账。
- csv-native.log：七库原生O0/O3精确目标，恰好1 passed；四份resources日志为逐库最大资源回归。
- workspace-tests.log、workspace-clippy.log及其余门禁日志：生成器19本机全workspace/严格Clippy、
  fmt/docs/usedocs/layout/覆盖率、TypeScript与Bun（301 passed / 5 skipped / 0 failed）。
- cg20-*日志：独立前置后的20C、原22D三目标、22E、七库原生精确目标及严格Clippy等真实回归。
  release-vm.log和fmt-domains.log分别验证六份释放向量及新增进制/填充参数负例。
- guards.json/log：维护工作流计数守卫28/28；不把计数模拟当三平台产品执行。

perf-native-max.json/log保留桥接修复前URL解码O0的6.8316662秒失败，队列实际返回1；
perf-native-url-after.json/log记录相同产品重测O0/O3为1.0438788/1.0116239秒。
str-bridge-runtime.log与str-bridge-clippy.log是无整串复制回归及Runtime全量门禁。

perf-vm-max、perf-native-all-after属于初版工具的探索记录。前者29份最大输入中最慢8.1658981秒，
后者190条记录的旧passed字段不能用于完整F10验收：初版校准没有保证短调用至少50ms，且原生队列
后段与其他门禁重叠。修正后的calibration-smoke只验证迭代校准能得到足够长的批次，亦非全批成绩。
完整F10以修正工具、生成器20和实际CI运行的JSON为准，所有旧数据保留。

cg20-workspace-tests.log为新增17项资源与六向量后完整workspace通过记录；
perf-vm-calibrated.json/log在用户中断时停在54条、complete=false，不能登记为完整结果。
quantized-*与bounded-calibration-*保存零读数量化回归、3项默认性能目标及新工具单项smoke，
完整三平台F10仍以CI整批报告为准。

hex-vm.log与csv-contract.log/csv-lowering.log是开发期失败：分别暴露未定元素数组被当数值相加、
分支内首次堆绑定的原生降低边界。产品已用显式str与函数级局部初始化修复；这些失败不计为通过。
释放失败及生成器20后的完整trace另见相邻两个release证据目录。
