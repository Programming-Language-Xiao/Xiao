# `xiao-config/src`

放置声明式 `config.xiao` 解析、字段模式、稳定诊断和规范化配置树。对应工程期
05-D、11、11A、11C、18；读取时禁止执行项目代码。

## 文件职责

- `lib.rs`：模块装配和稳定公共重导出，不承载解析状态。
- `model.rs`：不可执行的表、条目和值模型。
- `parser.rs`：Token 流解析和字面量解码。
- `validation.rs`：保留表、严格字段与路径规则。
- `diagnostics.rs`：配置错误编号和结果别名。
- `dependencies.rs`：D1 本地路径依赖声明及运行时/开发期分类提取。

新增配置语义必须先更新 DevDocs 决策与 05-D 交接记录，再增加对应测试和 UseDocs；
禁止把包下载、模块扫描或 Runtime 行为塞入本目录。
