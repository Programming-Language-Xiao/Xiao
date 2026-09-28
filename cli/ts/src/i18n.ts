/** CLI 自身界面消息目录；Rust 核心的错误由协议 text 渲染。 */
import type { SupportedLocale } from "./config/locale.ts";

const messages = {
  "xiao.cli.help": {
    "zh-CN": [
      "xiao 0.1.0",
      "用法：",
      "  xiao run <file.xiao> [-debug] [--json] [--color=auto|always|never]",
      "  xiao <file.xiao> [-debug]                运行源码快捷方式",
      "  xiao config [--global] <key.path> <value>",
      "  xiao test [project] [--timeout <ms>]     运行项目 tests/**/*.xiao",
      "  xiao build -o <output> <file.xiao> [-debug] [--emit-llvm <path>] [--json]",
      "  xiao venv [name]                         创建项目环境并输出激活提示",
      "  xiao sync [--keep-extra] [--locked|--frozen] 同步依赖并激活环境",
      "  xiao install [project-or-config-path]    安装已有锁文件到激活或全局环境（别名：i）",
      "  xiao lock                              创建锁文件或核对现有锁文件，不动环境",
      "  xiao update                            显式重解并更新锁文件，不动环境",
      "  xiao add <package> --path <path> [--version <range>] [--dev]  添加本地依赖并重新锁定",
      "  xiao remove <package> [--dev]           删除依赖并重新锁定",
      "  xiao shell-init <bash|zsh|fish|powershell|cmd> [--install|--uninstall] [--profile <绝对路径>]",
      "  xiao deactivate                          取消当前 Shell 环境激活",
      "  xiao --inLF [file.xiao]                  多行编辑或打开源码文件",
      "  xiao --help | --version",
      "",
      "无参数 xiao 启动单行交互会话；--inLF [file.xiao] 进入多行编辑。",
    ].join("\n") + "\n",
    "en-US": [
      "xiao 0.1.0",
      "Usage:",
      "  xiao run <file.xiao> [-debug] [--json] [--color=auto|always|never]",
      "  xiao <file.xiao> [-debug]                Run a source file directly",
      "  xiao config [--global] <key.path> <value>",
      "  xiao test [project] [--timeout <ms>]     Run tests/**/*.xiao in the project",
      "  xiao build -o <output> <file.xiao> [-debug] [--emit-llvm <path>] [--json]",
      "  xiao venv [name]                         Create a project environment and show activation instructions",
      "  xiao sync [--keep-extra] [--locked|--frozen] Sync dependencies and activate the environment",
      "  xiao install [project-or-config-path]    Install an existing lock file (alias: i)",
      "  xiao lock                              Create or verify the lock file without changing the environment",
      "  xiao update                            Resolve again and update the lock file",
      "  xiao add <package> --path <path> [--version <range>] [--dev]  Add a local dependency and relock",
      "  xiao remove <package> [--dev]           Remove a dependency and relock",
      "  xiao shell-init <bash|zsh|fish|powershell|cmd> [--install|--uninstall] [--profile <absolute-path>]",
      "  xiao deactivate                          Deactivate the current shell environment",
      "  xiao --inLF [file.xiao]                  Edit multiple lines or open a source file",
      "  xiao --help | --version",
      "",
      "Run xiao without arguments for an interactive prompt; --inLF [file.xiao] opens the multiline editor.",
    ].join("\n") + "\n",
  },
  "xiao.cli.repl.confirm": { "zh-CN": "按 Enter 确认并运行 ↩︎", "en-US": "Press Enter to confirm and run ↩︎" },
  "xiao.cli.repl.panel": { "zh-CN": "命令面板", "en-US": "Command Panel" },
  "xiao.cli.repl.save": { "zh-CN": "输入保存位置", "en-US": "Enter the save location" },
  "xiao.cli.repl.save.invalid_path": {
    "zh-CN": "X11-CLI-SAVE-001: 路径不能包含控制或分隔字符",
    "en-US": "X11-CLI-SAVE-001: Paths cannot contain control or separator characters",
  },
  "xiao.cli.repl.summary.time": { "zh-CN": "时间", "en-US": "time" },
  "xiao.cli.repl.summary.memory": { "zh-CN": "内存", "en-US": "memory" },
} as const;

/** CLI 自身消息的身份，不与 Rust 核心消息身份重名。 */
export type CliMessageId = keyof typeof messages;

/** 只呈现 CLI 自己的界面文本；无上下文的独立渲染沿用原英文界面。 */
export function cliMessage(id: CliMessageId, locale: SupportedLocale = "en-US"): string {
  return messages[id][locale];
}
