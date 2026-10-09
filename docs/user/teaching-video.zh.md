# 生成歌曲教学视频

[English](teaching-video.md) | 中文

## 请求生成视频

在 Codex 中打开 UtaBuild，说明歌曲和布局，例如「生成 spiral 教学视频，横屏 16:9」或「生成 spiral 教学视频，竖屏 9:16」。[项目 Skill](../../.agents/skills/generate-teaching-video/SKILL.md) 会准备录音和歌词来源，编写并复核教学内容，渲染完整 MP4，并验证导出的文件。歌曲名称与请求上下文无法确定具体录音时，需要补充歌手信息。

导出的画面包含逐步高亮的日语歌词与罗马音、Ruby 读音、当前句与相邻句的中文翻译、词汇卡片、表达讲解和播放进度条，并使用模糊封面作为背景。横屏使用三列词汇卡片；竖屏使用两列，并根据可用宽度调整歌词与讲解区域。

## 准备运行环境

命令流程需要 Rust、Node.js、pnpm、Python 3.10 或更新版本、`curl`、`yt-dlp`、FFmpeg 和 CJK 字体。Python 依赖包含用于自动歌词校准的 faster-whisper。在仓库根目录安装 JavaScript 与 Python 依赖：

```sh
pnpm install
python3 -m venv tools/teaching-video/.venv
tools/teaching-video/.venv/bin/python -m pip install -r tools/teaching-video/requirements.txt
```

Windows 创建的环境使用 `tools/teaching-video/.venv/Scripts/python.exe` 作为解释器。`pnpm video:calibrate`、`pnpm video:render` 和 `pnpm video:verify` 入口根据当前平台选择项目环境的解释器。选择包含歌曲与教学内容所需日语、中文字符的字体文件。[子系统参考](../subsystems/teaching-video.zh.md) 说明来源要求、验证方式和失败行为。

## 准备歌曲

在 `work/` 下选择新目录或已经存在的空目录保存输出：

```sh
pnpm video:prepare --title spiral --artist LONGMAN --output work/spiral
```

准备流程通过核对歌曲名称、歌手、专辑和时长，自动选择完整的官方录音，将歌曲元数据、原始歌词响应、解码后的来源歌词与 Ruby、来源项目、去重后的教学输入、音频和封面保存到输出目录。随后，流程在歌曲的三个区段测量已知歌词与录音的时间关系；测量支持共同偏移后，才生成校准后的 `project.json`。编写学习内容之前，检查保存的来源证据、录音身份和 `project.calibration.json`。

