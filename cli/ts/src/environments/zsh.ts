/** 从 Bash 的同一激活协议生成 zsh 钩子，修正数组下标与提示符着色。 */
export function zshShellInitScript(bashScript: string): string {
  return bashScript
    .replace("# xiao shell-init bash", "# xiao shell-init zsh")
    .replaceAll("_xiao_lines[1]", "_xiao_lines[2]")
    .replaceAll("_xiao_lines[0]", "_xiao_lines[1]")
    .replaceAll("\\[\\033[32m\\]", "%F{green}")
    .replaceAll("\\[\\033[0m\\]", "%f");
}
