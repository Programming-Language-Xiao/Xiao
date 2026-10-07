# 10S-A1：表方法与生命周期 ABI 开工清单

状态：本清单由 [10T](10t-table-method-abi-implementation.md) 实现；10S 本身仅准备。来源：[10S](10s-cross-platform-evidence-and-release-closeout.md)。

现有数据链：IR 的表签名可还原 Runtime TableSignature，成员已区分 method；
字节码表定义另存方法函数编号与字段初始化函数，VM 通过 TableInstance::with_initializer
执行字段函数，再执行 init。drop 通过 TableDefinition::with_drop_executor 交给 Runtime
最后强引用触发。LLVM 只有字段描述数组，XiaoTableDescriptor 只含 name/kind/fields/count，
没有函数表、字段初始化回调或 drop 入口；collect_table_initializers 和
emit_table_descriptor 的方法拒绝必须保留到整个契约实现。

建议按下面的契约增加独立 ABI 入口，开工时先冻结布局和版本，不修改旧描述符原地布局：

| 数据 | 建议表示与检查 |
| --- | --- |
| 新版表描述符 | repr(C)，带结构大小/版本、旧字段描述、方法数组指针与数量、字段初始化回调、可选 init/drop 回调 |
| 方法项 | 稳定方法名、签名 ID、实参数量/参数类型元数据、可见性、函数指针；拒绝重复名、空回调及无效签名 |
| 回调约定 | extern C 返回 i32 状态，参数使用 receiver 指针、连续 XiaoValue 实参指针及长度、结果输出指针；不按值返回聚合结构，避免 MSVC/SysV 返回约定差异 |
| 所有权 | 普通 receiver/实参在调用期间借用，被调方需持有则显式复制；结果是调用方唯一拥有值，失败输出保持 none；元数据由 Runtime 复制，函数指针必须在模块存活期有效 |
| drop 接收者 | 只能使用既有 TableDropView 借用契约，不得用强引用复活正在析构的实例；不得逃逸、写入 receiver 或跨线程 |
| 版本兼容 | 新入口配新描述符和所需次版本；若改变已有布局/所有权则升主版本。旧 Runtime 明确拒绝新产物，同步指纹/兼容矩阵 |

实施顺序：

1. xiao-runtime-abi 定义独立描述符及调用入口，增加尺寸/对齐和三目标布局测试。
2. xiao-runtime 复制并验证元数据，通过已有初始化状态机调用字段/init，最后强引用调用 drop；
   复用当前错误主因/抑制链、Fatal 绕过和构造失败回滚，不另造生命周期状态机。
3. xiao-codegen-llvm 生成方法独立函数与稳定函数表，降低 receiver/参数/结果以及失败边；
   值槽必须按函数/作用域区分，沿 10R 清理链消费计划。
4. 更新 Runtime 组成及可达符号报告、ABI 版本要求、工具链指纹、归档元数据与兼容矩阵。
5. 完成差分后再摘方法拒绝；不先删除拒绝然后用空函数顶替实现。

影响 crate：xiao-runtime-abi、xiao-runtime、xiao-codegen-llvm、xiao-driver；
IR/类型阶段仅在现有签名不足以承载调用约定时扩展。VM 作为语义基准，不改其行为。

验收必须包含：

- table-user-drop 从明确拒绝转为真实原生执行，VM 的 drop 输出、错误和释放顺序完全比对。
- container-dense 构建、运行及结果差分，更新探测基线；可构建不等于性能达标。
- 普通/私有方法、同名不同表方法、方法相互调用、非法参数、返回堆值与返回覆盖。
- 字段初始化顺序、init 失败、drop 恰好一次、共享引用、弱观察失效、drop 抛错及嵌套错误抑制。
- Fatal 不运行普通 drop，构造失败无泄漏；三平台 O0–O3、动态调试产物与非调试产物分别验证。
- 受控释放事件数/对象数/销毁顺序和引用持有机制同时有证据，不能只看总数相等。

10S 本批不动上述实现，不改变表方法拒绝点。A2 参数构造与 A3 函数值 ABI
后续复用 A1 契约；静态溢出排期与性能基线仍独立。
