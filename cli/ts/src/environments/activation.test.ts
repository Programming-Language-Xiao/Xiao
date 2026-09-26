/** 两套钩子真实运行：只接收两行数据，从不求值文件内容。 */

import { expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, delimiter, join } from "node:path";

import { shellInitScript } from "./index.ts";
import { requestActivation } from "./activation.ts";
import vectors from "../../../../tests/spec/11a-shell/activation.json";

const windows = process.platform === "win32";
const validPath = windows ? "C:\\project\\.venv" : "/home/user/project/.venv";
const commandName = "xiao-activation-probe";
const optionalWindowsBash = process.env.XIAO_TEST_MSYS_BASH;

/** 真实 Shell 测试中可选的命令形状与初始状态。 */
interface ActivationOptions {
  /** 可选的钩子实现；默认使用当前平台的原生 Shell。 */
  shell?: "bash" | "zsh" | "fish" | "powershell";
  /** 传给 xiao 函数的原始参数。 */
  args?: readonly string[];
  /** 已激活环境的初始绝对路径。 */
  initialActive?: string;
  /** 不捕获 PowerShell 失败，用于验证真正的进程退出码。 */
  uncaughtFailure?: boolean;
}

/** 用真实 Shell 调用假的可执行文件，观察钩子是否仅更新当前进程状态。 */
async function checkActivation(contents: string, success: boolean, expected: string | null, options: ActivationOptions = {}): Promise<void> {
  const { shell = windows ? "powershell" : "bash", args = ["sync"], initialActive, uncaughtFailure = false } = options;
  const directory = await mkdtemp(join(tmpdir(), "xiao-hook-test-"));
  const logPath = join(directory, "file-path.txt");
  const hookPath = join(directory, shell === "powershell" ? "hook.ps1" : shell === "fish" ? "hook.fish" : "hook.sh");
  const environment: NodeJS.ProcessEnv = {
    ...process.env,
    PATH: `${directory}${delimiter}${process.env.PATH ?? ""}`,
    XIAO_TEST_CONTENT: Buffer.from(contents).toString("base64"),
    XIAO_TEST_EXIT: success ? "0" : "1",
    XIAO_TEST_PATH_LOG: logPath,
    XIAO_TEST_HOOK: hookPath,
    XIAO_TEST_DIR: directory,
  };
  if (initialActive === undefined) delete environment.XIAO_ACTIVE_ENV;
  else environment.XIAO_ACTIVE_ENV = initialActive;
  delete environment.XIAO_ACTIVATION_FILE;
  try {
    if (shell === "powershell") {
      await writeFile(join(directory, `${commandName}.cmd`), [
        "@echo off",
        "powershell.exe -NoProfile -NonInteractive -Command \"[IO.File]::WriteAllText($env:XIAO_TEST_PATH_LOG,$env:XIAO_ACTIVATION_FILE);[IO.File]::WriteAllBytes($env:XIAO_ACTIVATION_FILE,[Convert]::FromBase64String($env:XIAO_TEST_CONTENT))\"",
        "echo xiao-test-payload",
        "exit /b %XIAO_TEST_EXIT%",
      ].join("\r\n"));
      await writeFile(hookPath, [
        `\uFEFF${shellInitScript("powershell", commandName)}`,
        uncaughtFailure ? `${commandName} ${args.join(" ")}` : `try { ${commandName} ${args.join(" ")}; Write-Output ('SUCCEEDED=' + $?) } catch { Write-Output 'CAUGHT=True' }`,
        "Write-Output ('LASTEXITCODE=' + $LASTEXITCODE)",
        "Write-Output ('ACTIVE=' + $env:XIAO_ACTIVE_ENV)",
        "Write-Output ('FILE=' + $env:XIAO_ACTIVATION_FILE)",
      ].join("\n"));
    } else {
      await writeFile(join(directory, commandName), [
        "#!/usr/bin/env bash",
        "_xiao_log=$XIAO_TEST_PATH_LOG",
        "if command -v cygpath >/dev/null 2>&1; then _xiao_log=$(cygpath -u \"$_xiao_log\"); fi",
        "printf '%s' \"$XIAO_ACTIVATION_FILE\" > \"$_xiao_log\"",
        "bun -e 'require(\"node:fs\").writeFileSync(process.env.XIAO_ACTIVATION_FILE,Buffer.from(process.env.XIAO_TEST_CONTENT,\"base64\"))'",
        "printf 'xiao-test-payload\\n'",
        "exit \"$XIAO_TEST_EXIT\"",
      ].join("\n"), { mode: 0o700 });
      await writeFile(hookPath, shellInitScript(shell, commandName));
    }
    const shellCommand = shell === "fish"
      ? 'source "$XIAO_TEST_HOOK"; xiao ' + args.join(" ") + '; set -l _xiao_status $status; printf "STATUS=%s\\nACTIVE=%s\\nFILE=%s\\n" $_xiao_status "$XIAO_ACTIVE_ENV" "$XIAO_ACTIVATION_FILE"'
      : `${windows ? 'export PATH="/usr/bin:$PATH"; export PATH="$(cygpath -u "$XIAO_TEST_DIR"):$PATH"; source "$(cygpath -u "$XIAO_TEST_HOOK")"' : 'source "$XIAO_TEST_HOOK"'}; xiao ${args.join(" ")}; printf "STATUS=%s\\nACTIVE=%s\\nFILE=%s\\n" "$?" "$XIAO_ACTIVE_ENV" "$XIAO_ACTIVATION_FILE"`;
    const result = shell === "powershell"
      ? spawnSync("powershell.exe", ["-NoProfile", "-NonInteractive", "-File", hookPath], { env: environment, encoding: "utf8", timeout: 20000 })
      : spawnSync(shell === "fish" ? "fish" : shell === "zsh" ? "zsh" : optionalWindowsBash ?? "bash", shell === "fish" ? ["--no-config", "-c", shellCommand] : shell === "zsh" ? ["-f", "-c", shellCommand] : ["--noprofile", "--norc", "-c", shellCommand], { env: environment, encoding: "utf8", timeout: 20000 });
    expect(result.error).toBeUndefined();
    expect(result.status).toBe(uncaughtFailure ? 1 : 0);
    if (uncaughtFailure) expect(result.stderr).not.toBe("");
    else expect(result.stderr).toBe("");
    const output = result.stdout.replaceAll("\r\n", "\n");
    expect(output).toContain("xiao-test-payload\n");
    expect(output).not.toMatch(/^0$/mu);
    if (!uncaughtFailure) {
      expect(output).toContain(`ACTIVE=${expected ?? ""}`);
      expect(output).toContain("FILE=\n");
      expect(output).toContain(shell === "powershell" ? (success ? "SUCCEEDED=True" : "CAUGHT=True") : `STATUS=${success ? 0 : 1}`);
    }
    const activationFile = await readFile(logPath, "utf8");
    expect(existsSync(activationFile)).toBe(false);
    expect(existsSync(dirname(activationFile))).toBe(false);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

for (const vector of vectors.cases) {
  test(`${vector.name} 向量经当前平台 Shell 校验并清理文件`, async () => {
    await checkActivation(vector.content.replaceAll("{{ABS}}", validPath), true, vector.accepted ? validPath : null);
  }, 30000);
}

for (const shell of ["zsh", "fish"] as const) {
  const available = !windows && spawnSync(shell, ["--version"], { encoding: "utf8" }).status === 0;
  test.skipIf(!available)(`${shell} 在安装时复用白名单并可取消激活`, async () => {
    for (const vector of vectors.cases) {
      await checkActivation(vector.content.replaceAll("{{ABS}}", validPath), true, vector.accepted ? validPath : null, { shell });
    }
    await checkActivation("", true, null, { shell, args: ["deactivate"], initialActive: validPath });
  }, 120000);
}

const fishAvailable = !windows && spawnSync("fish", ["--version"], { encoding: "utf8" }).status === 0;
test.skipIf(!fishAvailable)("fish 激活切换后恢复用户原函数，未激活取消无副作用", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-fish-prompt-"));
  const hook = join(directory, "hook.fish");
  try {
    await writeFile(hook, shellInitScript("fish"));
    const command = `function fish_prompt; printf 'user> '; end; source '${hook}'; __xiao_deactivate_environment; printf 'BEFORE=%s\\n' (fish_prompt); __xiao_activate_environment dev 0 /tmp/dev; __xiao_activate_environment test 0 /tmp/test; printf 'ACTIVE=%s\\n' (fish_prompt); __xiao_deactivate_environment; printf 'AFTER=%s\\n' (fish_prompt); functions -q __xiao_original_prompt; or printf 'SAVED=CLEARED\\n'`;
    const result = spawnSync("fish", ["--no-config", "-c", command], { encoding: "utf8", timeout: 20000 });
    expect(result.status).toBe(0);
    expect(result.stdout).toContain("BEFORE=user> ");
    expect(result.stdout).toContain("ACTIVE=$test$ user> ");
    expect(result.stdout).toContain("AFTER=user> ");
    expect(result.stdout).toContain("SAVED=CLEARED");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}, 30000);

test("失败命令也清理文件并保持 Shell 未激活", async () => {
  await checkActivation(`XIAO_ACTIVE_ENV='${validPath}'\nexport XIAO_ACTIVE_ENV\n`, false, null);
}, 30000);

test("前置全局选项能激活、取消激活，帮助请求不会更改环境", async () => {
  const payload = `XIAO_ACTIVE_ENV='${validPath}'\nexport XIAO_ACTIVE_ENV\n`;
  const previous = windows ? "C:\\other\\.venv" : "/home/other/.venv";
  await checkActivation(payload, true, validPath, { args: ["--json", "sync"] });
  await checkActivation(payload, true, null, { args: ["--color=never", "deactivate"], initialActive: validPath });
  await checkActivation(payload, true, previous, { args: ["sync", "--help"], initialActive: previous });
  await checkActivation(payload, true, previous, { args: ["--json", "deactivate", "--help"], initialActive: previous });
}, 90000);

test.skipIf(!windows)("PowerShell 钩子失败时保留非零进程码，成功时不混入数字输出", async () => {
  await checkActivation(`XIAO_ACTIVE_ENV='${validPath}'\nexport XIAO_ACTIVE_ENV\n`, false, null, { uncaughtFailure: true });
}, 30000);

test.skipIf(!windows || !optionalWindowsBash)("可选 MSYS Bash 在 Windows 同样按白名单激活和拒绝注入", async () => {
  await checkActivation(`XIAO_ACTIVE_ENV='${validPath}'\nexport XIAO_ACTIVE_ENV\n`, true, validPath, { shell: "bash" });
  await checkActivation(`XIAO_ACTIVE_ENV='${validPath}'\nexport XIAO_ACTIVE_ENV\nWrite-Output injected\n`, true, null, { shell: "bash" });
  await checkActivation("XIAO_ACTIVE_ENV='relative/.venv'\nexport XIAO_ACTIVE_ENV\n", true, null, { shell: "bash" });
  await checkActivation(`XIAO_ACTIVE_ENV='${validPath}'\nexport XIAO_ACTIVE_ENV\n`, true, validPath, { shell: "bash", args: ["--json", "sync"] });
  await checkActivation(`XIAO_ACTIVE_ENV='${validPath}'\nexport XIAO_ACTIVE_ENV\n`, true, null, { shell: "bash", args: ["--json", "deactivate"], initialActive: validPath });
}, 90000);

test("CLI 写入与 Shell 读取遵循相同白名单", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xiao-activation."));
  const file = join(directory, "activation.12345678");
  try {
    await writeFile(file, "");
    await requestActivation(validPath, { XIAO_ACTIVATION_FILE: file });
    expect(await readFile(file, "utf8")).toBe(`XIAO_ACTIVE_ENV='${validPath}'\nexport XIAO_ACTIVE_ENV\n`);
    for (const invalid of ["relative/path", `${validPath}'bad`, `${validPath}\nextra`]) {
      expect(requestActivation(invalid, { XIAO_ACTIVATION_FILE: file })).rejects.toThrow("X11-CLI-ACT-001");
    }
    const arbitrary = join(directory, "important.txt");
    await writeFile(arbitrary, "keep");
    expect(requestActivation(validPath, { XIAO_ACTIVATION_FILE: arbitrary })).rejects.toThrow("X11-CLI-ACT-001");
    expect(await readFile(arbitrary, "utf8")).toBe("keep");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
