# csv 模块

工程期：20O-B1-IMP-REM-B。职责：严格逻辑单记录与字符串字段互转，规范源码为 [csv.xiao](csv.xiao)。
边界：parse_record(str record) -> [str]、format_record([str] fields) -> str；
不做全文 CSV、表头、方言、流式、类型推断或任何新增 intrinsic/ABI/helper。

逻辑记录不带结束符。解析四状态分别为字段首、未引号字段、引号内、读到引号；后者只接受
转义双引号、逗号或 EOF。引号外 CR/LF、错误引号位置及未闭引号为 Error / CSV_INVALID_RECORD。
空记录为一个空字段，format_record([]) 为 Error / CSV_EMPTY_FIELDS。BOM、空白、NUL、
引号内 CR/LF/CRLF 和 Unicode 均不改写；仅含逗号/引号/CR/LF 的字段才加引号并双写内部引号。

记录最多 65,536 UTF-8 字节、1～1,024 字段、每字段解码后最多 16,384 字节。
format 先校验字段数/单字段/原始总量 S≤65,536，再计双引号 Q 和需引号字段 W，
在构造结果前检查 E=S+Q+2W+(k-1)≤65,536。所有超限为 ResourceError / CSV_LIMIT_EXCEEDED。
输入只读、结果数组独立、失败无部分输出；解析按状态机位置保留首错，逗号引入第 1,025 字段时即拒绝。

输出用有界短块和成对归并，不反复追加完整结果；辅助逻辑全部内联。接口版本 1，
固定身份 stdlib.csv，公共路径 std.csv，教程保持 planned。资源/性能和生命周期另行验收，
契约见 [B1](../docs/DevDocs/20o-b1-freeze-draft.md#35-csv两项已确认记录子集)，
教程见 [字段与记录](../docs/UseDocs/library/csv.md)。
