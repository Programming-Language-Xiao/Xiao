# B3-2 args/cli 原生与三平台验收

星崽批准将生成器从 23 升至 24，接通 args 原生调用与启动快照、cli 数组形参检查，并复验版本与缓存身份。产品身份为 ABI 1.20、生成器 24、标准库接口 1。[补充裁定](decision.json) · [B3-2A 开工基线](../entry-20261125-b3-2/README.md)

原生主入口在执行用户代码前严格检查宿主参数。标准库的两个 args 原语与并发子任务读取同一快照。直接嵌入的 C 入口默认使用空快照。`split_terminator` 在规范模块入口校验数组形参，后续预算与分组由 Xiao 源码执行。链接映射和 PE 映像按字节绑定，Windows 验收以产品验证器的 COFF `Complete` 为准。[实施规划](../../../20o-b3-2-args-cli-implementation-plan.md)

在固定提交和独立任务根运行采集器。它逐字核对双导入、原生 O0/O3 的正式教程黄金，记录旧 23 产物的元数据读取、新 24 产物对 23 上限的拒绝，以及同一 Runtime 下的新旧构建指纹。每份日志按 SHA-256 存一次。Windows 用 clang 的实际编译和链接轨迹绑定 MSVC 目标、优化等级与完整映射。[采集器](run.py) · [Runner 模板](runner.yml)

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261125-b3-2-final/run.py --output D:/project/Xiao/xiao-b3-2-verification
```
