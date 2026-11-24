import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { chromium, root, startDemoHarness } from './demo-provider.mjs';

const evidence = process.env.STUDY_COACH_DISPLAY_EVIDENCE || path.join(root, 'target', 'display-checks');
await mkdir(evidence, { recursive: true });
const harness = await startDemoHarness();
let browser;
try {
  browser = await chromium.launch({ headless: true, args: ['--disable-backgrounding-occluded-windows','--disable-renderer-backgrounding'], ...(process.platform === 'win32' ? {channel: 'msedge'} : {}) });
  const samples = [];
  for (const [width, height] of [[3840,2160], [5120,2880], [390,844]]) {
    const context = await browser.newContext({ viewport: { width, height }, deviceScaleFactor: 1, reducedMotion: 'no-preference' });
    const page = await context.newPage();
    await page.goto(harness.url);
    await page.bringToFront();
    await page.locator('#setup-notice').getByRole('button', {name:'Set up connection'}).waitFor();
    await page.waitForFunction(() => document.querySelector('.art-book')?.getAnimations().length > 0);
    const layerTransforms = () => page.locator('.art-book, .note-one').evaluateAll(nodes => nodes.map(node => getComputedStyle(node).transform));
    // A screenshot forces a compositor frame even when a headless window
    // is occluded on Windows. Compare two independently rendered frames.
    const shot = await page.screenshot({ path: path.join(evidence, `study-${width}.png`) });
    const first = await layerTransforms();
    await page.waitForTimeout(350);
    await page.screenshot({clip:{x:0,y:0,width:8,height:8}});
    const last = await layerTransforms();
    const sample = await page.evaluate(() => {
      const art = document.querySelector('.studio-art').getBoundingClientRect();
      return {
        viewport: [innerWidth, innerHeight],
        canvas: [document.querySelector('canvas').width, document.querySelector('canvas').height],
        overflow: document.documentElement.scrollWidth > innerWidth,
        visibleArtwork: art.width > 200 && art.height > 100 && art.x >= 0 && art.right <= innerWidth + 1,
      };
    });
    sample.movingLayers = last.filter((matrix, index) => matrix !== first[index]).length;
    assert.deepEqual(sample.canvas, [width, height]);
    assert.equal(sample.overflow, false);
    assert.equal(sample.visibleArtwork, true);
    assert.equal(sample.movingLayers, 2);
    assert.equal(shot.readUInt32BE(16), width);
    assert.equal(shot.readUInt32BE(20), height);
    await page.emulateMedia({reducedMotion:'reduce'});
    await page.reload();
    await page.waitForFunction(() => document.body.classList.contains('motion-off'));
    assert.equal(await page.locator('.art-book').evaluate(b => b.getAnimations().length), 0);
    samples.push({...sample, reducedMotionStopsArtwork: true});
    console.log(`Verified ${width} by ${height}.`);
    await context.close();
  }
  await writeFile(path.join(evidence, 'display-verification.json'), JSON.stringify(samples,null,2));
  console.log('PASS: native 3840x2160 and 5120x2880 screenshots, 390px phone layout, two independently moving layers, reduced-motion control.');
} finally {
  await browser?.close();
  await harness.close();
}
