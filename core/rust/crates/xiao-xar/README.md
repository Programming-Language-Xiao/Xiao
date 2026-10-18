# `xiao-xar`

## 目录职责

实现 ZIP/ZIP64 兼容的 `.xar` 打包、索引成员、入口定位、依赖闭包、资源和归档安全验证。

## 工程期

17A 已交付物理格式与编解码；17B 已接入显式资源、独立调试/源码对象和稳定资源诊断；17C 已接入归档运行器与 `run_archive` 入口；17D 继续接入平台入口；18 由 CLI 提供
`build -xar` 与 `-xar` 路由；19 做发布和损坏恢复验收。

## 模块放置

ZIP 读写、归档清单、入口解析、成员安全和验证放在 `src/`；平台文件关联放在 `xiao-platform`/CLI。

17A 的公开格式入口是 `encode_xar`、`decode_xar`、`validate_xar`、`list_xar` 和
`XarBuilder`；17B 另提供 `resource_declarations_from_config`、`collect_config_resources`、
`collect_resources`、`append_resource_entries`、`collect_diagnostic_objects`、
`prepare_xiaoc_for_archive` 和 `append_xiaoc_entries`；17C 提供 `run_archive`、
`run_archive_with_event_observer`、`XarRunOptions`、`XarRunError` 以及不含敏感值的
`ArchiveAuditRecord`/`audit_archive`。`XarRunOptions` 的事件容量和显式调试位沿用生产
VM/诊断入口；需要取消/截止时间时使用 `run_archive_with_control`。协议层把审计记录放在
`run_archive` 结果帧的 `audit` 字段。
成功打开后，`XarArchive` 只暴露已经通过结构、索引、摘要和载荷校验的内容。

20E 在入口对象定位前验证标准库接口/语言版本、已安装官方模块和本次请求的能力掩码。
标准库只解析、不初始化；归档不包含官方模块对象。`XarBuilder::with_requirements` 接收身份、
入口可达的官方模块及 intrinsic ID 闭包，并从权威契约生成能力并集；18B 负责实际闭包填充。
未设置版本身份的索引不能编码，缺少新 feature 的旧归档报 `X17-XAR-007`，缺模块报 `008`，
未知能力位报 `011`，能力缺失报 `X06-RUNTIME-026`，均在建立 VM 前拒绝。
执行时继续使用同一请求掩码；未扩展既有归档模块调用或 CLI 构建支持范围。

启动顺序是结构/索引校验 → `validate_archive_context` → 全部载荷校验 → 入口执行。
`inspect_archive_index` 只提供索引与索引层审计，未暴露对象读取接口；它的
`verified_member_count` 为 1，仅表示索引成员已校验。公开 `decode_xar` 仍完整验证全部对象。
编码和解码共用载荷契约：语言版本必须与索引一致，调试激活位和独立调试/源码对象约束也一致。

## 禁止事项

不执行未验证模块，不携带官方 Runtime 依赖，不扫描项目目录或读取未声明文件，不绕过
路径穿越、重复成员、摘要或版本检查。标准归档的 `.xiaoc` 只保留紧凑位置映射，完整
调试符号和源码正文必须由独立选项生成独立对象。
