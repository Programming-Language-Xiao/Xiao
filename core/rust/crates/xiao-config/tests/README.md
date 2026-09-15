# `xiao-config/tests`

这里放置 05-D 配置声明子集的规格测试。测试只通过 `xiao-config` 的公开接口
验证配置树、白名单、路径约束和不可执行边界，不调用 Runtime、模块扫描或包管理器。

对应工程期：05-D。新增配置语义必须先更新 `docs/DevDocs/00-decisions.md`、
05-D 交接文档和 UseDocs，再在本目录增加正反例。
