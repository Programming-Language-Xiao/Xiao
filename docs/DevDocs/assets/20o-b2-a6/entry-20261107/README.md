# A6入口与受控测量记录

当前为实施中，尚无有效收口或性能达标结论。入口`d4ae7877`，A5产品为`f21715d3`。bootstrap.json固定唯一任务根和干净入口，environment.json固定新的target、Runtime、Xiao缓存、临时目录与仓库stdlib。

entry-a5-audit、entry-a5-review、entry-package-tests、entry-a5-contract、entry-b1-contract与entry-b1-resources均实际通过，命令和原始输出分别保存在同名JSON/log。A5默认12项通过、1项原生环境依赖忽略；B1契约9、资源17。

run-step.py保存argv/cwd/PID/起止/退出码、日志与命令前后完整源码摘要，执行中源码变化直接拒绝。Cargo固定1.96.0与--locked，子进程清理RUST_BIGDECIMAL_*；不修改宿主全局环境。

A6样本使用新的输出根与新进程，绑定实际Rust/锁/Runtime/目标和源码内容；五批各预热一次，不把B1历史721条或对象计数当成当前性能/OS峰值。后续失败、缺测和独立债务分别保留，不覆盖旧记录。
