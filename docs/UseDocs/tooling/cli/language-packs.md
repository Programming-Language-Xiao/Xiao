---
id: tooling.cli.language-packs
title: 制作资源型语言包
status: verified
audience: developer
module: rust.xiao-i18n
stage: 11C-2
version: "0.1.0"
related:
  - localization.md
  - package-sources.md
  - package-lockfile.md
  - package-cache.md
  - ../../../DevDocs/11c2-language-pack-plugins.md
---

# 制作资源型语言包

语言包是只提供声明式消息目录的资源型包。它可以被 11A 的包源、锁文件、来源审计和不可变
缓存链路安装，但不会提供 Xiao 代码、初始化函数或安装钩子。首版语言包的加载器只读取并
校验 JSON，不执行包内任何内容。

## 最小目录

一个最小语言包只有清单和清单声明的目录文件：

```text
fr-pack/
  xiao-language-pack.json
  catalogs/
    fr-FR.json
```

清单文件名固定为 `xiao-language-pack.json`。目录资源必须位于 `catalogs/` 下，使用 `.json`
扩展名，并且每个文件只能映射一个规范语言标签。

## 清单

下面的清单提供一个法语目录。`xiao_version` 和 `runtime_abi` 只是兼容范围；实际当前版本
不应从这个示例硬编码，而应根据要支持的 Xiao/Runtime 版本填写：

```json
{
  "manifest_version": 1,
  "plugin_id": "org.example.french",
  "package_version": "1.0.0",
  "catalogs": {
    "fr-FR": "catalogs/fr-FR.json"
  },
  "catalog_version": 1,
  "xiao_version": ">=0.1.0,<1.0.0",
  "runtime_abi": ">=1.1.0,<2.0.0",
  "content_length": 0,
  "sha256": "<生成后的 64 位小写 SHA-256>",
  "display_name": "Français",
  "copyright": "Example Contributors",
  "contributors": ["Example Contributors"]
}
```

字段规则如下：

- `manifest_version`、`catalog_version` 当前固定为 `1`；未知字段会被拒绝。
- `plugin_id` 是稳定的 ASCII 标识，只允许字母、数字、点、短横线和下划线；它不是显示名称。
- `package_version` 必须是严格三段 SemVer。它用于 11A 的锁定、来源审计和诊断身份，
  不参与语言资源运行时兼容判断，也不进入语言资源缓存键。
- `catalogs` 的键是规范语言标签，值是包内相对路径。语言标签和路径不能包含控制字符、
  反斜线、根路径、`.`、`..` 或符号链接；语言标签不能重复规范化后指向同一目录。
- `xiao_version` 与 `runtime_abi` 使用受限的 SemVer 合取语法：`*`、严格三段版本、
  `^`、`~`、`=`、`>`、`>=`、`<`、`<=`、通配符和逗号连接均可用；项之间不能有空白。
  例如 `>=0.1.0,<1.0.0`。它们只决定当前宿主是否兼容。
- `content_length` 是所有声明目录文件原始字节长度之和，不包含清单。
- `sha256` 是目录资源规范摘要，不是清单文件摘要；必须是 64 位小写十六进制文本。
- `display_name`、`copyright` 和 `contributors` 是可选元数据，不得含控制字符；它们不会改变
  目录资源摘要。

## 目录文件

目录文件也使用严格 JSON，并且不能增加未知字段：

```json
{
  "version": 1,
  "locale": "fr-FR",
  "entries": [
    {
      "id": "plugin.greeting",
      "text": "Bonjour {name}",
      "params": {
        "name": "text"
      }
    },
    {
      "id": "plugin.count",
      "text": "Éléments : {count}",
      "params": {
        "count": "integer"
      }
    }
  ]
}
```

`version` 必须与清单的 `catalog_version` 相同，`locale` 必须与清单键规范化后完全相同。
`params` 支持 `text`、`integer` 和 `boolean`；模板中的每个 `{name}` 占位符必须恰好出现在
参数签名中。消息身份不能重复；同一 `message_id` 在不同语言目录中的参数签名也必须一致。

资源目录只能包含清单和声明的普通 JSON 文件。脚本、钩子、动态库、可执行文件、字体、图片、
压缩包、额外 JSON、符号链接和设备文件都会被拒绝；没有任何“安装后运行”步骤。

## 生成摘要

先写好每个目录文件，再生成清单中的 `content_length` 和 `sha256`，最后写入清单。摘要算法
由 `xiao_i18n::digest_resources` 固定，不能用对整个目录打包后的普通 `sha256sum` 替代：

