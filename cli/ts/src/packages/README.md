# `cli/ts/src/packages`

放置包管理命令的交互、进度和诊断显示。工程期 11A、18；核心求解和源优先级由 Rust `xiao-package` 提供。

11A-E2B 的 `index.ts` 只发送一次 `package` 协议请求并输出结果；`install` 和 `i`
共用路由，不在 TypeScript 侧选择目标环境或解析锁文件。远程源和版本求解尚未实现。
