/** `.xar` 文件关联的三平台设计与双击行为契约。
 *
 * 本模块只描述安装器将来要写入的注册信息，并生成统一的启动行为；它不修改注册表、
 * `.desktop` 或 `Info.plist`，真实安装/卸载属于发布阶段。
 */

import { cliMessage } from "../i18n.ts";

/** 支持文件关联设计的桌面平台。 */
export type AssociationPlatform = "win32" | "linux" | "darwin";

/** 一个可安装/可移除的关联设计。 */
export interface FileAssociationPlan {
  /** 目标平台。 */
  platform: AssociationPlatform;
  /** 关联扩展名。 */
  extension: ".xar";
  /** MIME 或平台内容类型。 */
  contentType: string;
  /** 设计中的注册位置。 */
  registrationTarget: string;
  /** 将归档交给 xiao 的命令参数。 */
  command: readonly string[];
  /** 设计中的卸载动作。 */
  uninstall: string;
}

/** 双击与命令行共享的启动契约。 */
export interface ArchiveLaunchContract {
  /** 双击启动时不使用桌面当前目录。 */
  cwdPolicy: "launcher-directory-independent";
  /** 传给 xiao 的参数，归档路径位于 `-xar` 后。 */
  argv: readonly string[];
  /** 标准输入、输出和错误流全部继承。 */
  stdio: "inherit";
  /** 退出码原样转发。 */
  exitCode: "forwarded";
}

/** 返回三平台关联设计；调用方只需把结果交给未来安装器。 */
export function fileAssociationPlan(platform: AssociationPlatform): FileAssociationPlan {
  if (platform === "win32") {
    return {
      platform,
      extension: ".xar",
      contentType: "Xiao.Archive",
      registrationTarget: "HKCU\\Software\\Classes\\.xar and Xiao.Archive\\shell\\open\\command",
      command: ["xiao.exe", "-xar", "%1"],
      uninstall: "remove the Xiao.Archive class and the .xar user association",
    };
  }
  if (platform === "linux") {
    return {
      platform,
      extension: ".xar",
      contentType: "application/x-xiao-xar",
      registrationTarget: "~/.local/share/applications/xiao-xar.desktop and shared-mime-info",
      command: ["xiao", "-xar", "%f"],
      uninstall: "remove xiao-xar.desktop and the user MIME override",
    };
  }
  return {
    platform,
    extension: ".xar",
    contentType: "com.programming-language-xiao.xar",
    registrationTarget: "~/Library/LaunchServices association for the exported UTI",
    command: ["xiao", "-xar", "%1"],
    uninstall: "remove the user LaunchServices UTI association",
  };
}

/** 生成双击/命令行共用的行为契约；不把桌面 cwd 当作项目根。 */
export function archiveLaunchContract(archivePath: string, args: readonly string[] = []): ArchiveLaunchContract {
  if (archivePath.trim() === "") throw new Error("归档路径不能为空");
  return {
    cwdPolicy: "launcher-directory-independent",
    argv: ["-xar", archivePath, ...args],
    stdio: "inherit",
    exitCode: "forwarded",
  };
}

/** 未安装关联或没有图形环境时显示的双语安装提示。 */
export function noAssociationPrompt(locale: "zh-CN" | "en-US" = "zh-CN"): string {
  return cliMessage("xiao.cli.archive.no_association", locale);
}
