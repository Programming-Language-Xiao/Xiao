# `cli/ts/src/environments`

放置虚拟环境命令、Shell 激活/取消激活钩子和 `$环境名$` 提示符前缀。工程期 11A；不实现依赖求解。

11A-E2B 的 `activation.ts` 只向随机私有临时文件写入两行激活数据；Bash/PowerShell
钩子在按白名单解析后导出绝对路径，始终删除文件，不执行文件内容。
