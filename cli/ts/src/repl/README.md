# `cli/ts/src/repl`

工程期 11B-I0/I1a 的 `session.ts` 处理无参数 `xiao` 的单行提交：使用 CLI 已有的 Rust `run` 协议，
每次绘制提示符前读取生效的 `CLI.git.summary` 配置并在启用时探测 Git。
`keys.ts`、`editor.ts`、`render.ts` 和 `multiline.ts` 提供 I1a 的 raw mode 多行编辑边界、
字素级缓冲区和按显示宽度渲染；`commands.ts` 统一剔除 `!outLF!`、`!save!`、`!panel!`。
I1b 的运行确认/完整缓冲区执行、I2/I3/I4 仍未实现；不在 TypeScript 侧实现第二套
Xiao 解析器或执行器，也不执行环境包模块。
