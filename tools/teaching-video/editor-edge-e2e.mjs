import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const evidence = resolve('work/editor-edge-e2e');
await mkdir(evidence, { recursive: true });
const checks = [];
async function hash(path) {
  const digest = createHash('sha256');
  for await (const chunk of createReadStream(path)) digest.update(chunk);
  return digest.digest('hex');
}
const sourcePaths = ['work/spiral/project.json', 'work/spiral/teaching.json', 'work/spiral/audio.m4a',
  'work/spiral/cover.jpg', 'work/haruhikage-final-source-e2e/project.json', 'work/haruhikage-final-source-e2e/teaching.json',
  'work/haruhikage-final-source-e2e/audio.m4a', 'work/haruhikage-final-source-e2e/cover.jpg'];
const before = Object.fromEntries(await Promise.all(sourcePaths.map(async path => [path, await hash(path)])));
const empty = () => ({ version: 2, text: {}, deleted: [], hidden: [], positions: { landscape: {}, portrait: {} },
  automatic: { landscape: false, portrait: false }, arrangements: { landscape: {}, portrait: {} } });
async function session(url) { return (await fetch(url + '/api/session')).json(); }
async function post(url, token, route, body, expected = 200) {
  const response = await fetch(url + route, { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Editor-Token': token }, body: JSON.stringify(body) });
  const result = await response.json();
  assert.equal(response.status, expected, JSON.stringify(result));
  return result;
}
function luminance(rgb) {
  const channels = rgb.map(v => v / 255 <= 0.04045 ? v / 255 / 12.92 : ((v / 255 + 0.055) / 1.055) ** 2.4);
  return channels.reduce((sum, v, i) => sum + v * [0.2126, 0.7152, 0.0722][i], 0);
}
const colors = [];
for (const [url, song] of [['http://127.0.0.1:49603', 'spiral'], ['http://127.0.0.1:49606', '春日影']]) {
  const s = await session(url);
  const body = { edits: empty(), layout: 'landscape', time_ms: s.lines[0].start_ms };
  for (const layout of ['landscape', 'portrait']) {
    const preview = await post(url, s.token, '/api/preview', { ...body, layout });
    const rgb = preview.accent.slice(1).match(/../g).map(v => parseInt(v, 16));
    const contrast = (luminance(rgb) + 0.05) / (luminance([61, 66, 74]) + 0.05);
    assert.ok(contrast >= 4.5);
    assert.ok(preview.nodes.length > 20);
    await writeFile(resolve(evidence, `${song}-${layout}.png`), Buffer.from(preview.image.split(',')[1], 'base64'));
    checks.push({ name: song + '-' + layout, accent: preview.accent, contrast, components: preview.nodes.length });
    if (layout === 'landscape') colors.push(preview.accent);
  }
  for (const [name, change] of [
    ['blank-text', edits => { edits.text.title = ' '; }],
    ['missing-font-glyph', edits => { edits.text.title = '\u{10ffff}'; }],
    ['component-outside-picture', edits => { edits.positions.landscape.title = [-500, 0]; }],
    ['unknown-component', edits => { edits.text['line:999999/translation'] = '文字'; }],
    ['timed-text-newline', edits => { edits.text[`line:${s.lines[0].id}/lyrics/word:0`] = 'あ\nあ'; }],
  ]) {
    const edits = empty(); change(edits);
    const result = await post(url, s.token, '/api/preview', { ...body, edits }, 422);
    checks.push({ name: song + '-' + name, rejected: result.detail });
  }
  await post(url, 'invalid-session-token', '/api/save', body, 403);
  await post(url, s.token, '/api/export', { ...body, end_ms: s.duration_ms + 1000 }, 422);
}
assert.notEqual(colors[0], colors[1]);
const url = 'http://127.0.0.1:49606';
const s = await session(url);
const body = { edits: empty(), layout: 'landscape', time_ms: s.lines[0].start_ms };
const preview = await post(url, s.token, '/api/preview', body);
const deleted = { ...body, edits: { ...empty(), deleted: preview.nodes.filter(node => !node.parent).map(node => node.id) } };
const blank = await post(url, s.token, '/api/preview', deleted);
assert.equal(blank.nodes.length, 0);
await writeFile(resolve(evidence, 'all-components-deleted.png'), Buffer.from(blank.image.split(',')[1], 'base64'));
const project = JSON.parse(await readFile('work/haruhikage-final-source-e2e/project.json', 'utf8'));
const whitespaceLine = project.lines.find(line => line.words.some(word => word.start_ms === word.end_ms));
const whitespaceIndex = whitespaceLine.words.findIndex(word => word.start_ms === word.end_ms);
const invalidWord = empty(); invalidWord.text[`line:${whitespaceLine.id}/lyrics/word:${whitespaceIndex}`] = 'あ';
await post(url, s.token, '/api/preview', { ...body, edits: invalidWord }, 422);
checks.push({ name: 'zero-duration-whitespace-rejected', line: whitespaceLine.id, word: whitespaceIndex });

async function exportClip(name, settings) {
  const start_ms = s.lines[0].start_ms;
  const end_ms = Math.min(start_ms + 3000, s.lines[1].start_ms);
  const job = await post(url, s.token, '/api/export', { ...settings, start_ms, end_ms }, 202);
  let finished;
  const deadline = Date.now() + 180000;
  while (Date.now() < deadline) {
    finished = await (await fetch(url + '/api/exports/' + job.id)).json();
    if (finished.status === 'passed' || finished.status === 'failed') break;
    await new Promise(resolve => setTimeout(resolve, 1000));
  }
  assert.equal(finished.status, 'passed', finished.error);
  const report = await (await fetch(url + finished.verification)).json();
  assert.equal(report.status, 'passed'); assert.equal(report.song_scope, 'clip'); assert.equal(report.full_decode, true);
  await writeFile(resolve(evidence, name + '-verification.json'), JSON.stringify(report, null, 2) + '\n');
  checks.push({ name, video: report.video, glyph_samples: report.baseline_check.glyph_samples, audio: report.audio.correlation });
  return report;
}
const blankReport = await exportClip('all-components-deleted-export', deleted);
assert.equal(blankReport.baseline_check.glyph_samples, 0);
const edgeEdits = empty();
const firstWord = preview.nodes.find(node => node.id.includes('/lyrics/word:'));
edgeEdits.positions.landscape[firstWord.id] = [0, -firstWord.bounds[1]];
const edgeBody = { ...body, edits: edgeEdits };
const edge = await post(url, s.token, '/api/preview', edgeBody);
assert.equal(edge.nodes.find(node => node.id === firstWord.id).bounds[1], 0);
await exportClip('glyph-at-picture-edge', edgeBody);
await post(url, s.token, '/api/save', body);
const after = Object.fromEntries(await Promise.all(sourcePaths.map(async path => [path, await hash(path)])));
assert.deepEqual(after, before);
checks.push({ name: 'prepared-sources-unchanged', sha256: after });
await writeFile(resolve(evidence, 'editor-edge-e2e.json'), JSON.stringify({ status: 'passed', checks }, null, 2) + '\n');
console.log(JSON.stringify({ status: 'passed', report: resolve(evidence, 'editor-edge-e2e.json') }));
