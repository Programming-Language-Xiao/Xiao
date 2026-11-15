# B4-5 整批内部收口证据

工程期：20O-B2-B4-5。入口为`d41b89b7`，B4最终产品与`bb0b8196`的core/stdlib一致，ABI1.18、生成器21、接口1。当前实施中，尚未签收；只修改本批证据与收口文档，产品或测试出问题须记录并退回所属阶段。

全新任务根`D:/project/Xiao/.b4-5/20261115/final`隔离target、Runtime、Home和TEMP，清理继承XIAO_/RUST_BIGDECIMAL_，两个性能开关为0；B1旧721不运行。日志按原始SHA-256只存一个对象，回执保留PID、时间、命令、环境、源码及DevDocs前后摘要。历史证据只读，不改写B4-4/FIX1文件。

本批复跑完整workspace门禁、B1计数守卫28/28、原生O0/O3、真实归档与pow清单；守卫仅模拟计数脚本，不能当三平台产品执行。原生双路径仅承诺选择导入，模块值/限定调用沿B1后置边界。Windows本机结果不冒充Linux/macOS、远端CI、性能、最终公共发布；cmath教程/registry仍planned，B5尚未开始。

完整 workspace 门禁在 `n0_b_dynamic_native::generic_value_copy_registers_weak_dependency_only_when_called` 处失败：实际 `false`、预期 `true`。该失败回执和精确复现均保留；B4-5不修改产品/测试，内部收口暂阻塞并退回原生 lowering 所属阶段。

已通过：独立 Runtime 冷构建、Clippy、fmt、锁、B4-4原生O0/O3、B1 VM/原生、10项历史审计、28/28计数守卫、固定1075项pow逐项对账（Runtime/cmath门面/字节码operator与15个VM载体，6450项）。工具链首次入口错误和pow探针依赖选择错误均保留并以新标签重跑成功。

默认只读审计见 `audit.py`；`collect.py --collect` 才生成 closeout/evidence/self-negative，文档定稿后若需重新绑定最终回执必须显式使用 `collect.py --collect --refresh`，旧清单会保留为 `*-previous.json`。审计固定Git对象、验证所有运行回执、阶段提交、公共planned边界、pow摘要和真实阻塞，不要求当前HEAD等于产品快照。反例核对实际拒绝文本和副本字节不变。

本批没有内部 `language_available=true`：workspace gate 尚未通过，因此不能进入 B4-5C/D 或 B5。Linux/macOS、远端 CI、性能、旧721仍未执行。
