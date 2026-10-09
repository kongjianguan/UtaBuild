import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { parseArgs } from 'node:util';
import { chromium } from 'playwright';

const { values } = parseArgs({ options: {
  url: { type: 'string', default: 'http://127.0.0.1:49607' },
  'evidence-dir': { type: 'string', default: 'work/editor-playback-e2e/appearance' },
} });
const evidence = resolve(values['evidence-dir']);
await mkdir(evidence, { recursive: true });
const browser = await chromium.launch({ channel: 'chrome', headless: true });
const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
const errors = [];
page.on('pageerror', error => errors.push(error.message));
page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
const checks = [];
const record = (name, detail) => checks.push({ name, detail });
async function ready() {
  await page.waitForFunction(() => document.getElementById('stage').getAttribute('aria-busy') === 'false' &&
    document.getElementById('preview').complete && document.getElementById('preview').naturalWidth > 0,
  undefined, { timeout: 120000 });
  assert.equal(await page.locator('#error').isVisible(), false);
}

try {
  await page.goto(values.url);
  await ready();
  for (const colorScheme of ['light', 'dark']) {
    await page.emulateMedia({ colorScheme, reducedMotion: 'no-preference', contrast: 'no-preference' });
    const materials = await page.evaluate(() => {
      const root = getComputedStyle(document.documentElement);
      const canvas = document.createElement('canvas'); canvas.width = 1; canvas.height = 1;
      const context = canvas.getContext('2d', { willReadFrequently: true });
      const rgba = color => {
        context.clearRect(0, 0, 1, 1); context.fillStyle = color; context.fillRect(0, 0, 1, 1);
        return [...context.getImageData(0, 0, 1, 1).data];
      };
      const composite = (foreground, background) => foreground.slice(0, 3).map((value, index) =>
        value * foreground[3] / 255 + background[index] * (1 - foreground[3] / 255));
      const luminance = rgb => rgb.slice(0, 3).map(value => {
        const channel = value / 255;
        return channel <= .04045 ? channel / 12.92 : ((channel + .055) / 1.055) ** 2.4;
      }).reduce((total, value, index) => total + value * [.2126, .7152, .0722][index], 0);
      const contrast = (a, b) => {
        const x = luminance(a), y = luminance(b);
        return (Math.max(x, y) + .05) / (Math.min(x, y) + .05);
      };
      const fill = rgba(root.getPropertyValue('--glass-fill'));
      const sheen = rgba(root.getPropertyValue('--glass-sheen'));
      const backgrounds = [0, 255].flatMap(value => {
        const base = composite(fill, [value, value, value]);
        return [base, composite(sheen, base)];
      });
      const foregrounds = ['#save-status', '.palette span', '#empty-selection'].map(selector => {
        const color = getComputedStyle(document.querySelector(selector)).color;
        return { selector, color, contrast_lower_bound: Math.min(...backgrounds.map(background => contrast(rgba(color), background))) };
      });
      const primary = document.getElementById('export');
      const primaryStyle = getComputedStyle(primary);
      const primaryFill = rgba(primaryStyle.backgroundColor);
      const primaryContrast = Math.min(...backgrounds.map(background => contrast(rgba(primaryStyle.color), composite(primaryFill, background))));
      return { glass: [...document.querySelectorAll('[data-glass]')].map(element => {
        const style = getComputedStyle(element);
        return { component: element.className, blur: style.backdropFilter, radius: style.borderRadius,
          background: style.backgroundImage, shadow: style.boxShadow };
      }), content_blur: getComputedStyle(document.getElementById('preview')).filter,
      foregrounds, primary_contrast_lower_bound: primaryContrast };
    });
    assert.equal(materials.glass.length, 5);
    for (const glass of materials.glass) {
      assert.ok(glass.blur.includes('blur(26px)'));
      assert.ok(glass.background.includes('radial-gradient'));
      assert.notEqual(glass.shadow, 'none');
    }
    assert.equal(materials.content_blur, 'none');
    for (const foreground of materials.foregrounds) assert.ok(foreground.contrast_lower_bound >= 4.5, JSON.stringify(foreground));
    assert.ok(materials.primary_contrast_lower_bound >= 4.5);
    record(colorScheme + '-glass-and-text-contrast', materials);

    for (const layout of ['landscape', 'portrait']) {
      await page.locator('#' + layout).click();
      await ready();
      assert.equal(await page.locator('#layout-control').getAttribute('data-layout'), layout);
      for (const width of [1440, 1100, 850, 768, 600, 375, 320]) {
        await page.setViewportSize({ width, height: 1000 });
        await page.waitForFunction(ratio => {
          const bounds = document.getElementById('stage').getBoundingClientRect();
          const area = document.querySelector('.stage-area').getBoundingClientRect();
          return Math.abs(bounds.width / bounds.height - ratio) < .002 &&
            bounds.left >= area.left && bounds.right <= area.right + 1 &&
            bounds.top >= area.top && bounds.bottom <= area.bottom + 1;
        }, layout === 'landscape' ? 16 / 9 : 9 / 16);
        const geometry = await page.evaluate(() => {
          const stage = document.getElementById('stage').getBoundingClientRect();
          const area = document.querySelector('.stage-area').getBoundingClientRect();
          return { viewport: innerWidth, scroll_width: document.documentElement.scrollWidth,
            stage: { x: stage.x, y: stage.y, width: stage.width, height: stage.height },
            contained: stage.left >= area.left && stage.right <= area.right + 1 && stage.top >= area.top && stage.bottom <= area.bottom + 1,
            save_visible: document.getElementById('save').getBoundingClientRect().width > 0,
            export_visible: document.getElementById('export').getBoundingClientRect().width > 0 };
        });
        assert.ok(geometry.scroll_width <= width, JSON.stringify(geometry));
        assert.ok(geometry.contained, JSON.stringify(geometry));
        assert.ok(geometry.save_visible && geometry.export_visible);
        record(`${colorScheme}-${layout}-${width}`, geometry);
      }
    }
  }

  await page.setViewportSize({ width: 1440, height: 1000 });
  const topbar = await page.locator('.topbar').boundingBox();
  await page.mouse.move(topbar.x + topbar.width * .7, topbar.y + topbar.height * .6);
  await page.waitForFunction(() => document.querySelector('.topbar').style.getPropertyValue('--glass-x') !== '');
  record('pointer-highlight', await page.locator('.topbar').evaluate(element => ({
    x: element.style.getPropertyValue('--glass-x'), y: element.style.getPropertyValue('--glass-y') })));
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const before = await page.locator('.topbar').getAttribute('style');
  await page.mouse.move(topbar.x + topbar.width * .3, topbar.y + topbar.height * .5);
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  assert.equal(await page.locator('.topbar').getAttribute('style'), before);
  const motion = await page.evaluate(() => ({
    button: getComputedStyle(document.getElementById('save')).transitionDuration,
    selector: getComputedStyle(document.getElementById('layout-control'), '::before').transitionDuration,
  }));
  assert.deepEqual(motion, { button: '0s', selector: '0s' });
  record('reduced-motion', motion);

  await page.emulateMedia({ contrast: 'more' });
  const highContrast = await page.locator('.topbar').evaluate(element => ({
    blur: getComputedStyle(element).backdropFilter, cover: getComputedStyle(document.body, '::before').display,
  }));
  assert.deepEqual(highContrast, { blur: 'none', cover: 'none' });
  record('increased-contrast', highContrast);

  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-transparency', value: 'reduce' }] });
  const transparency = await page.locator('.topbar').evaluate(element => ({
    requested: matchMedia('(prefers-reduced-transparency: reduce)').matches,
    blur: getComputedStyle(element).backdropFilter, cover: getComputedStyle(document.body, '::before').display,
  }));
  assert.deepEqual(transparency, { requested: true, blur: 'none', cover: 'none' });
  record('reduced-transparency', transparency);

  await page.locator('#save').focus();
  await page.keyboard.press('Tab');
  const focus = await page.locator('#export').evaluate(element => ({ focused: document.activeElement === element,
    visible: element.matches(':focus-visible'), outline: getComputedStyle(element).outlineWidth }));
  assert.deepEqual(focus, { focused: true, visible: true, outline: '3px' });
  record('keyboard-focus', focus);
  assert.deepEqual(errors, []);
  await writeFile(resolve(evidence, 'appearance-e2e.json'), JSON.stringify({ status: 'passed', url: values.url,
    references: ['https://developer.apple.com/design/human-interface-guidelines/materials',
      'https://developer.apple.com/documentation/technologyoverviews/adopting-liquid-glass',
      'https://developer.apple.com/videos/play/wwdc2025/219/'], checks, browser_errors: errors }, null, 2) + '\n');
  console.log(JSON.stringify({ status: 'passed', checks: checks.length, report: resolve(evidence, 'appearance-e2e.json') }));
} catch (error) {
  await writeFile(resolve(evidence, 'appearance-e2e.json'), JSON.stringify({ status: 'failed', checks,
    browser_errors: errors, error: error.message }, null, 2) + '\n');
  throw error;
} finally {
  await browser.close();
}
