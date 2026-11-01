# 官方标准库

20C 登记官方 Xiao 标准库的源码根目录、工具链加载边界和兼容身份。首批模块已经完成源码与 VM 回归；
原生双路径门控需要受控工具链运行，未取得运行号前仍不能把原生侧写成已验证。

模块实现必须同时更新本页索引、模块 README、阶段 DevDocs 和标准库兼容身份。官方库不通过普通包依赖安装。

## B1 教程草案

以下模块均为 `planned`，有 B0 与 FIX-A 的能力/边界探测，尚不可作为官方库导入。每页含已确认 API 契约、
错误/资源边界和被测试的完整示例；通过探测不等于模块已完成。

- [math：整数计算](math.md)
- [hex：十六进制编码](hex.md)
- [base64：Base64 编码](base64.md)
- [url：URL 组件转义](url.md)
- [csv：字段与记录](csv.md)
- [fmt：文本宽度与整数格式](fmt.md)
- [statistics：整数统计](statistics.md)

基线与历史探测见 [B0](../../DevDocs/20o-b0-baseline-and-b1-draft.md)；
精确签名、错误表、资源/性能预算和两个 CI 精确执行点见 [B1 冻结草案](../../DevDocs/20o-b1-freeze-draft.md)。
F1～F13 与内联导出策略已逐条确认；Base64 标准/URL-safe 模式参数必填。
error_code 已由独立 FIX-B 实现并验收（intrinsic 32 / ABI 1.18），证据见 [FIX-B 收口](../../DevDocs/20o-b1-fix-b-closeout.md)。
七库产品仍未实现，七库教程与七库 registry 条目继续保持 planned。

## 首批模块

- [`text`](text.md) —— 字符扫描、码点长度、查找/切片/切分；20M 增加 ASCII 首尾空白剥除、替换、重复与 ASCII 大写转换。
- [`path`](path.md) —— `/` 文本拼接，以及 20M 的末段、父段和扩展文本提取。

## 20D 首批资源接口

- [`os`](os.md) —— 文件/目录资源的 `open_file`、`open_directory`、`close` 与 `read_text`。
- [`set`](set.md) —— 20F/20G 通用集合的八个泛型入口，以及 20H 对应的
  `values.size()` 等静态集合方法式调用。
- [`dict`](dict.md) —— 20K 开放键集字典的 `get`、`set`、`remove`、`contains`、`keys`、`values` 自由函数。

首批与后续小批接口均是标准库接口版本 1 的兼容新增。`text` 的空白集合和大小写转换限定于
文档明确列出的 ASCII 范围，字符操作按 Unicode 码点计数；不提供完整 Unicode 大小写转换、
填充/对齐/进制格式化或数字格式化接口。`path` 只定义 `/` 的文本操作；文件系统访问通过
`os` 模块的冻结资源接口提供。20M 的本批证据见 [实施记录](../../DevDocs/20m-text-and-path-expansion.md)。
