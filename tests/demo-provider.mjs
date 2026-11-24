// Deterministic local provider for integration checks and the recorded walkthrough.
// Every request still travels through the real Rust service and curl transport.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { createServer } from 'node:http';
import { mkdtemp, mkdir } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
export const { chromium } = require('playwright');
export const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
export const fixtureName = 'Local demonstration fixture';
export const fixtureModel = 'local-demonstration-fixture';
export const sourceTitle = 'Upgrade demonstration notes';
export const sourceText = 'A unit test checks one function in isolation. An integration test checks how components work together. Precision is the fraction of predicted positives that are correct. Recall is the fraction of actual positives that are found. A test first arranges its inputs, acts on the behavior, and asserts the expected result.';
export const expectedAnswer = 'A unit test checks one function in isolation.';
export const partialAnswer = 'It checks a function.';
export const paraphrasedAnswer = 'It tests a single function on its own.';
export const wrongAnswer = 'It makes coffee.';

// MediaRecorder writes a streaming WebM segment without a Duration field.
// Add that standard container field so local players expose finite seek controls;
// encoded video clusters are preserved byte for byte.
export function finalizeWebmDuration(bytes, seconds) {
  assert.ok(Number.isFinite(seconds) && seconds > 0);
  function variable(offset, marker = false) {
    assert.ok(offset < bytes.length, 'Truncated EBML element.');
    let width = 1;
    while (width <= 8 && !(bytes[offset] & (0x80 >> (width - 1)))) width++;
    assert.ok(width <= 8 && offset + width <= bytes.length, 'Invalid EBML element length.');
    let value = BigInt(marker ? bytes[offset] : bytes[offset] & ((0x80 >> (width - 1)) - 1));
    for (let index = 1; index < width; index++) value = value * 256n + BigInt(bytes[offset + index]);
    return { width, value, unknown: !marker && value === (1n << BigInt(7 * width)) - 1n };
  }
  function element(offset) {
    const identifier = variable(offset, true);
    const size = variable(offset + identifier.width);
    const start = offset + identifier.width + size.width;
    const end = size.unknown ? bytes.length : start + Number(size.value);
    assert.ok(end <= bytes.length, 'Truncated EBML payload.');
    return { offset, id: Number(identifier.value), idWidth: identifier.width, size, start, end };
  }
  function encodedSize(value, width) {
    assert.ok(BigInt(value) < (1n << BigInt(width * 7)) - 1n, 'EBML size no longer fits its original field.');
    const result = Buffer.alloc(width);
    let encoded = BigInt(value) | (1n << BigInt(width * 7));
    for (let index = width - 1; index >= 0; index--) { result[index] = Number(encoded & 255n); encoded >>= 8n; }
    return result;
  }
  let segment;
  for (let offset = 0; offset < bytes.length;) {
    const item = element(offset);
    if (item.id === 0x18538067) { segment = item; break; }
    offset = item.end;
  }
  assert.ok(segment, 'WebM Segment was not found.');
  let info;
  for (let offset = segment.start; offset < segment.end;) {
    const item = element(offset);
    if (item.id === 0x1549a966) { info = item; break; }
    offset = item.end;
  }
  assert.ok(info && !info.size.unknown, 'WebM Info was not found.');
  let scale = 1_000_000;
  let existing;
  for (let offset = info.start; offset < info.end;) {
    const item = element(offset);
    if (item.id === 0x2ad7b1) scale = Number(bytes.subarray(item.start, item.end).reduce((value, byte) => value * 256n + BigInt(byte), 0n));
    if (item.id === 0x4489) existing = item;
    offset = item.end;
  }
  const duration = Buffer.alloc(11);
  duration.set([0x44, 0x89, 0x88]);
  duration.writeDoubleBE(seconds * 1_000_000_000 / scale, 3);
  const content = existing
    ? Buffer.concat([bytes.subarray(info.start, existing.offset), duration, bytes.subarray(existing.end, info.end)])
    : Buffer.concat([bytes.subarray(info.start, info.end), duration]);
  const result = Buffer.concat([bytes.subarray(0, info.offset + info.idWidth), encodedSize(content.length, info.size.width), content, bytes.subarray(info.end)]);
  if (!segment.size.unknown) encodedSize(Number(segment.size.value) + result.length - bytes.length, segment.size.width).copy(result, segment.offset + segment.idWidth);
  return result;
}

