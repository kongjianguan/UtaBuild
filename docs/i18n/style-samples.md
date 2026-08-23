# Translation style samples

This file is bilingual by construction. It is a small set of human-approved examples used to calibrate register; it is not a terminology table, product guide, or source of project facts.

## Architecture reference

**English**

The runtime has one owner for each boundary. The coordinator schedules work, the adapter translates external input, and the storage layer owns persistence. A caller may depend on an adapter contract, but it must not reach through the adapter into storage details. When a boundary changes, update the architecture map and the owning subsystem reference in the same change.

**中文**

运行时的每条边界都有明确的所有者。协调器负责调度工作，适配器负责转换外部输入，存储层负责持久化。调用方可以依赖适配器契约，但不得越过适配器直接访问存储细节。边界发生变化时，在同一变更中更新架构地图和对应的子系统参考页。

## Decision record

**English**

The service keeps validation at the boundary because malformed input has no useful internal representation. This gives callers one failure contract and keeps downstream code free of repeated defensive checks. The cost is that adapters must classify errors before handing data to the service.

**中文**

服务在边界处完成验证，因为格式错误的输入没有有用的内部表示。这样调用方只需面对一份失败契约，下游代码也不必重复防御性检查。代价是适配器必须先对错误分类，再把数据交给服务。

## Procedure

**English**

Create the configuration, run the focused check, inspect the generated output, and then run the repository-wide documentation verifier. Stop after the focused check if it identifies a contract error; fix the owning source before continuing.

**中文**

创建配置，运行定向检查，检查生成结果，然后运行仓库级文档验证器。如果定向检查发现契约错误，先停止后续步骤；修复其所有者源文件后再继续。

## Incident summary

**English**

A release accepted an empty value as a valid identifier. The parser treated absence and an empty string as equivalent, so the existing validation never saw the invalid state. The fix separates the states at the parser boundary and adds a regression test for the exact input.

**中文**

一次发布把空值接受成了有效标识符。解析器把缺失值和空字符串当成同一种状态，因此既有验证从未看到这个无效状态。修复在解析边界处分开两种状态，并为该输入增加回归测试。

## Rules

**English**

State the current contract, name the actor, and link to the owner. Put rationale in the decision record, chronology in the postmortem, and exhaustive facts in generated references. Remove prose that only repeats a neighboring rule.

**中文**

陈述当前契约，写明执行者，并链接到所有者。将理由放入决策记录，将时间线放入事故复盘，将完整事实放入生成参考资料。删除只是在重复相邻规则的正文。
