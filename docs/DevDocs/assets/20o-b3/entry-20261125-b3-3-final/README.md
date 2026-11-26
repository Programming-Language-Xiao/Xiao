# B3-3 fs 内部收口

产品快照 `4e480ebe` 交付 `std.fs` 与 `stdlib.fs` 的五项路径操作。VM/TAC、LLVM 和 C ABI 复用同一套参数、授权、限额与宿主操作规则。生成器按批准升至 25，产品身份为 ABI 1.20、生成器 25、标准库接口 1。[实施规划](../../../20o-b3-3-fs-implementation-plan.md) · [生成器裁定](../entry-20261125-b3-3/generator-decision.json)

| 目标 | Runtime / C ABI | VM | 原生 O0/O3 | 产品组件验证 |
| --- | --- | --- | --- | --- |
| Windows x86_64 MSVC | 8 项 | 2 项 | 4 份教程、2 份权限用例 | 6 份 COFF `Complete` |
| Linux x86_64 GNU | 8 项 | 2 项 | 4 份教程、2 份权限用例 | 6 份 ELF `Complete` |
| macOS arm64 | 7 项 | 2 项 | 4 份教程、2 份权限用例 | 6 份 Mach-O `Complete` |

教程以独立任务目录为 cwd，两个导入别名逐字匹配 B3-0E 黄金，运行后 `demo` 为空。三平台覆盖并发排他写、清理附带原因、非递归删除、末段链接及枚举预算。Linux 专项实测跨卷拒绝，并在维护门禁逐个链接对账 176 个 Runtime 导出。macOS 首轮 APFS 给非法字节文件名返回 `EILSEQ`，修订后由严格解码适配测试补验错误身份。Windows 以不可配对的 UTF-16 代理项测试同一边界。[原始产物与回执](runs.zip) · [整库维护](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45765203599)

三平台用 B3-2 的真实生成器 24 原件验证新校验器的向后兼容。生成器 25 产物在上限为 24 的校验条件下报 `X17-XAR-007`。同一平台使用相同工具链、目标与 Runtime 验证 24→25 的构建指纹差异。[Linux 与 macOS 专项](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45765203601)

归档保留首轮工作区覆盖率、旧编号负例、APFS 夹具和 Linux 单入口组件检查的失败日志。最终工作区门禁与六项维护作业验证相同产品树。全部原始 CI 回执均带有运行、作业和尝试身份。[工作区门禁](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/45765720394)

B3-4 接收 ABI 1.20、生成器 25、标准库接口 1 和已验证的普通宿主 fs。env 是下一个实施批。受限模式延期未排期，完整性能矩阵和公共教程验收归 B3-7。B1 的 4 项实际 Cargo 输出与 24 项派生负例继承自 B3-1，B3-2A 的 28 项模拟记录为历史。仓库登记和教程状态为 `planned`，派生状态见 `closeout.json`。[开工基线](../entry-20261125-b3-3/README.md)

## 只读复核

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261125-b3-3-final/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261125-b3-3-final/self_negative.py --temp-root D:/project/Xiao/.b3-3/20261126-4bea2e43/temp
```

审计读取固定 Git 对象，核对冻结输入、三平台日志、黄金输出、二进制、Windows 链接 map 和 CI 回执。自反例在临时副本中核对拒绝文本、退出码和审计前后字节。归档按 SHA-256 复用相同字节。

`doc-gates.json` 记录文档门禁与定稿 DevDocs、UseDocs 的 Git 树摘要。`self-negative-result.json` 保存自反例回执，`evidence.json` 绑定封存目录中每个文件的字节摘要。[封存入口](seal.py)
