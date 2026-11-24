// Full-chain upgrade verification: browser -> Rust API -> curl -> loopback model.
// The local model is deliberately labelled; no provider key or cloud call is used.
import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { inflateRawSync } from 'node:zlib';
import path from 'node:path';
import { chromium, startDemoHarness, fixtureName, fixtureModel, sourceTitle, sourceText, expectedAnswer, partialAnswer, paraphrasedAnswer, wrongAnswer } from './demo-provider.mjs';

const checks = [];
const harness = await startDemoHarness({ evidence: process.env.STUDY_COACH_UPGRADE_EVIDENCE });
let browser;
const browserErrors = [];
const check = (name, condition) => { assert.ok(condition, name); checks.push(name); };

function zipEntries(bytes) {
  const entries = new Map();
  let offset = 0;
  while (bytes.readUInt32LE(offset) === 0x04034b50) {
    const method = bytes.readUInt16LE(offset + 8);
    const length = bytes.readUInt32LE(offset + 18);
    const nameLength = bytes.readUInt16LE(offset + 26);
    const extraLength = bytes.readUInt16LE(offset + 28);
    const name = bytes.subarray(offset + 30, offset + 30 + nameLength).toString();
    const start = offset + 30 + nameLength + extraLength;
    const content = bytes.subarray(start, start + length);
    entries.set(name, method === 0 ? content : inflateRawSync(content));
    offset = start + length;
  }
  return entries;
}

async function navigate(page, name) {
  await page.locator(`.nav-item[data-page="${name}"]`).click();
  await page.locator(`#page-${name}`).waitFor({ state: 'visible' });
}

async function screenshot(page, name) {
  await page.screenshot({ path: path.join(harness.evidence, `${name}.png`) });
}

