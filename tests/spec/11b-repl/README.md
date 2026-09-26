# 11B-I0 提示符规格

`prompt.json` 固定 Windows、Linux 和 macOS 的单行提示符纯文本布局。颜色能力以显式参数注入，
由 `cli/ts/src/ui/prompt.test.ts` 验证非 TTY、`NO_COLOR`、`TERM=dumb`、`--color=never`、
24 位色、256 色和 16 色降级；这里不要求测试宿主安装三套操作系统。
