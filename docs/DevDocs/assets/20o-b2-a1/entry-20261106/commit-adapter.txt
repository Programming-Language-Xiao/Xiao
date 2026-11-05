import { readFileSync, writeFileSync, unlinkSync } from "node:fs";
import { resolve } from "node:path";
const a1Source = process.argv[2];
const a1Copy = resolve(process.cwd(), `.a1-commit-message-${process.pid}.tmp`);
const a1Bytes = readFileSync(a1Source);
writeFileSync(a1Copy, a1Bytes, { flag: "wx" });
try {
  if (!readFileSync(a1Copy).equals(a1Bytes)) throw new Error("提交消息复制不一致");
  const a1Result = Bun.spawnSync(["bun", "tools/repo-check/src/cli.ts", "commit-msg", "--file", a1Copy], {
    cwd: process.cwd(), stdout: "inherit", stderr: "inherit",
  });
  process.exitCode = a1Result.exitCode;
} finally {
  unlinkSync(a1Copy);
}
