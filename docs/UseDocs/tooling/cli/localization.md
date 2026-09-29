---
id: tooling.cli.localization
title: 系统文案语言配置
status: verified
audience: developer
module: rust.xiao-i18n
stage: 11C
version: "0.1.0"
related:
  - README.md
  - protocol.md
  - ../../../DevDocs/11c0-localization-core.md
---

# 系统文案语言配置

本页记录 11C-1 已交付的系统文案本地化行为；语言包插件（L3）、跨平台发布验收（L4）
和 `.xar` 启动入口元数据仍属于后续范围。

在项目根目录的 `config.xiao` 中写入：

```xiao
[language]
locale = "en-US"
```

项目配置优先于全局 `config.xiao`；未声明时使用 `zh-CN`。`zh` 和 `en` 分别规范化为
`zh-CN` 和 `en-US`；节名、键名必须使用小写。也可通过
`xiao config [--global] language.locale en` 写入。未知标签、非字符串和重复键在运行源码前
返回稳定配置错误码，并在机器诊断的 `details` 中指出文件、范围、键和支持的语言。

目前 `xiao run`、无参数 REPL、多行 REPL、测试、构建、环境和包操作为一次请求选择一个语言
上下文，并在协议请求中发送 `locale`。Rust 核心诊断、Runtime 报告、结构化日志和独立
`-debug` 窗口使用同一份内置双语目录；机器字段仍保留原始的 `code`、`message_id`、
`params`、位置与退出码。CLI 帮助、确认、保存、面板、耗时、环境、Shell 钩子、包操作、
构建摘要、测试统计和 Git 降级提示也由独立目录渲染；控制词和用户程序输出不会翻译。

目录回退仍按精确标签、基础语言、内置英语和消息身份/参数执行；目录缺参时会安全地以
原始 `message` 参数重试，不改变机器字段。Rust 目录覆盖已判定的核心消息；测试示例、
LLVM 内部标签、锁文件名等非用户消息明确排除。`.xar` 的实际启动入口尚未实现，不能据此
页面推断归档脱离项目时已经完成本地化。旧核心不返回 `text` 时，CLI 仍保留可读的降级输出。
