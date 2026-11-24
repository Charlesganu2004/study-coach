// Record only the isolated local Study Coach browser, with a visible fixture label.
// Each chapter samples the actual app tab at native UHD and holds its completed state for reading.
import assert from 'node:assert/strict';
import { mkdir, writeFile, stat } from 'node:fs/promises';
import path from 'node:path';
import { addRecordingCaptions, recordChapter, encodeChapters } from './record-tab.mjs';
import { chromium, root, startDemoHarness, fixtureName, fixtureModel, sourceTitle, sourceText, partialAnswer, paraphrasedAnswer, wrongAnswer } from '../tests/demo-provider.mjs';

if (!process.argv.includes('--record')) {
  console.log('Usage: node scripts/record-upgrade.mjs --record [--quick]');
  process.exit(0);
}
const quick = process.argv.includes('--quick');
const outputDirectory = quick ? path.join(root, 'target', 'recorder-check') : path.join(root, 'docs', 'demo');
await mkdir(outputDirectory, { recursive: true });
const basename = quick ? 'upgrade-recorder-check' : 'study-coach-upgrade';
const outputFile = path.join(outputDirectory, `${basename}.webm`);
const harness = await startDemoHarness();
let browser;
const frames = [];
const frameDirectory = path.join(root, 'target', 'walkthrough-capture', 'native-frames');
let timeline = 0;
const chapters = [];
const browserErrors = [];
const timestamp = seconds => {
  const milliseconds = Math.round(seconds * 1000);
  return `${String(Math.floor(milliseconds / 3600000)).padStart(2, '0')}:${String(Math.floor(milliseconds / 60000) % 60).padStart(2, '0')}:${String(Math.floor(milliseconds / 1000) % 60).padStart(2, '0')}.${String(milliseconds % 1000).padStart(3, '0')}`;
};

