// Opt in only: this test makes one real model request using the chosen key file.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import { mkdtemp, mkdir, readFile, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
const require = createRequire(import.meta.url);
const { chromium } = require('playwright');
if (!process.argv[2]) throw new Error('Usage: node tests/live-ui.mjs PATH_TO_LOCAL_CREDENTIAL_FILE');
const credentialFile = path.resolve(process.argv[2]);
const originalHash = createHash('sha256').update(await readFile(credentialFile)).digest('hex');
const temporary = await mkdtemp(path.join(os.tmpdir(), 'study-coach-live-ui-'));
const dataDir = path.join(temporary, 'data');
const evidence = path.resolve('docs/evidence/ui');
await mkdir(evidence, { recursive: true });
const executable = process.env.STUDY_COACH_APP || path.resolve('target/debug', process.platform === 'win32' ? 'study-coach-app.exe' : 'study-coach-app');
const child = spawn(executable, ['--data-dir', dataDir, '--no-open', '--credentials-file', credentialFile], { windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
let browser;
try {
  const url = await new Promise((resolve, reject) => {
    let output = '';
    const timeout = setTimeout(() => reject(new Error('Local service did not start.')), 20000);
    child.on('error', reject);
    child.on('exit', code => { clearTimeout(timeout); reject(new Error(`Local service exited (${code}).`)); });
    child.stdout.on('data', data => {
      output += data;
      const match = output.match(/http:\/\/127\.0\.0\.1:\d+\//);
      if (match) { clearTimeout(timeout); resolve(match[0]); }
    });
  });
  browser = await chromium.launch({ headless: true, ...(process.platform === 'win32' ? { channel: 'msedge' } : {}) });
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 }, deviceScaleFactor: 2, reducedMotion: 'no-preference' });
  let calls = 0;
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => { if (request.url().endsWith('/api/answer') && request.method() === 'POST') calls++; });
  await page.goto(url);
  await page.getByText('Groq key ready', { exact: true }).waitFor();
  await page.locator('#question').fill('What is a unit test?');
  const pending = page.waitForResponse(response => response.url().endsWith('/api/answer'), { timeout: 180000 });
  await page.locator('#ask-button').click();
  const response = await pending;
  const answer = await response.json();
  assert.equal(response.status(), 200, answer.error || 'Provider request failed.');
  assert.equal(answer.output.status, 'answer');
  const state = await (await page.request.get(`${url}api/state`)).json();
  for (const citation of answer.output.citations) {
    assert.equal(state.sources.some(source => source.id === citation.source_id && source.text.includes(citation.quote)), true);
  }
  await page.locator('#answer-card').waitFor();
  await page.screenshot({ path: path.join(evidence, 'live-answer.png'), fullPage: true });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({ path: path.join(evidence, 'live-answer-phone.png'), fullPage: true });
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
  await page.locator('.nav-item[data-page="history"]').click();
  await page.locator('.history-item').first().click();
  assert.equal(await page.locator('#answer-text').innerText(), answer.output.answer);
  assert.equal(calls, 1);
  assert.deepEqual(errors, []);
  assert.equal(await page.locator('#groq-key').inputValue(), '');
  assert.equal(await page.locator('#nvidia-key').inputValue(), '');
  assert.equal(/gsk_[A-Za-z0-9]{20,}|nvapi-[A-Za-z0-9]{20,}/.test(await page.locator('body').innerText()), false);
  assert.match(await readFile(path.join(dataDir, '.env'), 'utf8'), /GROQ_API_KEY=\s*\nNVIDIA_API_KEY=\s*$/);
  assert.equal(createHash('sha256').update(await readFile(credentialFile)).digest('hex'), originalHash);
  await writeFile(path.join(evidence, 'live-verification.json'), JSON.stringify({ date: '2026-11-08', result: 'PASS', browser: 'Official Playwright; installed Microsoft Edge on Windows', actual_provider_call: true, api_answer_requests: calls, provider: answer.provider, model: answer.model, approach: answer.approach, context_mode: answer.context_mode, latency_ms: answer.latency_ms, verified_citations: answer.output.citations, answer: answer.output.answer, history_click_restored_answer: true, history_click_additional_api_calls: 0, javascript_errors: errors.length, phone_horizontal_overflow: false, keys_entered_in_browser: false, process_credentials_not_persisted: true, original_credentials_file_unchanged: true, device_scale_factor: 2 }, null, 2));
  console.log(`PASS: real dimensional UI answer and history; ${answer.model}; ${answer.latency_ms.toFixed(2)} ms; one API request; no browser exceptions.`);
} finally {
  if (browser) await browser.close();
  child.kill();
}
