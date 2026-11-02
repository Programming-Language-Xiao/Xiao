# `xiao-codegen-llvm`

## 目录职责

把已验证的类型化 IR 降低为 LLVM IR，并通过调用方注入的 `clang`/`llvm-as` 生成原生产物。
静态标量继续走不链接 Runtime 的 N0-A 路径；N0-B 为字符串、动态值、容器和表生成固定
`xiao-runtime-abi` 调用，并在正常退出边消费所有权释放计划。22D 进一步消费同一份
`IrConcurrencyCall`/`IrOwnership` 计划生成不透明并发 Runtime 调用与 trampoline；差分和
跨平台原生回归仍在实施中。统一可恢复错误展开仍留给 N0-C。

## 工程期

10 建立未优化原生闭环；15A 接入 `-O0`–`-O3` 的安全 LLVM 参数、可解释 Runtime 裁剪和
构建指纹；15B 再接入调试/可复现构建与性能基线。平台顺序为 Windows → Linux → macOS。

## 模块放置

`src/ir.rs` 放置标量 LLVM 降低，`src/dynamic.rs` 是 N0-B 动态降低门面，具体实现拆在
`src/dynamic/` 的谓词、文本、Runtime ABI、入口、槽、释放、控制流、表达式和容器模块中；
`src/target.rs` 放置规范化目标，`src/toolchain.rs` 放置显式工具链适配，`src/build.rs` 放置
内部构建/运行观察面。稳定 Runtime ABI 位于独立的 `xiao-runtime-abi` crate；`.app`/签名
编排由平台与发布工具负责。

`optimization.rs` 复用 13A 的规范化配置，生成只启用函数内安全优化的 LLVM 计划；跨模块、
LTO、PGO 和目标特化开关默认关闭，并拒绝快速数学/未定义回绕等不安全扩展。计划指纹包含
级别、目标字段和 LLVM 版本摘要。`LlvmOptimizationReport` 同时解释 Runtime 组件依据 IR
效果摘要保留或删除的原因，不把链接器最终删掉什么当作裁剪证明。

`reproducible.rs` 提供路径/临时字段归一化、符号排序和显式差异白名单；`baseline.rs` 提供
固定条件下的编译、启动、运行、内存和体积五维样本、中位数与噪声离散度。两者的结果只用于
报告和验收，不进入 LLVM 文本、原生文件或构建指纹。

## N0-B 动态降低器

`dynamic.rs` 只保留生成器状态、生成流程和 LLVM 发射原语；职责实现见
`src/dynamic/README.md`。动态路径可以依赖 `xiao-ir`、`xiao-runtime-abi` 和 crate 级
`text.rs`，静态 `ir.rs` 不得反向依赖 `dynamic/`。源码级依赖回归由
`src/dynamic_architecture_tests.rs` 锁定，拆分不新增 Runtime 能力、不改变所有权释放顺序，
结构拆分本身不升 `CODEGEN_VERSION`；会改变生成文本或 trampoline ABI 的 lowering 语义必须递增版本。

## 允许与禁止依赖

后端消费 `xiao-ir`、零运行时实现依赖的 `xiao-runtime-abi` 声明及 `xiao-standard-library`
的身份元数据，不依赖 VM、完整 Runtime 或 CLI，也不引入 `inkwell`/`llvm-sys`。
不在后端改变类型、溢出或错误语义，不打包 `.xar`，不解析终端命令行，不自动发现工具链。

20O-B1-FIX-B 把 intrinsic 32 降低为 error_value 检查、只读 getter、string_new 复制和
str_owned 接管；输入强引用保留到复制结束，所有失败边先归还临时输入再传播。
FIX-B 的生成器版本 15 隔离新符号与调用顺序，产物要求 ABI 1.18；完整源码与故障清理回归
在 driver 的 `20o_b1_probe`，生产 Runtime 不包含测试 shim。

B1-IMP 开工前以生成器版本 16 单独修复 `std.` 既有别名的原生模块身份查找，
Runtime ABI 仍为 1.18。模块值/限定函数调用仍保留规范路径已有的原生拒绝边界，
不通过别名修复引入新语言能力。

生成器版本 17 接通既有固定参数的命名调用。`arguments.rs` 仅计算绑定排列，静态和动态
降低均先按源码顺序求值，再按形参顺序传递；不扩展默认值、可变参数、intrinsic 或 ABI。

版本 18 使精确负数字面量下标与字节码一样仅应用一次符号，支持最小 int 的安全解析。
