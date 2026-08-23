# Agent Notes

[English](README.md) | 中文

**Agent Note** 记录会影响仓库的决策或提案，包括动机、被考虑过的替代方案，以及源代码和普通参考文档无法承载的后果。本文件定义 Agent Note 的位置、撰写时机，以及[文件格式](#the-file-format)。

## 目录与命名

每份 Agent Note 都有两个维度，并且都编码在路径中：`{lifecycle}/{class}/yyyy-mm-dd-topic-title.md`。

- **生命周期**是顶层目录，也是记录的状态。`proposed/` 保存正在考虑的工作，`implemented/` 保存已经交付的决策，`rejected/` 保存经过考虑但被否决的提案。
- **类别**是嵌套目录，也是决策的种类。仓库应保持类别集合封闭，使归档位置可搜索并可由工具检查。

文件名中的日期是主题首次提出的日期。交叉引用使用相对 Markdown 链接，不使用裸编号或非正式称呼，确保记录在生命周期目录之间移动后仍可检查。

活动生命周期树是工作清单。浏览其中的类别目录或搜索仓库；除非仓库有具体且已记录的理由，否则不要创建中央索引。未来决策价值较低的已实现记录移入下文所述的独立冻结 `archived/` 树。

## 分类

每份记录都属于仓库封闭集合中的一个类别。常见集合如下：

| 类别 | 覆盖内容 |
|---|---|
| `feature` | 新增的用户或系统能力。 |
| `bug-fix` | 由缺陷或已发现缺口推动的修复。 |
| `simplification` | 在不新增能力的情况下移除代码、行为或表面积。 |
| `architecture` | 关于已交付源代码及其边界的结构性决策。 |
| `process` | 围绕源代码和发布流程的工具、政策或工作流。 |
| `testing` | 测试基础设施、覆盖策略或验证设计。 |

明确区分 architecture 和 process：architecture 描述系统交付的内容，process 描述外围工具和工作流。不要在已有 simplification 或 architecture 可以覆盖主题时增加 `refactor` 类别。

## 归档与删除

当已实现记录的决策已经完成，且其理由不太可能继续指导未来工作时，将它归档。当其中的替代方案、所有权边界、负向保证、持久化或线协议语义、安全规则，或重新引入条件仍有用时，保持活动状态。永远不要归档 proposed 记录；过时提案应当拒绝。

只有在 rejected 记录仍能阻止一个合理错误时才保留它。否则同时删除它的英文、中文和 sidecar 文件。仓库提供归档工作流时应使用该工作流；年龄、字数和目标配额都不是归档标准。

归档路径是 `archived/{class}/yyyy-mm-dd-topic-title.md`。其中有意不包含 `implemented`，因为只有已实现记录才能进入归档。归档变更必须移动完整的英文、中文和 sidecar 三件套，保留 `Status: implemented`，在两种语言文件的该状态行下方插入相同的 `Archived: YYYY-MM-DD` 行，重新记录 sidecar，并修复或删除入站链接。归档时只允许这些内容变化。

一旦封存，每个归档三件套都永久冻结。不要编辑、翻译、重新格式化、更新、移动或删除它，也不要把它当作当前事实的权威来源。仓库的归档验证器应检查封闭类别树、完整三件套、归档元数据、sidecar hash 和只追加的内容清单。

## 何时撰写

每个非平凡变更都 MUST 在同一变更中新增或更新至少一份 Agent Note。当变更修改行为、架构、跨文件共享的契约、流程或工具、测试策略、磁盘文件格式、线协议、配置格式，或其他维护者可能重新审视的决策时，它就是非平凡变更。重要未来工作的提案从 `proposed/` 开始，已经做出的决策从 `implemented/` 开始。

更新已经拥有该决策的记录即可满足要求，不要创建重复记录。Agent Note 不应被编辑成另一个决策：用新记录取代它，并保持两份记录互相链接，除非旧记录之后按下文规则被完整合并。

完全被取代的已实现记录可以合并到当前所有者记录中并删除。删除前必须保留所有独有的理由、替代方案、后果、必要验证和明确的覆盖缺口；修复所有入站链接；并在同一变更中删除中文对应文件和一致性记录。部分取代时保留两份记录，并让它们互相链接。

只有当新增功能已经从生产代码、配置、模式、持久化或线协议格式、迁移、兼容行为、当前文档和受支持测试中全部消失时，功能新增记录才可以合并到后续的移除记录。移除记录的所有者必须保留原始动机、功能不再合理的原因、完全移除之外的替代方案、放弃的能力、重新引入条件，以及完整缺失验证。

## 文件格式

每份活动 Agent Note 都使用统一的文件格式。归档记录保留封存时的格式，并额外保留下方的归档日期行。

### 文件头

每份 Agent Note 的前三行必须完全如下：

```markdown
# Agent Note: <title>

Status: <status>
```

状态必须与所在生命周期目录一致：

- `Status: proposed`
- `Status: implemented`
- `Status: rejected — <why, in one line>`

状态中不包含日期或括号。文件名保存首次提出日期，正文保存修订内容。rejected 状态是唯一带正文的状态形式，因为读者需要立即看到否决理由。

### 正文骨架

每份 Agent Note 都以 `## Problem` 开始，用不预设解决方案的方式说明动机。除确实需要的技术专节外，重复出现的章节使用下面的规范名称；专节可以放在必需章节之间。

#### `proposed/`

```markdown
## Problem
## Proposal
... bespoke sections ...
## Alternatives considered
## Acceptance criteria
## Risks
```

`## Proposal` 可以使用将来时。工作尚未完成时，计划、迁移步骤和开放问题放在这里。`## Acceptance criteria` 说明可观察的完成状态。`## Risks` 说明可能出错的地方以及变更明确放弃的内容。

#### `implemented/`

```markdown
## Problem
## Decision
... bespoke sections ...
## Alternatives considered
## Consequences
```

`## Decision` 用现在时描述已交付事实，整份记录也必须随其保持最新。`## Proposal`、`## Plan`、`## Migration plan` 和 `## Acceptance criteria` 等提案时期的标题不属于 implemented 记录。只要是在陈述当前事实，`## Testing`、`## Deferred` 或 `## Related` 章节就是允许的。

#### `rejected/`

rejected 记录是冻结的提案。它保留提案时期的章节，裁决写在 `Status:` 行中。文件头、`## Problem`、`## Proposal` 和替代方案要求仍然适用。

### 替代方案：强制要求

每份 Agent Note 都必须有 `## Alternatives considered` 章节。记录每个真实替代方案及其失败原因，每个替代方案使用一个粗体开头的段落，或使用一个 `### Why not <X>?` 子章节。只记录实际考虑过的替代方案，不要事后编造。

对于无法重建替代方案的旧记录，在仓库格式政策允许时，可以在该章节位置保留以下精确标记：

```markdown
<!-- agent-note-format: alternatives-not-recorded (pre-format Agent Note) -->
```

### 在生命周期之间移动

在生命周期目录之间移动文件时，必须在同一变更中更新 `Status:` 行并满足目标骨架。`proposed/` 移到 `implemented/` 时，将 `## Proposal` 改写为现在时的 `## Decision`，把验收标准和风险折入 `## Consequences` 或现在时的验证章节，并用实际交付内容替换计划。`proposed/` 移到 `rejected/` 时，只需在 `Status:` 行增加理由并冻结文件。

### 中文对应文件

`.zh.md` 文件按照 [i18n 契约](../../docs/i18n/README.zh.md) 与英文同级对应。机器检查的文件头 token，包括 `# Agent Note: ` 和 `Status:`，保持英文原样。格式检查器可以跳过中文文件；配对检查器负责它的一致性。
