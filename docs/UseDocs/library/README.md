# 官方标准库

20C 登记官方 Xiao 标准库的源码根目录、工具链加载边界和兼容身份。已交付模块随工具链提供；每个模块的已验证平台和范围以对应教程及运行记录为准。

模块实现必须同时更新本页索引、模块 README、阶段 DevDocs 和标准库兼容身份。官方库不通过普通包依赖安装。

## B1 七库

以下七个模块及教程已登记为verified，共20个公开入口。完整示例直接导入真实产品，
核对稳定错误与代表性边界；各项资源上限和性能由专门回归补齐。B0探测保留为历史附录。

- [math：整数计算](math.md)
- [hex：十六进制编码](hex.md)
- [base64：标准与URL-safe编码](base64.md)
- [url：UTF-8组件转义](url.md)
- [csv：严格单记录与字段](csv.md)
- [fmt：码点填充与整数格式](fmt.md)
- [statistics：整数统计](statistics.md)

适用Xiao0.1.0、Runtime ABI1.18、标准库接口版本1；公共路径`std.<module>`，固定身份`stdlib.<module>`。
error_code已由[独立FIX-B](../../DevDocs/20o-b1-fix-b-closeout.md)实现并验收。
精确签名、错误和资源预算见[冻结契约](../../DevDocs/20o-b1-freeze-draft.md)，
三平台产品、归档/包隔离和生成器20释放证据见[收口记录](../../DevDocs/20o-b1-closeout.md)。
固定源码身份的[完整F10](../../DevDocs/assets/20o-b1-imp/rem-identity-final-41464439400/README.md)共721条；
它记录已测硬件条件，旧报告仅作历史快照。当前状态与新门禁运行号统一在收口记录登记。

## B2数值基础与后续库

[cmath浮点数学](cmath.md)的契约已冻结，教程与登记仍planned，尚无产品；与显式精度除法lfloat_divide一起留B2-B实施。
lint/lfloat是语言数值基础，不另造一个大数标准库；B2-A内部1.18/20能力及最终发布planned范围见[转换说明](../language/basics/types/conversions.md)。

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
完整Unicode字素或locale接口。码点填充与整数进制格式由B1的fmt提供。`path` 只定义 `/` 的文本操作；文件系统访问通过
`os` 模块的冻结资源接口提供。20M 的本批证据见 [实施记录](../../DevDocs/20m-text-and-path-expansion.md)。
