import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { parseArgs } from 'node:util';
import { chromium } from 'playwright';

const { values } = parseArgs({ options: {
  url: { type: 'string', default: 'http://127.0.0.1:49611' },
  'evidence-dir': { type: 'string', default: 'work/selection-e2e/evidence' },
  'skip-export': { type: 'boolean', default: false },
} });
const evidence = resolve(values['evidence-dir']);
await mkdir(evidence, { recursive: true });
const browser = await chromium.launch({ channel: 'chrome', headless: true });
const page = await browser.newPage({ viewport: { width: 1600, height: 1100 }, reducedMotion: 'reduce' });
const errors = [], checks = [];
page.on('pageerror', error => errors.push(error.message));
page.on('console', entry => { if (entry.type() === 'error' && !entry.text().includes('422')) errors.push(entry.text()); });
let current;
const record = (name, detail) => { checks.push({ name, detail }); console.log(name); };
const catalog = () => new Map(current.catalog.map(node => [node.id, structuredClone(node)]));
const node = id => { const result = current.catalog.find(item => item.id === id); assert.ok(result, id); return result; };
const bounds = id => [...node(id).bounds];
async function ready() {
  await page.waitForFunction(() => document.getElementById('stage').getAttribute('aria-busy') === 'false' && document.getElementById('rendering').hidden, undefined, { timeout: 120000 });
  assert.equal(await page.locator('#error').isVisible(), false, await page.locator('#error').textContent());
}
async function change(action) {
  const pending = page.waitForResponse(response => response.url().endsWith('/api/preview') && response.request().method() === 'POST', { timeout: 120000 });
  await action();
  const response = await pending;
  assert.equal(response.status(), 200, await response.text());
  current = await response.json();
  await ready();
}
async function choose(scope, types) {
  await page.locator('#selection-scope').selectOption(scope);
  for (const input of await page.locator('#selection-types input').all()) {
    await input.setChecked(types.includes(await input.getAttribute('value')));
  }
  await page.locator('#select-components').click();
}
async function move(dx, dy) {
  await page.locator('#position-x').fill(String(dx));
  await page.locator('#position-y').fill(String(dy));
  await change(() => page.locator('#apply-position').click());
}
function shifted(before, ids, dx, dy) {
  for (const [id, item] of before) {
    const moved = ids.some(parent => id === parent || id.startsWith(parent + '/'));
    assert.deepEqual(node(id).bounds, item.bounds.map((value, index) => value + (moved ? index % 2 ? dy : dx : 0)), id);
  }
}
async function snapshot(name) {
  await writeFile(resolve(evidence, name + '.png'), Buffer.from(current.image.split(',')[1], 'base64'));
  await writeFile(resolve(evidence, name + '.json'), JSON.stringify({ ...current, image: undefined }, null, 2) + '\n');
}
async function exportVideo(layout) {
  if (layout !== (current.width === 1920 ? 'landscape' : 'portrait')) await change(() => page.locator('#' + layout).click());
  await page.locator('#export').click();
  await page.waitForFunction(() => ['完整视频已导出，验证通过', '视频导出失败'].includes(document.getElementById('export-message').textContent), undefined, { timeout: 1800000 });
  assert.equal(await page.locator('#export-message').textContent(), '完整视频已导出，验证通过', await page.locator('#error').textContent());
  const link = await page.locator('#downloads a').nth(1).getAttribute('href');
  const report = await (await page.request.get(new URL(link, values.url).href)).json();
  assert.equal(report.status, 'passed');
  assert.equal(report.song_scope, 'full');
  assert.equal(report.full_decode, true);
  assert.equal(report.baseline_check.max_encoded_vertical_offset, 0);
  assert.ok(report.baseline_check.glyph_samples > 500);
  assert.ok(report.audio.correlation > 0.97);
  await writeFile(resolve(evidence, layout + '-verification.json'), JSON.stringify(report, null, 2) + '\n');
  record(layout + '-full-export', { video: report.video, glyphs: report.baseline_check.glyph_samples, audio: report.audio.correlation });
}

