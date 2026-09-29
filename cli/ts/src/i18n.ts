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
  "xiao.cli.status.success": { "zh-CN": "成功", "en-US": "success" },
  "xiao.cli.status.label": { "zh-CN": "状态", "en-US": "status" },
  "xiao.cli.status.exit_code": { "zh-CN": "退出码", "en-US": "exit code" },
  "xiao.cli.test.summary": { "zh-CN": "测试结果  {passed}/{total} 通过，失败 {failed}", "en-US": "test results  {passed}/{total} passed, failed {failed}" },
  "xiao.cli.test.passed": { "zh-CN": "通过", "en-US": "passed" },
  "xiao.cli.test.failed": { "zh-CN": "失败", "en-US": "failed" },
  "xiao.cli.build.artifact": { "zh-CN": "产物  {path}", "en-US": "artifact  {path}" },
  "xiao.cli.build.fingerprint": { "zh-CN": "指纹  {value}", "en-US": "fingerprint  {value}" },
  "xiao.cli.build.debug": { "zh-CN": "调试  {path}", "en-US": "debug  {path}" },
  "xiao.cli.build.diagnostics": { "zh-CN": "诊断  {path}", "en-US": "diagnostics  {path}" },
  "xiao.cli.build.config": { "zh-CN": "配置  {path}", "en-US": "config  {path}" },
  "xiao.cli.env.created": { "zh-CN": "已创建环境 {name}：{path}", "en-US": "created environment {name}: {path}" },
  "xiao.cli.env.activation_missing": {
    "zh-CN": "未检测到激活钩子；可手工将 XIAO_ACTIVE_ENV 设为以上绝对路径，或先在 Bash/zsh/fish/PowerShell 初始化对应的 shell-init 钩子。",
    "en-US": "no activation hook detected; set XIAO_ACTIVE_ENV to the absolute path above, or initialize the shell-init hook for Bash/zsh/fish/PowerShell first.",
  },
  "xiao.cli.shell.hook.installed": { "zh-CN": "已安装 Shell 钩子：{profile}", "en-US": "installed shell hook: {profile}" },
  "xiao.cli.shell.hook.removed": { "zh-CN": "已移除 Shell 钩子：{profile}", "en-US": "removed shell hook: {profile}" },
  "xiao.cli.shell.hook.unchanged": { "zh-CN": "无需更改 Shell 钩子：{profile}", "en-US": "shell hook unchanged: {profile}" },
  "xiao.cli.shell.no_backup": { "zh-CN": "未改写既有文件，无备份。", "en-US": "existing file unchanged; no backup created." },
  "xiao.cli.shell.backup": { "zh-CN": "备份：{path}", "en-US": "backup: {path}" },
  "xiao.cli.package.activation_missing": {
    "zh-CN": "（未检测到激活钩子；可手工设置 XIAO_ACTIVE_ENV 为以上绝对路径，或在 Bash/zsh/fish/PowerShell 初始化 shell-init 钩子）",
    "en-US": " (no activation hook detected; set XIAO_ACTIVE_ENV to the absolute path above, or initialize the shell-init hook for Bash/zsh/fish/PowerShell)",
  },
  "xiao.cli.package.sync": { "zh-CN": "已同步：{path}", "en-US": "synced: {path}" },
  "xiao.cli.package.install": { "zh-CN": "已安装：{path}", "en-US": "installed: {path}" },
  "xiao.cli.package.lock": { "zh-CN": "已锁定：{path}", "en-US": "locked: {path}" },
  "xiao.cli.package.update": { "zh-CN": "已更新锁文件：{path}", "en-US": "lock file updated: {path}" },
  "xiao.cli.package.add": { "zh-CN": "已添加依赖：{path}", "en-US": "dependency added: {path}" },
  "xiao.cli.package.remove": { "zh-CN": "已移除依赖：{path}", "en-US": "dependency removed: {path}" },
  "xiao.cli.config.updated": { "zh-CN": "已更新 {path}：{key} = {value}", "en-US": "updated {path}: {key} = {value}" },
  "xiao.cli.repl.git.degraded": { "zh-CN": "Git 摘要已降级（{reason}）", "en-US": "Git summary degraded ({reason})" },
} as const;

/** CLI 自身消息的身份，不与 Rust 核心消息身份重名。 */
export type CliMessageId = keyof typeof messages;

/** CLI 消息的安全文本参数；参数值不参与二次模板解析。 */
export type CliMessageParams = Readonly<Record<string, string | number>>;

/** 只呈现 CLI 自己的界面文本；无上下文的独立渲染沿用原英文界面。 */
export function cliMessage(
  id: CliMessageId,
  locale: SupportedLocale = "en-US",
  params: CliMessageParams = {},
): string {
  return messages[id][locale].replace(/\{([A-Za-z0-9_]+)\}/gu, (placeholder, name: string) => (
    Object.prototype.hasOwnProperty.call(params, name) ? String(params[name]) : placeholder
  ));
}
