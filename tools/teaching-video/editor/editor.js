const $ = (id) => document.getElementById(id);
const state = { session: null, edits: null, layout: 'landscape', time: 0, preview: null, selected: new Set(), scenes: new Set(), catalog: [],
  undo: [], redo: [], revision: 0, rendered: -1, loading: false, blocking: false, arranging: false, valid: false, exporting: false, saved: '', inline: null };
let saveTimer, playbackTimer;
const audio = $('audio');

function message(error) {
  $('error').hidden = !error;
  $('error').textContent = error || '';
}

async function api(path, body) {
  const response = await fetch(path, body === undefined ? {} : {
    method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Editor-Token': state.session.token },
    body: JSON.stringify(body),
  });
  const result = await response.json();
  if (!response.ok) throw new Error(typeof result.detail === 'string' ? result.detail : JSON.stringify(result.detail));
  return result;
}

function payload() {
  return { edits: state.edits, layout: state.layout, time_ms: state.time };
}

function controls() {
  $('undo').disabled = !state.undo.length || state.arranging;
  $('redo').disabled = !state.redo.length || state.arranging;
  $('save').disabled = !state.valid || state.blocking || state.arranging;
  $('export').disabled = !state.valid || state.blocking || state.arranging || state.exporting;
  $('restore-deleted').disabled = !state.edits?.deleted.length;
  for (const id of ['select-components', 'arrange-once', 'automatic', 'hide', 'show', 'delete', 'apply-position', 'reset-position', 'apply-text']) $(id).disabled = !state.valid || state.blocking || state.arranging;
  $('arrange-once').disabled = !state.preview || state.blocking || state.arranging;
  if (state.valid && !state.blocking && !state.arranging) {
    const nodes = selectedNodes();
    $('hide').disabled = !nodes.length || nodes.every(node => node.hidden);
    $('show').disabled = !nodes.length || nodes.every(node => !node.hidden);
  }
}

function mutate(action) {
  if (state.arranging) return;
  state.undo.push(structuredClone(state.edits));
  if (state.undo.length > 100) state.undo.shift();
  state.redo = [];
  action(state.edits);
  $('save-status').textContent = '有未保存的修改';
  state.valid = false;
  refresh();
}

function history(direction) {
  if (state.arranging) return;
  finishInline();
  const from = state[direction];
  if (!from.length) return;
  state[direction === 'undo' ? 'redo' : 'undo'].push(structuredClone(state.edits));
  state.edits = from.pop();
  state.valid = false;
  refresh();
}

function setTime(time, background = false) {
  state.time = Math.max(0, Math.min(state.session.duration_ms - 1, time));
  $('timeline').value = state.time;
  $('time').value = (state.time / 1000).toFixed(1);
  $('clock').textContent = `${formatTime(state.time)} / ${formatTime(state.session.duration_ms)}`;
  refresh(background);
}

function formatTime(ms) {
  const seconds = Math.floor(ms / 1000);
  return `${String(Math.floor(seconds / 60)).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`;
}

async function refresh(background = false) {
  state.revision++;
  state.blocking ||= !background;
  $('rendering').hidden = !state.blocking;
  controls();
  if (state.loading || !state.session) return;
  state.loading = true;
  $('stage').setAttribute('aria-busy', 'true');
  controls();
  try {
    while (state.rendered !== state.revision) {
      const revision = state.revision;
      const request = structuredClone(payload());
      const preview = await api('/api/preview', request);
      state.rendered = revision;
      if (revision !== state.revision) continue;
      state.preview = preview;
      state.catalog = preview.catalog;
      state.selected = new Set([...state.selected].filter(id => state.catalog.some(node => node.id === id)));
      state.valid = true;
      $('preview').src = preview.image;
      $('stage').classList.toggle('portrait', state.layout === 'portrait');
      fitCanvas();
      $('dimensions').textContent = `${preview.width} × ${preview.height} · 30 fps`;
      $('page-info').textContent = `教学内容 ${preview.page + 1} / ${preview.page_count} 页`;
      $('accent').textContent = preview.accent;
      document.documentElement.style.setProperty('--accent', preview.accent);
      $('landscape').setAttribute('aria-pressed', String(state.layout === 'landscape'));
      $('portrait').setAttribute('aria-pressed', String(state.layout === 'portrait'));
      $('layout-control').dataset.layout = state.layout;
      $('automatic').checked = state.edits.automatic[state.layout];
      drawTargets();
      drawLayers();
      inspectSelection();
      drawLines();
      message('');
      clearTimeout(saveTimer);
      if (JSON.stringify(state.edits) !== state.saved) saveTimer = setTimeout(() => save().catch(fail), 800);
    }
  } catch (error) {
    state.valid = false;
    state.rendered = state.revision;
    fail(error);
  } finally {
    state.loading = false;
    state.blocking = false;
    $('stage').setAttribute('aria-busy', 'false');
    $('rendering').hidden = true;
    controls();
  }
}

