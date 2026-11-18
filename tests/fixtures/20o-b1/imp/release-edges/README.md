# REM-C 原生清理前置向量

工程期：20O-B1-IMP-REM-C。职责：完整复现普通 for 与 selector 的正常/错误/非局部出口。
边界：这是辅助回归，不计入七库或20入口；不修改22D规则、IR释放计划或Runtime ABI。

golden.json 只登记固定 stdout，不从产品算法生成结果。每份源码由默认VM测试、20C既有原生
差分目标（O0/O3）与显式释放诊断共用；后两者逐对象核对销毁顺序，保留完整原始trace。
success/with-bounds 与早期 string-index 复现逐字一致；其余补嵌套循环、continue/break、
临时循环源的return、空循环、索引实参求值失败、多级索引与循环体错误。
