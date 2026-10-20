# Xiao 官方标准库源码

工程期 20C/20K；这里是官方 Xiao 标准库的规范源码根目录。首批 `text` 与 `path` 模块已按冻结接口落地，
`text.find`、`text.slice`、`text.split` 已由 20I 接通，`dict` 的动态读写与开放字典查询已由 20K 接通。

## 职责与边界

- 负责保存官方库源码、模块级 README 和与阶段文档同步的兼容身份记录。
- VM 由工具链专用加载器按固定 `stdlib.` 命名空间提供，不作为普通包写入 `PackageRegistry` 或依赖锁文件；LLVM 将首批纯 Xiao 导入函数静态并入原生入口。
- 随工具链分发的源码根可通过 `XIAO_STDLIB_ROOT` 指向；未设置时使用源码树顶层 `stdlib/`。
- 本目录不放预编译发布对象、`.xar` 字段或普通包配置；预编译对象只允许作为本地构建缓存。
- 每个新增 `.xiao` 模块必须在本目录登记 README、UseDocs、DevDocs 阶段索引和兼容元数据，并由仓库检查器校验路径登记。
- `dict` 模块提供 `get`、`set`、`remove`、`contains`、`keys`、`values` 六个自由函数，动态缺失键保持
  `X06-RUNTIME-015`，不支持新增键。

`stdlib.xiao-standard-library` 的 registry 条目是工具链骨架登记，维持 `draft`；它不适用
`verified` 状态。可交付模块的状态只登记在 `stdlib.xiao-standard-library-first-modules`。

首批模块源码、VM 加载、Windows 本机 LLVM 差分与移除验证已完成；第二批已按探测结果加入 `text.length`；真实 push 运行号和 Linux/macOS 受控复跑仍待补齐后，才能关闭发布验收。
