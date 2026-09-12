# `xiao-codegen-llvm`

## 目录职责

把类型化 IR 降低为 LLVM IR，选择目标三元组、链接 Runtime ABI、调试信息和原生产物。性能发布标准是这里生成的 `xiao build` LLVM 原生模式。

## 工程期

10 建立未优化原生闭环；15 接入 `-O0`–`-O3`、Runtime 裁剪、链接和性能基线；平台顺序为 Windows → Linux → macOS。

## 模块放置

IR 降低、目标描述、Runtime ABI、链接、调试信息和目标产物写入 `src/`；`.app`/签名编排由平台与发布工具负责。

## 禁止事项

不在后端改变类型、溢出或错误语义，不打包 `.xar`，不解析终端命令行。