function boundsStyle(element, bounds) {
  const { width, height } = state.preview;
  element.style.left = `${bounds[0] / width * 100}%`;
  element.style.top = `${bounds[1] / height * 100}%`;
  element.style.width = `${(bounds[2] - bounds[0]) / width * 100}%`;
  element.style.height = `${(bounds[3] - bounds[1]) / height * 100}%`;
}

function fitCanvas() {
  const area = document.querySelector('.stage-area');
  const style = getComputedStyle(area);
  const width = area.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
  const height = area.clientHeight - parseFloat(style.paddingTop) - parseFloat(style.paddingBottom);
  const ratio = state.layout === 'portrait' ? 9 / 16 : 16 / 9;
  const fitted = Math.min(width, height * ratio);
  $('stage').style.width = `${fitted}px`;
  $('stage').style.height = `${fitted / ratio}px`;
}
new ResizeObserver(fitCanvas).observe(document.querySelector('.stage-area'));

let glassFrame;
document.addEventListener('pointermove', event => {
  if (event.pointerType !== 'mouse' || matchMedia('(prefers-reduced-motion: reduce)').matches ||
      matchMedia('(prefers-reduced-transparency: reduce)').matches || matchMedia('(prefers-contrast: more)').matches) return;
  const glass = event.target.closest('[data-glass]');
  if (!glass) return;
  cancelAnimationFrame(glassFrame);
  glassFrame = requestAnimationFrame(() => {
    const bounds = glass.getBoundingClientRect();
    glass.style.setProperty('--glass-x', `${(event.clientX - bounds.left) / bounds.width * 100}%`);
    glass.style.setProperty('--glass-y', `${(event.clientY - bounds.top) / bounds.height * 100}%`);
  });
});

function drawTargets() {
  $('targets').replaceChildren();
  if ($('clean').checked) return;
  const nodes = state.preview.nodes.filter(n => n.id !== 'background' && !n.hidden);
  const ordered = [...nodes].sort((a, b) => Number(a.text !== null) - Number(b.text !== null));
  for (const node of ordered) {
    const button = document.createElement('button');
    button.className = 'target';
    button.dataset.id = node.id;
    button.dataset.text = String(node.text !== null);
    button.setAttribute('aria-label', `${node.label}${node.text !== null ? '：' + node.text : ''}`);
    button.title = node.text !== null ? '双击编辑文字，拖动调整位置' : '拖动调整位置';
    boundsStyle(button, node.bounds);
    button.addEventListener('click', event => { if (!event.detail) { pause(); select(node.id, event.shiftKey); } });
    button.addEventListener('dblclick', () => beginInline(node));
    button.addEventListener('pointerdown', event => drag(event, button, node));
    $('targets').append(button);
  }
}

function drawLayers() {
  $('layers').replaceChildren();
  const unique = new Map(state.catalog.filter(node => node.line_id === null || node.line_id === state.preview.line_id).map(node => [node.id, node]));
  function append(node, depth) {
    const button = document.createElement('button');
    button.className = `layer ${depth > 1 ? 'deep' : depth ? 'child' : ''}`;
    button.dataset.id = node.id;
    button.setAttribute('aria-pressed', String(state.selected.has(node.id)));
    const kind = document.createElement('i');
    kind.textContent = node.hidden ? '隐藏' : node.text !== null ? '文字' : '区域';
    const label = document.createElement('span');
    label.textContent = node.text || node.label;
    button.title = node.label + (node.text ? '：' + node.text : '');
    button.append(kind, label);
    button.addEventListener('click', event => { pause(); select(node.id, event.shiftKey); });
    $('layers').append(button);
    for (const child of unique.values()) if (child.parent === node.id) append(child, depth + 1);
  }
  for (const node of unique.values()) if (!node.parent || !unique.has(node.parent)) append(node, 0);
}

