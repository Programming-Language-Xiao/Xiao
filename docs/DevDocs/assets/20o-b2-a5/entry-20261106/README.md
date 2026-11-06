# A5 语言全链路实施证据

当前为实施中记录，尚不声明A5退出。源码入口`bb7a856e`；独立任务根由`bootstrap.json`、子进程环境由`environment.json`固定。

`run-step.py`保存每条实际argv/cwd/PID/起止时间/退出码/原始日志与前后源码清单；执行期间源码变化即拒绝。所有Cargo命令固定Rust1.96.0、--locked；Runtime、target、缓存与临时输出隔离，清理继承的RUST_BIGDECIMAL_*，不修改宿主全局环境。

五份语言语料的VM结果见`a5-vm-five-fixtures.json`（6通过、1忽略），原生结果见`a5-native-five-fixtures.json`（精确1通过，五份语料各O0/O3）。它们是当前开发快照证据，不代替最终三平台回归。开发失败原始日志完整保留。

编译期值错误边界的实际答复固定在`constant-error-boundary-receipt.json`。A0的原始失败、A1～A4历史归档不改；ABI1.18/生成器20/intrinsic32/XCON边界不变。新数值教程范围与cmath保持planned，旧721条F10未运行。
