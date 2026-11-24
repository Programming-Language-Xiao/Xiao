# B3-0D 版本、intrinsic、ABI、协议与 Windows COFF 冻结收口

B3-0D 已按星崽六项批准完成冻结。固定产品为 `e79fab3c`，当前产品身份为 ABI 1.19、生成器 22、标准库接口 1。B3-0E 的实现输入为 ABI 1.20、生成器 23、标准库接口 1。15 个新增 intrinsic 使用 49～63，`environment` 使用位 7，旧请求省略能力时默认掩码为 `0x7F`。产品实现、registry、教程 verified 和旧 721 均未开始。[冻结契约](contract-draft.md) · [裁定台账](decisions.json) · [冻结表](freeze-table.json)

版本矩阵要求旧 1.19/22 产物在公共执行入口前拒绝并从源码重生成，缓存按 ABI、生成器、Runtime 内容、目标、能力和标准库身份失配重建。`RunOptions.arguments` 使用 `{program_name, argv}` 快照，新增 hello 能力 `arguments`。C ABI 的 15 个符号、借读输入、输出槽、状态码和能力掩码位置已冻结。[版本矩阵](version-matrix.json) · [C ABI](c-abi.json) · [交接](handoff.json)

Windows 公共和发布产物必须由产品验证器报告 COFF `Complete`。声明组件必须等于观察组件，未分类 Runtime 符号必须为零。真实 COFF 符号表或与产物哈希绑定的受信 linker map/PDB 清单可作为观察来源，导出表-only、同链接候选、未绑定 sidecar 和 `UnverifiedCoffExports` 均拒绝。本批只冻结验收判据，产品状态仍为 `pending`。[COFF 政策](coff-policy.json)

对固定提交 `8c8c5fb1` 中的 B3-0B `cli-import.zip` 做了四份 Windows O0/O3 产物可行性探测。`llvm-readobj` 22.1.2（SHA-256 `768a76a8…99328e`）读到四份产物的 COFF 符号表指针和符号数均为零，导出表可读并包含四个 `xiao_runtime_*` 导出，归档没有 PDB 或 map sidecar。因此这些既有产物仍是 `UnverifiedCoffExports`，探测不构成产品验收。[探测回执](coff-probe.json) · [探测脚本](coff_probe.py)

| 产物 | O0 符号表 | O3 符号表 | 导出表 |
| --- | --- | --- | --- |
| `std` | 指针 0、符号数 0 | 指针 0、符号数 0 | 可读，4 个 Runtime 导出 |
| `stdlib` | 指针 0、符号数 0 | 指针 0、符号数 0 | 可读，4 个 Runtime 导出 |

## 复跑

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261124-b3-0d/coff_probe.py
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261124-b3-0d/seal.py
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261124-b3-0d/self_negative.py
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261124-b3-0d/seal.py
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261124-b3-0d/audit.py
```

审计只读核对固定 Git 对象、B3-0C 产品基线、裁定台账、冻结表、版本矩阵、C ABI、COFF 政策、来源归档内的四份产物、原始工具输出、交接和实际文件清单。自反例包含一条原样基线和八条拒绝用例，覆盖清单篡改、删除 `.gitattributes`、伪造收口状态、伪造裁定/版本/COFF 政策、伪造 COFF 探测观察、伪造负例回执。反例核对完整拒绝文本、退出码和审计前后字节摘要。

文档门禁在收口脚本中执行并记录 `check:docs`、`check:usedocs`、`check:layout` 的原始标准输出、错误输出和摘要。B3-0D 只冻结契约，不把探测状态或历史 `UnverifiedCoffExports` 写成产品验收。