function drawLines() {
  const search = $('search').value.trim().toLowerCase();
  $('lines').replaceChildren();
  state.session.lines.forEach((line, index) => {
    if (search && !line.text.toLowerCase().includes(search)) return;
    const row = document.createElement('div');
    row.className = 'scene-row';
    const checkbox = document.createElement('input');
    checkbox.type = 'checkbox';
    checkbox.dataset.lineId = line.id;
    checkbox.setAttribute('aria-label', `选择片段 ${index + 1}：${line.text}`);
    checkbox.checked = state.scenes.has(line.id);
    checkbox.addEventListener('change', () => {
      if (checkbox.checked) state.scenes.add(line.id); else state.scenes.delete(line.id);
      sceneSelectionStatus();
    });
    const button = document.createElement('button');
    button.className = 'line';
    button.dataset.lineId = line.id;
    button.setAttribute('aria-current', String(state.preview?.line_id === line.id));
    const time = document.createElement('span');
    time.textContent = formatTime(line.start_ms);
    const text = document.createElement('strong');
    text.textContent = line.text;
    button.append(time, text);
    button.addEventListener('click', () => {
      pause(); finishInline();
      setTime(index === 0 ? line.start_ms : line.start_ms + 1);
      audio.currentTime = state.time / 1000;
    });
    row.append(checkbox, button);
    $('lines').append(row);
  });
  sceneSelectionStatus();
}

function sceneSelectionStatus() {
  $('scene-selection-count').textContent = `${state.scenes.size} 个已选`;
  $('all-scenes').checked = state.scenes.size === state.session.lines.length;
  $('all-scenes').indeterminate = state.scenes.size > 0 && state.scenes.size < state.session.lines.length;
}

function selectedNodes() {
  return state.catalog.filter(node => state.selected.has(node.id));
}

function roots() {
  return selectedNodes().filter(node => ![...state.selected].some(id => node.id.startsWith(id + '/')));
}

function extent(nodes) {
  return [Math.min(...nodes.map(n => n.bounds[0])), Math.min(...nodes.map(n => n.bounds[1])),
    Math.max(...nodes.map(n => n.bounds[2])), Math.max(...nodes.map(n => n.bounds[3]))];
}

function linkedIds(node) {
  return [node.id, ...($('link-ruby').checked ? node.linked || [] : [])];
}

function select(id, additive = false) {
  const node = state.catalog.find(n => n.id === id);
  if (!additive) state.selected.clear();
  if (node) {
    const ids = linkedIds(node);
    const remove = additive && state.selected.has(id);
    for (const key of ids) { if (remove) state.selected.delete(key); else state.selected.add(key); }
    if (remove && $('link-ruby').checked) {
      for (const selected of selectedNodes()) for (const key of selected.linked || []) state.selected.add(key);
    }
  }
  inspectSelection();
}

function inspectSelection() {
  const nodes = selectedNodes(), selected = roots();
  const node = nodes.length === 1 ? nodes[0] : null;
  const textNode = node || nodes.find(item => item.linked?.length && nodes.every(other => other.id === item.id || item.linked.includes(other.id)));
  state.textTarget = textNode?.text !== null ? textNode?.id : null;
  const local = state.preview.nodes.filter(n => !n.hidden && state.selected.has(n.id));
  const scenes = new Set(nodes.map(n => n.line_id).filter(id => id !== null));
  $('selection-summary').textContent = nodes.length ? `已选择 ${nodes.length} 个组件 · ${scenes.size} 个片段${nodes.some(n => n.line_id === null) ? ' · 包含歌曲公共组件' : ''}` : '尚未选择组件';
  $('selection').hidden = !local.length || $('clean').checked;
  $('inspector').hidden = !nodes.length;
  $('empty-selection').hidden = !!nodes.length;
  $('component-kind').textContent = node ? node.text !== null ? '文字' : '区域' : nodes.length ? '多选' : '选择组件';
  for (const button of document.querySelectorAll('.layer, .target')) {
    button.setAttribute('aria-pressed', String(state.selected.has(button.dataset.id)));
  }
  if (local.length) boundsStyle($('selection'), extent(local));
  if (!nodes.length) return;
  $('selected-name').textContent = node ? node.label : `同时操作 ${selected.length} 个组件`;
  $('text-controls').hidden = !textNode || textNode.text === null;
  if (textNode && document.activeElement !== $('text')) $('text').value = textNode.text || '';
  $('position-x-label').textContent = node ? '水平位置' : '水平位移';
  $('position-y-label').textContent = node ? '垂直位置' : '垂直位移';
  $('position-x').value = node ? node.bounds[0] : 0;
  $('position-y').value = node ? node.bounds[1] : 0;
  $('hide').disabled = nodes.every(n => n.hidden);
  $('show').disabled = nodes.every(n => !n.hidden);
  $('edit-scope').textContent = `移动作用于全部选中组件的${state.layout === 'landscape' ? '横屏' : '竖屏'}位置。文字、隐藏和删除同时应用于两种画面比例。`;
}

