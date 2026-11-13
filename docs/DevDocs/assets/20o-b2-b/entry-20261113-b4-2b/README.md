# B4-2B：Runtime 门面与 VM 执行

本目录收集 15 个 Rust-only Runtime 门面及 VM 直接原语的 Windows 本机证据。B4-2A 已退出；本批不接 LLVM、cmath 模块导入或归档，不升版本、不跑旧 721。

先登记固定入口与独立 baseline 任务根，复跑既有前端、wrapper、22c 和 TAC；产品实施后使用独立 fixed target。日志以 SHA-256 存一份对象，每次运行保留独立回执。DevDocs 静态文件（含本 README 与工具）以 raw/规范双摘要绑定门禁前后和固定 Git 提交；仅排除本目录运行时回执、日志对象、汇总清单及反例输出。
