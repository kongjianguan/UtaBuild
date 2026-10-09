import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { parseArgs } from 'node:util';
import { chromium } from 'playwright';

const { values } = parseArgs({ options: {
  url: { type: 'string', default: 'http://127.0.0.1:49604' },
  'evidence-dir': { type: 'string', default: 'work/editor-e2e/evidence' },
} });
const evidence = resolve(values['evidence-dir']);
await mkdir(evidence, { recursive: true });
const browser = await chromium.launch({ channel: 'chrome', headless: true });
const page = await browser.newPage({ viewport: { width: 1440, height: 1000 }, reducedMotion: 'reduce' });
const failures = [];
page.on('pageerror', error => failures.push(error.message));
page.on('console', message => { if (message.type() === 'error' && !message.text().includes('422')) failures.push(message.text()); });
const checks = [];
const record = (name, detail) => { checks.push({ name, detail }); console.log(name); };
let current;
const node = id => { const item = current.nodes.find(n => n.id === id); assert.ok(item, `组件缺失：${id}`); return item; };
async function ready() {
  await page.waitForFunction(() => document.getElementById('stage').getAttribute('aria-busy') === 'false', undefined, { timeout: 120000 });
  assert.equal(await page.locator('#error').isVisible(), false, await page.locator('#error').textContent());
}
async function change(action) {
  const response = page.waitForResponse(r => r.url().endsWith('/api/preview') && r.request().method() === 'POST', { timeout: 120000 });
  await action();
  const result = await response;
  assert.equal(result.status(), 200, await result.text());
  current = await result.json();
  await ready();
}
async function select(id) {
  await page.locator(`#layers button[data-id="${id}"]`).click();
}
async function edit(id, text) {
  await select(id);
  await page.locator('#text').fill(text);
  await change(() => page.locator('#apply-text').click());
  assert.equal(node(id).text, text);
}
async function savePreview(name) {
  await writeFile(resolve(evidence, name + '.png'), Buffer.from(current.image.split(',')[1], 'base64'));
  await writeFile(resolve(evidence, name + '.json'), JSON.stringify({ ...current, image: undefined }, null, 2) + '\n');
}
async function exportCurrent(layout) {
  await change(() => page.locator('#' + layout).click());
  await page.locator('#export').click();
  await page.waitForFunction(() => document.getElementById('export-message').textContent === '完整视频已导出，验证通过' || document.getElementById('export-message').textContent === '视频导出失败', undefined, { timeout: 1800000 });
  assert.equal(await page.locator('#export-message').textContent(), '完整视频已导出，验证通过', await page.locator('#error').textContent());
  const reportLink = await page.locator('#downloads a').nth(1).getAttribute('href');
  const response = await page.request.get(new URL(reportLink, values.url).href);
  const report = await response.json();
  assert.equal(report.status, 'passed');
  assert.equal(report.song_scope, 'full');
  assert.equal(report.full_decode, true);
  assert.ok(report.audio.correlation > 0.97);
  assert.ok(report.baseline_check.glyph_samples > 500);
  await writeFile(resolve(evidence, layout + '-verification.json'), JSON.stringify(report, null, 2) + '\n');
  record(layout + '-full-export', { video: report.video, report: report.manifest, frames: report.sampled_frames.length, audio: report.audio.correlation });
}

