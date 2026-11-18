# url 模块

工程期：20O-B1-IMP-REM-B/20O-B1-CLOSE。职责：UTF-8 组件百分号编码，规范源码为 [url.xiao](url.xiao)。
边界：encode_component(str text) -> str 与 decode_component(str text) -> str；
无新增 intrinsic、ABI、helper、路径解析或 Unicode 规范化。

encode 只保留 ASCII 字母数字及 -._~，其余 UTF-8 字节输出大写 %HH；输入最多 65,536 字节，
输出最多 196,608 字节。decode 接受大小写 %HH，保留普通 Unicode 和 +；输入最多
196,608 字节，实际结果最多 65,536 字节。NUL、BOM 和组合字符保留，错误无部分输出。

先查输入预算，再按 B1 §6.1 从左到右处理每个转义：其 %HH 语法先于该字节的 UTF-8 校验，
不为后面的坏转义回头覆盖已发现的 UTF-8 错误。首续字节范围即时拒绝非最短编码、代理和
超范围；未完成序列在 EOF 拒绝。已完成码点再查结果预算，未解码的后续内容不改写首错。
错误为 Error / URL_INVALID_ESCAPE、URL_INVALID_UTF8，资源为 ResourceError / URL_LIMIT_EXCEEDED。

算法用有界顺序扫描、短块和成对归并；不调用宿主 URL/UTF-8 库。接口版本 1，身份 stdlib.url，
公共路径std.url，状态verified。契约见 [B1](../docs/DevDocs/20o-b1-freeze-draft.md#34-url两项)，
教程见 [组件转义](../docs/UseDocs/library/url.md)。

状态升级与实际门禁见[B1-CLOSE收口](../docs/DevDocs/20o-b1-closeout.md)；Runtime ABI1.18、生成器20，产品函数与预算未因状态升级改变。
