# cmath B4 测试数据

`reference-vectors.tsv` 是 17,243 条 MPFR 4.2.2/GMP 6.3.0 离线参考数据，供 Runtime 内核和 C ABI 测试复用，不随工具链打包，也不在普通教程中作为公开 API 示例。

来源：`docs/DevDocs/assets/20o-b2-b/entry-20261108-b3/b1/reference-vectors.tsv`。B4-0 迁移保持逐字节相同，SHA-256：

```text
0e61a2da361448477b15f5508df6d612a3f01e29a06e50699d6ed48a917d1ed3
```

DevDocs 来源副本作为历史取证保留；产品测试只引用本目录。

B3的float幂切换差异已从历史`pow-diff.json`迁入`pow-legacy-diff.tsv`：来源SHA-256 `6611ec3d83ea92b8b59edfe1b3b5db7a906d3258912cde9f709e0893b2215e8b`，fixture SHA-256 `cab916ea19b4f1ec1006c4b112401b206af77c871ff519d8b507fc2d6246e6c8`。1075项中1011项一致、40项值/错误边界、24项位模式差异。B4-3逐行核对输入、独立pow参考与历史共享内核值，并对O0/O3的Float `**`和`cmath.pow`逐项实跑，不依赖DevDocs路径。
