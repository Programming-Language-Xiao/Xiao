# B4-2A 自审与受控复跑

B4-2A 仍未收口，B4-2B 未启动。本次自审修正测试、取证工具与状态文字；没有修改 Runtime、VM、归档、LLVM、intrinsic、ABI、锁或公开状态。测试源码固定为 `09fa912b45122d0c038a3cb8ad303fdf5078c0da`，相对 `4042aba5` 只改两份 Rust 测试。

自审发现并修正：

- 原 TAC 测试只比调用的部分字段。现在完整比较解码前后 TAC 和编码目录，并校验解码产物、实参类别、float 结果寄存器以及既有版本。
- 原未知 ID 负例只到编码前。现在两种宽度下，49/100/未知 ID 均须由编码器和损坏字节流的两个解码入口按精确原因拒绝。
- 原证据没有 PID、时间、源码前后摘要，路径也有手填转义问题。本目录以全新任务根受控复跑，JSON 和清单均由工具产生；旧记录保留，不补造缺失字段。
- B4-0/1 历史审计仍依赖当前工作树。已改为固定 `c7a45e10` Git tree/blob；默认复跑和仅有 Git 对象的裸仓库复跑均通过。原失败记录保留在 `b40_history.*`，修复后记录为 `b40_fixed_history.*` 与 `b40_bare.*`。原历史日志和 `closeout.json` 未改写。
- 两份 B4-2 规划和索引仍写“尚未开工”，现已同步为“实施中、22c 阻塞”。

本轮结果：字节码包 65/65（含 66 条调用/宽度观测），B4-1 前端 3/3，B3-4 wrapper 8/8；Clippy、fmt 和文档门禁通过。B1/B2/B3/B3-4 固定历史审计通过。三平台、远端 CI、性能和旧 721 F10 未执行。

22c 在新任务根仍为 27/29，源码与锁前后未变，实际日志为 `concurrency.log`。两项失败保持原断言：子 VM 的未使用 cmath 导入成功；planned cmath 的 `.xar` 构建成功。`c7a45e10` 至原实施入口 `4042aba5` 的受保护产品差异只有新增 TAC 测试，因此本轮没有将它们登记为新 VM/归档实现，也没有用固定历史审计代替当前失败。当前 30 条导入后调用探测仍拒绝，不能据此宣称未使用导入和归档边界完整。执行计划禁止 B4-2A 修改 VM/归档；相关阻塞保留，未升级教程或 registry。

所有命令使用全新 `D:\project\Xiao\.b4-2a-review\20261112-09fa912b` 下的 target、Xiao Home 和临时目录，显式绑定仓库 stdlib，清理继承的 `XIAO_*`、`RUST_BIGDECIMAL_*`。本批没有构建或消费原生 Runtime staticlib；`XIAO_RUNTIME_LIBRARY` 指向本任务内未构建的位置。`source.json` 逐路径记录 1049 个受保护文件的 UTF-8/LF 规范摘要，`*.json` 运行回执记录命令、PID、时间、环境、原始日志身份及源码前后摘要。日志按原始字节保留，目录属性禁止 Git 改写换行。

只读复核（从主仓库执行）：

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261112-b4-2a/review-20261112/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261112-b4-2a/review-20261112/audit.py --self-negative
```

默认审计只读证据和固定 Git 对象，不要求当前 HEAD、当前产品文件或本机 target/Runtime 存在。输出 `evidence_valid=true` 只表示记录自洽；`stage_passed=false`、`b4_2a_closed=false` 明确表示阶段仍受阻塞。它支持 `--root` 指向证据副本、`--repository-root` 指向持有固定对象的仓库。

`--self-negative` 仅改临时副本，逐项检查准确拒绝文本、退出码和全部文件的前后摘要：删除属性、篡改记录、篡改日志后重建清单、伪造来源后重建清单、虚假收口，以及历史 B4-0/1 来源篡改。基线故意不带 `-B`，确认默认 Python 调用也不生成缓存。任何无关异常都不算通过。

重新执行应使用新证据目录和新任务根；`record.py init --task-root <新目录>`、`record.py run <标签> -- <命令>` 都拒绝覆盖已有记录。全部命令及文档定稿后，显式运行 `audit.py --collect` 才生成一次实际字节清单；工具或说明更新时，`--refresh-inventory` 会先重新核验原运行和阶段结果，再仅刷新实际文件清单。默认审计和反例不会写清单或阶段结果。运行命令表固定在 `audit.py`，完整原始记录和生成结果在同目录。
