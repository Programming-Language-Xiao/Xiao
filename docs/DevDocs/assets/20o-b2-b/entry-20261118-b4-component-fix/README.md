# B4-3 FIX.2 Runtime组件推导修复

FIX.2 已完成，产品提交为 `43f18274175ba09ac84e759999c40cde72eae1c5`。本批只调整 LLVM Runtime 组件推导及必要回归；ABI 保持 1.19、生成器 22、标准库接口 1，未改 Runtime 函数、cmath 语义、registry、教程或工作流。

FIX-0 在未修改代码时复现了平台差异。相同真实包装语料在 Windows 的 `std.cmath`/`stdlib.cmath` O0/O3 通过，但 Windows COFF 符号表状态是 `UnverifiedCoffExports`，验证器只能看到链接参数导出的符号，不能证明内部组件完整。Linux ELF 状态为 `Complete`，同一产物因 IR 声明缺少 `containers` 而以 101 退出；LLVM IR 和 ELF 都含 `xiao_runtime_array_new`。因此旧 Windows 通过不构成完整性证明。

FIX-1 把组件归属映射与 LLVM 控制流扫描集中到 artifact 层。扫描从 `main` 开始跟随可达函数、函数地址和 `br`、`switch`、`indirectbr` 的基本块边，组件仍按实际 Runtime 符号去重并排序；未调用函数及未注册表回调继续排除。此前 helper 只跟随 `br`，遗漏 Cmath 状态检查发出的 `switch` 后路径，数组调用因而没进清单。

FIX-2 的最终原生矩阵在 Windows/Linux 两平台均通过：`std.cmath` 与 `stdlib.cmath` 各跑 O0/O3，四项逐字匹配黄金并捕获 009。两平台声明组件为 `containers/rc/value/weak`；Linux ELF 每格均为 `Complete` 且声明集合等于观察集合。Windows 仍记录 `UnverifiedCoffExports`，不据此声称完整导出表验证。codegen 测试为 109 passed、0 failed；cmath/VM/LLVM/容器 driver 回归为 29 passed、0 failed。

Linux 1.96.0 工具链没有安装 Clippy 组件，Clippy 回执为不可执行，原因原文保留。Windows Clippy 通过。这个工具缺项不改变已通过的 codegen 测试和双平台功能矩阵。

FIX.2 已向 B6-1 交回重跑：B6-1 必须在自己的任务根重跑 Linux 官方包装与相关回归；其既有 `stage_passed=false` / `status=blocked` 历史记录不改写。B6-1 自身通过后才可开始 B6-2；本批没有运行性能、旧 721、macOS 或远端 CI。

任务根分别为 `D:/project/Xiao/.b4/b4-3-fix2-20261118-01/windows` 与 `/home/xiaoczx/.b4/b4-3-fix2-20261118-01`。每条命令使用独立 target、Runtime、临时目录和 PID；日志按 SHA-256 内容寻址。`result.json` 由 `finish.py prepare` 从回执生成。首次完整docs门禁报告通过但收集器封装失败；r2门禁通过后，封存校验发现脚本有后续修正；r3门禁通过后，首次只读审计因规范化辅助函数调用错误而拒绝。r4门禁通过后，封存器拒绝覆盖先前生成的清单；该旧清单对象留在Git历史，新门禁r5通过后将按实际字节原子更新清单。`audit.py`为只读检查。

复核命令：

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261118-b4-component-fix/audit.py
```

执行顺序：`python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261118-b4-component-fix/finish.py prepare`，若存在首次失败的收集尝试则先运行`finish.py capture`，定稿文档后运行 `finish.py docs`，再运行 `finish.py seal`。提交证据后再运行只读审计。
