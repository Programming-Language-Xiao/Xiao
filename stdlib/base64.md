# base64 模块

工程期：20O-B1-IMP-REM-A。职责：标准/URL-safe 两种字母表的严格字节编码，规范源码为
[base64.xiao](base64.xiao)。边界：两个冻结入口，无新增 intrinsic、ABI、helper 或默认模式。

| 签名 | 行为 |
| --- | --- |
| encode([int] bytes, bool url_safe) -> str | 0～255 字节，必须指定模式，输出带正确补位且不换行 |
| decode(str text, bool url_safe) -> [int] | 仅接受选定字母表、规范补位和零未使用尾位，返回独立数组 |

输入字节最多 65,536 项，文本最多 87,384 UTF-8 字节，实际解码结果最多 65,536 项。
仅文本长度合法不代表结果合法；最后一组先完整校验字母表/补位/尾位，再检查结果预算。
模式专属字符不可混用，空白/非 ASCII/缺少补位/中间补位均拒绝，空输入互转。
内容错误为 Error / BASE64_INVALID_BYTE、BASE64_INVALID_ENCODING，
超限为 ResourceError / BASE64_LIMIT_EXCEEDED；输入只读，错误无部分输出。

编码用 24 位有界累积，每 64 组生成短块并成对合并；解码两次有界扫描，以常量状态验证四元组。
解码进入循环前把模式解析为两个专用码点，避免逐字符反复读取模式和组合布尔条件；
签名、字母表隔离、错误顺序与全部限额保持冻结契约。
固定身份 stdlib.base64、公共路径 std.base64、接口版本 1。教程继续 planned，
F9/F10、生命周期及三平台验收另行取证。契约见 [B1](../docs/DevDocs/20o-b1-freeze-draft.md#33-base64两项)，
教程见 [Base64 编码](../docs/UseDocs/library/base64.md)。
