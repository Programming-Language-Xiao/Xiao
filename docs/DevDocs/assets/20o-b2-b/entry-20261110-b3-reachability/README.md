# B3 模块可达性与 planned 边界修复

固定基线为 `0e33e6bf`。本轮只修正模块依赖收集和拒绝断言：从 `program.body` 的导入出发逐个展开可达模块体，完整索引只供查找，访问集合负责导入环去重。函数、表和控制流中的语法导入均保守纳入，不做运行时死分支裁剪。

`before-reachability.log` 保存旧实现的真实反例：未导入模块引用 `std.cmath` 后，正常库的原生降低仍在 `cmath_abs` 失败，退出101。修复后同一反例通过；把该模块改成入口可达时，仍按未接线原语拒绝。正例同时检查项目中间模块、传递依赖、函数内导入、标准库别名、模块顺序及导入环。三个目标的 LLVM 文本均在 Windows 本机生成，不能写成三平台执行证据。

`b3_handoff_rejects_all_planned_language_imports` 对两条路径、15个函数逐条断言未接线错误；任何一条意外成功都令测试失败。B4 接入时必须同时更新这个边界测试和相应可执行语料。

最终源码的执行点由 `codegen-accepted`、`driver-accepted`、`clippy-accepted`、`fmt-accepted`、`final-docs` 五份记录绑定：LLVM 56个单测与4项绑定回归、driver 8项包装及既有模块12项回归、Clippy零警告、格式与仓库门禁。首轮 `final-codegen.log` 的55通过/1失败也保留：新增架构检查误把单测的父模块访问计入生产依赖，修正为只检查生产实现后重新完整运行；`final-clippy.log` 保留布尔表达式简化警告，修改为等价的 `is_none_or` 后重跑。没有删除或忽略失败测试。

执行器 [record-check.py](record-check.py) 保存实际命令、PID、退出码、源码前后字节身份和原始日志摘要；相对固定基线的完整受保护路径差异与未跟踪源码均纳入身份。文件采用 `-text` 保存日志原字节。[audit.py](audit.py) 只读核验清单、执行结果、反例原因和当前源码，不生成或改写 closeout。

复跑审计：

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261110-b3-reachability/audit.py
```

本目录只证明可达性修复及 planned 拒绝回归，**B3-4 独立审计/closeout 仍未交付，cmath 教程与 registry 仍为 planned**；不证明 `std.cmath` 可用，B4 语言接线、三平台和远端 CI 未在本轮执行。
