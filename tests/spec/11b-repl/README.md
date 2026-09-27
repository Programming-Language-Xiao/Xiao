# 11B-I0/I1a/I1c 终端规格

`prompt.json` 固定 Windows、Linux 和 macOS 的单行提示符纯文本布局。颜色能力以显式参数注入，
由 `cli/ts/src/ui/prompt.test.ts` 验证非 TTY、`NO_COLOR`、`TERM=dumb`、`--color=never`、
24 位色、256 色和 16 色降级；这里不要求测试宿主安装三套操作系统。

`multiline.json` 固定空缓冲、恰好填满、软换行、双宽和组合字符的显示布局；按键、编辑、
控制指令与 Kitty 能力规格由 `cli/ts/src/repl` 和 `cli/ts/src/ui/terminal.test.ts`
使用可注入流验证，不要求 CI 提供真实 TTY。

I1c 的覆盖模式、选区、共享编辑器剪贴板、SGR 鼠标反向映射、滚轮忽略和终端状态恢复
由同目录的 TypeScript 单测验证；程序不读取或写入系统剪贴板，也不使用 OSC 52。

I1b 的 `confirm.json` 固定窄、中、宽终端的确认标题与独立 `>` 行；`confirm.test.ts`
加载该夹具。追加输出、CRLF、取消、尺寸变化和保留编辑状态由 `output.test.ts`、
`multiline.test.ts` 与 CLI 入口测试覆盖。

I2 的 `save.json` 固定保存标题、路径输入提示符及分隔线的三档宽度；
`save.test.ts` 执行夹具，文件读写、取消、失败回滚与绑定由 `file.test.ts`、
`atomic-write.test.ts`、`multiline.test.ts` 和 CLI 入口测试覆盖。
