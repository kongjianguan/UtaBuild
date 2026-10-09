import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { parseArgs } from 'node:util';
import { chromium } from 'playwright';

const { values } = parseArgs({ options: {
  url: { type: 'string', default: 'http://127.0.0.1:49603' },
  'evidence-dir': { type: 'string', default: 'work/selection-delivery/evidence' },
} });
const evidence = resolve(values['evidence-dir']);
await mkdir(evidence, { recursive: true });
const browser = await chromium.launch({ channel: 'chrome', headless: true });
const page = await browser.newPage({ viewport: { width: 1600, height: 1100 } });
const errors = [], checks = [];
page.on('pageerror', error => errors.push(error.message));
let current;
async function ready() {
  await page.waitForFunction(() => document.getElementById('stage').getAttribute('aria-busy') === 'false', undefined, { timeout: 120000 });
  assert.equal(await page.locator('#error').isVisible(), false, await page.locator('#error').textContent());
}
async function rectangle(bounds) {
  await page.locator('#stage').scrollIntoViewIfNeeded();
  const stage = await page.locator('#stage').boundingBox();
  await page.mouse.move(stage.x + 2, stage.y + bounds[1] / current.height * stage.height);
  await page.mouse.down();
  await page.mouse.move(stage.x + bounds[2] / current.width * stage.width, stage.y + bounds[3] / current.height * stage.height, { steps: 8 });
  await page.mouse.up();
}
const selected = () => page.locator('#layers button[aria-pressed=true]').evaluateAll(buttons => buttons.map(button => button.dataset.id).sort());

try {
  const pending = page.waitForResponse(response => response.url().endsWith('/api/preview'), { timeout: 120000 });
  await page.goto(values.url);
  current = await (await pending).json();
  await ready();
  const session = await (await page.request.get(new URL('/api/session', values.url).href)).json();
  const prefix = `line:${current.line_id}`;
  const ids = ['lyrics', 'roman', 'translation'].map(kind => prefix + '/' + kind);
  const rows = ids.map(id => current.nodes.find(node => node.id === id));
  await rectangle([0, Math.min(...rows.map(node => node.bounds[1])) - 4, current.width - 4, Math.max(...rows.map(node => node.bounds[3])) + 4]);
  assert.deepEqual(await selected(), ids.sort());
  checks.push({ name: 'marquee-selects-three-regions-without-duplicate-children', selected: await selected() });
  const before = new Map(current.catalog.map(node => [node.id, node.bounds]));
  const handle = await page.locator('#move-selection').boundingBox();
  let rendered = page.waitForResponse(response => response.url().endsWith('/api/preview'), { timeout: 120000 });
  await page.mouse.move(handle.x + handle.width / 2, handle.y + handle.height / 2);
  await page.mouse.down();
  await page.mouse.move(handle.x + handle.width / 2 + 5, handle.y + handle.height / 2 + 5, { steps: 8 });
  await page.mouse.up();
  let response = await rendered;
  assert.equal(response.status(), 200, await response.text());
  current = await response.json();
  await ready();
  const moved = current.catalog.find(node => node.id === ids[0]);
  const delta = moved.bounds.slice(0, 2).map((value, index) => value - before.get(ids[0])[index]);
  assert.ok(delta[0] > 0 && delta[1] > 0);
  for (const node of current.catalog) {
    const affected = ids.some(id => node.id === id || node.id.startsWith(id + '/'));
    assert.deepEqual(node.bounds, before.get(node.id).map((value, index) => value + (affected ? delta[index % 2] : 0)), node.id);
  }
  const beforeKeyboard = new Map(current.catalog.map(node => [node.id, node.bounds]));
  rendered = page.waitForResponse(response => response.url().endsWith('/api/preview'));
  await page.locator('#move-selection').press('Shift+ArrowDown');
  current = await (await rendered).json();
  await ready();
  for (const node of current.catalog) {
    const affected = ids.some(id => node.id === id || node.id.startsWith(id + '/'));
    assert.deepEqual(node.bounds, beforeKeyboard.get(node.id).map((value, index) => value + (affected && index % 2 ? 10 : 0)), node.id);
  }
  checks.push({ name: 'selection-handle-supports-keyboard-movement', vertical_delta: 10 });
  rendered = page.waitForResponse(response => response.url().endsWith('/api/preview'));
  await page.locator('#undo').click();
  current = await (await rendered).json();
  await ready();
  rendered = page.waitForResponse(response => response.url().endsWith('/api/preview'));
  await page.locator('#undo').click();
  current = await (await rendered).json();
  await ready();
  await page.locator('#save').click();
  await page.waitForFunction(() => document.getElementById('save-status').textContent === '修改已保存');
  checks.push({ name: 'selection-handle-moves-group-once-and-undo-restores-document', delta });
  await page.locator(`#layers button[data-id="${prefix}/roman"]`).click({ modifiers: ['Shift'] });
  assert.deepEqual(await selected(), [prefix + '/lyrics', prefix + '/translation'].sort());
  checks.push({ name: 'shift-click-removes-one-selected-region', selected: await selected() });
  await page.locator('#clear-selection').click();
  const card = current.nodes.find(node => /\/vocabulary\/card:\d+$/.test(node.id));
  await rectangle([0, card.bounds[1] - 2, card.bounds[2] + 2, card.bounds[3] + 2]);
  assert.deepEqual(await selected(), [card.id]);
  checks.push({ name: 'marquee-selects-complete-card', selected: await selected() });

  await page.locator('#selection-scope').selectOption('all');
  await page.locator('#selection-types input[value=roman]').check();
  await page.locator('#select-components').click();
  assert.match(await page.locator('#selection-summary').textContent(), new RegExp(`${session.lines.length} 个片段`));
  const navigation = page.waitForResponse(response => response.url().endsWith('/api/preview'));
  await page.locator(`#lines button[data-line-id="${session.lines[2].id}"]`).click();
  current = await (await navigation).json();
  await ready();
  assert.match(await page.locator('#selection-summary').textContent(), new RegExp(`${session.lines.length} 个片段`));
  assert.equal(await page.locator(`#layers button[data-id="line:${session.lines[2].id}/roman"]`).getAttribute('aria-pressed'), 'true');
  checks.push({ name: 'cross-scene-selection-survives-navigation', count: session.lines.length });
  await page.locator('#stage').focus();
  await page.keyboard.press('Escape');
  assert.equal(await page.locator('#selection-summary').textContent(), '尚未选择组件');
  checks.push({ name: 'escape-clears-selection', selected: await selected() });
  const saved = await (await page.request.get(new URL('/api/session', values.url).href)).json();
  assert.deepEqual(saved.edits, session.edits);
  assert.deepEqual(errors, []);
  await writeFile(resolve(evidence, 'selection-delivery-e2e.json'), JSON.stringify({ status: 'passed', url: values.url, checks, browser_errors: errors, saved_edits_unchanged: true }, null, 2) + '\n');
  console.log(JSON.stringify({ status: 'passed', checks: checks.length }));
} finally {
  await browser.close();
}
