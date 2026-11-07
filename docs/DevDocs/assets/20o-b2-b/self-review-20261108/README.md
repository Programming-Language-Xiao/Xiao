# B2 自审证据

自审基线`70e98d72`，产品代码`37b92c53`。五个临时突变复现审计遗漏，修复后必须以完整预期原因返回1，成功和拒绝路径均保持文件字节、长度与修改时间不变。

```text
python -X utf8 docs/DevDocs/assets/20o-b2-b/self-review-20261108/audit-cases.py --expect-rejection
python -X utf8 docs/DevDocs/assets/20o-b2-b/self-review-20261108/numeric-reference.py
```

`audit-before.json`保存初轮4项原始复现，`audit-before-complete.json`保存补充源码遗漏后的5项复现；其中`expect_rejection=false`表示确认旧审计错误接受突变。`audit-after.json`是最终5项拒绝证据。对临时副本重建清单是反例构造步骤，真实B2清单只在审计脚本有意修改后按实际字节更新，未改写历史命令、日志、closeout或源码身份。

`numeric-reference.py`通过Rust1.96.0编译独立探针，链接原任务target中已有的Runtime/ABI rlib。Python Decimal/libmpdec计算2300组数值黄金；同一探针另外调用C ABI验证3组别名。结果、版本、固定种子、编译命令、原件和输入输出摘要在`numeric-result.json`。该命令只生成任务根下的探针可执行文件与本目录回执，不重编译产品或改动旧Runtime。

生成器代码中的`xiao_runtime`等公开名称是实际被测API；本目录不新增产品入口。最终固定清单在所有自审文件定稿后生成，清单自身不计入摘要。
