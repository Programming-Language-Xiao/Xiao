# 静态Float/Sfloat相等修复验收

本目录记录星崽批准后的按位比较修复和生成器21→22。ABI维持1.19，标准库接口维持1。本项没有改写B5的1.19/21固定证据；URL默认workspace看门狗、B5关闭状态、三平台与远端CI仍分别保持原状。

## 实施结果

静态比较先按既有共同类型规则转换操作数，再将Float按位bitcast为i64、Sfloat按位bitcast为i32，分别使用icmp eq/ne。该实现保留异号零不相等的Runtime语言语义；整数、布尔、排序和动态Runtime算子不变。生成文本语义变化使CODEGEN_VERSION由21升至22。

永久回归位于driver的20o_static_float_equality.rs及support/float_equality.rs，62组真实函数参数覆盖Float/Sfloat相等、不等、正负零双向、有限值、混合类型提升和舍入边界、整数/布尔、排序及B5原始pow复现。普通workspace执行为2 passed、0 failed、1 ignored；忽略的是需显式外部工具的原生项。专用取证实际启用该测试并完成62组VM及248组静态/打印原生O0/O3：真实llvm-as、构建和执行248/248成功。

同一隔离target还执行了B4-3/B5现行身份、cmath、VM、前端及numeric_range路由相关的五个既有测试文件：19 passed、0 failed、1 ignored。Clippy仅针对xiao-driver全目标运行，结果0警告；workspace Rust fmt检查通过，回执为existing-regressions.json与clippy.json。

版本探针分别链接固定来源的真实生成器21与本批构建的生成器22。真实22校验器接纳21元数据；真实21校验器在入口执行前拒绝22，诊断为X17-XAR-007。元数据的ABI均精确为1.19，标准库接口为1。

缓存矩阵使用同一输入、目标、优化计划、Runtime及工具链，分别测试静态/打印原生O0/O3。每组先由生产ArtifactStore/GlobalIndex写入真实21对象，再以22查询：指纹和请求键变化，旧对象不命中并重建；22随后重复请求的4个控制项均命中同一对象摘要。探针显式编排生产缓存API，不冒称CLI缓存或平台发布验收。

## 执行与回执

任务使用Rust 1.96.0、LLVM 22.1.2、全新的CARGO_TARGET_DIR、TEMP、XIAO_HOME和Runtime输出路径。environment.json记录任务根及环境。record.py逐条记录命令、PID、运行时间、源码前后摘要、Runtime摘要和按内容地址去重的原始输出；失败回执保留，后续运行使用新标签，不覆盖历史日志。

comparisons-default-r3.json是最终默认语言回归，comparisons-native.json是248项显式LLVM/原生结果。初轮version-probes.json保留指纹校准失败；version-matrix-r3.json记录逐条计时的真实21/22矩阵及缓存对象摘要，version-matrix-r2.json保留第一轮成功复跑。comparisons-default-r2/r3.json保留一次混合提升期望的错误校准及修正后的成功回归。其余JSON文件逐条绑定宿主构建、元数据读入和缓存调用。

复核命令：python -B -X utf8 docs/DevDocs/assets/20o-b2-static-float-equality/entry-20261117-fix/audit.py

产品代码提交为cfab9771。早期生成的evidence.json及evidence-final*.json均作为历史快照保留；权威清单为最后文档门禁后的evidence-final-r3.json，不覆盖历史清单。audit.py只读核对本机回执、精确允许路径、真实拒绝原因、源码/工具/Runtime身份与对象摘要；完整审计还将evidence-final-r3.json工作区字节与最终Git提交blob逐字节比较，覆盖清单自引用缺口。最终成功文档回执为docs-gate-final-r11.json，输入含DevDocs和完整产品源码摘要。bin内容寻址对象按二进制文件处理，摘要对应实际归档字节。首次门禁失败与初轮源码校准记录均保留。

Linux/macOS、远端CI、完整B1 721条回归未执行；B5和URL阻塞、22C/22D及19/11/22债务均不因本项自动关闭。
