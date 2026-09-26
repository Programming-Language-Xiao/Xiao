# `cli/ts/src/repl`

工程期 11B-I0 的 `session.ts` 处理无参数 `xiao` 的单行提交：使用 CLI 已有的 Rust `run` 协议，
每次绘制提示符前读取生效的 `CLI.git.summary` 配置并在启用时探测 Git。
不执行环境包模块；`--inLF`、多行编辑、保存和命令面板留给 I1–I4，
也不在 TypeScript 侧实现第二套 Xiao 解析器或执行器。
