# 20O-B1-FIX-B. `error_code` 实现与证据

> 状态：实现与完整本地门禁已完成，远端三平台取证进行中。
> 开工基线为 a91c15d；本批单独实现 FIX-B，七库仍未写产品函数，教程继续 planned。
> 契约唯一正文为 [B1 §二](20o-b1-freeze-draft.md#二error_code-的名称位置与签名)，
> 实施步骤见 [FIX-B 规划](20o-b1-fix-b-implementation-plan.md)。

## 一、实际交付

- 全局 `error_code(Error value, /) -> str`，intrinsic 32、Error 参数类别、VM/ABI 绑定一致；
  100 保留墓碑，新增声明仅一项。
- 类型层拒绝具体非错误类型及非一元位置形态，带 expected/actual；内部动态边界
  由 VM 与 LLVM 处理，不增加 RuntimeCheckKind、字节码编码或普通 Error/dynamic 注解。
- VM 直接借读指定错误对象再复制；LLVM 使用 error_value 检查、专用 C getter、
  string_new/str_owned，把输入强引用保留到复制完成，失败先归还临时输入。
- Runtime ABI 1.18 与 CODEGEN_VERSION 15；`.xiaoc`/`.xar`/原生要求与缓存指纹同步。
  低层 C ABI 的兼容旧次版本规则保留，不与当前产物的精确版本范围混同。

getter 不分配、不 retain/release、不触碰挂起槽；状态 0/1/2/3 和失败 out 两字段不变
均由直接 C 入口单测覆盖。源级类型失败为 002，字符串分配失败沿用 **ResourceError / 011**，
getter 无效句柄的 LLVM 防御路径沿用 **MemoryError / 004**。

## 二、借用视图的实现修正

规划最初建议借 `AbiError.inners.last()`。复核发现 `XiaoError` 是深复制 Box；
retain 向尾部压入副本，release 弹出尾部。释放另一个别名会销毁 last 的 code，
即使输入强引用仍存活。实际实现改为 **first()**，直到最后引用释放才销毁。
此修正落实原已冻结借用期，不改公开签名、状态、布局或所有权规则。

回归先比较两次 getter 的地址，失败时不解引用可能悬空的指针；该用例在 last 实现下
确实失败，在 first 下通过。随后验证复制结果在源销毁后仍有效。错误 code 与非空
location/context/cause/suppressed 的完整投影、别名计数和挂起哨兵前后相等。

成功的 error_value 检查同样不能分配。把它纳入实际 System 分配计数后，最初测得
操作名 String 导致一次分配/释放；成功分支现先直接比较借用字节与 Error 标签，
再进入既有通用检查路径。该断言从 `(1,1)` 转为 `(0,0)`，不改变其他检查行为。

## 三、测试层次和边界

| 层次 | 实际断言 |
| --- | --- |
| intrinsic 契约 | 唯一活动 32、33 条声明含墓碑、完整签名/效果/能力/绑定 |
| C getter 四项 | 空/Unicode/长 code、0/1/2/3、失败 out/挂起槽不变、零分配/释放、别名借用期、复制失败 |
| 静态与 LLVM 文本 | 形态/类型/Fatal、expected/actual；三目标 getter→复制→接管→release |
| VM 三载体 | 读取后重抛的 error_id/code/cause/suppressed/location/context 不变 |
| VM 失败注入 | 无字符串常量源码，下一次字符串分配正好发生于结果复制，注入确被消费且可恢复 |
| 七份完整 FIX-B 源码 | 默认/自定义、嵌套/重复/重抛、逃逸、finally、动态负例、长 code、ABI 文件 |
| 原生清理三路径 | 成功/getter 失败/复制失败，O0/O3 各一次，临时输入恰好归还一次 |
| 产物版本 | 新产物在 1.18 接受，在 1.16/1.17 Runtime 拒绝；篡改旧版本声明也拒绝 |

VM 的结构化身份测试通过真实前端编译合法根程序，再仅在测试 TAC 中把唯一 Error 构造
换成宿主输入；getter/raise 不替换。此为语义核测试，不宣称源码开放 Error/dynamic 参数注解。

`test-failpoints` 是 driver 开发依赖显式启用的 Runtime 测试 feature，仅线程局部、一次性、
RAII 恢复，无生产环境变量或 C 注入协议；默认 release Runtime 不启用。
原生清理通过只链接测试程序的 C shim 重定向一条 getter/复制调用，不改生成器控制流。
普通源码测试使用公开 FrontendNativeDriver；注入程序单列，不冒充发布产物。

ABI 负例区分真实当前产物与“修改版本声明”的测试数据，后者不冒充旧编译器产物。
真实原生文件的旧 Runtime 拒绝从内嵌节读取，ABI 场景借用既有 current_runtime 产生元数据，
不扩大元数据格式或并发范围。

## 四、本地与远端证据

本机 Windows 的原生精确执行已通过：普通 B0/FIX-A 各七份、FIX-B 七份各 O0/O3，
普通原生运行共 28 次；另有清理三路径各 O0/O3 共 6 次。Rust 精确汇总为
`1 passed; 0 failed; 0 ignored; 10 filtered out`，最终重跑耗时 51.00 秒。
日志为本机临时目录 xiao-fix-b-native-final.log，不能当作三平台远端证据。

本地结果（不替代远端证据）：

| 命令/门禁 | 结果 |
| --- | --- |
| workspace test，排除 VM 精确目标后单独运行 | 1,532 passed / 0 failed / 49 ignored，共 170 个目标汇总 |
| VM 精确目标 | 1 passed / 0 failed / 0 ignored，21 个普通场景，10.39 秒 |
| 原生精确目标 | 1 passed / 0 failed / 0 ignored，28 次普通 + 6 次清理，51.00 秒 |
| fmt / workspace all-targets Clippy `-D warnings` | 通过 |
| docs / usedocs / layout | 通过 |
| 文档覆盖率 | 10,842 / 12,016（90.23%）；公共 5,775 / 5,775（100%） |
| B1 计数守卫，显式 MSYS Bash | 28/28，实际报告写入本机临时目录 |
| workspace/benchmark check 与锁文件 | 通过，锁文件不变 |
| benchmark tests / performance self-test | 通过，不生成新产品性能成绩 |
| research concurrency fmt/test/Clippy | 通过，不改变研究代码或发布债务 |
| TypeScript 类型检查 / Bun | 通过；301 passed / 0 failed / 5 个宿主依赖 skip |

远端 run/job 尚在收集；完成后在本节回填实际运行号与日志摘要。
`error-handling` 页面增量暂为 draft；七库页面与 registry 仍为 planned。

版本基线审查：固定体积源码不变，`.xiaoc` 保持 1,256 字节，索引保持 237 字节；
ABI 头/指令版本及指令校验值改变，使 DEFLATE 载荷从 526 变为 525 字节，
归档精确基线从 1,169 变为 1,168。未放宽相等断言，未更新 19D 性能报告或发布债务。
兼容矩阵新增三种产物的 1.17 拒绝格，保留 1.16 拒绝与未来版本负例，共 27 格。

## 五、进入 IMP 的交接

只有本批完整门禁与三平台实际执行证据齐备，FIX-B 才能验收完成。后续 IMP 按
math→hex→base64→url→fmt→statistics→csv 顺序独立实现，helper 内联，核对真实导出集合，
保留逐函数移除、真实 `.xar`、普通包隔离、资源/性能和教程执行门槛。
CSV 全量扫描性能仍未证实；FIX-B 通过不改变既有 60 秒看门狗观察或预算验收要求。
