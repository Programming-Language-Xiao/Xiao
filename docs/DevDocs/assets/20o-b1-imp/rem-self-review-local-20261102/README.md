# REM自审核的取证工具隔离回归

工程期：20O-B1-IMP-REM自审核。职责：保存旧缓存、加载根与源码身份修复的真实执行。
边界：只修改诊断/性能工具，七库产品、ABI1.18、生成器20与预算均未改；不把单项重测冒充721条新成绩。

[默认目标](performance-tests.log)为5 passed，含旧程序目录碰撞、替代加载根/中途源码改写、
原有量化时钟及fmt最大输出/翻倍；[严格Clippy](clippy.log)通过。
[VM单项](vm-smoke.json)与[原生O0/O3单项](native-smoke.json)均完整通过，日志保存独占产物目录。
[真实CLI拒绝](root-rejection.log)显示加载另一stdlib时在校准前退出101，未使用该目录。
[释放诊断](release-diagnostic.log)改用新目录后仍为六组/12条逐对象销毁顺序全部匹配，
结果另存 [release-results.json](release-results.json)。

[已有macOS失败](macos-22d-failure.log)属于8791d7b4维护回归的22D完整释放事件排序断言，
不是本轮工具失败；源码与断言保持。原运行号、完整源日志摘要和截取行段见 [清单](evidence.json)。
所有日志统一LF、清理行尾空白，保留原始摘要供复查。
