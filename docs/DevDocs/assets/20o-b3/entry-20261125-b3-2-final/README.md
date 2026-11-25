# B3-2 args/cli 原生与三平台验收

星崽批准将生成器从 23 升至 24，接通 args 原生调用与启动快照、cli 数组形参检查，并复验版本与缓存身份。产品身份为 ABI 1.20、生成器 24、标准库接口 1。[补充裁定](decision.json) · [B3-2A 开工基线](../entry-20261125-b3-2/README.md)

args/cli 从程序入口可达时，原生启动适配器在执行用户代码前严格检查宿主参数。标准库的两个 args 原语与并发子任务读取同一快照。直接嵌入的 C 入口默认使用空快照。`split_terminator` 在规范模块入口校验数组形参，后续预算与分组由 Xiao 源码执行。链接映射和 PE 映像按字节绑定，Windows 验收以产品验证器的 COFF `Complete` 为准。[实施规划](../../../20o-b3-2-args-cli-implementation-plan.md)

在固定提交和独立任务根运行采集器。它逐字核对双导入、原生 O0/O3 的正式教程黄金，记录旧 23 产物的元数据读取、新 24 产物对 23 上限的拒绝，以及同一 Runtime 下的新旧构建指纹。每份日志按 SHA-256 存一次。Windows 用 clang 的实际编译和链接轨迹绑定 MSVC 目标、优化等级与完整映射。[采集器](run.py) · [Runner 模板](runner.yml)

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261125-b3-2-final/run.py --output D:/project/Xiao/xiao-b3-2-verification
```

三平台专项固定于 `879ce625`。每个平台执行 7 项 VM、5 项 Runtime/C ABI、8 份原生教程、4 份原生边界和 4 项启动拒绝。Windows 12 份产物以显式 MSVC 目标构建，产品验证器均报告 COFF `Complete`。Linux/macOS 专项和工作区门禁使用同一产品快照。[Linux/macOS 专项](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45648610239) · [工作区门禁](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45648595869)

维护门禁的六个作业均在 `2aae3a74` 通过。该提交只修改 IR 单测，审计逐字比较 `#[cfg(test)]` 之前的生产代码。归档保留历史运行的原始 API 回执与首轮 Windows 编译失败日志。失败涉及原生并发测试签名、已激活编号的旧负例、协议所有权边界和运行时检查名称表。[维护门禁](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45650370637)

三平台均读取了 B3-1 的生成器 23 原件。生成器上限为 23 时，24 产物以 `X17-XAR-007` 拒绝。相同 Runtime 和 LLVM 工具下，23 与 24 的原生构建指纹不同。源码、字节码与归档的合成快照经实际协议帧验证，执行前后 `.xiaoc` 与 `.xar` 原件字节一致。

B1 守卫沿用 B3-1 的 4 项实际 Cargo 输出和 24 项派生负例。B3-2A 独立 Windows 回执有 28 项模拟和 0 项实际 Cargo 输出。完整性能矩阵及教程和 registry 验收属于 B3-7。[B3-2A 守卫来源](../entry-20261125-b3-2/README.md)

从主仓库直接复跑。两条命令均只读，依据固定 Git 对象、平台原件与原始日志。自反例核对原样通过、逐条拒绝文本和审计前后摘要。文档门禁绑定定稿 DevDocs/UseDocs 的 Git 树摘要。

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261125-b3-2-final/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261125-b3-2-final/self_negative.py
```

`closeout.json` 由实测回执及自反例派生 B3-2 的结论和 B3-3 的入口。`evidence.json` 依据最终字节生成，包含 `.gitattributes`。`runs.zip` 中每个日志或产物摘要只存一份。[只读审计](audit.py) · [自反例](self_negative.py) · [封存入口](seal.py)
