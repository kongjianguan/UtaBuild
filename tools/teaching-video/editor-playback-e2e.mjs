import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { parseArgs } from 'node:util';
import { chromium } from 'playwright';

const { values } = parseArgs({ options: {
  url: { type: 'string', default: 'http://127.0.0.1:49607' },
  'evidence-dir': { type: 'string', default: 'work/editor-playback-e2e/evidence' },
} });
const evidence = resolve(values['evidence-dir']);
await mkdir(evidence, { recursive: true });
const browser = await chromium.launch({ channel: 'chrome', headless: true });
const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
const browserErrors = [];
page.on('pageerror', error => browserErrors.push(error.message));
const previews = [];
const pending = new Set();
let current;
page.on('response', response => {
  if (!response.url().endsWith('/api/preview') || response.status() !== 200) return;
  const task = response.json().then(preview => {
    current = preview;
    previews.push({ time_ms: response.request().postDataJSON().time_ms,
      sha256: createHash('sha256').update(preview.image).digest('hex') });
  });
  pending.add(task);
  task.finally(() => pending.delete(task));
});
const checks = [];
const record = (name, detail) => checks.push({ name, detail });

async function ready() {
  await page.waitForFunction(() => document.getElementById('stage').getAttribute('aria-busy') === 'false',
    undefined, { timeout: 120000 });
  assert.equal(await page.locator('#error').isVisible(), false, await page.locator('#error').textContent());
  await Promise.all(pending);
}

async function change(action) {
  const response = page.waitForResponse(r => r.url().endsWith('/api/preview'), { timeout: 120000 });
  await action();
  assert.equal((await response).status(), 200);
  await ready();
}

async function monitor() {
  await page.evaluate(() => {
    const snapshot = () => ({ renderingVisible: !document.getElementById('rendering').hidden,
      saveDisabled: document.getElementById('save').disabled,
      exportDisabled: document.getElementById('export').disabled });
    let last = snapshot();
    const changes = [];
    const observer = new MutationObserver(() => {
      const next = snapshot();
      if (JSON.stringify(next) !== JSON.stringify(last)) {
        changes.push({ at_ms: performance.now(), ...next });
        last = next;
      }
    });
    for (const id of ['rendering', 'save', 'export']) {
      observer.observe(document.getElementById(id), { attributes: true, attributeFilter: ['hidden', 'disabled'] });
    }
    window.playbackEvidence = { initial: last, changes, observer };
  });
}

async function stopMonitor() {
  return page.evaluate(() => {
    const result = window.playbackEvidence;
    result.observer.disconnect();
    return { initial: result.initial, changes: result.changes };
  });
}

try {
  const first = page.waitForResponse(r => r.url().endsWith('/api/preview'), { timeout: 120000 });
  await page.goto(values.url);
  assert.equal((await first).status(), 200);
  await ready();
  const session = await (await page.request.get(new URL('/api/session', values.url).href)).json();
  record('real-project', { title: session.title, duration_ms: session.duration_ms });

  for (const layout of ['landscape', 'portrait']) {
    await change(() => page.locator('#' + layout).click());
    await monitor();
    const start = Number(await page.locator('#time').inputValue());
    const count = previews.length;
    await page.locator('#play').click();
    await page.waitForFunction(start => {
      const audio = document.getElementById('audio');
      return !audio.paused && audio.currentTime >= start + 6;
    }, start, { timeout: 30000 });
    const saveResponse = page.waitForResponse(r => r.url().endsWith('/api/save'));
    await page.locator('#save').click();
    assert.equal((await saveResponse).status(), 200);
    assert.equal(await page.locator('#audio').evaluate(audio => audio.paused), false);
    await page.locator('#play').click();
    await ready();
    const transitions = await stopMonitor();
    const frames = previews.slice(count);
    const detail = { ...transitions, preview_count: frames.length,
      distinct_frames: new Set(frames.map(frame => frame.sha256)).size,
      times_ms: frames.map(frame => frame.time_ms), paused: await page.locator('#audio').evaluate(audio => audio.paused) };
    record(layout + '-playback-controls', detail);
    await writeFile(resolve(evidence, layout + '-last-frame.png'), Buffer.from(current.image.split(',')[1], 'base64'));
  }

  await monitor();
  await change(() => page.locator('#time').fill('15').then(() => page.locator('#time').press('Tab')));
  record('manual-seek-loading', await stopMonitor());

  await page.locator('#layers button[data-id="title"]').click();
  await page.locator('#text').fill(session.title + ' · 歌曲学习');
  await monitor();
  await change(() => page.locator('#apply-text').click());
  assert.equal(current.nodes.find(node => node.id === 'title').text, session.title + ' · 歌曲学习');
  record('text-edit-loading', await stopMonitor());
  await change(() => page.locator('#undo').click());

  await page.locator('#position-y').fill('-100');
  const invalid = page.waitForResponse(r => r.url().endsWith('/api/preview'));
  await page.locator('#apply-position').click();
  assert.equal((await invalid).status(), 422);
  await page.waitForFunction(() => document.getElementById('error').hidden === false);
  assert.equal(await page.locator('#save').isDisabled(), true);
  assert.equal(await page.locator('#export').isDisabled(), true);
  record('invalid-edit-blocks-actions', { message: await page.locator('#error').textContent() });
  await change(() => page.locator('#undo').click());
  await page.locator('#save').click();
  await page.waitForFunction(() => document.getElementById('save-status').textContent === '修改已保存');
  const saved = await (await page.request.get(new URL('/api/session', values.url).href)).json();
  assert.deepEqual(saved.edits, session.edits);
  record('document-restored', { workspace: saved.workspace, edits: saved.edits });

  for (const check of checks.filter(check => check.name.endsWith('-playback-controls'))) {
    assert.ok(check.detail.preview_count >= 8, JSON.stringify(check));
    assert.ok(check.detail.distinct_frames >= 4, JSON.stringify(check));
    assert.deepEqual(check.detail.initial, { renderingVisible: false, saveDisabled: false, exportDisabled: false });
    assert.deepEqual(check.detail.changes, [], JSON.stringify(check));
    assert.equal(check.detail.paused, true);
  }
  for (const name of ['manual-seek-loading', 'text-edit-loading']) {
    const check = checks.find(check => check.name === name);
    assert.ok(check.detail.changes.some(change => change.renderingVisible && change.saveDisabled && change.exportDisabled));
    assert.deepEqual(check.detail.changes.at(-1), {
      at_ms: check.detail.changes.at(-1).at_ms, renderingVisible: false, saveDisabled: false, exportDisabled: false,
    });
  }
  assert.deepEqual(browserErrors, []);
  await writeFile(resolve(evidence, 'playback-e2e.json'), JSON.stringify({ status: 'passed', url: values.url,
    checks, browser_errors: browserErrors }, null, 2) + '\n');
  console.log(JSON.stringify({ status: 'passed', report: resolve(evidence, 'playback-e2e.json'), checks: checks.length }));
} catch (error) {
  await writeFile(resolve(evidence, 'playback-e2e.json'), JSON.stringify({ status: 'failed', url: values.url,
    checks, browser_errors: browserErrors, error: error.message }, null, 2) + '\n');
  throw error;
} finally {
  await browser.close();
}