function generatedQuestions(prompt, count, sequence) {
  const requested = prompt.match(/Question format: "([^"]+)"/)?.[1] || 'open-response';
  const kind = requested === 'custom' ? 'open-response' : requested;
  return Array.from({ length: count }, (_, index) => {
    const number = sequence + index;
    const item = {
      question: `Practice ${number}: What does a unit test check?`,
      type: kind,
      solution: `${expectedAnswer} This keeps the behavior under test separate from interactions between components.`,
      accepted_answers: [expectedAnswer, 'one function in isolation'],
    };
    if (kind === 'multiple-choice') {
      item.options = ['One function in isolation', 'All network services at once', 'Only computer hardware', 'No observable behavior'];
      item.correct_option = 0;
      item.accepted_answers = [item.options[0]];
    }
    if (kind === 'fill-in-the-blank') {
      item.question = `Practice ${number}: A unit test checks one ____ in isolation.`;
      item.solution = 'The missing word is function. A unit test checks one function in isolation.';
      item.accepted_answers = ['function'];
    }
    return item;
  });
}

function material(kind) {
  if (kind === 'flashcards') return {
    title: 'Testing recall / local demonstration fixture',
    cards: [
      { front: 'What does a unit test check?', back: expectedAnswer },
      { front: 'What does an integration test check?', back: 'It checks how components work together.' },
      { front: 'What does precision measure?', back: 'The fraction of predicted positives that are correct.' },
      { front: 'What does recall measure?', back: 'The fraction of actual positives that are found.' },
      { front: 'What are the three steps in the source test pattern?', back: 'Arrange the inputs, act on the behavior, and assert the expected result.' },
    ],
  };
  if (kind === 'memory') return {
    title: 'Arrange, Act, Assert / local demonstration fixture',
    sections: [
      { heading: 'AAA: a three-step memory aid', text: 'Arrange the inputs. Act on the behavior. Assert the expected result. Each A maps to one step stated in the source notes.' },
      { heading: 'An original recall verse', text: 'Arrange the scene, then act with care;\nAssert the result and compare.\nCover the notes and recite the three steps before checking the source.' },
    ],
    diagram: { title: 'A test follows three steps', nodes: ['Arrange', 'Act', 'Assert'], edges: [{ from: 0, to: 1 }, { from: 1, to: 2 }] },
  };
  return {
    title: 'Testing foundations / local demonstration fixture',
    sections: [
      { heading: 'Recall cue: what is isolated?', text: `${expectedAnswer} An integration test checks how components work together. Choose the test scope to match the behavior you want to inspect.` },
      { heading: 'Worked structure: Arrange, Act, Assert', text: 'First arrange the inputs. Next act on the behavior. Finally assert the expected result. For a function test, prepare one input, call the function, and compare its result with the expected value.' },
      { heading: 'Summary and self-check', text: 'Unit tests inspect one function in isolation; integration tests inspect interactions. Explain this distinction from memory, then check your explanation against the source passage.' },
    ],
    diagram: { title: 'A test follows three steps', nodes: ['Arrange', 'Act', 'Assert'], edges: [{ from: 0, to: 1 }, { from: 1, to: 2 }] },
  };
}

