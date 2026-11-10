// Real local processes and saved data. Provider replies and browser refusal to
// close a tab are labelled fixtures; this test makes no external model requests.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
import { mkdtemp, mkdir, readFile, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const { chromium } = require('playwright');
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const executable = process.env.STUDY_COACH_APP || path.join(root, 'target', 'debug', process.platform === 'win32' ? 'study-coach-app.exe' : 'study-coach-app');
const temporary = await mkdtemp(path.join(os.tmpdir(), 'study-coach-lifecycle-ui-'));
const dataDir = path.join(temporary, 'data');
const evidence = process.env.STUDY_COACH_LIFECYCLE_EVIDENCE || path.join(temporary, 'evidence');
await mkdir(dataDir, { recursive: true });
await mkdir(evidence, { recursive: true });
const history = [{ id: 'lifecycle-fixture', question: 'Saved history fixture', output: { status: 'clarify', answer: 'Which topic should we study?', citations: [] }, model: 'local-fixture', created_at_unix: 1 }];
await writeFile(path.join(dataDir, 'history.json'), JSON.stringify(history));
const children = [];
const checks = [];
const failures = [];
let browser;
const environment = { ...process.env };
delete environment.GROQ_API_KEY;
delete environment.NVIDIA_API_KEY;

async function launch() {
  const child = spawn(executable, ['--data-dir', dataDir, '--no-open'], { env: environment, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
  children.push(child);
  const url = await new Promise((resolve, reject) => {
    let output = '';
    const timeout = setTimeout(() => reject(new Error('App startup timed out.')), 20000);
    child.once('error', error => { clearTimeout(timeout); reject(error); });
    child.once('exit', code => { clearTimeout(timeout); reject(new Error(`App exited before startup (${code}).`)); });
    child.stdout.on('data', chunk => {
      output += chunk.toString();
      const match = output.match(/http:\/\/127\.0\.0\.1:\d+\//);
      if (match) { clearTimeout(timeout); resolve(match[0]); }
    });
  });
  return { child, url };
}

async function expectStopped(child) {
  for (let attempt = 0; attempt < 100 && child.exitCode === null; attempt++) {
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  assert.equal(child.exitCode, 0, 'X must stop the service normally, not only hide its page');
}

async function expectXInViewport(page) {
  const box = await page.getByRole('button', { name: 'Close Study Coach', exact: true }).boundingBox();
  const viewport = page.viewportSize();
  assert.ok(box && box.x >= 0 && box.y >= 0 && box.x + box.width <= viewport.width && box.y + box.height <= viewport.height);
  assert.ok(box.width >= 44 && box.height >= 44, 'Close control must have a usable touch target');
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
}

try {
  const first = await launch();
  browser = await chromium.launch({ headless: true, ...(process.platform === 'win32' ? { channel: 'msedge' } : {}) });
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 }, reducedMotion: 'reduce', deviceScaleFactor: 2 });
  context.setDefaultTimeout(10000);
  // Exercise the supported fallback for browsers that refuse window.close().
  await context.addInitScript(() => { window.close = () => { window.closeAttempted = true; }; });
  const page = await context.newPage();
  page.on('pageerror', error => failures.push(error.message));
  await page.goto(first.url);
  await page.getByText('Connection needed', { exact: true }).waitFor();
  await expectXInViewport(page);
  checks.push('Labelled desktop X has a 44px touch target.');
  await page.screenshot({ path: path.join(evidence, 'desktop-close-control.png') });
  for (const width of [390, 320]) {
    await page.setViewportSize({ width, height: 844 });
    await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
    await expectXInViewport(page);
  }
  checks.push('X remains in view after scrolling at 390px and 320px without overflow.');
  await page.screenshot({ path: path.join(evidence, 'phone-close-control.png') });
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.locator('.nav-item[data-page="notes"]').click();
  await page.locator('#note-title').fill('Retained lifecycle notes');
  await page.locator('#note-text').fill('Saved notes must remain after closing and reopening.');
  await page.locator('#save-note').click();
  await page.locator('#note-count').filter({ hasText: '13' }).waitFor();
  await page.locator('#note-title').fill('Unsaved draft');
  page.once('dialog', dialog => dialog.dismiss());
  await page.locator('#close-app').click();
  assert.equal(await page.locator('#note-title').inputValue(), 'Unsaved draft');
  assert.equal((await fetch(`${first.url}api/state`)).status, 200);
  checks.push('Cancelling close preserves an unsaved note and keeps the service running.');
  await page.locator('#note-title').fill('');
  await page.locator('.nav-item[data-page="settings"]').click();
  await page.locator('#profile').selectOption('groq-20b');
  page.once('dialog', dialog => dialog.dismiss());
  await page.locator('#close-app').click();
  assert.equal(await page.locator('#profile').inputValue(), 'groq-20b');
  checks.push('Cancelling close also preserves an unsaved model selection.');
  await page.locator('#groq-key').fill('lifecycle-ui-only-fake-credential');
  await page.locator('#save-settings').click();
  await page.getByText(/Connection saved locally/).waitFor();
  const credentialBefore = await readFile(path.join(dataDir, '.env'));

  await page.route('**/api/shutdown', route => route.fulfill({ status: 503, contentType: 'application/json', body: JSON.stringify({ error: 'Lifecycle fixture: shutdown unavailable.' }) }));
  await page.locator('#close-app').click();
  await page.getByRole('heading', { name: 'Closing could not be confirmed.' }).waitFor();
  assert.equal(await page.locator('#retry-close').isVisible(), true);
  await page.locator('#return-to-app').click();
  assert.equal(await page.locator('#close-app').isEnabled(), true);
  assert.equal((await fetch(`${first.url}api/state`)).status, 200);
  checks.push('A labelled shutdown error fixture allows retry or return to the running workspace.');
  await page.unroute('**/api/shutdown');

  let finishRetainedAnswer;
  let failShutdown;
  const retainedAnswerGate = new Promise(resolve => { finishRetainedAnswer = resolve; });
  const failedShutdownGate = new Promise(resolve => { failShutdown = resolve; });
  await page.route('**/api/answer', async route => {
    await retainedAnswerGate;
    await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ output: { status: 'clarify', answer: 'Retained answer fixture after failed close.', citations: [] }, model: 'fixture-only', latency_ms: 0 }) });
  });
  await page.route('**/api/shutdown', async route => {
    await failedShutdownGate;
    await route.fulfill({ status: 503, contentType: 'application/json', body: JSON.stringify({ error: 'Lifecycle fixture: delayed shutdown unavailable.' }) });
  });
  await page.locator('.nav-item[data-page="study"]').click();
  await page.locator('#question').fill('Pending answer retention fixture');
  await page.locator('#ask-button').click();
  await page.locator('#close-app').click();
  await page.getByText(/The current answer will finish/).waitFor();
  finishRetainedAnswer();
  await page.waitForFunction(() => !document.getElementById('ask-button').disabled);
  failShutdown();
  await page.getByRole('heading', { name: 'Closing could not be confirmed.' }).waitFor();
  await page.locator('#return-to-app').click();
  await page.locator('#answer-text').getByText('Retained answer fixture after failed close.', { exact: true }).waitFor();
  checks.push('An answer that arrives during failed shutdown remains visible on return to the workspace.');
  await page.unroute('**/api/answer');
  await page.unroute('**/api/shutdown');

  let releaseAnswer;
  let releaseShutdown;
  const answerGate = new Promise(resolve => { releaseAnswer = resolve; });
  const shutdownGate = new Promise(resolve => { releaseShutdown = resolve; });
  await page.route('**/api/answer', async route => {
    await answerGate;
    await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ output: { status: 'clarify', answer: 'Lifecycle presentation fixture; choose a topic.', citations: [] }, model: 'fixture-only', latency_ms: 0 }) });
  });
  await page.route('**/api/shutdown', async route => { await shutdownGate; await route.continue(); });
  await page.locator('.nav-item[data-page="study"]').click();
  await page.locator('#question').fill('Lifecycle presentation fixture');
  await page.locator('#ask-button').click();
  await page.locator('#close-app').click();
  await page.getByText(/The current answer will finish/).waitFor();
  assert.equal(await page.locator('#close-tab').isVisible(), false);
  checks.push('Pending answer fixture shows a waiting message and does not offer premature tab close.');
  releaseAnswer();
  await page.waitForFunction(() => !document.getElementById('ask-button').disabled);
  releaseShutdown();
  await page.getByRole('heading', { name: 'Study Coach is closed.' }).waitFor();
  await expectStopped(first.child);
  assert.equal(await page.evaluate(() => window.closeAttempted), true);
  assert.equal(await page.locator('#close-tab').isVisible(), true);
  checks.push('Real authenticated shutdown exits the service and displays a closed page if the browser refuses tab closure.');
  await page.locator('#close-tab').click();
  await page.getByText(/press Ctrl\+W to close it/).waitFor();
  await page.screenshot({ path: path.join(evidence, 'closed-workspace.png') });
  await page.close();

  const reopened = await launch();
  const restoredPage = await context.newPage();
  restoredPage.on('pageerror', error => failures.push(error.message));
  await restoredPage.goto(reopened.url);
  await restoredPage.getByText('Groq key ready', { exact: true }).waitFor();
  assert.equal(await restoredPage.locator('#note-count').innerText(), '13');
  const restored = await (await fetch(`${reopened.url}api/state`)).json();
  assert.equal(restored.selected_profile, 'groq-20b');
  assert.equal(restored.keys.groq, true);
  assert.ok(restored.sources.some(source => source.title === 'Retained lifecycle notes'));
  assert.equal(JSON.stringify(restored).includes('lifecycle-ui-only-fake-credential'), false);
  assert.deepEqual(await readFile(path.join(dataDir, '.env')), credentialBefore);
  assert.deepEqual((await (await fetch(`${reopened.url}api/history`)).json()).entries, history);
  checks.push('Second launch reuses the data folder without a lock error and retains notes, history, profile and fake key.');
  await restoredPage.screenshot({ path: path.join(evidence, 'reopened-workspace.png') });
  await restoredPage.locator('#close-app').click();
  await restoredPage.getByRole('heading', { name: 'Study Coach is closed.' }).waitFor();
  await expectStopped(reopened.child);
  assert.deepEqual(failures, []);
  checks.push('Reopened application closes normally again with no browser exceptions.');
  await writeFile(path.join(evidence, 'lifecycle-verification.json'), JSON.stringify({ package_version: restored.version, checks, actual_provider_calls: 0, fixtures: ['provider presentation only', 'shutdown error responses', 'browser window.close refusal'], screenshot_directory: evidence }, null, 2));
  console.log(`PASS: ${checks.length} lifecycle browser checks; two real launches, retained data, normal shutdown.`);
  console.log(`Evidence: ${evidence}`);
} catch (error) {
  await writeFile(path.join(evidence, 'lifecycle-failure.txt'), String(error.stack || error));
  throw error;
} finally {
  if (browser) await browser.close();
  for (const child of children) if (child.exitCode === null && child.signalCode === null) child.kill();
}
