# 11B-I0/I1a/I1c 终端规格

`prompt.json` 固定 Windows、Linux 和 macOS 的单行提示符纯文本布局。颜色能力以显式参数注入，
由 `cli/ts/src/ui/prompt.test.ts` 验证非 TTY、`NO_COLOR`、`TERM=dumb`、`--color=never`、
24 位色、256 色和 16 色降级；这里不要求测试宿主安装三套操作系统。

`multiline.json` 固定空缓冲、恰好填满、软换行、双宽和组合字符的显示布局；按键、编辑、
控制指令与 Kitty 能力规格由 `cli/ts/src/repl` 和 `cli/ts/src/ui/terminal.test.ts`
使用可注入流验证，不要求 CI 提供真实 TTY。

I1c 的覆盖模式、选区、共享编辑器剪贴板、SGR 鼠标反向映射、滚轮忽略和终端状态恢复
由同目录的 TypeScript 单测验证；程序不读取或写入系统剪贴板，也不使用 OSC 52。
