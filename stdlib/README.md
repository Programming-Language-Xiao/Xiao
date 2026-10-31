# Xiao 官方标准库源码

工程期 20C–20M；这里是官方 Xiao 标准库的规范源码根目录。20M 在既有 `text` 与 `path`
模块上增加七项纯 Xiao 接口；接口版本保持 1。`dict` 的动态读写、开放字典查询和方法调用由
20K/20L 接通。

## 职责与边界

- 负责保存官方库源码、模块级 README 和与阶段文档同步的兼容身份记录。
- VM 由工具链专用加载器按固定 `stdlib.` 命名空间提供，不作为普通包写入 `PackageRegistry` 或依赖锁文件；LLVM 将首批纯 Xiao 导入函数静态并入原生入口。
- 随工具链分发的源码根可通过 `XIAO_STDLIB_ROOT` 指向；未设置时使用源码树顶层 `stdlib/`。
- 本目录不放预编译发布对象、`.xar` 字段或普通包配置；预编译对象只允许作为本地构建缓存。
- 每个新增 `.xiao` 模块必须在本目录登记 README、UseDocs、DevDocs 阶段索引和兼容元数据，并由仓库检查器校验路径登记。
- `text` 提供 11 项自由函数：字符扫描、长度、ASCII 大小写、查找/切片/切分，20M 新增
  `trim`、`replace_all`、`repeat`、`upper_ascii`。前后缀判断使用既有接口组合。
- `path` 提供 `join`、`basename`、`dirname`、`extension` 四项纯文本操作，只识别 `/`，
  保留空末段与重复分隔符，不访问文件系统。
- `std.concurrent` 是虚拟原生模块：冻结的 17 项签名位于 `xiao-types/src/concurrency.rs`，
  行为由 VM/TAC 计划和 Runtime 实现提供；不新增 `concurrent.xiao`，不作为普通源码对象
  放入归档。公共接口唯一权威页为 [并发执行与共享句柄](../docs/UseDocs/language/concurrency/README.md)，
  兼容身份沿用工具链标准库接口版本 1 与 Runtime ABI 1.17。
- `dict` 模块提供 `get`、`set`、`remove`、`contains`、`keys`、`values` 六个自由函数，动态缺失键保持
  `X06-RUNTIME-015`，不支持新增键；20L 同时提供八个等价的静态字典方法。

`stdlib.xiao-standard-library` 的 registry 条目是工具链骨架登记，维持 `draft`；它不适用
`verified` 状态。可交付模块的状态只登记在 `stdlib.xiao-standard-library-first-modules`。

20M 冻结探测的真实推送为 `d8a3d32`：维护回归
[41226898352](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41226898352)
三平台原生作业均通过，完整工作区门禁
[41226898301](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41226898301)
通过。本批产品接口正在验证，最终证据由 [20M](../docs/DevDocs/20m-text-and-path-expansion.md)
独立核销；19 发布收口及 11/22 的登记差异仍归各自文档。
