# 文件系统路径操作

`std.fs` 与 `stdlib.fs` 共用 `stdlib.fs`，需要 `filesystem` 能力。普通宿主模式按进程工作目录解释相对路径，绝对路径和中间链接由宿主解析。教程登记为 `planned`。[冻结契约](../docs/DevDocs/assets/20o-b3/entry-20261123-b3-0c/contract-draft.md)

| 函数 | 行为 |
| --- | --- |
| `write_text(path, text, overwrite)` | 在目标同目录暂存 UTF-8 文本，再原子提交。禁止覆盖时已有目录项返回 `FS_EXISTS` |
| `list_dir(path)` | 枚举直接子项的名称，按 UTF-8 字节排序 |
| `remove(path)` | 删除文件、空目录或链接本身 |
| `rename(source, destination, overwrite)` | 在同卷移动目录项，显式指定是否覆盖 |
| `metadata(path)` | 读取末段本身的 `kind`、`size` 和 `modified_ms` |

单条路径至多 4,096 字节，单次写入至多 8 MiB。枚举至多 1,024 项，单项至多 4,096 字节，名称合计至多 65,536 字节。违规的枚举整次失败。需要读取和关闭文件句柄时使用 `std.os`。[教程候选](../docs/DevDocs/assets/20o-b3/entry-20261124-b3-0e/tutorials/fs.md)
