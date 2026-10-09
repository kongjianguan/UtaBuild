# Agent Note: 生成完整歌曲教学视频

Status: implemented

[English](2026-10-03-song-teaching-video.md) | 中文

## Problem

指定歌曲名称和展示形式后，应该能够生成完整教学视频，包含录音、逐步高亮的原文与罗马音、Ruby、中文翻译、符合语境的词汇、表达讲解和播放进度。现有歌词展示和 JSON 输出缺少完整学习内容与视频编码能力。

准确的 QRC 时间区间与选定录音可能具有不同的时间起点。来源准备需要在视频导出之前确定当前录音的歌词偏移。

## Decision

流程将确定性的来源准备与渲染和语言内容编写分别交给对应模块。Rust 领域层获取并解密可靠的歌词与读音来源。来源命令在项目文件中保留录音身份和时间信息。当前 Agent 通过可以自动发现的[项目 Skill](../../../skills/generate-teaching-video/SKILL.md) 生成并复核完整教学内容。Pillow 合成横屏或竖屏画面，FFmpeg 编码选定的完整录音。

自动校准在录音前段、中段和后段进行已知歌词的声学匹配，确定一个共同的歌词偏移。文本的 token probability 筛选测量位置，剩余误差和区段一致性确认这些测量在统计上是否一致。保留 QRC 区间可以保留准确的相对时间；共同使用的渲染器应用通过验收的偏移，使预览、片段跳转、高亮和教学分页采用当前录音的同一时间。选定音频保留完整时长。

教学条目使用经过规范化并去重后的演唱原文作为键。重复歌词由一个已经复核的条目负责，每次演唱分别保留来源时间。来源 Ruby 负责确定语境读音和歌曲特定读音。渲染器在编码之前验证字段、来源覆盖、字形和布局容量；语言复核负责确认语义准确性。

共享行内基线保留字体本身的字形位置。白色文字与高亮共享字形像素遮罩，使颜色推进时字符位置保持一致。

本地后期编辑器使用与视频导出相同的 Pillow 合成逻辑。独立编辑文件记录显示文字、可见性、删除的组件和分别保存的横竖屏位置；准备好的来源时间信息和已经复核的教学输入保留原有身份。[组件选择决定](2026-10-09-video-component-selection.zh.md)负责选择、批量操作和明确的排版策略。封面颜色量化提供高亮色相，并保证清晰的对比度。预览与导出的 MP4 共同采用这套合成逻辑确定画面位置。

视频卡片材质使用 Pillow 与 NumPy，在组件最终位置完成合成。因此，折射、色调、高光与阴影成为编码画面的一部分，同时保留共同使用的文字位置。材质在教学分页合成时计算，使光学处理无需在逐帧高亮循环中执行。

可复用的执行步骤归属[用户教程](../../../../docs/user/teaching-video.zh.md)，运行契约归属[子系统参考](../../../../docs/subsystems/teaching-video.zh.md)。Skill 通过这些持续维护的接口处理自然语言请求，返回完整 MP4 和可以验证的证据。

## Alternatives considered

**独立运行的语言服务** 为当前 Agent 已经处理的自然语言工作流增加了模型服务边界。Agent 编写教学内容可以在当前请求中完成语境翻译与复核，并为确定性的渲染器提供完整内容文件。

**仅通过词典生成教学内容** 可以提供词条，但无法确认句子的语境翻译，也无法解释句中实际使用的结构。流程结合可靠读音、查阅的参考资料和已经复核的语言内容。

## Consequences

在线来源可能采用不同录音版本，也可能缺少完整时间或读音数据。来源身份、时长检查、完整 Ruby 映射和来源文本匹配拒绝这些不一致的输入。同名歌曲需要足够的元数据才能确定所请求的录音。符合标准的测量位置不足或三个歌曲区段的偏移不一致时，自动校准终止。

语言准确性需要通过完整语境复核确认翻译、读音、原形、活用、表达和有依据的 JLPT 标签。当前 Agent 负责复核每条不同的演唱歌词。完整导出之前，根据实际完整内容验证字体与布局容量。

完整导出以零音频偏移保留来源录音的开头与结尾。已经准备的输入通过当前来源、校准和教学复核后可以复用。通过验收的校准报告将测量位置绑定到音频与原始歌词数据，并保留模型来源证据。加载时重新计算测量统计并检查来源哈希。新的准备流程使用空目录，渲染要求整个编码期间的输入与校准报告保持一致。开始时保存的输入哈希将验证证据与完整导出文件绑定。

## Testing

`spiral / LONGMAN` 与 `春日影 / MyGO!!!!!` 的完整来源项目包含 53 条与 45 条演唱歌词，分别保留全部 84 个与 89 个来源 Ruby 范围。第二首歌曲在渲染项目中保留伴唱文本、空白和零时长空白区间。

