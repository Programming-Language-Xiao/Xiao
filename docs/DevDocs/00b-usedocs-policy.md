# 00B. UseDocs 用户文档与同步门禁

> 本文规定 Xiao 面向自然人的使用文档（UseDocs）如何组织、编写、互相引用和随模块交付。UseDocs 与 `docs/DevDocs` 完全分离：DevDocs 解释“如何设计和实现”，UseDocs 解释“用户如何安装、使用和排错”。

## Agent 交接上下文

### 接手前提

- 先阅读 [00A. 工程框架与目录布局](00a-project-layout.md)、[00. 决策基线](00-decisions.md) 和 [12. 测试与开发里程碑](12-tests-and-milestones.md)。
- `docs/DevDocs/README.md` 只索引开发规格；`docs/UseDocs/README.md` 只索引面向用户的文档，两者不能互相替代。
- A0 会提供 UseDocs 链接、模块登记和目录完整性检查；在检查器落地前，人工审查仍是临时门槛。

### 本阶段交付与不负责事项

- 交付：UseDocs 目录层级、页面元数据、交叉引用规则、模块完成同步流程和自动检查契约。
- 不负责：立即补写尚未实现功能的使用说明，不在 UseDocs 中复制内部算法、crate 依赖或未兑现的命令。

## 一级工程目标：建立面向自然人的文档树

### 固定层级

UseDocs 根目录只能放总索引 `README.md` 和少量全局说明；主题内容必须进入至少一层子目录，复杂主题再继续分层：

```text
docs/UseDocs/
  README.md                     总导航、版本入口和阅读路线
  getting-started/              安装、环境和第一个程序
    README.md
    installation/               安装与平台准备
      README.md
  language/                     语言学习与语法主题
    README.md
    basics/                     变量、类型、表达式
      README.md
    collections/                数组、元组、集合和字典
      README.md
    modules/                    模块、导入和工程配置
      README.md
  tooling/                     CLI、REPL、虚拟环境和调试
    README.md
    cli/                        命令逐项说明
      README.md
    repl/                       交互式解释器
      README.md
  guides/                      按目标组织的任务教程
    README.md
  reference/                   可检索的稳定参考
    README.md
  troubleshooting/             常见错误和恢复步骤
    README.md
  _templates/                  新页面和模块文档模板
    README.md
```

目录名称使用小写英文和连字符，页面标题使用中文；目录只表达主题，不按 Rust crate 名称暴露内部实现。

### 页面阅读路径

每个主题索引必须提供“适合谁、前置知识、推荐顺序、下一步”四项。具体页面至少包含返回上级索引、相关页面和下一页链接，形成自然人可连续阅读的路径；参考页可以通过锚点和交叉链接提供随机访问。

## 一级工程目标：定义模块 UseDocs 契约

### 完成条件

一个模块只有同时满足以下条件，才能在工程状态中标记为“完成”：

1. 代码实现通过该模块的单元/规格/集成测试。
2. 公共 API 和用户可观察行为已有对应 UseDocs 页面。
3. 页面示例通过可自动执行的示例测试，或明确标注为平台/版本限定示例。
4. 页面已加入主题索引、相关页面和故障排查链接。
5. 模块登记表记录代码路径、测试路径、UseDocs 路径、适用工程期和验证版本。

代码、测试和 UseDocs 必须在同一提交或同一可审计变更集中完成。只提交实现而把使用文档留到“以后补”视为模块未完成，不能更新阶段状态。

### 页面元数据

每个具体 UseDocs 页面使用 YAML front matter，至少包含：

```yaml
---
id: language.basics.variables
title: 变量与类型
status: planned
audience: beginner
module: xiao-types
stage: 02
related:
  - ../README.md
---
```

`status` 取 `planned`、`draft`、`verified` 或 `deprecated`。只有 `verified` 页面可以作为已完成模块的交付证明；`deprecated` 页面必须链接到替代页面。元数据不是用户教程正文，正文仍需用自然语言说明目标、步骤、结果和错误处理。

### 示例与版本

- Xiao 代码示例必须注明所需语言/Runtime 版本；可执行示例放在 `tests/fixtures` 或 UseDocs 专用示例目录，并由测试任务引用。
- 示例输出不能依赖本地化译文、临时路径或未冻结的颜色；需要展示错误时同时给出稳定错误码或诊断类别。
- 平台差异使用独立小节和链接，不把 Windows、Linux、macOS 的步骤混成无法验证的一段文字。

## 一级工程目标：建立引用与审查规则

### 引用规则

1. UseDocs 页面只使用相对 Markdown 链接，链接路径大小写必须与仓库实际路径一致。
2. 主题索引链接到子索引和页面；页面不能成为孤立文件，也不能反向链接到不存在的未来功能。
3. DevDocs 可以链接到 UseDocs 作为验收证据，但 UseDocs 不应暴露内部实现细节作为用户前提。
4. 删除或移动页面时，同一变更必须更新所有入链；目录 README 不能保留失效链接。
5. 链接检查器需要识别 Markdown 锚点、代码示例引用和 `module` 登记路径，输出文件/行号和稳定错误码。

### 自然人审查清单

- 首次阅读者能否在三分钟内找到安装、运行和下一步入口。
- 每个命令是否说明输入、输出、失败时的恢复方式，以及 Windows/Linux/macOS 差异。
- 术语是否与语言规范一致，但没有要求读者先理解 Rust crate、IR 或内部 ABI。
- 示例是否最小、可复制、可验证，是否明确哪些功能尚未实现。
- 页面是否提供从当前任务到相关任务的下一条阅读路径。

## 二级实现任务

### U0：目录与模板

1. 创建 `docs/UseDocs` 的根索引、主题索引和 `_templates`。
2. 提供模块教程模板、命令参考模板、故障排查模板和版本迁移模板。
3. 在 DevDocs README 中只放 UseDocs 入口链接，不复制用户文档正文。

### U1：模块登记与同步

1. 在 A0 的模块登记文件中为每个代码模块声明测试路径和 UseDocs 路径。
2. 模块状态从 `planned` 到 `verified` 必须同时拥有通过测试的实现和 UseDocs 页面。
3. 检查器拒绝缺失页面、孤立页面、失效链接、未验证状态冒充交付和跨目录路径穿越。

### U2：示例验证与发布

1. 为可执行示例建立测试入口，记录语言/Runtime、平台和输入约束。
2. 发布前生成 UseDocs 链接图和孤立页报告；中文文案变化不能破坏机器字段和示例语义。
3. 在 Windows → Linux → macOS 的平台阶段分别验证平台专属步骤，并在页面中标注验证环境。

## 验收标准

### 结构验收

- UseDocs 不与 DevDocs 平铺在同一目录；根目录和主题目录都有 README 索引。
- 每个已完成模块至少有一个 `verified` UseDocs 页面，且能从根索引沿链接到达并返回。
- 不存在孤立页面、断链、重复 ID、错误模块路径或未登记的已完成模块。

### 同步验收

- 模块代码、测试和 UseDocs 在同一变更集中出现；缺任一项不能标记完成。
- UseDocs 示例与规格测试使用同一份可追踪输入，示例失败时能定位到代码、测试和页面。
- 页面不承诺尚未冻结或尚未实现的行为，废弃页面提供替代链接。

## 待定决策

- UseDocs 是否在未来生成静态站点，以及站点生成器和版本发布方式。
- YAML front matter 的最终解析库和多语言页面的目录策略；字段契约已经确定。
- 示例执行是否纳入每次 CI，还是按主题/发布门槛分层执行。

