# URL Unicode预算用例优化

星崽裁定优化测试用例，不调整60秒Driver窗口或冻结预算。改动位于`core/rust/crates/xiao-driver/tests/20o_b1_imp_resources.rs`：主预算用例仍要求16,384个U+1F680码点、输出65,536 UTF-8字节；输入改为16,383个普通Unicode码点加末尾一个等价percent-encoded码点。另新增256个连续百分号编码码点，用来覆盖UTF-8状态复位。产品URL实现、错误顺序、输出预算及工作流阈值不变。

测试提交为`67319d61`。最终受控完整默认workspace使用Rust1.96.0、Cargo锁定模式、隔离target/TEMP/Home，沿用仓库默认4线程，未过滤测试：191个测试目标、1,809个测试用例，其中1,754 passed、0 failed、55 ignored，退出码0，PID70636，耗时625.688秒。`workspace-final.json`绑定命令、PID、起止时间、前后产品树、工具及Runtime/测试二进制摘要；本次使用本任务内已构建的独立缓存，未声称冷构建。

补充运行：默认并行资源目标18/18通过，测试140.30秒（命令140.921秒）；Unicode预算单项测试5.65秒（命令6.405秒）；较早完整workspace耗时622.413秒。这三份日志没有完整PID回执，只作补充，权威默认门禁结论引用最终受控运行。测试负载优化不等于URL产品性能改善，也不是F10成绩；原全转义Unicode最大负载仍保留在F10测量工具中。

B5原有八次直接计时、右删失样本、ABI1.19/生成器21矩阵和失败报告继续作为历史证据，未重写。上述完整workspace绑定当前ABI1.19/生成器22产品；B5仍待阶段closeout，B6尚未开始。

完整workspace、Clippy与fmt均退出0。原始日志位于任务根`D:/project/Xiao/.b5/url-case-opt-20261118-01`；本目录按SHA-256保存去重对象，较大日志使用确定性gzip。`result.json`从原始日志与回执生成统计，`evidence.json`由实际字节生成清单（文件数不含清单自身）。文档定稿后只运行一次完整仓库文档门禁；`docs-final.json`同时绑定七份修改的DevDocs、本README、采集/审计脚本和根层回执的前后字节，避免仅用产品树证明文档身份。

只读复核不依赖当前HEAD或本机临时目录：固定Git对象校验产品源码与当时文档，清单自身另与其最近提交的Git blob核对；审计不重写结果或清单。默认审计核验已归档的本机运行身份，不重新执行测试或检查本机Runtime文件，不能替代三平台/CI验证。

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261117-url-optimization/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261117-url-optimization/self_negative.py
```

自反例在独立临时副本中改写清单、删除已登记文件或伪造统计，逐项核对实际拒绝文本、退出码及审计前后只读性；生成器`finish.py seal`与只读审计分离，不在审计中临时重建清单。

本轮实际采集日期为2026-11-17；本机任务根名称保留url-case-opt-20261118-01仅作唯一目录标识。首次资源测试启动器在测试完成后等待控制台退出，因未捕获其完整退出回执而排除；最终资源回归以resources-final.log为准。
