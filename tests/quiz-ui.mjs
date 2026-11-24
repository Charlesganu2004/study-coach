// Isolated UI fixtures verify interaction and layout without credentials,
// provider calls, or the user's running browser. Backend integration is separate.
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { readFile, mkdir, mkdtemp, writeFile } from 'node:fs/promises';
import { once } from 'node:events';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.STUDY_COACH_PLAYWRIGHT || 'playwright');
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const evidence = process.env.STUDY_COACH_UI_EVIDENCE || await mkdtemp(path.join(os.tmpdir(), 'study-coach-new-ui-'));
await mkdir(evidence, { recursive: true });
const assets = new Map(await Promise.all(['index.html', 'app.js', 'materials.js', 'app.css', 'print.css'].map(async name => [name === 'index.html' ? '/' : `/${name}`, await readFile(path.join(root, 'web', name))])));
const citation = { source_id: 'S3', quote: 'A unit test checks one function in isolation.' };
const state = {
  version: '0.2.0', csrf_token: 'isolated-ui-fixture-token', data_dir: 'Isolated browser fixture',
  sources: [{ id: 'S3', title: 'Testing concepts', text: `${citation.quote} An integration test checks collaborating components.` }],
  profiles: [
    { id: 'groq-20b', name: 'groq-20b', provider: 'groq', model: 'openai/gpt-oss-20b' },
    { id: 'groq-120b', name: 'groq-120b', provider: 'groq', model: 'openai/gpt-oss-120b' },
    { id: 'nvidia-lightning', name: 'nvidia-lightning', provider: 'nvidia', model: 'nvidia/nemotron-3.5-lightning-30b-a3b' },
  ], selected_profile: 'groq-120b', keys: { groq: true, nvidia: false },
  providers: [
    { id: 'groq', label: 'Groq', endpoint: 'https://api.groq.com/openai/v1/chat/completions', requires_key: true },
    { id: 'nvidia', label: 'NVIDIA', endpoint: 'https://integrate.api.nvidia.com/v1/chat/completions', requires_key: true },
    { id: 'openai', label: 'OpenAI', endpoint: 'https://api.openai.com/v1/chat/completions', requires_key: true },
    { id: 'openrouter', label: 'OpenRouter', endpoint: 'https://openrouter.ai/api/v1/chat/completions', requires_key: true },
    { id: 'ollama', label: 'Ollama', endpoint: 'http://127.0.0.1:11434/v1/chat/completions', requires_key: false },
    { id: 'custom', label: 'Custom', endpoint: '', requires_key: true },
  ],
};
let savedMaterials = [];
let quizSequence = 0;
let failGrade = false;
let failConnection = false;
const requests = [];
const server = createServer(async (request, response) => {
  const url = new URL(request.url, 'http://127.0.0.1');
  if (request.method === 'GET' && assets.has(url.pathname)) {
    response.setHeader('Content-Security-Policy', "default-src 'self'; connect-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; base-uri 'none'; frame-ancestors 'none'; form-action 'self'");
    response.setHeader('Content-Type', url.pathname.endsWith('.js') ? 'text/javascript' : url.pathname.endsWith('.css') ? 'text/css' : 'text/html');
    response.end(assets.get(url.pathname));
    return;
  }
  response.setHeader('Content-Type', 'application/json');
  let raw = '';
  for await (const chunk of request) raw += chunk;
  const body = raw ? JSON.parse(raw) : {};
  requests.push({ method: request.method, path: url.pathname, body });
  const send = value => response.end(JSON.stringify(value));
  if (url.pathname === '/api/state') return send(state);
  if (url.pathname === '/api/history') return send({ entries: [] });
  if (url.pathname === '/api/progress') return send({ topics: [{ topic: 'Testing concepts', correct: 2, incorrect: 1, close: 1, attempts: 4 }], recent: [{ topic: 'Testing concepts', question: 'How does an integration test differ?', status: 'incorrect' }, { topic: 'Testing concepts', question: 'What is isolation?', status: 'correct' }] });
  if (url.pathname === '/api/quiz') {
    const count = body.count || 3;
    return send({ id: `quiz-${++quizSequence}`, topic: body.topic, model: 'UI fixture model', profile: state.selected_profile, questions: Array.from({ length: count }, (_, index) => ({ id: `q${index + 1}`, type: body.type || 'open-response', question: body.type === 'fill-in-the-blank' ? 'A unit test checks one function in ____.' : `Question ${index + 1}: What does a unit test check?`, ...(body.type === 'multiple-choice' ? { options: ['One function in isolation', 'Only the user interface', 'Every external service'] } : {}) })) });
  }
  if (url.pathname === '/api/quiz/grade') {
    if (failGrade) { response.statusCode = 502; return send({ error: 'Provider timed out.' }); }
    const status = /isolation/i.test(body.answer) ? 'correct' : /function|close/i.test(body.answer) ? 'close' : 'incorrect';
    return send({ status, feedback: status === 'correct' ? 'You included the key idea: one function, in isolation.' : status === 'close' ? 'You identified the unit. Say how it is tested.' : 'This answer describes a different kind of test.', ...(status === 'close' ? {} : { solution: citation.quote, citations: [citation] }) });
  }
  if (url.pathname === '/api/quiz/solution') return send({ status: 'incorrect', feedback: 'Solution revealed. Revisit this question in your next practice session.', solution: citation.quote, citations: [citation] });
  if (url.pathname === '/api/settings') { state.selected_profile = body.profile; return send(state); }
  if (url.pathname === '/api/connections' && request.method === 'POST') {
    if (failConnection) { response.statusCode = 400; return send({ error: 'Remote endpoints must use HTTPS.' }); }
    state.profiles.push({ id: 'custom-ui-fixture', name: body.name, provider: 'openai-compatible', provider_label: body.provider === 'ollama' ? 'Ollama' : 'Custom', model: body.model, endpoint: body.endpoint, key_configured: Boolean(body.api_key), requires_key: !body.endpoint.startsWith('http://127.0.0.1') });
    return send(state);
  }
  if (url.pathname.startsWith('/api/connections/') && request.method === 'DELETE') {
    state.profiles = state.profiles.filter(profile => profile.id !== url.pathname.split('/').at(-1));
    if (!state.profiles.some(profile => profile.id === state.selected_profile)) state.selected_profile = 'groq-120b';
    return send(state);
  }
  if (url.pathname === '/api/import') return send({ title: 'Screenshot study notes', text: 'Extracted screenshot text for review.\nA unit test checks one function in isolation.', format: 'png', method: 'local OCR fixture' });
  if (url.pathname === '/api/sources/import' || url.pathname === '/api/sources') { state.sources.push({ id: 'U1', title: body.title, text: body.text }); return send(state); }
  if (url.pathname === '/api/materials' && request.method === 'GET') return send({ entries: savedMaterials });
  if (url.pathname === '/api/materials' && request.method === 'POST') {
    const material = { id: `material-${savedMaterials.length + 1}`, kind: body.kind, title: body.kind === 'flashcards' ? 'Testing flashcards' : body.kind === 'memory' ? 'Testing memory aids' : 'Testing study notes', sections: body.kind === 'flashcards' ? [] : [{ heading: body.kind === 'memory' ? 'A memory phrase' : 'Unit tests', text: body.kind === 'memory' ? 'One unit alone, check it on its own. Original study song fixture.' : citation.quote }, { heading: 'Integration tests', text: 'An integration test checks collaborating components.' }], cards: body.kind === 'flashcards' ? [{ front: 'What does a unit test check?', back: citation.quote }, { front: 'What does an integration test check?', back: 'Collaborating components.' }] : [], diagram: body.kind === 'flashcards' ? undefined : { title: 'Testing relationships', nodes: ['Unit test', 'One function in isolation', 'Integration test', 'Collaborating components'], edges: [{ from: 0, to: 1 }, { from: 2, to: 3 }] }, citations: [citation] };
    savedMaterials.push(material);
    return send(material);
  }
  if (url.pathname.startsWith('/api/materials/') && request.method === 'DELETE') { savedMaterials = savedMaterials.filter(entry => entry.id !== url.pathname.split('/').at(-1)); return send({ entries: savedMaterials }); }
  if (url.pathname === '/api/export/docx') { response.setHeader('Content-Type', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'); response.end(Buffer.from('UI download fixture; DOCX validity is tested in backend integration.')); return; }
  response.statusCode = 404;
  send({ error: `Unconfigured UI fixture: ${request.method} ${url.pathname}` });
});
server.listen(0, '127.0.0.1');
await once(server, 'listening');
const url = `http://127.0.0.1:${server.address().port}/`;
let browser;
const errors = [];
const checks = [];
function check(name) { checks.push(name); }
try {
  browser = await chromium.launch({ headless: true, ...(process.platform === 'win32' ? { channel: 'msedge' } : {}) });
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 }, reducedMotion: 'reduce', deviceScaleFactor: 2 });
  await context.addInitScript(() => {
    window.print = () => { window.printRequested = true; };
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { write: async items => { window.copiedMaterialTypes = items[0].types; window.copiedMaterialHtml = await (await items[0].getType('text/html')).text(); } } });
  });
  const page = await context.newPage();
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => { if (message.type() === 'error' && /Content Security Policy/.test(message.text())) errors.push(message.text()); });
  const capture = async name => {
    await page.evaluate(() => scrollTo(0, 0));
    await page.screenshot({ path: path.join(evidence, name), fullPage: true });
  };
  await page.goto(url);
  await page.getByText('Groq key ready', { exact: true }).waitFor();
  assert.equal(await page.title(), 'Study Coach');
  await capture('desktop-workspace.png');
  await page.locator('#question').fill('Quiz me on unit tests');
  await page.locator('#ask-button').click();
  await page.locator('#quiz-question').filter({ hasText: 'Question 1' }).waitFor();
  assert.equal(new URL(page.url()).hash, '#quiz');
  assert.equal(await page.locator('#quiz-solution').isVisible(), false);
  assert.equal(await page.locator('#quiz-solution-text').innerText(), '');
  check('Quiz request opens test screen without exposing solution');
  const gradeCount = requests.filter(request => request.path === '/api/quiz/grade').length;
  await page.locator('#check-quiz-answer').click();
  await page.locator('#quiz-error').getByText(/Write an answer first/).waitFor();
  assert.equal(requests.filter(request => request.path === '/api/quiz/grade').length, gradeCount);
  check('Blank answer rejected before provider request');
  await page.locator('#quiz-answer').fill('It checks a function');
  await page.locator('#check-quiz-answer').click();
  await page.locator('#quiz-feedback-title').getByText('Close. Try again.', { exact: true }).waitFor();
  assert.equal(await page.locator('#quiz-feedback').evaluate(node => getComputedStyle(node).color), 'rgb(244, 216, 139)');
  assert.equal(await page.locator('#quiz-solution').isVisible(), false);
  await capture('quiz-close.png');
  await page.locator('#retry-quiz-answer').click();
  await page.locator('#quiz-answer').fill('One function in isolation');
  await page.locator('#check-quiz-answer').click();
  await page.locator('#quiz-feedback-title').getByText('Correct', { exact: true }).waitFor();
  assert.equal(await page.locator('#quiz-feedback').evaluate(node => getComputedStyle(node).color), 'rgb(155, 228, 178)');
  assert.equal(await page.locator('#quiz-answer').isDisabled(), true);
  await capture('quiz-correct.png');
  check('Yellow close retry becomes green correct and locks submitted answer');
  await page.locator('#next-quiz-question').click();
  await page.locator('#quiz-answer').fill('Only the user interface');
  failGrade = true;
  await page.locator('#check-quiz-answer').click();
  await page.locator('#quiz-error').getByText(/timed out/).waitFor();
  assert.equal(await page.locator('#quiz-exam').getAttribute('data-status'), 'unanswered');
  assert.equal(await page.locator('#check-quiz-answer').isEnabled(), true);
  failGrade = false;
  await page.locator('#check-quiz-answer').click();
  await page.locator('#quiz-feedback-title').getByText('Review this one', { exact: true }).waitFor();
  assert.equal(await page.locator('#quiz-feedback').evaluate(node => getComputedStyle(node).color), 'rgb(255, 178, 168)');
  assert.equal(await page.locator('#quiz-solution-text').innerText(), citation.quote);
  await capture('quiz-incorrect.png');
  check('Provider error remains unmarked; incorrect answer becomes red and reveals cited solution');
  await page.locator('#next-quiz-question').click();
  await page.locator('#see-quiz-solution').click();
  await page.locator('#quiz-solution-text').filter({ hasText: citation.quote }).waitFor();
  assert.equal(await page.locator('#quiz-exam').getAttribute('data-status'), 'incorrect');
  await page.locator('#next-quiz-question').click();
  await page.locator('#see-quiz-solution').click();
  await page.locator('#next-quiz-question').waitFor({ state: 'visible' });
  await page.locator('#next-quiz-question').click();
  await page.locator('#see-quiz-solution').click();
  await page.locator('#next-quiz-question').filter({ hasText: 'Finish test' }).click();
  await page.locator('#quiz-summary').waitFor({ state: 'visible' });
  assert.match(await page.locator('#quiz-summary-text').innerText(), /1 of 5 correct/);
  await page.locator('#restart-quiz').click();
  await page.locator('#quiz-type').selectOption('multiple-choice');
  await page.locator('#quiz-count').fill('1');
  await page.locator('#start-quiz').click();
  await page.locator('#quiz-choices').waitFor({ state: 'visible' });
  await page.getByRole('radio', { name: /One function in isolation/ }).check();
  await page.locator('#check-quiz-answer').click();
  await page.locator('#quiz-feedback-title').getByText('Correct', { exact: true }).waitFor();
  await page.locator('#next-quiz-question').click();
  await page.locator('#restart-quiz').click();
  await page.locator('#quiz-type').selectOption('custom');
  await page.locator('#quiz-count').fill('30');
  await page.locator('#quiz-instructions').fill('Ask applied debugging questions.');
  await page.locator('#start-quiz').click();
  await page.locator('#quiz-progress-label').getByText('1 / 30', { exact: true }).waitFor();
  assert.equal(requests.filter(request => request.path === '/api/quiz').at(-1).body.instructions, 'Ask applied debugging questions.');
  page.once('dialog', dialog => dialog.accept());
  await page.locator('#new-quiz').click();
  assert.equal(await page.locator('#quiz-setup').isVisible(), true);
  await page.locator('#topic-progress').getByRole('button', { name: 'Review next' }).click();
  await page.getByRole('button', { name: 'Practice this question' }).click();
  assert.equal(await page.locator('#quiz-count').inputValue(), '1');
  assert.equal(await page.locator('#quiz-topic').inputValue(), 'How does an integration test differ?');
  await page.getByRole('heading', { name: 'Questions answered correctly' }).waitFor();
  await page.locator('#quiz-type').selectOption('fill-in-the-blank');
  await page.locator('#quiz-count').fill('1');
  await page.locator('#start-quiz').click();
  await page.locator('#quiz-answer-help').getByText('Type the missing word or phrase.', { exact: true }).waitFor();
  check('Reveal, next, summary, restart, multiple choice, custom count30, fill blank and review list');
  await page.locator('.nav-item[data-page="settings"]').click();
  assert.equal(await page.locator('.connection-row').count(), 3);
  assert.equal(await page.locator('.provider-option').count(), 6);
  await page.getByRole('button', { name: 'Ollama Run on this computer' }).click();
  assert.equal(await page.locator('#connection-endpoint').inputValue(), 'http://127.0.0.1:11434/v1/chat/completions');
  await page.locator('#connection-name').fill('My local study model');
  await page.locator('#connection-model').fill('local-study-model');
  failConnection = true;
  await page.locator('#add-connection').click();
  await page.locator('#add-connection-message').getByText(/Remote endpoints must use HTTPS/).waitFor();
  assert.equal(await page.locator('#add-connection').isEnabled(), true);
  assert.equal(await page.locator('#connection-name').inputValue(), 'My local study model');
  failConnection = false;
  await page.locator('#add-connection').click();
  await page.getByRole('button', { name: 'Use local-study-model', exact: true }).click();
  await page.getByText('Ollama configured', { exact: true }).waitFor();
  await capture('connections-directory.png');
  page.once('dialog', dialog => dialog.accept());
  await page.getByRole('button', { name: 'Remove My local study model', exact: true }).click();
  await page.getByText('Connection removed.', { exact: true }).waitFor();
  assert.equal(await page.locator('.connection-row').count(), 3);
  check('Provider catalog, local preset, recoverable add error, add/select/delete connection');
  await page.locator('.nav-item[data-page="notes"]').click();
  await page.locator('#note-file').setInputFiles({ name: 'testing.png', mimeType: 'image/png', buffer: Buffer.from('UI import fixture, not a real image') });
  await page.waitForFunction(() => document.getElementById('note-title').value === 'Screenshot study notes');
  await page.locator('#note-text').fill(`${await page.locator('#note-text').inputValue()}\nReviewed before saving.`);
  await page.locator('#save-note').click();
  await page.getByText('Source saved. It will be included with your next question.').waitFor();
  assert.match(requests.filter(request => request.path === '/api/sources/import').at(-1).body.text, /Reviewed before saving/);
  await page.locator('#note-file').setInputFiles({ name: 'lecture.mp4', mimeType: 'video/mp4', buffer: Buffer.from('UI video fixture, not a real video') });
  await page.locator('#video-transcript').fill('A unit test checks one function in isolation.');
  await page.locator('#extract-video').click();
  await page.locator('#note-message').getByText(/File loaded for review/).waitFor();
  assert.equal(requests.filter(request => request.path === '/api/import').at(-1).body.transcript, citation.quote);
  check('Screenshot extraction preview remains editable before chunked source save');
  await page.locator('.nav-item[data-page="materials"]').click();
  await page.locator('#material-topic').fill('Testing concepts');
  await page.locator('#material-style').selectOption('cornell');
  await page.locator('#material-document-type').fill('Exam preparation guide');
  await page.locator('#material-instructions').fill('Explain isolation with an example.');
  await page.locator('#generate-material').click();
  await page.locator('#material-document-title').getByText('Testing study notes', { exact: true }).waitFor();
  assert.equal(await page.locator('.study-diagram rect').count(), 4);
  const materialRequest = requests.filter(request => request.path === '/api/materials' && request.method === 'POST').at(-1);
  assert.equal(materialRequest.body.style, 'cornell');
  assert.equal(materialRequest.body.document_type, 'Exam preparation guide');
  await capture('study-notes-diagram.png');
  const download = page.waitForEvent('download');
  await page.locator('#export-word').click();
  assert.match((await download).suggestedFilename(), /Testing study notes\.docx$/);
  const popup = page.waitForEvent('popup');
  await page.locator('#export-pdf').click();
  const printPage = await popup;
  await printPage.waitForFunction(() => window.printRequested === true);
  assert.equal(await printPage.locator('h1').innerText(), 'Testing study notes');
  assert.equal(await printPage.locator('svg').count(), 1);
  assert.equal(await printPage.locator('body').evaluate(node => getComputedStyle(node).color), 'rgb(22, 52, 45)');
  await printPage.close();
  await page.locator('#copy-onenote').click();
  await page.locator('#export-message').getByText(/^Copied\./).waitFor();
  assert.deepEqual(await page.evaluate(() => [...window.copiedMaterialTypes].sort()), ['text/html', 'text/plain']);
  assert.equal(await page.evaluate(() => window.copiedMaterialHtml.includes('data:image/png;base64,')), true);
  check('Cornell/document instructions, diagram, Word download, PDF print view, OneNote rich copy');
  await page.locator('#material-kind').selectOption('flashcards');
  await page.locator('#generate-material').click();
  await page.locator('.flashcard').first().locator('summary').click();
  assert.equal(await page.locator('.flashcard').first().getAttribute('open'), '');
  assert.equal(await page.locator('.flashcard').first().locator('p').innerText(), citation.quote);
  await page.locator('#material-kind').selectOption('memory');
  await page.locator('#memory-style').selectOption('both');
  await page.locator('#generate-material').click();
  await page.locator('#material-document-title').getByText('Testing memory aids', { exact: true }).waitFor();
  assert.equal(requests.filter(request => request.path === '/api/materials' && request.method === 'POST').at(-1).body.style, 'both');
  assert.match(await page.locator('#material-content').innerText(), /Original study song fixture/);
  await page.reload();
  await page.locator('#material-document-title').getByText('Testing memory aids', { exact: true }).waitFor();
  assert.equal(await page.locator('.material-library-item').count(), 3);
  page.once('dialog', dialog => dialog.accept());
  await page.locator('#remove-material').click();
  await page.getByText('Study material removed.', { exact: true }).waitFor();
  assert.equal(await page.locator('.material-library-item').count(), 2);
  check('Flashcard reveal, memory technique, persisted library reload and material removal');
  for (const [name, viewport] of [['phone', { width: 390, height: 844 }], ['4k', { width: 3840, height: 2160 }]]) {
    await page.setViewportSize(viewport);
    for (const screen of ['study', 'quiz', 'materials', 'settings', 'notes', 'history']) {
      await page.locator(`.nav-item[data-page="${screen}"]`).click();
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false, `${screen} overflows at ${name}`);
      const button = page.locator('#close-app');
      const targetWorks = await button.evaluate(node => { const rect = node.getBoundingClientRect(); return node.contains(document.elementFromPoint(rect.x + rect.width / 2, rect.y + rect.height / 2)); });
      assert.equal(targetWorks, true, `Close control obscured at ${screen}/${name}`);
      if (['study', 'quiz', 'materials', 'settings'].includes(screen)) await capture(`${name}-${screen}.png`);
    }
  }
  check('390px phone and 3840px desktop layouts: all six screens no overflow, close hit-tested');
  await page.locator('.nav-item[data-page="study"]').click();
  assert.equal(await page.locator('#motion-toggle').isDisabled(), true);
  assert.equal(await page.locator('.knowledge-cube').evaluate(node => getComputedStyle(node).animationName), 'none');
  assert.equal(await page.locator('#depth-field').evaluate(node => node.width >= innerWidth * 2), true);
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.waitForFunction(() => !document.getElementById('motion-toggle').disabled);
  const transforms = await page.locator('.knowledge-cube').evaluate(async node => { const before = getComputedStyle(node).transform; await new Promise(resolve => setTimeout(resolve, 80)); return [before, getComputedStyle(node).transform]; });
  assert.notEqual(transforms[0], transforms[1]);
  await page.locator('#motion-toggle').click();
  assert.equal(await page.locator('#motion-toggle').getAttribute('aria-pressed'), 'true');
  await page.reload();
  await page.waitForFunction(() => document.body.classList.contains('motion-off'));
  await page.keyboard.press('Tab');
  assert.equal(await page.evaluate(() => Boolean(document.activeElement?.matches('button,a,input,select,textarea,summary'))), true);
  assert.deepEqual(errors, []);
  check('High-density canvas, animated 3D, system reduced motion, persistent pause, keyboard focus');
  await writeFile(path.join(evidence, 'quiz-ui-verification.json'), JSON.stringify({ checks, backend: 'isolated labelled UI fixtures', cloud_calls: 0, screenshot_directory: evidence, browser_errors: errors }, null, 2));
  console.log(`PASS: ${checks.length} UI flow groups, all six screens at phone and 4K widths; isolated fixtures, no cloud calls.`);
  console.log(`Screenshots: ${evidence}`);
} finally {
  if (browser) await browser.close();
  server.close();
  await once(server, 'close');
}