try {
  const response = page.waitForResponse(r => r.url().endsWith('/api/preview'));
  await page.goto(values.url);
  current = await (await response).json();
  await ready();
  const session = await (await page.request.get(new URL('/api/session', values.url).href)).json();
  assert.deepEqual(session.edits.text, {}, '请使用新建的 E2E 编辑目录');
  assert.deepEqual(session.edits.deleted, []);
  record('real-source-loaded', { title: session.title, lines: session.lines.length, accent: current.accent });
  await change(() => page.locator('#automatic').check());
  await savePreview('initial');
  const prefix = `line:${current.line_id}`;

  await page.locator('#targets button[data-id="title"]').dblclick();
  await page.locator('#inline-text').fill(session.title + ' · 歌曲学习');
  await change(() => page.locator('#inline-text').press('Enter'));
  assert.equal(node('title').text, session.title + ' · 歌曲学习');
  record('direct-preview-text-edit', node('title'));

  const wordId = current.nodes.find(n => n.id.startsWith(prefix + '/lyrics/word:')).id;
  const wordText = node(wordId).text;
  await edit(wordId, wordText + 'あ');
  const rubyId = current.nodes.find(n => n.id.startsWith(prefix + '/lyrics/ruby:')).id;
  await edit(rubyId, node(rubyId).text + 'あ');
  const romanId = current.nodes.find(n => n.id.startsWith(prefix + '/roman/word:')).id;
  await edit(romanId, node(romanId).text + 'a');
  record('timed-text-and-ruby-edit', { word: node(wordId), ruby: node(rubyId), roman: node(romanId) });
  // 保留原歌词和读音，导出验证仍使用真实歌曲内容。
  await edit(wordId, wordText);
  await change(() => page.locator('#undo').click());
  await change(() => page.locator('#redo').click());
  await edit(rubyId, node(rubyId).text.slice(0, -1));
  await edit(romanId, node(romanId).text.slice(0, -1));
  await edit(prefix + '/translation', '曾经绘制的地图，已经被撕裂。');
  const cardId = current.nodes.find(n => /\/vocabulary\/card:\d+$/.test(n.id)).id;
  await edit(cardId + '/meaning', node(cardId + '/meaning').text + '（结合歌词理解）');
  const expressionId = current.nodes.find(n => n.id.endsWith('/explanation')).id;
  await edit(expressionId, node(expressionId).text + '（结合上下文理解）');
  await edit(prefix + '/vocabulary/heading', '重点单词');
  await edit('clock', '歌曲学习 · ' + session.title);
  record('teaching-and-footer-text-edit', { translation: node(prefix + '/translation').text, heading: node(prefix + '/vocabulary/heading').text });

  const beforeDrag = [...node('title').bounds];
  await select('title');
  await change(() => page.locator('#layers button[data-id="title"]').press('ArrowRight'));
  assert.equal(node('title').bounds[0], beforeDrag[0] + 1);
  record('keyboard-move', node('title'));
  await page.locator('#stage').scrollIntoViewIfNeeded();
  const hit = await page.locator('#targets button[data-id="title"]').boundingBox();
  await change(async () => {
    await page.mouse.move(hit.x + hit.width / 2, hit.y + hit.height / 2);
    await page.mouse.down();
    await page.mouse.move(hit.x + hit.width / 2 + 14, hit.y + hit.height / 2 + 6, { steps: 6 });
    await page.mouse.up();
  });
  assert.ok(node('title').bounds[0] > beforeDrag[0]);
  record('pointer-drag-real-geometry', { before: beforeDrag, after: node('title').bounds });
  const lyricBefore = [...node(wordId).bounds];
  await select(prefix + '/lyrics');
  await page.locator('#position-x').fill(String(node(prefix + '/lyrics').bounds[0] + 12));
  await page.locator('#position-y').fill(String(node(prefix + '/lyrics').bounds[1] + 8));
  await change(() => page.locator('#apply-position').click());
  assert.equal(node(wordId).bounds[0], lyricBefore[0] + 12);
  assert.equal(node(wordId).bounds[1], lyricBefore[1] + 8);
  record('group-move-with-shared-baselines', { before: lyricBefore, after: node(wordId).bounds });

  const beforeCard = [...node(cardId).bounds];
  await select(cardId + '/meaning');
  await change(() => page.locator('#delete').click());
  assert.equal(current.nodes.some(n => n.id === cardId + '/meaning'), false);
  assert.ok(node(cardId).bounds[3] - node(cardId).bounds[1] < beforeCard[3] - beforeCard[1]);
  record('delete-text-reflows-card', { before: beforeCard, after: node(cardId).bounds });
  const nextCard = current.nodes.find(n => /\/vocabulary\/card:\d+$/.test(n.id) && n.id !== cardId).id;
  const cardOrigin = [...node(cardId).bounds];
  await select(cardId);
  await change(() => page.locator('#delete').click());
  assert.equal(current.nodes.some(n => n.id === cardId), false);
  assert.equal(node(nextCard).bounds[0], cardOrigin[0]);
  assert.equal(node(nextCard).bounds[1], cardOrigin[1]);
  record('delete-card-repacks-columns', node(nextCard));

  const headingBefore = [...node(prefix + '/vocabulary/heading').bounds];
  await select(prefix + '/translation');
  await change(() => page.locator('#delete').click());
  assert.ok(node(prefix + '/vocabulary/heading').bounds[1] < headingBefore[1]);
  await change(() => page.locator('#undo').click());
  assert.equal(node(prefix + '/vocabulary/heading').bounds[1], headingBefore[1]);
  await change(() => page.locator('#redo').click());
  assert.equal(current.nodes.some(n => n.id === prefix + '/translation'), false);
  await change(() => page.locator('#undo').click());
  record('delete-region-reflow-undo-redo', { restored: node(prefix + '/translation').text });
  await select(prefix + '/lyrics');
  await change(() => page.locator('#delete').click());
  assert.equal(current.nodes.some(n => n.id.startsWith(prefix + '/lyrics')), false);
  await change(() => page.locator('#undo').click());
  record('delete-whole-lyric-region', { restored: true });
  const expressionBefore = [...node(prefix + '/expressions').bounds];
  await select(prefix + '/vocabulary');
  await change(() => page.locator('#delete').click());
  assert.ok(node(prefix + '/expressions').bounds[1] < expressionBefore[1]);
  record('delete-vocabulary-moves-expression-region', { before: expressionBefore, after: node(prefix + '/expressions').bounds });
  await change(() => page.locator('#undo').click());
  await savePreview('edited-landscape');

  await change(() => page.locator('#portrait').click());
  await change(() => page.locator('#automatic').check());
  assert.equal(node('title').bounds[0], 64);
  assert.equal(node('title').bounds[1], 55);
  await select('title');
  await page.locator('#position-y').fill('63');
  await change(() => page.locator('#apply-position').click());
  assert.equal(node('title').bounds[1], 63);
  record('portrait-independent-position', node('title'));
  await savePreview('edited-portrait');
  await page.locator('#save').click();
  await page.waitForFunction(() => document.getElementById('save-status').textContent === '修改已保存');
  const savedSession = await (await page.request.get(new URL('/api/session', values.url).href)).json();
  const reloadResponse = page.waitForResponse(r => r.url().endsWith('/api/preview'));
  await page.reload();
  current = await (await reloadResponse).json();
  await ready();
  assert.equal(node('title').text, session.title + ' · 歌曲学习');
  const reloaded = await (await page.request.get(new URL('/api/session', values.url).href)).json();
  assert.deepEqual(reloaded.edits, savedSession.edits);
  record('save-reload-identical-document', { path: savedSession.workspace, edits: savedSession.edits });

  for (const layout of ['landscape', 'portrait']) {
    await change(() => page.locator('#' + layout).click());
    for (const width of [1440, 768, 375]) {
      await page.setViewportSize({ width, height: 1000 });
      await page.waitForFunction(ratio => {
        const rect = document.getElementById('stage').getBoundingClientRect();
        return Math.abs(rect.width / rect.height - ratio) < 0.002;
      }, layout === 'landscape' ? 16 / 9 : 9 / 16);
      const dimensions = await page.evaluate(() => {
        const rect = document.getElementById('stage').getBoundingClientRect();
        return { ratio: rect.width / rect.height, overflow: document.documentElement.scrollWidth > innerWidth };
      });
      assert.equal(dimensions.overflow, false);
      assert.ok(Math.abs(dimensions.ratio - (layout === 'landscape' ? 16 / 9 : 9 / 16)) < 0.002);
      record(`responsive-${layout}-${width}`, dimensions);
    }
  }
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.locator('#play').click();
  await page.waitForFunction(() => document.getElementById('audio').currentTime > 0 && !document.getElementById('audio').paused);
  await select('title');
  assert.equal(await page.locator('#audio').evaluate(audio => audio.paused), true);
  record('real-audio-preview-and-edit-pause', { audio: '/api/audio' });
  await exportCurrent('landscape');
  await exportCurrent('portrait');
  assert.deepEqual(failures, []);
  await writeFile(resolve(evidence, 'editor-e2e.json'), JSON.stringify({ status: 'passed', url: values.url, checks, browser_errors: failures }, null, 2) + '\n');
  console.log(JSON.stringify({ status: 'passed', report: resolve(evidence, 'editor-e2e.json') }));
} finally {
  await browser.close();
}
