# Runtime精确算术

工程期：20O-B2-A2/A3内核与A6资源/性能。

职责：在A1接纳基础上执行有界整数系数运算，保留显示与规范身份，逐工作值核对预算。

模块：text_add以已校验十进制系数线性执行加减并虚拟对齐；common承接A1接纳并核对工作/最终预算；integer与decimal分派精确系数算子；exact_division先约分判有限；quotient/modulus分别验收商余；power/decimal_power执行有界平方求幂；comparison用规范数值位权和虚拟补零比较；float_lift只供算子私有精确提升。

边界：不新增依赖/ABI/intrinsic，A4转换在相邻目录，A5语言接线已独立验收；A6完整语言性能另有独立进程证据，A7整批收口后置。单元测试中的独立整数/成熟大数参考与工作轨迹只证明Runtime内核。
