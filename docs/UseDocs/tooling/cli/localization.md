---
id: tooling.cli.localization
title: 系统文案语言配置（接入中）
status: draft
audience: developer
module: rust.xiao-i18n
stage: 11C
version: "0.1.0"
related:
  - README.md
  - protocol.md
  - ../../../DevDocs/11c0-localization-core.md
---

# 系统文案语言配置（接入中）

本页记录 11C 当前已接入的行为，**不是完整双语交付声明**。

在项目根目录的 `config.xiao` 中写入：

```xiao
[language]
locale = "en-US"
```

项目配置优先于全局 `config.xiao`；未声明时使用 `zh-CN`。`zh` 和 `en` 分别规范化为
`zh-CN` 和 `en-US`；节名、键名必须使用小写。也可通过
`xiao config [--global] language.locale en` 写入。未知标签、非字符串和重复键在运行源码前
返回稳定配置错误码，并在机器诊断的 `details` 中指出文件、范围、键和支持的语言。

目前 `xiao run`、无参数 REPL 和多行 REPL 为一次运行选择一个语言上下文，并在 `run` 请求中
发送 `locale`。Rust 核心为已入目录的诊断附加本地化 `text`，同时保留原始的 `code`、
`message_id`、`params`、`message` 与退出码。英文帮助、确认、保存、面板和运行耗时标签已接入
CLI 自己的消息目录；控制词 `!save!`、`!panel!`、`!outLF!`、`!inLF!`、`!ovr!`
以及用户程序输出不会翻译。

仍在进行：Rust 的完整双语目录和其余 CLI/包管理器/构建/测试文案、日志与调试窗口渲染、
构建产物的语言元数据。当前 Rust 目录只有基础示例条目；英文遇到缺失条目时会显示稳定的
消息身份与参数，而非完整英语错误描述。`.xar` 的实际启动入口尚未实现，不能据此页面
推断归档脱离项目时已经完成本地化。旧核心不返回 `text` 时，CLI 仍保留可读的降级输出。
