# B2 显式精度除法证据

入口 `faecd26d`，任务根 `D:\project\Xiao\.b2b-divide\20261108-faecd26d`。本目录分开保存开工基线、开发失败与最终有效验收；最终代码身份和有效命令由 `code-identity.json` 与 `closeout.json` 明确列出，不把旧日志提升为新代码证据。

全局 `lfloat_divide`、intrinsic33、C入口与VM/LLVM接线仅作B2内部单入口验收；ABI1.18/生成器20保持，34～48与cmath未激活，registry不变。最终公共版本、三平台发布和721条留B5/B2-C；教程新增段落继续planned。

`run-step.py` 对源码输入按路径记录原始字节与UTF-8/LF摘要，命令前后必须一致；`environment.json`固定独立target、Runtime、Xiao Home、临时目录与仓库stdlib。B1 F10与B2-A性能开关始终为0。TypeScript环境修复只复制相同的5.9.3包到本任务忽略目录，记录见 `typescript-environment.json`。

`audit.py` 默认只读，检查清单实际字节、固定Git代码对象、允许路径、intrinsic1～32不变/33精确声明/100墓碑、同源执行记录、四个精确入口的1 passed、B1 9/17回归、维护入口B2标记、28/28守卫和独立Runtime摘要。支持 `--root` 和 `--source-root`；不要求当前HEAD、不重建closeout或清单。

```text
python -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261108-b2/audit.py
```

源码提交后，显式运行 `rebuild-evidence.py` 才生成清单；审计与反例不会调用它。`self-negative.py` 使用临时副本验证缺失属性文件、清单篡改和语义证据篡改必须被明确拒绝，结果另存于自审回执。Windows本机VM与原生O0/O3提供真实执行；本批尚无Linux/macOS新作业，未声称CI通过。
