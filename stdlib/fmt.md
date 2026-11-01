# `fmt` 模块

工程期：20O-B1-IMP。目录职责：提供 Unicode 码点填充与固定 int 的进制格式化，
规范源码为 [fmt.xiao](fmt.xiao)。无新增 intrinsic、ABI、helper、locale 或浮点格式化。

| 签名 | 行为 |
| --- | --- |
| `pad_left(str text, int width, str fill) -> str` | 左侧补足码点宽度，原文不截断 |
| `pad_right(str text, int width, str fill) -> str` | 右侧补足码点宽度，原文不截断 |
| `format_int(int value, int radix) -> str` | 2～36 进制、小写无前缀、负号最前，支持 MIN |

负 width 为 `FMT_INVALID_WIDTH`，非单码点 fill 为 `FMT_INVALID_FILL`，非法 radix 为
`FMT_INVALID_RADIX`，均属 Error。即使无需填充也检查 fill；不按字素或显示列计算宽度。
text 最多 65,536 UTF-8 字节，width 最多 65,536，结果最多 262,144 字节；
超限为 `ResourceError / FMT_LIMIT_EXCEEDED`，先检查结果预算再构造。

填充使用二进制倍增；整数始终留在非正域提取数字，避免对 MIN 取正绝对值，最多 64 次。
源码与各项实际资源/性能测量分开验收。接口版本 1，身份 `stdlib.fmt`，用户路径 `std.fmt`。
契约见 [B1](../docs/DevDocs/20o-b1-freeze-draft.md#36-fmt三项)，教程见
[文本宽度与整数格式](../docs/UseDocs/library/fmt.md)，状态保持 planned。