try {
  const initial = page.waitForResponse(response => response.url().endsWith('/api/preview'), { timeout: 120000 });
  await page.goto(values.url);
  const response = await initial;
  assert.equal(response.status(), 200, await response.text());
  current = await response.json();
  await ready();
  const session = await (await page.request.get(new URL('/api/session', values.url).href)).json();
  assert.deepEqual(session.edits.text, {});
  assert.deepEqual(session.edits.deleted, []);
  assert.deepEqual(session.edits.hidden, []);
  assert.deepEqual(session.edits.automatic, { landscape: false, portrait: false });
  record('real-source-and-default-manual-layout', { title: session.title, scenes: session.lines.length, components: current.catalog.length });
  await snapshot('initial-landscape');
  const line = current.line_id, prefix = `line:${line}`;
  const group = ['lyrics', 'roman', 'translation'].map(kind => prefix + '/' + kind);

  await page.locator(`#layers button[data-id="${group[0]}"]`).click();
  await page.locator(`#layers button[data-id="${group[1]}"]`).click({ modifiers: ['Shift'] });
  await page.locator(`#layers button[data-id="${group[2]}"]`).click({ modifiers: ['Shift'] });
  assert.match(await page.locator('#selection-summary').textContent(), /3 个组件 · 1 个片段/);
  let before = catalog();
  await move(8, 12);
  shifted(before, group, 8, 12);
  record('shift-multiselect-moves-three-rows-and-ruby', group);

  const originalWord = current.nodes.find(item => item.id.startsWith(prefix + '/lyrics/word:')).id;
  before = catalog();
  await page.locator('#stage').scrollIntoViewIfNeeded();
  const hit = await page.locator(`#targets button[data-id="${originalWord}"]`).boundingBox();
  await change(async () => {
    await page.mouse.move(hit.x + hit.width / 2, hit.y + hit.height / 2);
    await page.mouse.down();
    await page.mouse.move(hit.x + hit.width / 2 + 5, hit.y + hit.height / 2 + 5, { steps: 6 });
    await page.mouse.up();
  });
  const delta = [bounds(group[0])[0] - before.get(group[0]).bounds[0], bounds(group[0])[1] - before.get(group[0]).bounds[1]];
  assert.ok(delta[0] > 0 && delta[1] > 0);
  shifted(before, group, ...delta);
  record('pointer-drag-retains-group-selection', delta);
  before = catalog();
  await change(() => page.locator(`#layers button[data-id="${group[0]}"]`).press('Shift+ArrowDown'));
  shifted(before, group, 0, 10);
  record('keyboard-moves-multiple-components', { delta: 10 });
  await page.locator(`#layers button[data-id="${originalWord}"]`).click({ modifiers: ['Shift'] });
  before = catalog();
  await move(2, 0);
  shifted(before, group, 2, 0);
  record('selected-parent-and-child-move-once', { originalWord });
  await change(() => page.locator('#undo').click());
  await change(() => page.locator('#reset-position').click());

  const ruby = current.nodes.find(item => item.id.startsWith(prefix + '/lyrics/ruby:')).id;
  before = catalog();
  await page.locator(`#layers button[data-id="${ruby}"]`).click();
  await change(() => page.locator('#delete').click());
  for (const [id, item] of before) if (id.startsWith(prefix + '/lyrics/word:')) {
    assert.deepEqual(bounds(id), item.bounds);
    assert.equal(node(id).baseline, item.baseline);
  }
  await change(() => page.locator('#undo').click());
  record('deleting-ruby-preserves-original-word-baselines', { ruby });
  await page.locator(`#layers button[data-id="${originalWord}"]`).click();
  before = catalog();
  await page.locator('#text').fill('あ' + node(originalWord).text);
  await change(() => page.locator('#apply-text').click());
  for (const [id, item] of before) if (id.startsWith(prefix + '/lyrics/word:')) assert.equal(node(id).baseline, item.baseline);
  await change(() => page.locator('#undo').click());
  record('text-edit-preserves-baselines-in-fixed-layout', { originalWord });

  await page.locator(`#layers button[data-id="${prefix}/lyrics"]`).click();
  await change(() => page.locator('#hide').click());
  await page.locator(`#layers button[data-id="${originalWord}"]`).click();
  const readingIds = node(originalWord).linked || [];
  await change(() => page.locator('#show').click());
  assert.equal(node(originalWord).hidden, false);
  for (const item of current.catalog.filter(item => item.id.startsWith(prefix + '/lyrics/') && item.id !== originalWord && !readingIds.includes(item.id))) assert.equal(item.hidden, true, item.id);
  await change(() => page.locator('#undo').click());
  await change(() => page.locator('#undo').click());
  record('show-selected-child-within-hidden-parent', { originalWord, readingIds });

  await page.locator(`#layers button[data-id="${prefix}/lyrics"]`).click();
  await page.locator('#position-y').fill('-100');
  let invalid = page.waitForResponse(response => response.url().endsWith('/api/preview'));
  await page.locator('#apply-position').click();
  assert.equal((await invalid).status(), 422);
  await page.waitForFunction(() => !document.getElementById('error').hidden);
  assert.equal(await page.locator('#save').isDisabled(), true);
  assert.equal(await page.locator('#arrange-once').isDisabled(), false);
  await change(() => page.locator('#arrange-once').click());
  assert.equal(await page.locator('#automatic').isChecked(), false);
  invalid = page.waitForResponse(response => response.url().endsWith('/api/preview'));
  await page.locator('#undo').click();
  assert.equal((await invalid).status(), 422);
  await page.waitForFunction(() => !document.getElementById('error').hidden);
  await change(() => page.locator('#undo').click());
  record('explicit-layout-resolves-invalid-scene-offset-and-restores-history', { original: prefix + '/lyrics' });

  const lyricBounds = group.map(id => bounds(id));
  await page.locator('#stage').scrollIntoViewIfNeeded();
  const stage = await page.locator('#stage').boundingBox();
  const top = Math.min(...lyricBounds.map(b => b[1])) - 4;
  const bottom = Math.max(...lyricBounds.map(b => b[3])) + 4;
  await page.mouse.move(stage.x + 2, stage.y + top / current.height * stage.height);
  await page.mouse.down();
  await page.mouse.move(stage.x + stage.width - 2, stage.y + bottom / current.height * stage.height, { steps: 8 });
  await page.mouse.up();
  const pressed = await page.locator('#layers button[aria-pressed=true]').count();
  assert.ok(pressed >= 3);
  record('real-pointer-marquee-selects-components', { count: pressed });
  await page.locator('#clear-selection').click();

  before = catalog();
  await choose('all', ['roman']);
  assert.match(await page.locator('#selection-summary').textContent(), new RegExp(`${session.lines.length} 个组件`));
  const romans = session.lines.map(item => `line:${item.id}/roman`);
  await move(0, 15);
  shifted(before, romans, 0, 15);
  record('all-scenes-same-row-batch-move', { count: romans.length });
  await snapshot('all-romaji-moved');
  await change(() => page.locator('#portrait').click());
  const portrait = catalog();
  const portraitSession = await (await page.request.get(new URL('/api/session', values.url).href)).json();
  assert.deepEqual(portraitSession.edits.positions.portrait, {});
  await change(() => page.locator('#landscape').click());
  await page.locator('#search').fill('');
  const chosenLines = [session.lines[0].id, session.lines[2].id];
  for (const id of chosenLines) await page.locator(`#lines input[data-line-id="${id}"]`).check();
  await choose('marked', ['roman', 'translation']);
  assert.match(await page.locator('#selection-summary').textContent(), /4 个组件 · 2 个片段/);
  before = catalog();
  await move(3, 4);
  const subset = chosenLines.flatMap(id => [`line:${id}/roman`, `line:${id}/translation`]);
  shifted(before, subset, 3, 4);
  record('marked-scenes-component-filter', { chosenLines, count: subset.length });
  await change(() => page.locator('#delete').click());
  for (const [id, item] of before) {
    if (subset.some(parent => id === parent || id.startsWith(parent + '/'))) assert.equal(current.catalog.some(n => n.id === id), false);
    else assert.deepEqual(bounds(id), item.bounds, id);
  }
  record('batch-delete-preserves-unselected-geometry', subset);
  await change(() => page.locator('#undo').click());
  for (const id of subset) node(id);
  record('batch-delete-single-undo', { restored: subset.length });

  await choose('all', ['translation']);
  before = catalog();
  const imageBeforeHide = current.image;
  await change(() => page.locator('#hide').click());
  for (const [id, item] of before) assert.deepEqual(bounds(id), item.bounds, id);
  for (const item of session.lines) assert.equal(node(`line:${item.id}/translation`).hidden, true);
  assert.equal(await page.locator(`#targets button[data-id="${prefix}/translation"]`).count(), 0);
  assert.ok(await page.locator(`#layers button[data-id="${prefix}/translation"]`).isVisible());
  assert.notEqual(current.image, imageBeforeHide);
  record('batch-hide-retains-content-geometry-and-list', { count: session.lines.length });
  await snapshot('all-translations-hidden');
  await change(() => page.locator('#show').click());
  assert.equal(current.image, imageBeforeHide);
  record('batch-show-restores-rendered-pixels', { exactPng: true });

  await choose('marked', ['roman']);
  before = catalog();
  await change(() => page.locator('#delete').click());
  const card = current.nodes.find(item => /\/vocabulary\/card:\d+$/.test(item.id)).id;
  assert.deepEqual(bounds(card), before.get(card).bounds);
  await page.locator('#selection-scope').selectOption('current');
  before = catalog();
  await change(() => page.locator('#arrange-once').click());
  assert.equal(await page.locator('#automatic').isChecked(), false);
  assert.ok(bounds(card)[1] < before.get(card).bounds[1]);
  for (const [id, item] of before) if (item.line_id !== null && item.line_id !== line) assert.deepEqual(bounds(id), item.bounds, id);
  const arrangedBounds = bounds(card);
  record('one-shot-layout-only-current-scene-and-orientation', { before: before.get(card).bounds, after: arrangedBounds });
  await change(() => page.locator('#undo').click());
  assert.deepEqual(bounds(card), before.get(card).bounds);
  await change(() => page.locator('#redo').click());
  assert.deepEqual(bounds(card), arrangedBounds);
  record('layout-is-one-undoable-operation', { card });
  await change(() => page.locator('#portrait').click());
  for (const [id, item] of portrait) if (!romans.some(parent => id === parent || id.startsWith(parent + '/'))) assert.deepEqual(bounds(id), item.bounds, id);
  record('deletion-shared-portrait-arrangement-independent', { width: current.width });
  await change(() => page.locator('#landscape').click());

  await change(() => page.locator('#automatic').check());
  await change(() => page.locator('#automatic').uncheck());
  before = catalog();
  await choose('current', ['translation']);
  await change(() => page.locator('#hide').click());
  for (const [id, item] of before) assert.deepEqual(bounds(id), item.bounds, id);
  record('turning-off-auto-layout-freezes-current-arrangement', { hidden: prefix + '/translation' });
  await choose('current', ['lyrics']);
  await page.locator('#position-x').fill(String(bounds(prefix + '/lyrics')[0] + 4));
  await page.locator('#position-y').fill(String(bounds(prefix + '/lyrics')[1] + 6));
  await change(() => page.locator('#apply-position').click());
  await page.locator('#save').click();
  await page.waitForFunction(() => document.getElementById('save-status').textContent === '修改已保存');
  const saved = await (await page.request.get(new URL('/api/session', values.url).href)).json();
  await writeFile(resolve(evidence, 'saved-edit.json'), JSON.stringify(saved.edits, null, 2) + '\n');
  before = catalog();
  await change(() => page.reload());
  const reopened = await (await page.request.get(new URL('/api/session', values.url).href)).json();
  assert.deepEqual(reopened.edits, saved.edits);
  for (const [id, item] of before) assert.deepEqual(node(id), item, id);
  record('save-and-reopen-complete-selection-document', { version: saved.edits.version });

  for (const width of [1600, 768, 375, 320]) {
    await page.setViewportSize({ width, height: 1100 });
    for (const orientation of ['landscape', 'portrait']) {
      await change(() => page.locator('#' + orientation).click());
      await page.waitForFunction(() => document.documentElement.scrollWidth <= innerWidth);
      const box = await page.locator('#stage').boundingBox();
      assert.ok(Math.abs(box.width / box.height - current.width / current.height) < 0.01);
    }
  }
  record('responsive-selection-interface', { widths: [1600, 768, 375, 320] });
  await page.setViewportSize({ width: 1600, height: 1100 });
  await snapshot('edited-portrait');
  if (!values['skip-export']) {
    await exportVideo('landscape');
    await exportVideo('portrait');
  }
  assert.deepEqual(errors, []);
  await writeFile(resolve(evidence, 'selection-e2e.json'), JSON.stringify({ status: 'passed', checks, browser_errors: errors, source: { title: session.title, lines: session.lines.length }, exports: !values['skip-export'] }, null, 2) + '\n');
} finally {
  await browser.close();
}