try {
  const initial = await harness.api('/api/state');
  check('All supported provider templates are listed', initial.providers.length === 6);
  check('Fresh app has no configured cloud keys', initial.keys.groq === false && initial.keys.nvidia === false);
  await harness.api('/api/sources', { title: sourceTitle, text: sourceText });

  browser = await chromium.launch({ headless: true, ...(process.platform === 'win32' ? { channel: 'msedge' } : {}) });
  const context = await browser.newContext({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1, reducedMotion: 'no-preference', acceptDownloads: true });
  const page = await context.newPage();
  page.on('pageerror', error => browserErrors.push(error.message));
  await page.goto(harness.url);
  await page.locator('#note-count').filter({ hasText: String(initial.sources.length + 1) }).waitFor();
  await navigate(page, 'settings');
  await page.locator('#show-add-connection').click();
  await page.locator('#connection-provider').selectOption('custom');
  await page.locator('#connection-name').fill(fixtureName);
  await page.locator('#connection-model').fill(fixtureModel);
  await page.locator('#connection-endpoint').fill(harness.endpoint);
  await page.locator('#add-connection').click();
  await page.locator('.connection-row').filter({ hasText: fixtureName }).waitFor();
  await page.getByRole('button', { name: `Use ${fixtureModel}`, exact: true }).click();
  await page.getByRole('button', { name: `${fixtureModel} selected`, exact: true }).waitFor();
  const connected = await harness.api('/api/state');
  check('Custom keyless loopback model can be added and selected in UI', connected.selected_profile.startsWith('custom-'));
  check('Connection metadata exposes readiness without credentials', connected.profiles.at(-1).requires_key === false && connected.profiles.at(-1).api_key === undefined);
  await screenshot(page, 'connections');

  await navigate(page, 'study');
  await page.locator('#question').fill('What does a unit test check?');
  await page.locator('#ask-button').click();
  await page.locator('#answer-text').getByText(/A unit test checks one function/).waitFor();
  check('Study answers traverse actual custom provider transport', harness.calls.some(call => call.kind === 'answer'));
  check('Returned quotation is verified and rendered', (await page.locator('#citations blockquote').innerText()) === sourceText);
  await screenshot(page, 'study-real-local-transport');

  const callsBeforeBatch = harness.calls.filter(call => call.kind === 'quiz').length;
  const batch = await harness.api('/api/quiz', { topic: 'Unit and integration tests', count: 7, type: 'open-response' });
  check('Seven-question quiz uses bounded generation batches', batch.questions.length === 7 && harness.calls.filter(call => call.kind === 'quiz').length === callsBeforeBatch + 2);
  check('Creation response withholds solution and answer rubric', batch.questions.every(question => !('solution' in question) && !('accepted_answers' in question)) && !JSON.stringify(batch).includes(expectedAnswer));
  const grade = (question, answer) => harness.api('/api/quiz/grade', { quiz_id: batch.id, question_id: question.id, answer });
  const close = await grade(batch.questions[0], partialAnswer);
  check('Partial answer is close and withholds solution', close.status === 'close' && close.solution === undefined);
  const correct = await grade(batch.questions[0], paraphrasedAnswer);
  check('Close answer can retry to semantically correct', correct.status === 'correct' && correct.solution.includes(expectedAnswer));
  const wrong = await grade(batch.questions[1], wrongAnswer);
  check('Wrong answer shows solution', wrong.status === 'incorrect' && wrong.solution.includes(expectedAnswer));
  const reveal = await harness.api('/api/quiz/solution', { quiz_id: batch.id, question_id: batch.questions[2].id });
  check('Reveal records incorrect and shows solution', reveal.status === 'incorrect' && reveal.solution.includes(expectedAnswer));
  check('Revealed terminal result cannot be changed to correct', (await grade(batch.questions[2], expectedAnswer)).status === 'incorrect');
  for (const question of batch.questions.slice(3)) await grade(question, expectedAnswer);
  const progress = await harness.api('/api/progress');
  check('Progress stores seven question outcomes rather than retry duplicates', progress.recent.length === 7);

  await navigate(page, 'quiz');
  await page.locator('#quiz-topic').fill('Unit tests');
  await page.locator('#quiz-count').fill('3');
  await page.locator('#start-quiz').click();
  await page.locator('#quiz-exam').waitFor({ state: 'visible' });
  check('Test opens dedicated question screen with solution hidden', await page.locator('#quiz-solution').isHidden());
  await page.locator('#quiz-answer').fill(partialAnswer);
  await page.locator('#check-quiz-answer').click();
  await page.locator('#quiz-exam[data-status="close"]').waitFor();
  check('Close state offers Try again while solution stays hidden', await page.locator('#retry-quiz-answer').isVisible() && await page.locator('#quiz-solution').isHidden());
  const closeColor = await page.locator('#quiz-feedback').evaluate(node => getComputedStyle(node).color);
  await screenshot(page, 'quiz-close-yellow');
  await page.locator('#retry-quiz-answer').click();
  await page.locator('#quiz-answer').fill(paraphrasedAnswer);
  await page.locator('#check-quiz-answer').click();
  await page.locator('#quiz-exam[data-status="correct"]').waitFor();
  const correctColor = await page.locator('#quiz-feedback').evaluate(node => getComputedStyle(node).color);
  check('Correct feedback locks answer and displays reasoning', await page.locator('#quiz-answer').isDisabled() && await page.locator('#quiz-solution').isVisible());
  await screenshot(page, 'quiz-correct-green');
  await page.locator('#next-quiz-question').click();
  await page.locator('#quiz-answer').fill(wrongAnswer);
  await page.locator('#check-quiz-answer').click();
  await page.locator('#quiz-exam[data-status="incorrect"]').waitFor();
  const incorrectColor = await page.locator('#quiz-feedback').evaluate(node => getComputedStyle(node).color);
  check('Correct, close and wrong feedback have distinct rendered colors', new Set([closeColor, correctColor, incorrectColor]).size === 3);
  await screenshot(page, 'quiz-incorrect-red');
  await page.locator('#next-quiz-question').click();
  await page.locator('#see-quiz-solution').click();
  await page.locator('#quiz-exam[data-status="incorrect"]').waitFor();
  check('See solution is red and terminal in UI', await page.locator('#quiz-answer').isDisabled());
  await page.locator('#next-quiz-question').click();
  await page.locator('#quiz-summary').waitFor({ state: 'visible' });
  check('Test summary includes all outcomes', await page.locator('.summary-result').count() === 3);

  for (const kind of ['multiple-choice', 'fill-in-the-blank', 'custom']) {
    const generated = await harness.api('/api/quiz', { topic: 'Unit tests', count: 1, type: kind, instructions: kind === 'custom' ? 'Ask one short explanatory question.' : '' });
    check(`${kind} question has supported structure`, generated.questions[0].type === (kind === 'custom' ? 'open-response' : kind));
    const answer = kind === 'multiple-choice' ? generated.questions[0].options[0] : kind === 'fill-in-the-blank' ? 'function' : expectedAnswer;
    const assessed = await harness.api('/api/quiz/grade', { quiz_id: generated.id, question_id: 'q1', answer });
    check(`${kind} answers are assessed through the real API`, assessed.status === 'correct');
  }

  await navigate(page, 'notes');
  await page.locator('#note-file').setInputFiles({ name: 'Imported upgrade notes.md', mimeType: 'text/markdown', buffer: Buffer.from('A focused revision session starts by recalling the idea, then checking the original source.') });
  await page.waitForFunction(() => document.getElementById('note-text').value.includes('focused revision'));
  const sourceCount = (await harness.api('/api/state')).sources.length;
  await page.locator('#save-note').click();
  await page.waitForFunction(count => Number(document.getElementById('note-count').textContent) === count + 1, sourceCount);
  check('Text import is previewed and saved through the UI', (await harness.api('/api/state')).sources.length === sourceCount + 1);
  const bulkText = ('Recall the definition, compare its evidence, and explain the distinction in your own words.\n').repeat(220);
  const bulk = await harness.api('/api/sources/import', { title: 'Long revision notes', text: bulkText });
  check('Bulk import splits long Unicode-safe source chunks', bulk.sources.filter(source => source.title.startsWith('Long revision notes')).length >= 2);

  await navigate(page, 'materials');
  await page.locator('#material-topic').fill('Unit tests and the Arrange Act Assert pattern');
  await page.locator('#material-style').selectOption('cornell');
  await page.locator('#material-document-type').fill('Exam preparation guide');
  await page.locator('#generate-material').click();
  await page.locator('#material-document').waitFor({ state: 'visible' });
  await page.getByText('Study material saved to your library.', { exact: true }).waitFor();
  check('Notes and source-supported vector diagram render', await page.locator('#material-content .material-section').count() >= 2 && await page.locator('#material-diagram svg').count() === 1);
  const savedNotes = (await harness.api('/api/materials')).entries.find(entry => entry.kind === 'notes');
  check('Requested note style and document type persist', savedNotes.style === 'cornell' && savedNotes.document_type === 'Exam preparation guide');
  await screenshot(page, 'notes-and-diagram');
  const downloadPromise = page.waitForEvent('download');
  await page.locator('#export-word').click();
  const download = await downloadPromise;
  const wordPath = path.join(harness.evidence, 'upgrade-notes.docx');
  await download.saveAs(wordPath);
  const word = zipEntries(await readFile(wordPath));
  check('Actual Word download contains editable notes, citations and vector diagram', word.has('word/document.xml') && word.has('word/media/diagram.svg') && word.get('word/document.xml').toString().includes('Recall cue: what is isolated?') && word.get('word/document.xml').toString().includes(sourceText));
  const popupPromise = page.waitForEvent('popup');
  await page.locator('#export-pdf').click();
  const printPage = await popupPromise;
  await printPage.waitForLoadState('domcontentloaded');
  check('PDF action opens complete printable document', (await printPage.locator('body').innerText()).includes('Recall cue: what is isolated?') && await printPage.locator('svg').count() === 1);
  // The application's page already owns the native print dialog. Render an
  // exact copy of its populated document in a separate print context.
  const preparedDocument = await printPage.content();
  await printPage.close();
  const proof = await context.newPage();
  await proof.setContent(preparedDocument, { waitUntil: 'load' });
  assert.ok(await proof.locator('link[rel="stylesheet"]').evaluate(link => Boolean(link.sheet)));
  const pdf = await proof.pdf({ path: path.join(harness.evidence, 'upgrade-notes.pdf'), format: 'A4', printBackground: true });
  check('Native browser creates a real PDF artifact', pdf.subarray(0, 5).toString() === '%PDF-');
  await proof.close();
  const html = await page.evaluate(() => exportMaterialDocument().documentElement.outerHTML);
  await writeFile(path.join(harness.evidence, 'upgrade-notes.html'), `<!doctype html>${html}`);
  check('OneNote/HTML export retains source passages and diagram', html.includes('Source passages') && html.includes('<svg') && html.includes(sourceText));
  const downloads = [];
  page.on('download', item => downloads.push(item));
  await page.locator('#copy-onenote').click();
  await page.waitForFunction(() => /^(Copied\.|Rich copy was blocked\.)/.test(document.getElementById('export-message').textContent));
  check('OneNote action reports rich copy or usable HTML fallback', /Copied\.|downloaded HTML/.test(await page.locator('#export-message').innerText()));
  for (const [index, item] of downloads.entries()) await item.saveAs(path.join(harness.evidence, `onenote-fallback-${index}.html`));

  await page.locator('#material-kind').selectOption('flashcards');
  await page.locator('#generate-material').click();
  await page.waitForFunction(() => document.querySelectorAll('#material-content .flashcard').length === 5);
  await page.locator('.flashcard summary').first().click();
  check('Five flashcards reveal answers on interaction', await page.locator('.flashcard').first().getAttribute('open') !== null);
  await screenshot(page, 'flashcards');
  await page.locator('#material-kind').selectOption('memory');
  await page.locator('#memory-style').selectOption('both');
  await page.locator('#generate-material').click();
  await page.locator('#material-document-title').filter({ hasText: 'Arrange, Act, Assert' }).waitFor();
  check('Original mnemonic and song text render with source mapping', (await page.locator('#material-content').innerText()).includes('Each A maps to one step') && (await page.locator('#material-content').innerText()).includes('Arrange the scene'));
  await screenshot(page, 'memory-aids');

  await page.setViewportSize({ width: 390, height: 844 });
  for (const destination of ['study', 'quiz', 'materials', 'notes', 'settings']) {
    await navigate(page, destination);
    check(`Phone ${destination} has no horizontal overflow`, await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
  }
  await screenshot(page, 'phone-connections');
  await page.setViewportSize({ width: 3840, height: 2160 });
  await navigate(page, 'study');
  check('4K viewport renders without horizontal overflow', await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
  await screenshot(page, '4k-study');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  check('Device reduced motion stops CSS animation', await page.locator('.knowledge-cube').evaluate(node => getComputedStyle(node).animationName === 'none'));

  const libraryBeforeRestart = await harness.api('/api/materials');
  const progressBeforeRestart = await harness.api('/api/progress');
  await context.close();
  await harness.restart();
  const reopened = await harness.api('/api/state');
  check('Custom connection and selected model survive restart', reopened.selected_profile === connected.selected_profile && reopened.profiles.some(item => item.name === fixtureName));
  check('Generated materials survive restart', JSON.stringify(await harness.api('/api/materials')) === JSON.stringify(libraryBeforeRestart));
  check('Quiz progress survives restart', JSON.stringify(await harness.api('/api/progress')) === JSON.stringify(progressBeforeRestart));
  check('Mock provider accepted every real request', harness.providerErrors.length === 0);
  check('Browser reported no runtime exceptions', browserErrors.length === 0);
  const receipt = { checked_at: new Date().toISOString(), status: 'PASS', checks, browser: 'Microsoft Edge headless', actual_rust_service: true, actual_curl_transport: true, api_route_interception: false, cloud_requests: false, fixture: fixtureName, provider_calls: harness.calls.map(call => ({ kind: call.kind, model: call.model, source_count: call.source_count })), browser_errors: browserErrors };
  await writeFile(path.join(harness.evidence, 'upgrade-verification.json'), JSON.stringify(receipt, null, 2));
  console.log(`PASS: ${checks.length} full-chain upgrade checks; ${harness.calls.length} actual loopback model requests; no cloud calls.`);
  console.log(`Evidence: ${harness.evidence}`);
} finally {
  await browser?.close();
  await harness.close();
}
