// Isolated UI fixture: source selection requests and validation, without cloud calls.
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { once } from 'node:events';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.STUDY_COACH_PLAYWRIGHT || 'playwright');
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const evidence = path.join(root, 'target', 'source-selection-verification');
await mkdir(evidence, { recursive: true });
const assets = new Map(await Promise.all(['index.html', 'app.js', 'materials.js', 'app.css', 'print.css'].map(async name => [name === 'index.html' ? '/' : `/${name}`, await readFile(path.join(root, 'web', name))])));
const state = {
  version: '0.2.0', csrf_token: 'source-selection-test', data_dir: 'Isolated fixture',
  sources: [
    { id: 'S1', title: 'Unit tests', text: 'A unit test checks one function in isolation.' },
    { id: 'S2', title: 'Integration tests', text: 'An integration test checks collaborating components.' },
  ],
  profiles: [{ id: 'groq-120b', provider: 'groq', model: 'openai/gpt-oss-120b' }],
  selected_profile: 'groq-120b', keys: { groq: true, nvidia: false }, providers: [],
};
const requests = [];
let sequence = 0;
const server = createServer(async (request, response) => {
  const route = new URL(request.url, 'http://127.0.0.1').pathname;
  if (assets.has(route)) {
    response.writeHead(200, { 'Content-Type': route.endsWith('.js') ? 'text/javascript' : route.endsWith('.css') ? 'text/css' : 'text/html' });
    response.end(assets.get(route));
    return;
  }
  let value = {};
  if (request.method === 'GET' && route === '/api/state') value = state;
  else if (request.method === 'GET' && route === '/api/materials') value = { entries: [] };
  else if (request.method === 'GET' && route === '/api/progress') value = { entries: [], topics: [], total: 0, correct: 0, close: 0, incorrect: 0 };
  else if (request.method === 'POST') {
    const chunks = [];
    for await (const chunk of request) chunks.push(chunk);
    const body = JSON.parse(Buffer.concat(chunks).toString());
    requests.push({ route, body });
    if (route === '/api/quiz') value = { id: `quiz-${++sequence}`, topic: body.topic, questions: [{ id: 'q1', question: 'What does a unit test check?', type: 'open-response', options: [] }] };
    else if (route === '/api/materials') value = { id: `notes-${++sequence}`, kind: 'notes', title: 'Testing notes', sections: [{ heading: 'Definition', text: 'A unit test checks one function in isolation.' }], cards: [], diagram: null, citations: [] };
  }
  response.writeHead(200, { 'Content-Type': 'application/json' });
  response.end(JSON.stringify(value));
});
server.listen(0, '127.0.0.1');
await once(server, 'listening');
const browser = await chromium.launch({ channel: process.platform === 'win32' ? 'msedge' : undefined, headless: true });
const context = await browser.newContext({ viewport: { width: 1440, height: 1100 }, reducedMotion: 'reduce' });
const page = await context.newPage();
page.setDefaultTimeout(10000);
const errors = [];
page.on('pageerror', error => errors.push(error.message));
try {
  await page.goto(`http://127.0.0.1:${server.address().port}`);
  console.log('Loaded source selection fixture');
  await page.locator('#quiz-use-all-sources').waitFor({ state: 'attached' });
  assert.equal(await page.locator('#quiz-use-all-sources').isChecked(), true);
  await page.locator('.nav-item[data-page="quiz"]').click();
  console.log('Opened quiz setup');
  await page.locator('#quiz-topic').fill('Integration tests');
  await page.locator('#quiz-count').fill('1');
  await page.locator('#quiz-use-all-sources').uncheck();
  await page.locator('#quiz-source-picker button').getByText('Clear', { exact: true }).click();
  await page.locator('#start-quiz').click();
  await page.getByText('Choose at least one source, or turn on Use all saved sources.', { exact: true }).waitFor();
  assert.equal(requests.filter(item => item.route === '/api/quiz').length, 0);
  await page.locator('#quiz-source-picker button').getByText('Select all', { exact: true }).click();
  await page.locator('#quiz-source-options input[value="S1"]').uncheck();
  await page.screenshot({ path: path.join(evidence, 'desktop-source-picker.png') });
  await page.locator('#start-quiz').click();
  await page.locator('#quiz-question').waitFor({ state: 'visible' });
  assert.deepEqual(requests.find(item => item.route === '/api/quiz').body.source_ids, ['S2']);

  await page.locator('.nav-item[data-page="materials"]').click();
  await page.locator('#material-topic').fill('Unit tests');
  await page.locator('#generate-material').click();
  await page.locator('#material-document').waitFor({ state: 'visible' });
  assert.equal(Object.hasOwn(requests.find(item => item.route === '/api/materials').body, 'source_ids'), false);
  await page.locator('#material-use-all-sources').uncheck();
  await page.locator('#material-source-picker button').getByText('Clear', { exact: true }).click();
  const previous = requests.filter(item => item.route === '/api/materials').length;
  await page.locator('#generate-material').click();
  await page.locator('#material-message').getByText('Choose at least one source, or turn on Use all saved sources.', { exact: true }).waitFor();
  assert.equal(requests.filter(item => item.route === '/api/materials').length, previous);
  await page.locator('#material-source-picker button').getByText('Select all', { exact: true }).click();
  await page.locator('#material-source-options input[value="S2"]').uncheck();
  await page.locator('#generate-material').click();
  await page.waitForFunction(() => !document.getElementById('generate-material').disabled);
  assert.deepEqual(requests.filter(item => item.route === '/api/materials').at(-1).body.source_ids, ['S1']);
  await page.setViewportSize({ width: 390, height: 844 });
  await page.locator('#material-source-picker').scrollIntoViewIfNeeded();
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  await page.screenshot({ path: path.join(evidence, 'phone-source-picker.png') });
  assert.deepEqual(errors, []);
  await writeFile(path.join(evidence, 'verification.json'), JSON.stringify({ fixture: true, quiz_source_ids: ['S2'], material_source_ids: ['S1'], default_all: true, empty_selection_rejected: true, phone_overflow: false, page_errors: errors }, null, 2));
  console.log('PASS: quiz/material source pickers; default all; empty selection blocked; exact IDs sent; phone layout');
} finally {
  await context.close();
  await browser.close();
  server.close();
}
