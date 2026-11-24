# B3-0D 版本、intrinsic、ABI、协议与 COFF 冻结契约

本契约记录星崽已批准的 B3-0D 六项裁定。B3-0D 是冻结批，不实现产品代码。当前产品基线保持 ABI 1.19、生成器 22、标准库接口 1，B3-0E 的实现输入为 ABI 1.20、生成器 23、标准库接口 1。[裁定台账](decisions.json) · [冻结表](freeze-table.json) · [版本矩阵](version-matrix.json)

## 15 个 intrinsic

15 个入口使用 49～63。time 为 49～52，args 为 53～54，fs 为 55～59，env 为 60～62，log 为 63。1～48 保持原义，100 保持墓碑。C 符号与参数表见 [C ABI](c-abi.json)。`log.write` 同时需要 `clock` 和 `stdout`，`environment` 使用新增位 7。

## 能力与运行上下文

已知能力掩码为 `0xFF`。旧请求省略能力时使用 `0x7F`，环境访问必须显式授权。授权 CLI 与直接原生入口在应用开始前捕获一次宿主环境，嵌入请求省略时使用空快照。运行内环境写入和删除对同一运行及其 Xiao 并发任务共享，快照只参与执行。

`RunOptions.arguments` 为 `{program_name: str, argv: [str]}`，省略时为空快照。hello 增加 `arguments` 能力。旧核心未声明该能力时在执行用户代码前拒绝。启动器先完成参数编码、NUL 和预算检查，再注入已校验快照。参数快照不进入静态产物或缓存身份。

## 版本与产物

新产物目标为 ABI 1.20、生成器 23、标准库接口 1。`.xiaoc`、`.xar`、索引、IR 和原生元数据格式保持 1.0/1。旧 1.19/22 产物在公共执行入口前拒绝，回归从源码重生成。缓存身份包含 ABI、生成器、Runtime 内容、目标、能力和标准库身份。B2-C 的 1.19/22 只作为历史证据。

## C ABI

新入口采用 `extern "C"`、借读 `const XiaoValue*`、独立初始化的 `XiaoValue* out` 和末尾 `capabilities: u64`。无结果入口省略 `out`。成功返回 0 并替换拥有的旧输出，失败返回 1/2/3/6/8 并保留输出槽。调用期间输入由强引用覆盖，输入、输出和结果不重叠。15 个精确符号和参数顺序以 `c-abi.json` 为准。

## Windows COFF

B3 起 Windows 公共和发布产物必须达到产品验证器 `Complete`。声明组件集合必须等于观察组件集合，未分类 Runtime 符号必须为零。验证器读取保留的真实 COFF 符号表，或读取与产物 SHA-256 绑定的受信 linker map/PDB 完整清单。导出表-only、同链接候选、未绑定 sidecar 和 `UnverifiedCoffExports` 直接拒绝。本批只冻结判据，产品状态保持 `pending`。[COFF 政策](coff-policy.json)

## 边界与交接

B3-0D 不修改产品、intrinsics 登记、VM/LLVM、registry、教程 verified 或旧 721。B3-0E 接收 15 个入口、六模块契约、版本/缓存矩阵、C ABI、arguments 协议和 COFF 政策。受限模式、旧历史债务和三平台产品验收按各自后续批次处理。
