# `tools/repo-check/src`

## 目录职责

实现仓库完整性检查器的 TypeScript 源码，包括根目录解析、manifest 读取、workspace 交叉校验、README/路径规则、单文件行数与大纲、UseDocs 登记和结构化报告。

## 工程期

A0.2 实现目录与 workspace 规则；A0.4 接入 CI、链接图和退出码。A0 不在此处实现编译器、包求解器或 REPL。

## 模块边界

实现按 `manifest`、`workspace`、`layout`、`size`、`docs`、`commit`、`report` 分模块：`paths` 在既有遍历中
同时提供稳定排序的源文件列表，`size` 执行 2500 物理行门禁、四段旁置豁免校验与按需大纲，
`commit` 锁定提交标题前缀与非空正文，`report` 将大纲详情分别映射到文本、JSON 和 SARIF。
每个模块新增代码时必须同步更新本 README、单元测试和对应 UseDocs 页面。