function beginInline(node) {
  if (node.text === null) return;
  pause();
  state.selected = new Set([node.id]);
  inspectSelection();
  state.inline = node.id;
  const input = $('inline-text');
  boundsStyle(input, node.bounds);
  input.style.width = `${Math.max(24, (node.bounds[2] - node.bounds[0]) / state.preview.width * 100)}%`;
  input.value = node.text;
  input.hidden = false;
  input.focus();
  input.select();
}

function finishInline(cancel = false) {
  if (!state.inline) return;
  const key = state.inline;
  const text = $('inline-text').value;
  const previous = state.preview.nodes.find(n => n.id === key)?.text;
  state.inline = null;
  $('inline-text').hidden = true;
  if (!cancel && text !== previous) {
    if (!text.trim()) { message('文字内容不能为空白；请使用删除组件操作。'); return; }
    mutate(edits => { edits.text[key] = text; });
  }
}

function move(dx, dy) {
  if (!dx && !dy) return;
  const ids = roots().map(node => node.id);
  mutate(edits => {
    for (const id of ids) {
      const previous = edits.positions[state.layout][id] || [0, 0];
      edits.positions[state.layout][id] = [previous[0] + dx, previous[1] + dy];
    }
  });
}

function drag(event, button, node) {
  if (event.button !== 0 || state.inline || state.blocking || state.arranging) return;
  pause();
  if (event.shiftKey && node) { select(node.id, true); return; }
  if (node && ![...state.selected].some(id => node.id === id || node.id.startsWith(id + '/'))) select(node.id);
  const nodes = roots();
  const local = state.preview.nodes.filter(n => !n.hidden && state.selected.has(n.id));
  if (!nodes.length || !local.length) return;
  const localBounds = extent(local);
  const x = event.clientX, y = event.clientY;
  const rect = $('stage').getBoundingClientRect();
  let delta = [0, 0], moved = false;
  button.setPointerCapture(event.pointerId);
  const onMove = e => {
    if (!moved && Math.hypot(e.clientX - x, e.clientY - y) < 4) return;
    moved = true;
    delta = [Math.round((e.clientX - x) * state.preview.width / rect.width), Math.round((e.clientY - y) * state.preview.height / rect.height)];
    const b = extent(nodes);
    delta = [Math.max(-b[0], Math.min(state.preview.width - b[2], delta[0])), Math.max(-b[1], Math.min(state.preview.height - b[3], delta[1]))];
    boundsStyle($('selection'), localBounds.map((value, i) => value + delta[i % 2]));
    const px = delta[0] * rect.width / state.preview.width, py = delta[1] * rect.height / state.preview.height;
    if (button.id !== 'move-selection') button.style.transform = `translate(${px}px,${py}px)`;
  };
  const end = e => {
    button.removeEventListener('pointermove', onMove);
    button.removeEventListener('pointerup', end);
    button.removeEventListener('pointercancel', end);
    button.style.transform = '';
    if (e.type !== 'pointercancel' && moved) { pause(); move(...delta); }
    else inspectSelection();
  };
  button.addEventListener('pointermove', onMove);
  button.addEventListener('pointerup', end);
  button.addEventListener('pointercancel', end);
}

