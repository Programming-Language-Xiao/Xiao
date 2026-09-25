/** Shell 与 CLI 之间的一次性数据文件；不包含可执行内容。 */

import { constants } from "node:fs";
import { lstat, open } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, isAbsolute, resolve } from "node:path";

/** 路径只允许作为单引号包裹的纯数据，拒绝控制字符和引号。 */
export async function requestActivation(environmentPath: string, env: NodeJS.ProcessEnv): Promise<void> {
  const file = env.XIAO_ACTIVATION_FILE;
  if (!file) return;
  const directory = dirname(file);
  if (!isAbsolute(file) || !isAbsolute(environmentPath) || /['\x00-\x1f\x7f]/u.test(environmentPath)
    || !/^xiao-activation\.[A-Za-z0-9]{6,32}$/u.test(basename(directory))
    || !/^activation\.[A-Za-z0-9]{6,32}$/u.test(basename(file))
    || resolve(dirname(directory)) !== resolve(tmpdir())) {
    throw new Error("X11-CLI-ACT-001: 激活路径无效");
  }
  const [folder, destination] = await Promise.all([lstat(directory), lstat(file)]);
  if (!folder.isDirectory() || folder.isSymbolicLink() || !destination.isFile() || destination.isSymbolicLink()
    || (process.platform !== "win32" && (folder.mode & 0o077) !== 0)) {
    throw new Error("X11-CLI-ACT-001: 激活通道不是私有普通文件");
  }
  const handle = await open(file, constants.O_WRONLY | (process.platform === "win32" ? 0 : constants.O_NOFOLLOW));
  try {
    if (!(await handle.stat()).isFile()) throw new Error("X11-CLI-ACT-001: 激活文件不是普通文件");
    await handle.truncate(0);
    await handle.writeFile(`XIAO_ACTIVE_ENV='${environmentPath}'\nexport XIAO_ACTIVE_ENV\n`, "utf8");
  } finally {
    await handle.close();
  }
}
