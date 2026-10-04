# `xiao-xar`

## 目录职责

实现 ZIP/ZIP64 兼容的 `.xar` 打包、索引成员、入口定位、依赖闭包、资源和归档安全验证。

## 工程期

17A 已交付物理格式与编解码；17B 已接入显式资源、独立调试/源码对象和稳定资源诊断；17C–17D 继续接入运行器和平台入口；18 由 CLI 提供
`build -xar` 与 `-xar` 路由；19 做发布和损坏恢复验收。

## 模块放置

ZIP 读写、归档清单、入口解析、成员安全和验证放在 `src/`；平台文件关联放在 `xiao-platform`/CLI。

17A 的公开格式入口是 `encode_xar`、`decode_xar`、`validate_xar`、`list_xar` 和
`XarBuilder`；17B 另提供 `resource_declarations_from_config`、`collect_config_resources`、
`collect_resources`、`append_resource_entries`、`collect_diagnostic_objects`、
`prepare_xiaoc_for_archive` 和 `append_xiaoc_entries`。
成功打开后，`XarArchive` 只暴露已经通过结构、索引、摘要和载荷校验的内容。

## 禁止事项

不执行未验证模块，不携带官方 Runtime 依赖，不扫描项目目录或读取未声明文件，不绕过
路径穿越、重复成员、摘要或版本检查。标准归档的 `.xiaoc` 只保留紧凑位置映射，完整
调试符号和源码正文必须由独立选项生成独立对象。
