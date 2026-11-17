# B6-1资源、错误与原子性

截至2026-11-18，B6-1处于阻断状态：Windows规定范围通过；Linux Runtime、VM/归档、Cmath原生核心和数值原生回归通过，但Linux真实Cmath包装O0/O3被产物组件完整性校验拒绝；macOS未验证。B6-2性能没有开始，B6-1不签收。最终记录result-final-r5.json明确保持stage_passed=false、status=blocked、b6_closed=false。

本批产品代码入口基线为67319d61，测试起始提交为c32c6be6，Linux原生组件诊断提交为8111d7c5，最终测试提交为a067bd90；各提交只改20o_b2_b6_resources.rs。相对B5受保护源码，最终差异仍仅此一文件。ABI保持1.19、生成器22、标准库接口1；没有改Runtime产品实现、数学规则、资源上限、工作流或历史B5/FIX.1/21→22/URL证据。

Windows使用Rust 1.96.0 x86_64-pc-windows-msvc，全新B6 Runtime静态库SHA-256为fa493d79b129cf7fa600993605381830f6abeaab23ec0ea48c0a910554de0fff。Runtime release单测305 passed、0 failed、1 ignored；正式15项Cmath C ABI逐条执行MPFR/GMP向量共17,243项。A6资源/原子性8/8；B6新语言测试1/1，在std.cmath及stdlib.cmath下均核对15个包装和009/013/033/034捕获、finally、显式除法、巨大商/独立余数及输入保持。VM、归档、错误优先级、数值路由和模块边界回归41 passed、0 failed、3 ignored。Cmath内核参考O0/O3、真实模块包装两别名O0/O3、A5错误/011受控分配O0/O3、lfloat_divide O0/O3以及Clippy均通过。

Linux运行于Ubuntu 26.04 WSL2，Rust/Cargo 1.96.0，Ubuntu clang与LLVM 21.1.8；隔离任务根位于Linux本地/home目录，新Runtime SHA-256为08b88569645eb45d7cc6fea793adbe91a459a174392a91d1b57497d22349ad89。Runtime release单测305 passed、0 failed、1 ignored，17,243项C ABI正式参考通过；VM/归档与B6语言路径、A6资源8/8、Cmath核心O0/O3、A5错误/011及lfloat_divide原生O0/O3通过。

Linux包装原生入口真实退出101，错误为ArtifactVerification发现containers组件。绑定的LLVM文本确实调用xiao_runtime_array_new，ELF确实导出该数组符号，而IR登记只有rc、value、weak；这不是工具缺失或包装黄金失败。独立诊断能链接并执行相同黄金，但绕过FrontendNativeDriver产物完整性校验，不计产品路径通过。此差异落在B4 LLVM Runtime组件清单边界；生成器22冻结在本批有效，B6-1保留阻断并等待所属边界另行处理，未自行绕过或升级版本。

macOS、远端CI、性能五批、OS驻留峰值和完整721未执行。全转义Unicode最大输入196,608字节、输出65,536字节仍是B6-2必测；默认混合/256码点回归不能替代，相关规则见B6规划§2.1。B1原件来源标记、22C/22D及19/11/22债务仍独立。

Linux失败定位：优化版本Runtime组件清单只记rc/value/weak，Cmath真实包装IR还直接调用xiao_runtime_array_new，ELF也包含该符号；因此驱动器拒绝包装产物。独立诊断直接经工具链链接并执行同一黄金，只用于定位，不替代FrontendNativeDriver完整性验收。该边界交回B4/codegen维护，不在B6绕过。

review-final-r8.json记录封存前清单缺失、阶段冒进和Linux失败冒进反例的实际拒绝原因。封存后的文件数错误、清单条目摘要错误和更新清单后偏离固定Git对象三类反例由self_negative.py分别验证；审计不写回证据目录。finish.py在最终docs门禁通过后按实际字节原子重建清单，提交后审计固定Git清单对象。

Windows任务根为D:/project/Xiao/.b6/b6-1-20261117-01；WSL任务根为/home/xiaoczx/.b6/b6-1-20261117-01。每条命令记录PID、UTC时间、命令、完整受保护树前后摘要、工具和Runtime身份、隔离环境与原始输出SHA-256内容对象；复跑日志不覆盖。首次语言回归因finally/catch黄金顺序不符而失败，修正观察顺序后两别名通过；Linux Runtime组件阻断两轮原生包装复跑均保留。

定稿正文和所有结果后，运行完整docs门禁docs-final-r9.json并生成实际字节清单。最终代码及版本身份列于result-final-r5.json；Windows/WSL运行后的测试二进制、Runtime库、LLVM与Xiao临时原生产物摘要列于artifacts-final-r5.json。Linux默认线程数由Cargo仓库配置设为4，本批没有用命令参数覆盖。审核器不运行性能，也不把B6-1阻断提升为通过。

    python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261117-b6/audit.py
    python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261117-b6/self_negative.py