function sceneRange() {
  const scope = $('selection-scope').value;
  const ids = scope === 'all' ? state.session.lines.map(line => line.id) : scope === 'marked' ? [...state.scenes] : [state.preview.line_id];
  if (!ids.length) throw new Error('请先勾选需要操作的歌词片段。');
  return ids;
}

function chooseComponents() {
  const scenes = new Set(sceneRange());
  const types = new Set([...document.querySelectorAll('#selection-types input:checked')].map(input => input.value));
  if (!types.size) throw new Error('请选择至少一种组件类型。');
  const ids = [];
  for (const node of state.catalog) {
    if (!scenes.has(node.line_id)) continue;
    const parts = node.id.split('/');
    if ((parts.length === 2 && types.has(parts[1])) || (parts[2]?.startsWith('ruby:') && types.has('ruby'))) ids.push(...linkedIds(node));
  }
  pause(); finishInline();
  state.selected = new Set(ids);
  inspectSelection();
  message(ids.length ? '' : '所选片段中没有这些组件。');
}

async function arrange(lines = sceneRange(), automatic = null) {
  pause(); finishInline();
  state.arranging = true;
  controls();
  $('rendering').hidden = false;
  try {
    const body = structuredClone(payload());
    const result = await api('/api/arrange', { ...body, lines });
    if (automatic === false) result.edits.positions[body.layout] = body.edits.positions[body.layout];
    if (automatic !== null) result.edits.automatic[body.layout] = automatic;
    state.arranging = false;
    mutate(edits => { Object.assign(edits, result.edits); });
  } finally {
    state.arranging = false;
    if (!state.loading) $('rendering').hidden = true;
    controls();
  }
}

$('stage').addEventListener('pointerdown', event => {
  if (event.button !== 0 || state.inline || state.blocking || state.arranging || $('clean').checked || event.target.closest('button, textarea')) return;
  pause();
  const rect = $('stage').getBoundingClientRect();
  const start = [event.clientX - rect.left, event.clientY - rect.top];
  const prior = event.shiftKey ? new Set(state.selected) : new Set();
  $('stage').setPointerCapture(event.pointerId);
  const update = e => {
    const end = [Math.max(0, Math.min(rect.width, e.clientX - rect.left)), Math.max(0, Math.min(rect.height, e.clientY - rect.top))];
    const box = [Math.min(start[0], end[0]), Math.min(start[1], end[1]), Math.max(start[0], end[0]), Math.max(start[1], end[1])];
    $('marquee').hidden = false;
    Object.assign($('marquee').style, { left: box[0] + 'px', top: box[1] + 'px', width: box[2] - box[0] + 'px', height: box[3] - box[1] + 'px' });
    const pixels = box.map((value, i) => value * (i % 2 ? state.preview.height / rect.height : state.preview.width / rect.width));
    state.selected = new Set(prior);
    for (const node of state.preview.nodes) {
      if (node.hidden || node.id === 'background') continue;
      const b = node.bounds;
      if (pixels[0] <= b[0] && pixels[1] <= b[1] && pixels[2] >= b[2] && pixels[3] >= b[3]) {
        for (const id of linkedIds(state.catalog.find(item => item.id === node.id))) state.selected.add(id);
      }
    }
    state.selected = new Set([...state.selected].filter(id => ![...state.selected].some(parent => id.startsWith(parent + '/'))));
    inspectSelection();
  };
  const end = e => {
    if (e.type === 'pointercancel') state.selected = prior; else update(e);
    $('stage').removeEventListener('pointermove', update);
    $('stage').removeEventListener('pointerup', end);
    $('stage').removeEventListener('pointercancel', end);
    $('marquee').hidden = true;
    inspectSelection();
  };
  $('stage').addEventListener('pointermove', update);
  $('stage').addEventListener('pointerup', end);
  $('stage').addEventListener('pointercancel', end);
});
$('move-selection').addEventListener('pointerdown', event => { event.stopPropagation(); drag(event, $('move-selection'), null); });

async function save() {
  if (!state.valid) return;
  const body = structuredClone(payload());
  await api('/api/save', body);
  state.saved = JSON.stringify(body.edits);
  $('save-status').textContent = state.saved === JSON.stringify(state.edits) ? '修改已保存' : '有未保存的修改';
}