1. 只取 `catalogs` 中声明的资源，使用包内 `/` 分隔的相对路径。
2. 按路径字节序排序。
3. 先输入领域标签 `xiao-language-pack-v1` 加一个零字节。
4. 对每一项依次输入大端 `u64` 长度和路径字节，再输入大端 `u64` 长度和文件原始字节。
5. `content_length` 是文件字节长度的总和，摘要输出为小写 64 位十六进制 SHA-256。

下面的脚本只演示摘要和清单生成算法；它不负责从包源下载或安装包：

```python
import hashlib
import json
import struct
from pathlib import Path

root = Path("fr-pack")
resources = {
    "catalogs/fr-FR.json": (root / "catalogs/fr-FR.json").read_bytes(),
}

hasher = hashlib.sha256(b"xiao-language-pack-v1\0")
content_length = 0
for name in sorted(resources):
    path_bytes = name.encode("utf-8")
    data = resources[name]
    hasher.update(struct.pack(">Q", len(path_bytes)))
    hasher.update(path_bytes)
    hasher.update(struct.pack(">Q", len(data)))
    hasher.update(data)
    content_length += len(data)

manifest = {
    "manifest_version": 1,
    "plugin_id": "org.example.french",
    "package_version": "1.0.0",
    "catalogs": {"fr-FR": "catalogs/fr-FR.json"},
    "catalog_version": 1,
    "xiao_version": ">=0.1.0,<1.0.0",
    "runtime_abi": ">=1.1.0,<2.0.0",
    "content_length": content_length,
    "sha256": hasher.hexdigest(),
}
(root / "xiao-language-pack.json").write_text(
    json.dumps(manifest, ensure_ascii=False, indent=2) + "\n",
    encoding="utf-8",
)
```

清单本身不参与摘要，因此清单写入后不能再修改目录资源；如果修改目录文件，必须重新生成
长度和摘要。加载器会再次计算摘要，任何不一致都会拒绝资源。

## 安装与缓存边界

语言包没有独立的安装器。正常路径是：

1. 在 11A 的源配置中声明来源，并把语言包作为资源型包候选交给普通源选择流程。
2. 由 11A 解析、锁定包版本，保存来源身份、选定版本和来源/正文摘要，完成来源审计。
3. 将已经通过 11A 验证的包正文交给 `xiao-package` 的语言资源导入接口；它再次校验清单、
   目录、兼容范围和内容摘要，然后写入独立的只读缓存对象。

语言资源对象位于 `XIAO_HOME/cache/objects/language/sha256/`，路径包含目录版本、Xiao 版本、
Runtime ABI 和内容摘要。它与 `source`、`metadata` 缓存隔离；同一资源内容可以被多个项目复用，
而不会修改源码对象。缓存对象损坏时会移到同分片下的 `.corrupt` 名称，不能继续使用。

语言包是依赖图中的叶资源，不声明插件间依赖，也不执行普通包的入口、初始化、安装后脚本或
卸载钩子。`package_version` 只负责锁定和审计；`xiao_version`、`runtime_abi` 只负责受限兼容检查。

## 冲突、更新与回退

- 内置目录和已经注册的目录拥有优先保护权；重复 `message_id`、参数签名不一致或目录版本
  不兼容会拒绝候选，不按安装顺序覆盖。
- 同一 `plugin_id` 重新激活时，旧版本目录会在候选注册表中整体移除并原子替换；新候选
  任何一步失败，旧注册表保持不变。
- 语言包缺少翻译时仍按“精确标签 → 基础语言 → 内置英语 → 原始 `message_id` 和安全参数”
  回退，不显示空白，也不把译文写回机器错误字段。
- 当前进程加载失败时保留上一次已经验证的注册表并记录诊断；从未验证成功则只有内置目录。
  新进程不继承旧进程信任，已知摘要不匹配的对象必须重新验证并拒绝。

稳定诊断编号为：`X11-I18N-PACK-001`（清单/结构）、`X11-I18N-PACK-002`（Xiao 或 Runtime
兼容性）、`X11-I18N-PACK-003`（长度或摘要）、`X11-I18N-PACK-004`（目录冲突）和
`X11-I18N-PACK-005`（资源路径、文件或 JSON 无效）。机器输出仍保留原始 `code`、`message_id`、
参数、位置和退出码。

## 当前边界

本页描述的是已经交付的 L3 核心资源契约。`.xar` 启动入口元数据、完整 L4 跨平台发布验收、
语言包签名方案和语言级交互输入不属于本批；它们完成前，归档启动不能假定已经携带外部语言包。
