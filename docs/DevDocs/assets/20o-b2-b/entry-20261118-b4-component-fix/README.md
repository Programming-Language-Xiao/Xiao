# B4-3 FIX.2 Runtime组件推导修复

FIX.2 已完成，产品提交为 `43f18274175ba09ac84e759999c40cde72eae1c5`。本批只调整 LLVM Runtime 组件推导及必要回归；ABI 保持 1.19、生成器 22、标准库接口 1，未改 Runtime 函数、cmath 语义、registry、教程或工作流。

FIX-0 在未修改代码时复现了平台差异。相同真实包装语料在 Windows 的 `std.cmath`/`stdlib.cmath` O0/O3 通过，但 Windows COFF 符号表状态是 `UnverifiedCoffExports`，验证器只能看到链接参数导出的符号，不能证明内部组件完整。Linux ELF 状态为 `Complete`，同一产物因 IR 声明缺少 `containers` 而以 101 退出；LLVM IR 和 ELF 都含 `xiao_runtime_array_new`。因此旧 Windows 通过不构成完整性证明。

FIX-1 把组件归属映射与 LLVM 控制流扫描集中到 artifact 层。扫描从 `main` 开始跟随可达函数、函数地址和 `br`、`switch`、`indirectbr` 的基本块边，组件仍按实际 Runtime 符号去重并排序；未调用函数及未注册表回调继续排除。此前 helper 只跟随 `br`，遗漏 Cmath 状态检查发出的 `switch` 后路径，数组调用因而没进清单。

扫描器仅覆盖当前生成器的上述控制流边。未来引入`invoke`、`callbr`等指令时，须同时扩展扫描器和“新边后独有Runtime调用”的回归；当前实现不是通用LLVM控制流解析器，维护约束见[FIX.2规划](../../../20o-b2-b4-3-fix2-runtime-components-plan.md)。

FIX-2 的最终原生矩阵在 Windows/Linux 两平台均通过：`std.cmath` 与 `stdlib.cmath` 各跑 O0/O3，四项逐字匹配黄金并捕获 009。两平台声明组件为 `containers/rc/value/weak`；Linux ELF 每格均为 `Complete` 且声明集合等于观察集合。Windows 仍记录 `UnverifiedCoffExports`，不据此声称完整导出表验证。codegen 测试为 109 passed、0 failed；cmath/VM/LLVM/容器 driver 回归为 29 passed、0 failed。

Linux 1.96.0 工具链没有安装 Clippy 组件，Clippy 回执为不可执行，原因原文保留。Windows Clippy 通过。这个工具缺项不改变已通过的 codegen 测试和双平台功能矩阵。

FIX.2 已向 B6-1 交回重跑：B6-1 必须在自己的任务根重跑 Linux 官方包装与相关回归；其既有 `stage_passed=false` / `status=blocked` 历史记录不改写。B6-1 自身通过后才可开始 B6-2；本批没有运行性能、旧 721、macOS 或远端 CI。

任务根分别为 `D:/project/Xiao/.b4/b4-3-fix2-20261118-01/windows` 与 `/home/xiaoczx/.b4/b4-3-fix2-20261118-01`。两平台各自使用独立target和Runtime，最终回放每条命令另用独占临时目录；回执保存PID，日志按SHA-256内容寻址。`result.json`由原始回执生成并保持原字节。r2～r5文档回执与先前收集/封存失败说明保留历史身份；r6因本README新增维护说明的相对链接错误而失败，回执及日志保留。本次口径复核的最终门禁为`docs-final-r7.json`，另绑定B7规划及补充反例脚本。清单由`finish.py seal`按实际字节生成。

补充反例见[审计反例目录](../entry-20261118-b4-component-review/README.md)：清单篡改、缺文件、重算清单后的B6状态/Windows验证等级伪造，以及文件和清单同时改写均须按具体原因拒绝。反例报告在封存后生成于该独立目录。`audit.py`支持`--root <临时副本> --repository-root <仓库>`，只读复核副本字节和固定Git对象；文档回执与封存提交的Git文档快照对照，后续文档更新不改变历史结论。

复核命令：

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261118-b4-component-fix/audit.py
```

本次执行顺序：保留既有`result.json`和原始回执，文档与反例脚本定稿后运行`finish.py docs`，再运行`finish.py seal`。提交证据后运行只读审计及补充目录的`self_negative.py --record`；`prepare`/`capture`仅用于首次归档，不重复执行。