function fail(error) { message(error.message); }
function pause() { audio.pause(); clearInterval(playbackTimer); $('play').textContent = '播放'; $('play').setAttribute('aria-label', '播放预览'); }

async function play() {
  if (!audio.paused) { pause(); return; }
  finishInline();
  audio.currentTime = state.time / 1000;
  await audio.play();
  $('play').textContent = '暂停'; $('play').setAttribute('aria-label', '暂停预览');
  playbackTimer = setInterval(() => setTime(audio.currentTime * 1000, true), 250);
}

async function exportVideo() {
  finishInline(); pause();
  if (!state.valid) return;
  state.exporting = true; controls();
  $('export-status').hidden = false;
  $('downloads').replaceChildren();
  $('export-message').textContent = '正在准备完整视频';
  try {
    const job = await api('/api/export', payload());
    localStorage.setItem('utabuild-export:' + state.session.workspace, job.id);
    await watchExport(job.id);
  } catch (error) { state.exporting = false; controls(); fail(error); }
}

async function watchExport(id) {
  state.exporting = true; controls(); $('export-status').hidden = false;
  while (true) {
    const job = await api(`/api/exports/${id}`);
    $('export-progress').value = job.progress;
    $('export-message').textContent = { queued:'正在准备完整视频', rendering:`正在编码 · ${Math.floor(job.progress * 100)}%`, verifying:'正在检查完整视频、画面和音频', passed:'完整视频已导出，验证通过', failed:'视频导出失败' }[job.status];
    if (job.status === 'failed') throw new Error(job.error);
    if (job.status === 'passed') {
      for (const [href, label] of [[job.download, '下载 MP4'], [job.verification, '下载验证记录']]) {
        const a = document.createElement('a'); a.href = href; a.textContent = label; a.download = ''; $('downloads').append(a);
      }
      state.exporting = false; controls(); return;
    }
    await new Promise(resolve => setTimeout(resolve, 1500));
  }
}

