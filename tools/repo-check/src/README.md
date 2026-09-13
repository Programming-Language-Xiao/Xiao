# `tools/repo-check/src`

## 目录职责

实现仓库完整性检查器的 TypeScript 源码，包括根目录解析、manifest 读取、workspace 交叉校验、README/路径规则、UseDocs 登记和结构化报告。

## 工程期

A0.2 实现目录与 workspace 规则；A0.4 接入 CI、链接图和退出码。A0 不在此处实现编译器、包求解器或 REPL。

## 模块边界

实现按 `manifest`、`workspace`、`layout`、`docs`、`report` 分模块；每个模块新增代码时必须同步更新本 README、单元测试和对应 UseDocs 页面。
