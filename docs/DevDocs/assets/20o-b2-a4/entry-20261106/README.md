# B2-A4 转换内核取证

工程期：20O-B2-A4。当前仍在收集最终回归与CI证据，不声明有效收口。

[入口](bootstrap.json)为2abea3f3，包含A2/A3代码5ef17bb2和测试隔离修复13ecd687；A2/A3固定归档只读审计、入口四包、B1契约9/资源17通过。
[执行器](run-step.py)逐命令记录argv/cwd/PID/起止/退出码/原始日志及执行前后源码树，期间源码变化则拒绝。[环境](environment.json)绑定新的target、Runtime路径、Xiao缓存及临时目录；每个Cargo构建/测试/检查固定1.96.0并--locked，清理子进程RUST_BIGDECIMAL_*。

源码输入包含core/rust、stdlib、fixtures、benchmarks、工作流、.cargo、工具、CLI、UseDocs、registry及锁；DevDocs滚动证据不计入该产品输入树。原始CRLF和UTF-8/LF规范摘要并存，证据目录以.gitattributes保留原始字节，不覆盖旧日志或A0/A1/A2/A3归档。

首轮转换组15通过/2失败保留：转str误用不负责接纳的display_text，修正为A1规范渲染后17/17通过。后续补齐lint直达浮点中点和大整数默认scale0显示的独立检查；不是删例换绿。

本批只有Runtime转换：A3私有提升不改，A5完整语言差分、A6性能、A7整批收口后置。ABI1.18/生成器20/intrinsic最大32保持；依赖、XCON、registry不变，cmath及新增教程范围继续planned，旧721条F10不运行，22C/22D和19/11/22债务独立。
