# `xiao-package/src`

放置包源适配、联邦索引、依赖求解、锁文件和环境物化实现。对应工程期 11A、16；不执行安装脚本。

D1 首批只实现 `config.xiao` 本地路径依赖的静态读取和包身份图。图节点独立于
`xiao-modules::ModuleGraph`，使用 `source_id`、`alias`、展示名三个分开的来源字段，
并按依赖优先顺序生成内存解析计划；版本约束只作为结构化输入保留。

文件职责：`diagnostics.rs` 保存 `X05-PACKAGE-*` 与 `X05-ENV-*` 编号，`model.rs` 保存包身份/来源/图
模型，`resolver.rs` 只读本地包配置并递归建立图，`environment.rs` 消费规范化配置文档并
生成环境布局、稳定指纹和最小元数据。缓存、锁文件、远程下载、安装脚本和 CLI 命令属于后续工程期。