try {
  browser = await chromium.launch({headless:true, ...(process.platform === 'win32' ? {channel:'msedge'} : {})});
  const context = await browser.newContext({viewport:{width:3840,height:2160},deviceScaleFactor:1,reducedMotion:'no-preference',acceptDownloads:true});
  const page = await context.newPage();
  page.on('pageerror',error=>browserErrors.push(error.message));
  await page.goto(harness.url);
  await page.locator('#setup-notice').getByRole('button',{name:'Set up connection'}).waitFor();
  await addRecordingCaptions(page);
  async function scene(title, caption, action, seconds = 6) {
    const number=chapters.length+1;
    const length=quick ? 2 : seconds;
    await page.evaluate(args=>window.walkthroughCaption(...args), [title,caption,number,24]);
    await recordChapter(page,frameDirectory,frames,number,length,action,quick);
    chapters.push({start:timeline,end:timeline+length,title,caption});
    timeline+=length;
    await writeFile(path.join(frameDirectory,'chapters.json'),JSON.stringify(chapters));
    console.log('Recorded: '+title);
  }
  async function navigate(name) {
    await page.locator(`.nav-item[data-page="${name}"]`).click();
    await page.locator(`#page-${name}`).waitFor({ state: 'visible' });
  }
  await scene('Study Coach: an upgraded study workspace', 'This walkthrough uses a deterministic local model fixture. Every answer still travels through the real application service and its model transport.', async () => {
    await page.mouse.move(1470, 320);
  }, 7);
  await scene('Choose a connection', 'Connections lists the built-in models and supported provider templates. A custom endpoint lets you connect another OpenAI-compatible model.', async () => {
    await navigate('settings');
  }, 5);
  await scene('Add a model connection', 'The visible connection name identifies the local demonstration fixture. No cloud account, real API key, or billing is used in this recording.', async () => {
    await page.locator('#show-add-connection').click();
    await page.locator('#connection-provider').selectOption('custom');
    await page.locator('#connection-name').fill(fixtureName);
    await page.locator('#connection-model').fill(fixtureModel);
    await page.locator('#connection-endpoint').fill(harness.endpoint);
  }, 7);
  await scene('Save and select your study model', 'Saved connections show their model, endpoint, and readiness. Use model selects the connection for study, tests, and materials.', async () => {
    await page.locator('#add-connection').click();
    await page.getByRole('button', { name: `Use ${fixtureModel}`, exact: true }).click();
    await page.getByRole('button', { name: `${fixtureModel} selected`, exact: true }).waitFor();
  });
  await scene('Import your source material', 'Import a document or screenshot, review the extracted text, then save it. This example imports original Markdown notes; videos require captions or a transcript.', async () => {
    await navigate('notes');
    await page.locator('#note-file').setInputFiles({ name: `${sourceTitle}.md`, mimeType: 'text/markdown', buffer: Buffer.from(sourceText) });
    await page.waitForFunction(() => document.getElementById('note-text').value.includes('A unit test checks'));
  }, 7);
  await scene('Keep notes in your local workspace', 'The preview becomes a saved source only after Save source. Longer text imports are split into bounded source passages.', async () => {
    const count = (await harness.api('/api/state')).sources.length;
    await page.locator('#save-note').click();
    await page.waitForFunction(total => Number(document.getElementById('note-count').textContent) === total + 1, count);
  }, 5);
  await scene('Ask a question from your notes', 'Study uses the selected connection. The original source quotation appears beside the answer so you can inspect its evidence.', async () => {
    await navigate('study');
    await page.locator('#question').fill('What does a unit test check?');
    await page.locator('#ask-button').click();
    await page.locator('#answer-text').getByText(/A unit test checks one function/).waitFor();
    await page.locator('#answer-card').evaluate(node => node.scrollIntoView({block:'center',behavior:'instant'}));
  }, 7);
  await scene('Read the source, then check your understanding', 'This response is visibly labelled as a local fixture. The service verified its quotation against the saved notes before displaying it.', null);
  await scene('Build a practice test', 'Choose open response, multiple choice, fill in the blank, or a custom format. Choose one to thirty questions; larger tests are generated in smaller batches.', async () => {
    await navigate('quiz');
    await page.locator('#quiz-topic').fill('Unit tests and integration tests');
    await page.locator('#quiz-count').fill('3');
    await page.locator('#quiz-use-all-sources').uncheck();
    await page.locator('#quiz-source-picker').getByRole('button', { name: 'Clear', exact: true }).click();
    await page.locator('#quiz-source-picker').getByRole('checkbox', { name: new RegExp(sourceTitle) }).check();
  });
  await scene('Answer one question at a time', 'Starting a test opens a focused question screen. Expected answers and solutions remain on the service until assessment or an explicit reveal.', async () => {
    await page.locator('#start-quiz').click();
    await page.locator('#quiz-exam').waitFor({ state: 'visible' });
    assert.equal(await page.locator('#quiz-solution').isHidden(), true);
  });
  await scene('Yellow: close, with another attempt', 'A partly correct explanation turns the test yellow and offers Try again. The solution remains hidden while you refine the answer.', async () => {
    await page.locator('#quiz-answer').fill(partialAnswer);
    await page.locator('#check-quiz-answer').click();
    await page.locator('#quiz-exam[data-status="close"]').waitFor();
  }, 7);
  await scene('Green: correct, with the reasoning', 'A complete explanation turns the test green. A semantically correct paraphrase is accepted, and the source-supported solution becomes visible.', async () => {
    await page.locator('#retry-quiz-answer').click();
    await page.locator('#quiz-answer').fill(paraphrasedAnswer);
    await page.locator('#check-quiz-answer').click();
    await page.locator('#quiz-exam[data-status="correct"]').waitFor();
    await page.locator('#quiz-feedback').evaluate(node => node.scrollIntoView({block:'center',behavior:'instant'}));
  }, 7);
  await scene('Red: review an incorrect answer', 'An incorrect answer turns the test red, locks that outcome, and shows the solution and exact source passage for review.', async () => {
    await page.locator('#next-quiz-question').click();
    await page.locator('#quiz-answer').fill(wrongAnswer);
    await page.locator('#check-quiz-answer').click();
    await page.locator('#quiz-exam[data-status="incorrect"]').waitFor();
    await page.locator('#quiz-feedback').evaluate(node => node.scrollIntoView({block:'center',behavior:'instant'}));
  }, 7);
  await scene('See solution records a question to revisit', 'Reveal is also red. Looking at a solution does not count as answering correctly; the recorded outcome stays fixed.', async () => {
    await page.locator('#next-quiz-question').click();
    await page.locator('#see-quiz-solution').click();
    await page.locator('#quiz-exam[data-status="incorrect"]').waitFor();
    await page.locator('#quiz-feedback').evaluate(node => node.scrollIntoView({block:'center',behavior:'instant'}));
  });
  await scene('Finish with a review list', 'The summary lists the result of each question. The topic review list is saved locally so you can return to ideas that need practice.', async () => {
    await page.locator('#next-quiz-question').click();
    await page.locator('#quiz-summary').waitFor({ state: 'visible' });
  }, 5);
  await scene('Create notes in your preferred style', 'Choose concise, detailed, Cornell, outline, study guide, or custom notes. Document type and instructions are saved with the material.', async () => {
    await navigate('materials');
    await page.locator('#material-topic').fill('Unit tests and the Arrange Act Assert pattern');
    await page.locator('#material-style').selectOption('cornell');
    await page.locator('#material-document-type').fill('Exam preparation guide');
  }, 7);
  await scene('Keep editable notes and a source-supported diagram', 'Generated sections, the relationship diagram, and exact source passages stay together in your local library.', async () => {
    await page.locator('#generate-material').click();
    await page.locator('#material-document').waitFor({ state: 'visible' });
    await page.getByText('Study material saved to your library.', { exact: true }).waitFor();
    await page.locator('#material-document-title').evaluate(node => node.scrollIntoView({block:'center',behavior:'instant'}));
  }, 7);
  await scene('Download an editable Word document', 'The Word export contains native editable text, source passages, and a vector diagram. It is a real document download from the Rust service.', async () => {
    const downloaded = page.waitForEvent('download');
    await page.locator('#export-word').click();
    assert.match((await downloaded).suggestedFilename(), /\.docx$/i);
  }, 5);
  let printPage;
  await scene('Open the print view to save as PDF', 'Save as PDF opens a clean printable view with notes, all flashcard answers, diagrams, and source passages. Choose Save as PDF in the print dialog.', async () => {
    // Leave the native printer dialog closed in this headless recording. The
    // application still opens and populates its own printable document.
    await page.evaluate(() => {
      const open = window.open;
      window.open = (...args) => {
        const popup = open.apply(window, args);
        if (popup) popup.print = () => {};
        window.open = open;
        return popup;
      };
    });
    const opened = context.waitForEvent('page');
    await page.locator('#export-pdf').click();
    printPage = await opened;
    await printPage.waitForLoadState('domcontentloaded');
    await printPage.locator('svg').first().waitFor({state:'attached'});
    // Present the actual generated print view in the one tab being recorded.
    const printImage = await printPage.screenshot({ type: 'png' });
    assert.equal(printImage.readUInt32BE(16), 3840);
    assert.equal(printImage.readUInt32BE(20), 2160);
    await page.evaluate(async pixels => {
      const preview = document.createElement('img');
      preview.alt = 'Actual generated printable study notes';
      preview.id = 'walkthrough-print-preview';
      preview.style.cssText = 'position:fixed;inset:0;width:100vw;height:100vh;border:0;background:white;z-index:2147483646';
      preview.src = 'data:image/png;base64,' + pixels;
      await preview.decode();
      document.body.append(preview);
    }, printImage.toString('base64'));
    await page.bringToFront();
  });
  await page.locator('#walkthrough-print-preview').evaluate(node => node.remove());
  await printPage.close();
  await page.bringToFront();
  await scene('Bring the notes into OneNote', 'Copy for OneNote prepares rich notes and the diagram. If browser clipboard access is blocked, a downloadable HTML file provides a manual copy-and-paste path.', async () => {
    await page.locator('#copy-onenote').click();
    await page.waitForFunction(() => /^(Copied\.|Rich copy was blocked\.)/.test(document.getElementById('export-message').textContent));
  }, 5);
  await scene('Use flashcards for active recall', 'Create five flashcards from the same source collection. Open a card only after trying to recall the answer yourself.', async () => {
    await page.locator('#material-kind').selectOption('flashcards');
    await page.locator('#generate-material').click();
    await page.waitForFunction(() => document.querySelectorAll('#material-content .flashcard').length === 5);
    await page.locator('.flashcard summary').first().click();
    await page.locator('#material-content').evaluate(node => node.scrollIntoView({block:'center',behavior:'instant'}));
  }, 7);
  await scene('Make an original mnemonic or study song', 'Memory aids map each letter or lyric back to a source fact. This feature creates original text; it does not generate audio.', async () => {
    await page.locator('#material-kind').selectOption('memory');
    await page.locator('#memory-style').selectOption('both');
    await page.locator('#generate-material').click();
    await page.locator('#material-document-title').filter({ hasText: 'Arrange, Act, Assert' }).waitFor();
    await page.locator('#material-content').evaluate(node => node.scrollIntoView({block:'center',behavior:'instant'}));
  }, 7);
  await scene('Return to the ideas worth practicing', 'Your review list distinguishes correct attempts from topics to revisit. Saved notes, generated materials, connections, and progress survive reopening the app.', async () => {
    await navigate('quiz');
    await page.getByRole('heading', { name: 'Questions to revisit' }).evaluate(node => node.scrollIntoView({block:'center',behavior:'instant'}));
  });
  await scene('Study Coach: connect, practice, and keep the evidence', 'The recorded model responses are deterministic local fixtures. Real account availability, model behavior, and cloud quotas are not claimed by this demonstration.', async () => {
    await navigate('study');
    await page.mouse.move(1470, 320);
  });
  const duration = timeline;
  assert.deepEqual(browserErrors, []);
  assert.deepEqual(harness.providerErrors, []);
  await encodeChapters(frameDirectory,frames,outputFile);
  const size=(await stat(outputFile)).size;
  const vtt = `WEBVTT\n\n${chapters.map((chapter, index) => `${index + 1}\n${timestamp(chapter.start)} --> ${timestamp(chapter.end)}\n${chapter.title}\n${chapter.caption}\n`).join('\n')}`;
  await writeFile(path.join(outputDirectory, `${basename}.vtt`), vtt);
  const transcript = `# Study Coach upgrade walkthrough\n\nRecorded ${new Date().toISOString()}. The recording captures only an isolated local app browser. No desktop, microphone, user credentials, or cloud requests were captured. All model responses come from the visibly labelled deterministic local demonstration fixture through the real Rust service and curl transport.\n\n${chapters.map(chapter => `## ${timestamp(chapter.start)} ${chapter.title}\n\n${chapter.caption}`).join('\n\n')}\n`;
  await writeFile(path.join(outputDirectory, `${basename}-transcript.md`), transcript);
  const receipt = {recorded_at:new Date().toISOString(),file:path.basename(outputFile),format:'video/webm;codecs=vp9',width:3840,height:2160,capture_surface:'native browser screenshots from completed product interactions',duration_seconds:duration,size_bytes:size,actual_rust_service:true,actual_curl_transport:true,model_fixture:fixtureName,cloud_requests:false,desktop_capture:false,audio_capture:false,provider_calls:harness.calls.length,browser_errors:browserErrors,chapters};
  await writeFile(path.join(outputDirectory,basename+'-verification.json'),JSON.stringify(receipt,null,2));
  console.log('Recorded '+frames.length+' native UHD frames.');
  console.log(`Video: ${outputFile}`);
} finally {
  await browser?.close();
  await harness.close();
}
