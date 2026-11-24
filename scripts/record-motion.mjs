// Render the real landing screen's CSS animation at fixed frame intervals.
// Every encoded picture is a new native 4K browser render.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { writeFile } from 'node:fs/promises';
import path from 'node:path';
import { chromium, root, startDemoHarness } from '../tests/demo-provider.mjs';

const width = 3840, height = 2160, fps = 30, frames = 150;
const directory = path.join(root, 'docs', 'demo');
const harness = await startDemoHarness();
let browser, encoder;
try {
  browser = await chromium.launch({headless:true, ...(process.platform === 'win32' ? {channel:'msedge'} : {})});
  const page = await browser.newPage({viewport:{width,height}, deviceScaleFactor:1, reducedMotion:'no-preference'});
  await page.goto(harness.url);
  await page.locator('#setup-notice').getByRole('button',{name:'Set up connection'}).waitFor();
  await page.evaluate(async () => {
    await document.fonts.ready;
    window.motionPreviewAnimations = document.getAnimations();
    for (const animation of window.motionPreviewAnimations) animation.pause();
  });
  const cdp = await page.context().newCDPSession(page);
  encoder = spawn('ffmpeg',['-hide_banner','-loglevel','error','-y','-f','image2pipe',
    '-framerate',String(fps),'-c:v','mjpeg','-i','pipe:0','-frames:v',String(frames),
    '-c:v','libx264','-threads','3','-preset','veryfast','-crf','17','-pix_fmt','yuv420p',
    '-movflags','+faststart',path.join(directory,'study-coach-4k-motion.mp4')], {stdio:['pipe','ignore','inherit'],windowsHide:true});
  const exit = once(encoder,'exit');
  const shapes = new Set();
  for (let number = 0; number < frames; number++) {
    const shape = await page.evaluate(t => {
      for (const animation of window.motionPreviewAnimations) animation.currentTime = t;
      return ['.art-book','.note-one','.note-two'].map(selector =>
        getComputedStyle(document.querySelector(selector)).transform).join('|');
    }, 1200 + number * 1000 / fps);
    shapes.add(shape);
    const result = await cdp.send('Page.captureScreenshot',{format:'jpeg',quality:93,fromSurface:true,captureBeyondViewport:false});
    if (!encoder.stdin.write(Buffer.from(result.data,'base64'))) await once(encoder.stdin,'drain');
  }
  encoder.stdin.end();
  assert.deepEqual(await exit,[0,null]);
  assert.equal(shapes.size,frames,'Each frame must show a distinct arrangement of the illustration layers.');
  await writeFile(path.join(directory,'upgrade-checks','motion-playback.json'),JSON.stringify(
    {width,height,fps,frames,duration_seconds:frames/fps,unique_layer_arrangements:shapes.size,source:'Actual CSS animation sampled at fixed intervals in the isolated Study Coach tab'},null,2)+'\n');
  console.log(`PASS: ${frames} distinct native UHD application renders at ${fps} frames per second.`);
} finally {
  if (encoder && encoder.exitCode === null) encoder.kill();
  await browser?.close();
  await harness.close();
}
