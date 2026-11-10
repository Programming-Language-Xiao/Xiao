# B4-0/B4-1 向量与前端类型/IR

本批以 B3-4 固定产品提交 `7d8b85e4` 为基线，执行源码向量迁移和 cmath 类型/IR 绑定。独立任务根为 `D:\project\Xiao\.b4-a0-a1\20261110-7d8b85e4`，使用全新 Cargo target、XIAO_HOME 和临时目录；未构建或绑定新 Runtime，不执行 cmath 的 VM/LLVM 调用。

正式参考向量在 B4-0 进入 `tests/fixtures/docs/cmath/reference-vectors.tsv`。它与 DevDocs 来源文件逐字节一致，1,755,726 字节，SHA-256 为 `0e61a2da361448477b15f5508df6d612a3f01e29a06e50699d6ed48a917d1ed3`。来源副本未修改；两处 Runtime 测试只引用共享 fixture。

B4-1 使用既有 34～48 intrinsic 声明和 `cmath_parameters` 生成一元/二元 float 参数签名，经通用调用参数匹配器检查位置/命名绑定，再由既有 lowering 将稳定 ID、实参和源码 span 写入 IR `IntrinsicCall`。本批不把这些调用交给 VM/LLVM 执行。

| 验证 | 结果 |
| --- | --- |
| Runtime/B3-1+B3-3 | `xiao-runtime` 全量 269 passed；包含内核向量与 17,243 条 ABI 向量逐位回归 |
| B4-1前端 | `20o_b2_b4_frontend`：3 passed；15 项位置/命名调用、包装模块 IR、10 类错误及 span |
| B3-4边界 | `20o_b2_b3_wrapper`：8 passed；30 条路径仍按 planned 拒绝 |
| 历史门禁 | B1 18/18、B2 14 检查/4 精确入口、B3-CABI 22 检查及 B3-4 固定 Git 对象审计均复核通过 |
| 版本与公开状态 | ABI 1.18、生成器 20、intrinsic 1～48、ID 100 墓碑、cmath registry/教程 planned |

原始输出、执行 PID、源码前后身份、两个 Cargo.lock 摘要和 fixture 摘要见本目录 JSON/LOG。B4-0/B4-1 不构成 cmath 可执行性或三平台证据；B4-2 起、B5～B7、B2-C 与旧 721 F10 仍后置。

从仓库根目录复核：

```powershell
python -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261110-b4-0-1/audit.py
```

只读审计应返回 `passed=true`、产品提交 `c7a45e10b76b26915499717563761ed20fda6e6c`、1048 个受保护产品文件、`b4_0_1_closed=true` 和 `b4_closed=false`。前两阶段收口不表示 B4 整批结束。逐项运行记录和实际文件字节摘要由 `finalize-evidence.py` 生成，`audit.py` 只读复核且不写证据文件。
