# `cli/ts/src/repl`

工程期 11B-I0/I1a/I1b/I1c/I2 的 `session.ts` 处理无参数 `xiao` 的单行提交：使用 CLI 已有的 Rust `run` 协议，
每次绘制提示符前读取生效的 `CLI.git.summary` 配置并在启用时探测 Git。
`keys.ts`、`editor.ts`、`render.ts` 和 `multiline.ts` 提供 I1a/I1c 的 raw mode 多行编辑边界、
字素级缓冲区、覆盖模式、锚点选区、SGR 鼠标和按显示宽度渲染；`commands.ts` 统一剔除
`!outLF!`、`!save!`、`!panel!`、`!ovr!`。
`confirm.ts` 统一绘制确认态和输出分隔线；`output.ts` 追加写出运行结果与耗时、Runtime
峰值对象字节摘要。`multiline.ts` 按编辑、确认、保存、执行四态分派输入，并在执行前后成对切换
raw mode 和终端键鼠协议。`file.ts` 负责 `.xiao` 路径、严格 UTF-8 读取、CR/LF 归一化、
共用原子写及文件诊断；`save.ts` 只维护临时路径输入与保存态渲染，不修改源码缓冲。
I3 的 `panel.ts` 提供单行与多行共用的空面板状态、输入和渲染；会话接线在本批后续步骤完成。
I4 仍未实现；不在 TypeScript 侧实现第二套 Xiao 解析器或执行器，也不执行环境包模块。