首次校准将默认的 `small` 声学模型下载到 `work/whisper-models/`；后续校准复用该缓存。`project-source.json` 保留来源时间区间。`project.json` 保存通过验收的 `lyric_offset_ms` 与校准报告路径，`project.measurements.json` 保留实际测量位置。音频保留完整的原始时间。[校准参考](../subsystems/teaching-video.zh.md#自动歌词校准)说明验收标准与失败行为。

独立校准已经保存的来源项目时，选择尚未使用的输出路径：

```sh
pnpm video:calibrate \
  --project work/spiral/project-source.json \
  --output work/spiral/project-calibrated.json
```

编辑器和渲染器使用生成的项目。[命令声明](../../tools/teaching-video/calibrate.py)负责定义可用模型选项。校准失败时，需要检查录音与保留的测量证据，再进行渲染。

## 复核教学内容

当前 Agent 根据 `teaching-input.json` 和来源 Ruby 编写 `work/spiral/teaching.json`。每条不同的演唱歌词都包含中文翻译、符合语境的词汇信息和表达讲解。重复歌词复用已经复核的条目。词汇卡片显示歌词中的表达、读音、原形、词性、活用信息、中文释义，以及存在依据的 JLPT 等级。

阅读完整教学文件，并与原文及其语境逐条核对。确认释义、语法讲解和来源读音。渲染器验证[教学契约](../subsystems/teaching-video.zh.md)，语言内容的准确性需要通过这项内容复核确认。

## 使用图形界面编辑视频

使用准备好的输入与独立的编辑目录打开本地编辑器：

```sh
pnpm video:edit \
  --project work/spiral/project.json \
  --teaching work/spiral/teaching.json \
  --font /absolute/path/to/CJK-font.ttf \
  --workspace work/spiral-editor
```

在浏览器打开 `http://127.0.0.1:49603`。选择歌词场景或者调整录音播放时间。双击任意可见文字即可在预览中直接编辑；Enter 应用修改，Shift+Enter 为普通文字插入段落，Escape 取消编辑。属性面板也能修改歌词、Ruby、罗马音、翻译、词汇详情、表达讲解、标题、歌曲信息、分页文字和播放时间标签。歌词与读音修改保留来源时间区间；导出之前需要复核措辞和读音。

在预览或组件列表中选择组件。Shift 点击添加或者移除选择；从预览空白位置拖出矩形，可以选择完全包含在矩形中的组件。完整包含的区域会一起选择其中的内容。原文片段默认联动选择对应的 Ruby；关闭 Ruby 联动选项后，可以独立选择。双击文字字段会单独选择该字段进行直接编辑。一起移动当前片段的三行歌词时，选择「当前片段」范围，勾选「原文」「罗马字」「当前句翻译」，然后点击「选择组件」。

选择「全部片段」和「罗马字」，可以选择整首歌曲中的这一行。操作部分片段时，在左侧勾选所需片段，再选择「选定片段」范围。可以同时勾选多种组件类型。选择状态显示组件数量和片段数量，预览显示当前画面中的选中组件。切换查看的片段会保留选择。

拖动选中组件或者「移动选中」控件，输入单个组件的坐标或多个组件的位移，或者使用方向键移动。Shift+方向键每次移动十个像素。同时选中父组件和子组件时，位移只应用一次。「隐藏」使选中组件不出现在渲染视频中，同时在组件列表保留内容和位置；「显示」恢复可见性。「删除」移除组件。每次批量操作可以一起撤销。「恢复删除」恢复已经删除的组件，「恢复自动位置」移除选中组件的手动位移。

自动排版默认关闭，编辑、隐藏或者删除后，其他组件保留原位置。「排版一次」整理上方片段范围中的全部可见组件，清除该范围在当前画面比例下的手动位移，同时保留自动排版开关状态。撤销操作恢复原有排版和位移。需要在内容及可见性变化时持续重新计算卡片与分页时，启用「自动排版」；关闭后保留当前排版。排版控件说明完整片段范围，独立于组件类型选择。

歌曲信息、背景、时间标签和进度条的修改应用于整首录音。其他组件属于各自选定的歌词演唱场景。文字、可见性和删除修改应用于两种画面比例；横屏和竖屏分别保存位置与排版。高亮颜色自动从歌曲封面提取。「纯净预览」隐藏选择控件，同时保留实际渲染画面。

有效的修改自动保存到编辑目录中的 `edit.json`。「保存项目」也能明确保存该文件。使用相同输入与编辑目录重新打开后，编辑器恢复已经保存的内容。「导出完整视频」创建新的导出目录，保存编辑文件快照，编码完整 MP4，并执行完整歌曲验证。导出通过后显示下载链接；失败时，界面显示具体错误。编辑与导出期间需要保持编辑器进程运行。

为下方渲染命令添加 `--edits work/spiral-editor/edit.json`，命令渲染器即可使用同一个编辑结果。[编辑器 E2E](../../tools/teaching-video/editor-e2e.mjs) 验证真实浏览器中的编辑、保存、重新打开、不同宽度的预览和完整视频导出；使用新建编辑目录启动编辑器后运行该检查，并保留证据目录。

[多选 E2E](../../tools/teaching-video/editor-selection-e2e.mjs) 验证多选、跨片段移动、隐藏、删除、单次排版、画面比例独立性、保存、重新打开和完整导出。使用新建编辑目录启动编辑器后，运行 `node tools/teaching-video/editor-selection-e2e.mjs --url http://127.0.0.1:49603 --evidence-dir work/editor-verification/selection-evidence`。输出包含保存的编辑文件、实际预览图和位置、交互报告，以及完整歌曲验证报告。

## 渲染完整视频

```sh
pnpm video:render \
  --project work/spiral/project.json \
  --teaching work/spiral/teaching.json \
  --layout landscape \
  --font /absolute/path/to/CJK-font.ttf \
  --output work/spiral/spiral-landscape.mp4
```

使用 `--layout portrait` 和另外的输出文件名生成竖屏画面。渲染器的[命令声明](../../tools/teaching-video/render.py) 负责定义其他可用选项。导出时省略时间范围限制，并保留默认的零音频偏移，以包含完整录音。

## 验证并取得产物

```sh
pnpm video:verify \
  --manifest work/spiral/spiral-landscape.render.json \
  --require-full-song \
  --evidence-dir work/spiral/spiral-landscape-evidence
```

`--require-full-song` 要求导出从歌曲开始位置覆盖整首歌曲的完整画面数量，并使用零音频偏移。验证流程检查校准报告及其三个区段的测量结果，解码完整 MP4，检查视频与音频元数据，将每个教学分页和选定的校准后高亮时刻与编码画面比较，并将成品音频与来源录音比较。证据目录包含 `verification.json` 和采样画面文件。确认报告中的 `status: passed` 与 `song_scope: full`，并检查 `lyric_alignment` 中通过验收的偏移与区段剩余误差。检查通过后，Agent 返回完整视频和证据文件的链接。