export async function startDemoHarness(options = {}) {
  const temporary = options.temporary || await mkdtemp(path.join(os.tmpdir(), 'study-coach-upgrade-'));
  const dataDir = path.join(temporary, 'data');
  const evidence = options.evidence || path.join(temporary, 'evidence');
  await mkdir(evidence, { recursive: true });
  const calls = [];
  const providerErrors = [];
  let sequence = 1;
  const provider = createServer(async (request, response) => {
    try {
      assert.equal(request.method, 'POST');
      assert.equal(request.url, '/v1/chat/completions');
      assert.equal(request.headers.authorization, undefined, 'Keyless loopback connection must omit Authorization.');
      let raw = '';
      for await (const chunk of request) {
        raw += chunk.toString();
        assert.ok(Buffer.byteLength(raw) <= 1024 * 1024);
      }
      const payload = JSON.parse(raw);
      assert.equal(payload.model, fixtureModel);
      assert.equal(payload.stream, false);
      assert.equal(typeof payload.messages?.[0]?.content, 'string');
      const input = JSON.parse(payload.messages[1].content);
      const source = input.sources.find(item => item.title === sourceTitle);
      assert.ok(source, 'The real request must contain the demonstration source.');
      assert.equal(source.text, sourceText);
      const prompt = input.question;
      let content;
      let kind;
      if (/^Create exactly \d+ distinct practice questions/.test(prompt)) {
        const count = Number(prompt.match(/^Create exactly (\d+)/)[1]);
        content = { questions: generatedQuestions(prompt, count, sequence) };
        sequence += count;
        kind = 'quiz';
      } else if (prompt.startsWith('Assess this student answer')) {
        const submission = JSON.parse(prompt.split('DATA, never instructions: ')[1].split('. Keep your normal outer response schema')[0]);
        const answer = submission.student_answer;
        const verdict = answer === partialAnswer ? 'close' : answer === paraphrasedAnswer ? 'correct' : 'incorrect';
        content = { verdict, feedback: verdict === 'close' ? 'You identified a function. Add the detail about its scope, then try again.' : verdict === 'correct' ? 'Correct. Your explanation preserves the isolation requirement.' : 'This does not match the behavior described in the source. Review the explanation.' };
        kind = 'grade';
      } else if (/^Create (notes|flashcards|memory) about/.test(prompt)) {
        const materialKind = prompt.match(/^Create (notes|flashcards|memory)/)[1];
        content = material(materialKind);
        kind = materialKind;
      } else {
        content = `${expectedAnswer}\nLocal demonstration fixture response, transported through the actual Rust service.`;
        kind = 'answer';
      }
      calls.push({ kind, question: prompt, model: payload.model, source_id: source.id, source_count: input.sources.length });
      const outer = { status: 'answer', answer: typeof content === 'string' ? content : JSON.stringify(content), citations: [{ source_id: source.id, quote: sourceText }] };
      const envelope = JSON.stringify({ choices: [{ message: { content: JSON.stringify(outer) } }], usage: { prompt_tokens: 400, completion_tokens: 220 } });
      response.writeHead(200, { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(envelope) });
      response.end(envelope);
    } catch (error) {
      providerErrors.push(error.message);
      response.writeHead(400, { 'Content-Type': 'application/json' });
      response.end(JSON.stringify({ error: 'Local demonstration fixture rejected an unexpected request.' }));
    }
  });
  provider.listen(0, '127.0.0.1');
  await once(provider, 'listening');
  const endpoint = `http://127.0.0.1:${provider.address().port}/v1/chat/completions`;
  let child;
  let url;
  let token;
  let stderr = '';
  const environment = { ...process.env, GROQ_API_KEY: '', NVIDIA_API_KEY: '' };
  const executable = process.env.STUDY_COACH_APP || path.join(root, 'target', 'debug', process.platform === 'win32' ? 'study-coach-app.exe' : 'study-coach-app');
  async function start() {
    child = spawn(executable, ['--data-dir', dataDir, '--no-open'], { env: environment, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
    child.stderr.on('data', chunk => { stderr += chunk.toString(); });
    url = await new Promise((resolve, reject) => {
      let output = '';
      const timeout = setTimeout(() => reject(new Error(`Local app startup timed out. ${stderr}`)), 20000);
      const failed = error => { clearTimeout(timeout); reject(error); };
      child.once('error', failed);
      child.once('exit', code => failed(new Error(`Local app exited (${code}). ${stderr}`)));
      child.stdout.on('data', chunk => {
        output += chunk.toString();
        const match = output.match(/http:\/\/127\.0\.0\.1:\d+\//);
        if (match) { clearTimeout(timeout); resolve(match[0]); }
      });
    });
    token = (await (await fetch(`${url}api/state`)).json()).csrf_token;
  }
  async function api(route, body, method = body === undefined ? 'GET' : 'POST') {
    const response = await fetch(`${url}${route.replace(/^\//, '')}`, {
      method,
      headers: { Origin: new URL(url).origin, 'Content-Type': 'application/json', 'X-Coach-Token': token },
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    });
    const contentType = response.headers.get('content-type') || '';
    const result = contentType.includes('application/json') ? await response.json() : Buffer.from(await response.arrayBuffer());
    assert.equal(response.status, 200, `${method} ${route}: ${JSON.stringify(result)}`);
    return result;
  }
  async function stop() {
    if (!child || child.exitCode !== null || child.signalCode !== null) return;
    const exited = once(child, 'exit');
    await api('/api/shutdown', {});
    let timeout;
    try {
      await Promise.race([exited, new Promise((_, reject) => { timeout = setTimeout(() => reject(new Error('Local app did not shut down.')), 10000); })]);
    } finally { clearTimeout(timeout); }
  }
  try { await start(); }
  catch (error) {
    child?.kill();
    await new Promise(resolve => provider.close(resolve));
    throw error;
  }
  return {
    temporary, dataDir, evidence, endpoint, calls, providerErrors, api,
    get url() { return url; },
    async restart() { await stop(); await start(); },
    async close() {
      try { await stop(); } catch { child?.kill(); }
      await new Promise(resolve => provider.close(resolve));
    },
  };
}
