# B2主仓库与Git导出复核

`result.json` 保存本次实际提交、导出命令、tar摘要、两次审计的参数/工作目录/退出码/完整输出。两次都必须通过99项证据、14个有效执行点、4个精确入口，并保证文件字节、长度和修改时间不变。原始B2归档保持固定身份，不为本回执重建其清单。

```text
python -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261108-b2-replay/verify-replay.py
```

默认只读；`--output <file>`才写入新的验证回执。该检查只复核已归档证据，不重新执行产品测试，也不代表跨平台CI。
