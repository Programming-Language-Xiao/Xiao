/** 17D 三平台关联设计与双击行为契约测试。 */

import { describe, expect, test } from "bun:test";

import { archiveLaunchContract, fileAssociationPlan, noAssociationPrompt } from "./file-association.ts";

describe(".xar 文件关联契约", () => {
  test("三平台只生成设计，不执行安装副作用", () => {
    for (const platform of ["win32", "linux", "darwin"] as const) {
      const plan = fileAssociationPlan(platform);
      expect(plan.extension).toBe(".xar");
      expect(plan.command).toContain("-xar");
      expect(plan.registrationTarget.length).toBeGreaterThan(0);
      expect(plan.uninstall.length).toBeGreaterThan(0);
    }
  });

  test("双击契约不依赖 cwd，参数、IO 和退出码与命令行一致", () => {
    const contract = archiveLaunchContract("C:/packages/demo.xar", ["--arg"]);
    expect(contract.cwdPolicy).toBe("launcher-directory-independent");
    expect(contract.argv).toEqual(["-xar", "C:/packages/demo.xar", "--arg"]);
    expect(contract.stdio).toBe("inherit");
    expect(contract.exitCode).toBe("forwarded");
  });

  test("未安装关联时的提示保持双语且不承诺自包含 Runtime", () => {
    expect(noAssociationPrompt("zh-CN")).toContain("xiao -xar");
    expect(noAssociationPrompt("en-US")).toContain("Xiao Runtime");
    expect(noAssociationPrompt("zh-CN")).not.toContain("自包含");
  });
});