$('undo').addEventListener('click', () => history('undo'));
$('redo').addEventListener('click', () => history('redo'));
$('save').addEventListener('click', () => save().catch(fail));
$('export').addEventListener('click', () => exportVideo().catch(fail));
$('play').addEventListener('click', () => play().catch(fail));
audio.addEventListener('ended', pause);
$('search').addEventListener('input', drawLines);
$('clean').addEventListener('change', () => { drawTargets(); inspectSelection(); });
for (const layout of ['landscape', 'portrait']) $(layout).addEventListener('click', () => { finishInline(); pause(); state.layout = layout; refresh(); });
$('all-scenes').addEventListener('change', () => {
  state.scenes = new Set($('all-scenes').checked ? state.session.lines.map(line => line.id) : []);
  drawLines();
});
$('select-components').addEventListener('click', () => { try { chooseComponents(); } catch (error) { fail(error); } });
$('clear-selection').addEventListener('click', () => { state.selected.clear(); inspectSelection(); });
$('arrange-once').addEventListener('click', () => arrange().catch(fail));
$('automatic').addEventListener('change', () => {
  if ($('automatic').checked) mutate(edits => { edits.automatic[state.layout] = true; });
  else arrange(state.session.lines.map(line => line.id), false).catch(fail);
});
$('timeline').addEventListener('input', () => { pause(); finishInline(); setTime(Number($('timeline').value)); audio.currentTime = state.time / 1000; });
$('time').addEventListener('change', () => { pause(); setTime(Number($('time').value) * 1000); audio.currentTime = state.time / 1000; });
for (const [id, direction] of [['previous-line', -1], ['next-line', 1]]) $(id).addEventListener('click', () => {
  pause(); finishInline();
  const index = state.session.lines.findIndex(line => line.id === state.preview.line_id);
  const line = state.session.lines[Math.max(0, Math.min(state.session.lines.length - 1, index + direction))];
  setTime(line.start_ms + 1); audio.currentTime = state.time / 1000;
});
$('inspector').addEventListener('submit', event => {
  event.preventDefault();
  const text = $('text').value;
  if (!text.trim()) { message('文字内容不能为空白；请使用删除组件操作。'); return; }
  const id = state.textTarget;
  if (!id) return;
  mutate(edits => { edits.text[id] = text; });
});
$('apply-position').addEventListener('click', () => {
  const nodes = selectedNodes();
  const origin = nodes.length === 1 ? nodes[0].bounds : [0, 0];
  move(Number($('position-x').value) - origin[0], Number($('position-y').value) - origin[1]);
});
$('reset-position').addEventListener('click', () => {
  const ids = roots().map(node => node.id);
  mutate(edits => { for (const id of ids) delete edits.positions[state.layout][id]; });
});
$('delete').addEventListener('click', () => {
  const ids = roots().map(node => node.id);
  mutate(edits => { edits.deleted = [...new Set([...edits.deleted, ...ids])]; });
});
$('hide').addEventListener('click', () => {
  const ids = roots().map(node => node.id);
  mutate(edits => { edits.hidden = [...new Set([...edits.hidden, ...ids])]; });
});
$('show').addEventListener('click', () => {
  const ids = roots().map(node => node.id);
  mutate(edits => {
    const hidden = new Set();
    function retain(key) {
      if (ids.some(id => key === id || key.startsWith(id + '/'))) return;
      if (!ids.some(id => id.startsWith(key + '/'))) { hidden.add(key); return; }
      for (const node of state.catalog) {
        if (node.id.startsWith(key + '/') && !node.id.slice(key.length + 1).includes('/')) retain(node.id);
      }
    }
    for (const key of edits.hidden) retain(key);
    edits.hidden = [...hidden];
  });
});
$('restore-deleted').addEventListener('click', () => mutate(edits => { edits.deleted = []; }));
$('inline-text').addEventListener('blur', () => finishInline());
$('inline-text').addEventListener('keydown', event => {
  event.stopPropagation();
  if (event.key === 'Escape') { event.preventDefault(); finishInline(true); $('stage').focus(); }
  if (event.key === 'Enter' && !event.shiftKey && !event.isComposing) { event.preventDefault(); finishInline(); $('stage').focus(); }
});
document.addEventListener('keydown', event => {
  if (event.defaultPrevented) return;
  if (['INPUT', 'TEXTAREA'].includes(document.activeElement.tagName)) return;
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'z') { event.preventDefault(); history(event.shiftKey ? 'redo' : 'undo'); return; }
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's') { event.preventDefault(); save().catch(fail); return; }
  if (event.key === 'Escape') { state.selected.clear(); inspectSelection(); return; }
  const active = document.activeElement;
  if (active.closest('button, a, select') && !active.classList.contains('target') && !active.classList.contains('layer') && active.id !== 'move-selection') return;
  const nodes = selectedNodes();
  if (!nodes.length || state.blocking || state.arranging) return;
  if (event.key === 'Enter' && state.textTarget) { event.preventDefault(); beginInline(nodes.find(node => node.id === state.textTarget)); }
  if (event.key === 'Delete' || event.key === 'Backspace') { event.preventDefault(); $('delete').click(); }
  const delta = { ArrowLeft:[-1,0], ArrowRight:[1,0], ArrowUp:[0,-1], ArrowDown:[0,1] }[event.key];
  if (delta) { event.preventDefault(); move(...delta.map(value => value * (event.shiftKey ? 10 : 1))); }
});
window.addEventListener('beforeunload', event => {
  if (state.edits && state.saved !== JSON.stringify(state.edits)) { event.preventDefault(); event.returnValue = ''; }
});

async function init() {
  state.session = await api('/api/session');
  state.edits = state.session.edits;
  state.saved = JSON.stringify(state.edits);
  $('song-title').textContent = state.session.title;
  $('song-artist').textContent = state.session.artist;
  document.title = `${state.session.title} · UtaBuild 视频后期编辑`;
  $('line-count').textContent = `${state.session.lines.length} 句`;
  $('timeline').max = state.session.duration_ms - 1;
  $('time').max = state.session.duration_ms / 1000;
  for (const line of state.session.lines) {
    const marker = document.createElement('i'); marker.style.left = `${line.start_ms / state.session.duration_ms * 100}%`; $('markers').append(marker);
  }
  $('save-status').textContent = '项目已保存';
  setTime(state.session.lines[0].start_ms);
  const exportId = localStorage.getItem('utabuild-export:' + state.session.workspace);
  if (exportId) watchExport(exportId).catch(error => { state.exporting = false; controls(); fail(error); });
}
init().catch(fail);
