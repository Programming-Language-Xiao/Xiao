# hex 模块

工程期：20O-B1-IMP-REM-A。职责：严格的字节十六进制互转，规范源码为 [hex.xiao](hex.xiao)。
边界：只导出 encode/decode，不新增 intrinsic、ABI、helper、默认参数或方法别名。

| 签名 | 行为 |
| --- | --- |
| encode([int] bytes) -> str | 字节 0～255，小写两位 ASCII，无前缀，最多 65,536 项 |
| decode(str text) -> [int] | 接受大小写，拒绝前缀/空白/非 ASCII，最多 131,072 UTF-8 字节 |

空输入互转。decode 先查字节预算，再查码点数的偶数性，最后从左到右校验字符。
内容错误为 Error / HEX_INVALID_BYTE、HEX_INVALID_LENGTH、HEX_INVALID_CHARACTER；
资源错误为 ResourceError / HEX_LIMIT_EXCEEDED。数组输入只读，返回数组独立，错误无部分输出。

encode 在有界预检后每 128 字节构造一个短块，再成对合并，避免反复复制完整输出；
decode 两次有界顺序扫描，不用宿主编解码或整串替换。接口身份 stdlib.hex，公共别名 std.hex，
接口版本 1。函数体、F9/F10 和三平台证据分开验收，状态保持 planned。

契约见 [B1](../docs/DevDocs/20o-b1-freeze-draft.md#32-hex两项)，教程与完整产品样例见
[字节十六进制](../docs/UseDocs/library/hex.md)。