[`calibration-e2e.py`](../../../../tools/teaching-video/calibration-e2e.py) 使用默认模型校准真实的 `spiral` 录音，并检查完整来源区间与输入哈希保持一致。通过验收的偏移为 `-60 ms`，90 个通过 token probability 筛选的测量位置中有 66 个通过偏移验收。添加 `1,250 ms` 前导静音后，偏移为 `1,193 ms`，测量到 `1,253 ms` 的变化，误差为 `3 ms`。两种画面比例的检查保留逐字区间，并应用相同偏移。替换音频后，渲染器拒绝原校准报告。`work/calibration-e2e-20261009/verification.json` 记录这些检查；对应的完整歌曲媒体报告覆盖 `231,100 ms`、`6,933` 张画面和 `115` 张采样画面，来源音频相关系数为 `0.9984967661935833`。

`node tools/teaching-video/python.mjs calibrate` 包装命令也成功校准 `春日影`。`work/calibration-haruhikage-final/project.calibration.json` 记录 `-71 ms` 偏移，88 个通过 token probability 筛选的测量位置中有 72 个通过偏移验收，三个区段均通过统计检查。这些报告验证跨区段统计一致性，并通过哈希验证来源身份。

真实编辑器 HTTP 检查加载校准后的会话时间，请求预览，保存编辑文件，并导出和验证八秒片段。`work/calibration-editor-e2e-20261009/verification.json` 记录 `-60 ms` 歌词偏移、零音频偏移、保持一致的来源时间、保存的编辑文件哈希，以及通过验证的媒体导出。

headless Chrome DOM 检查操作「下一句」、中段与末段歌词选择，以及「上一句」。每次预览请求使用选定来源开始时间加上 `-60 ms`，再加上编辑器切换场景使用的 `1 ms` 增量。每次操作之后，预览时钟、DOM 时钟、所选歌词与音频位置保持一致。`work/calibration-browser-e2e-20261009/verification.json` 记录全部四次操作、保持完整的来源数据，以及没有页面错误的结果。

使用保留的来源项目与新建证据目录，可以重复执行校准和完整歌曲媒体验证：

```sh
tools/teaching-video/.venv/bin/python tools/teaching-video/calibration-e2e.py \
  --project work/spiral/project.json \
  --teaching work/spiral/teaching.json \
  --font /absolute/path/to/CJK-font.ttf \
  --evidence-dir work/calibration-verification
```

已经保留的编辑器脚本自行启动真实服务，并分别保存证据目录：

```sh
node work/lyric-calibration-editor-e2e.mjs \
  --project work/calibration-e2e-20261009/project.json \
  --teaching work/spiral/teaching.json \
  --font /absolute/path/to/CJK-font.ttf \
  --evidence work/calibration-editor-verification
node work/lyric-calibration-browser-e2e.mjs \
  --project work/calibration-e2e-20261009/project.json \
  --teaching work/spiral/teaching.json \
  --font /absolute/path/to/CJK-font.ttf \
  --evidence work/calibration-browser-verification
```

经过编辑的 `spiral` 横屏与竖屏媒体报告记录完整解码、媒体检查、教学分页比较和来源音频比较。两份报告分别包含 115 和 114 张采样编码画面，覆盖 59 和 58 个教学分页。独立字体像素与编码画面检查覆盖 1,070 和 1,056 次原文字符采样；两份报告测量到的最大纵向偏移均为零。来源音频相关系数为 0.998497。

在仓库根目录使用新建编辑目录，可以重复执行浏览器编辑与完整媒体验证：

```sh
pnpm video:edit --project work/calibration-e2e-20261009/project.json --teaching work/spiral/teaching.json --font /absolute/path/to/CJK-font.ttf --workspace work/editor-verification --port 49604
pnpm video:editor-e2e --url http://127.0.0.1:49604 --evidence-dir work/editor-verification/evidence
```

编辑器为每次导出保留独立的编辑快照、MP4、渲染清单、日志和采样验证画面。E2E 证据目录包含交互报告、实际预览图与组件位置，以及完整歌曲验证报告。检查覆盖直接编辑文字、逐字片段与 Ruby、教学文字、拖动、整组移动、删除后的自动排版、撤销、重做、分别保存的横竖屏位置、保存、重新打开和不同宽度下的预览比例。

[`editor-glass-material-e2e.py`](../../../../tools/teaching-video/editor-glass-material-e2e.py) 通过编辑服务检查实际横屏与竖屏预览。检查测量卡片背景变化、边缘高光，以及白色、次要文字和强调颜色至少 4.5:1 的对比度。移动与删除卡片需要保持材质位置和自动排版；删除全部组件需要得到空白背景。脚本保存实际预览图和可以重复执行的 `glass-material-e2e.json` 报告。

两首歌曲通过横屏与竖屏编辑器渲染及封面颜色对比度检查。HTTP 与媒体 E2E 拒绝空白文字、缺少字形、组件超出画面、多行逐字片段，以及将零时长空白区间改成可见文字。`春日影` 片段验证删除全部组件和将原文字形放置在画面边缘；后者覆盖 112 次字形采样，纵向偏移为零。片段来源音频相关系数为 0.998384。来源项目、教学文件、封面和音频哈希保持一致。

共享歌词变更需要完整 fixture 检查；教学视频变更需要真实来源准备、完整内容复核、完整导出和对应媒体验证。文档验证、prose lint 和空白检查覆盖持续维护的双语参考与教程。
