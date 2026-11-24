'use strict';
const $ = id => document.getElementById(id);
const app = { state: null, page: 'study', answer: null, quiz: null, busy: false, closing: false, closed: false };
const pages = ['study', 'quiz', 'materials', 'notes', 'history', 'settings'];
const labels = { study: 'Your study workspace', quiz: 'Your practice test', materials: 'Your study materials', notes: 'Your source library', history: 'Your learning history', settings: 'Your model connections' };
const byteLength = text => new TextEncoder().encode(text).length;

function element(tag, text, className) {
  const node = document.createElement(tag);
  if (text !== undefined && text !== null) node.textContent = String(text);
  if (className) node.className = className;
  return node;
}
function message(id, text, error = false) {
  const node = $(id);
  node.textContent = text;
  node.className = error ? 'notice error' : 'success-message';
  node.hidden = !text;
}
async function api(path, options = {}) {
  const headers = { 'Content-Type': 'application/json' };
  if (options.method && options.method !== 'GET') headers['X-Coach-Token'] = app.state?.csrf_token || '';
  let response;
  try { response = await fetch(path, { ...options, headers, cache: 'no-store', credentials: 'same-origin' }); }
  catch { throw new Error('The local app is not responding. Keep its launcher open, then refresh this page.'); }
  let data;
  try { data = await response.json(); }
  catch { throw new Error('The local app returned an unreadable response. Restart the app and try again.'); }
  if (!response.ok) {
    let detail = data.error || `The request could not be completed (${response.status}).`;
    if (/HTTP 429/.test(detail)) detail += ' Wait for your quota to reset, shorten your notes, or choose another provider with a key in Connections.';
    else if (/HTTP (401|403)/.test(detail)) detail += ' Check your key and model access in your provider console, then save a replacement key in Connections.';
    else if (/timed out/i.test(detail)) detail += ' Try again when your connection is stable, or choose another connected model.';
    throw new Error(detail);
  }
  return data;
}
function profileId(profile) { return profile.id || profile.name; }
function selectedProfile() { return app.state?.profiles.find(p => profileId(p) === $('profile').value); }
function providerLabel(profile) { return profile?.provider_label || ({ groq: 'Groq', nvidia: 'NVIDIA' })[profile?.provider] || 'Custom'; }
function hasKey(profile) { return Boolean(profile && (profile.key_configured || profile.requires_key === false || app.state?.keys[profile.provider])); }
function setConnection() {
  const profile = selectedProfile();
  $('connection-status').textContent = hasKey(profile) ? `${providerLabel(profile)} ${profile.requires_key === false ? 'configured' : 'key ready'}` : 'Connection needed';
  $('setup-notice').hidden = hasKey(profile);
}
function renderSources() {
  const sources = app.state.sources;
  $('note-count').textContent = sources.length;
  $('source-total').textContent = `${sources.length} source${sources.length === 1 ? '' : 's'}`;
  $('context-sources').replaceChildren();
  $('note-list').replaceChildren();
  if (!sources.length) {
    $('context-sources').append(element('p', 'Add your first source in My notes, then ask a question.', 'empty-list'));
    $('note-list').append(element('p', 'No sources yet. Paste a passage or import your first text file.', 'empty-list'));
  }
  for (const source of sources) {
    const item = element('details', null, 'source-item');
    const summary = element('summary');
    summary.append(element('span', source.id, 'source-id'), element('span', source.title));
    item.append(summary, element('p', source.text));
    $('context-sources').append(item);
    const detail = element('details', null, 'note-detail');
    const heading = element('summary');
    heading.append(element('span', source.id, 'source-id'), document.createTextNode(source.title));
    const remove = element('button', 'Remove source', 'text-button danger-button');
    remove.type = 'button';
    remove.addEventListener('click', async () => {
      if (!window.confirm(`Remove “${source.title}” from this computer? Saved past answers will remain.`)) return;
      remove.disabled = true;
      try {
        const data = await api(`/api/sources/${encodeURIComponent(source.id)}`, { method: 'DELETE' });
        app.state.sources = data.sources;
        renderSources();
      } catch (error) { message('note-message', error.message, true); remove.disabled = false; }
    });
    detail.append(heading, element('pre', source.text), remove);
    $('note-list').append(detail);
  }
  renderSourcePickers();
}

const sourcePickerState = new Map();

function renderSourcePickers() {
  if (!app.state) return;
  for (const prefix of ['quiz', 'material']) {
    const container = $(`${prefix}-source-picker`);
    if (!container) continue;
    let selection = sourcePickerState.get(prefix);
    if (!selection) {
      selection = { all: true, ids: new Set(app.state.sources.map(source => source.id)) };
      sourcePickerState.set(prefix, selection);
    }
    const available = new Set(app.state.sources.map(source => source.id));
    if (selection.all) selection.ids = new Set(available);
    selection.ids = new Set([...selection.ids].filter(id => available.has(id)));
    container.replaceChildren(element('legend', prefix === 'quiz' ? 'Sources for this test' : 'Sources for these materials'));
    const allLabel = element('label', null, 'source-choice source-toggle');
    const all = element('input');
    all.type = 'checkbox'; all.id = `${prefix}-use-all-sources`; all.checked = selection.all;
    allLabel.append(all, document.createTextNode(`Use all ${app.state.sources.length} saved sources`));
    const options = element('div', null, 'source-options');
    options.id = `${prefix}-source-options`; options.hidden = selection.all;
    const actions = element('div', null, 'source-picker-actions');
    for (const [label, select] of [['Select all', true], ['Clear', false]]) {
      const button = element('button', label, 'text-button');
      button.type = 'button';
      button.addEventListener('click', () => {
        selection.all = false;
        selection.ids = new Set(select ? app.state.sources.map(source => source.id) : []);
        renderSourcePickers();
        $(`${prefix}-source-picker`).querySelectorAll('.source-picker-actions button')[select ? 0 : 1].focus();
      });
      actions.append(button);
    }
    const list = element('div', null, 'source-options-list');
    for (const [index, source] of app.state.sources.entries()) {
      const label = element('label', null, 'source-choice');
      const checkbox = element('input');
      checkbox.type = 'checkbox'; checkbox.id = `${prefix}-selected-source-${index}`;
      checkbox.value = source.id; checkbox.checked = selection.ids.has(source.id);
      checkbox.addEventListener('change', () => checkbox.checked ? selection.ids.add(source.id) : selection.ids.delete(source.id));
      label.append(checkbox, element('span', `${source.id} · ${source.title}`));
      list.append(label);
    }
    if (!app.state.sources.length) list.append(element('p', 'Add or import source notes first.', 'field-hint'));
    all.addEventListener('change', () => { selection.all = all.checked; options.hidden = selection.all; });
    options.append(actions, list, element('p', 'Only checked sources are sent to the model. Choose at least one.', 'field-hint'));
    container.append(allLabel, options);
  }
}

function sourceSelectionRequest(prefix) {
  const selection = sourcePickerState.get(prefix);
  if (!selection || selection.all) return {};
  const source_ids = [...selection.ids];
  if (!source_ids.length) throw new Error('Choose at least one source, or turn on Use all saved sources.');
  return { source_ids };
}
function renderState(state) {
  app.state = state;
  const selection = state.selected_profile || 'groq-120b';
  $('profile').replaceChildren();
  for (const profile of state.profiles) {
    const option = element('option', `${providerLabel(profile)} · ${profile.model}`);
    option.value = profileId(profile);
    $('profile').append(option);
  }
  $('profile').value = selection;
  $('groq-key-state').textContent = state.keys.groq ? 'Saved' : 'Not set';
  $('nvidia-key-state').textContent = state.keys.nvidia ? 'Saved' : 'Not set';
  $('key-location').textContent = `Saved in ${state.data_dir} as .env. Access is limited to your local account where supported.`;
  $('version').textContent = state.version ? `Version ${state.version}` : '';
  renderSources();
  setConnection();
  renderConnections();
}
async function navigate(page, focus = true) {
  if (!pages.includes(page)) page = 'study';
  app.page = page;
  for (const name of pages) $(`page-${name}`).hidden = name !== page;
  for (const button of document.querySelectorAll('.nav-item')) {
    const active = button.dataset.page === page;
    button.classList.toggle('active', active);
    if (active) button.setAttribute('aria-current', 'page'); else button.removeAttribute('aria-current');
  }
  $('page-label').textContent = labels[page];
  document.body.classList.toggle('exam-mode', page === 'quiz' && Boolean(app.quiz));
  history.replaceState(null, '', `#${page}`);
  if (focus) $('main').focus({ preventScroll: true });
  if (page === 'history') await loadHistory();
  if (page === 'materials') await window.loadMaterials?.();
  if (page === 'quiz') await loadQuizProgress();
}

function renderConnections() {
  const list = $('connection-list');
  list.replaceChildren();
  for (const profile of app.state.profiles) {
    const id = profileId(profile);
    const active = id === app.state.selected_profile;
    const row = element('article', null, `connection-row${active ? ' selected' : ''}`);
    const mark = element('span', providerLabel(profile).slice(0, 1), 'provider-mark');
    mark.setAttribute('aria-hidden', 'true');
    const details = element('div', null, 'connection-details');
    details.append(element('h3', profile.provider === 'openai-compatible' ? profile.name || profile.model : profile.model));
    details.append(element('p', `${providerLabel(profile)} · ${profile.model}`));
    if (profile.endpoint) details.append(element('small', profile.endpoint));
    const state = element('span', active ? 'Selected' : hasKey(profile) ? 'Configured' : 'Key needed', `connection-state${hasKey(profile) ? ' ready' : ''}`);
    const actions = element('div', null, 'connection-row-actions');
    const use = element('button', active ? 'Selected' : 'Use model', 'text-button');
    use.type = 'button';
    use.disabled = active;
    use.setAttribute('aria-label', active ? `${profile.model} selected` : `Use ${profile.model}`);
    use.addEventListener('click', async () => {
      use.disabled = true;
      message('connection-action-message', '');
      try {
        renderState(await api('/api/settings', { method: 'POST', body: JSON.stringify({ profile: id }) }));
        message('connection-action-message', 'Study model selected.');
      } catch (error) { message('connection-action-message', error.message, true); use.disabled = false; }
    });
    actions.append(use);
    if (id.startsWith('custom-')) {
      const remove = element('button', 'Remove', 'text-button danger-button');
      remove.type = 'button';
      remove.setAttribute('aria-label', `Remove ${profile.name || profile.model}`);
      remove.addEventListener('click', async () => {
        if (!window.confirm(`Remove “${profile.name || profile.model}” and its saved key from this computer?`)) return;
        remove.disabled = true;
        try {
          renderState(await api(`/api/connections/${encodeURIComponent(id)}`, { method: 'DELETE' }));
          message('connection-action-message', 'Connection removed.');
        } catch (error) { message('connection-action-message', error.message, true); remove.disabled = false; }
      });
      actions.append(remove);
    }
    row.append(mark, details, state, actions);
    list.append(row);
  }
  const previousProvider = $('connection-provider').value;
  $('connection-provider').replaceChildren();
  $('provider-list').replaceChildren();
  for (const provider of app.state.providers || []) {
    const option = element('option', provider.label);
    option.value = provider.id;
    $('connection-provider').append(option);
    const button = element('button', null, 'provider-option');
    button.type = 'button';
    button.append(element('strong', provider.label), element('span', provider.requires_key === false ? 'Run on this computer' : provider.id === 'custom' ? 'Your compatible endpoint' : 'Use your provider account'));
    button.addEventListener('click', () => {
      $('connection-provider').value = provider.id;
      applyProviderPreset();
      showConnectionForm(true);
    });
    $('provider-list').append(button);
  }
  if ([...$('connection-provider').options].some(option => option.value === previousProvider)) $('connection-provider').value = previousProvider;
}

function applyProviderPreset() {
  const provider = app.state.providers?.find(item => item.id === $('connection-provider').value);
  if (!provider) return;
  $('connection-endpoint').value = provider.endpoint || '';
  $('connection-key-hint').textContent = provider.requires_key === false ? 'Optional for local models' : 'Required for remote models';
  $('connection-name').placeholder = provider.id === 'custom' ? 'My study model' : `${provider.label} study model`;
}

function showConnectionForm(show) {
  $('add-connection-panel').hidden = !show;
  $('show-add-connection').setAttribute('aria-expanded', String(show));
  if (show) {
    $('add-connection-panel').scrollIntoView({ block: 'start' });
    $('connection-name').focus({ preventScroll: true });
  } else $('show-add-connection').focus({ preventScroll: true });
}

function quizIntent(question) {
  return /^\s*quiz\b|\b(?:quiz me|test me)\b|\b(?:make|create|give|start|generate|want|take)\b.{0,80}\b(?:quiz|practice test|practice questions)\b/i.test(question);
}

function setQuizBusy(busy) {
  app.busy = busy;
  $('start-quiz').disabled = busy;
  $('check-quiz-answer').disabled = busy;
  $('see-quiz-solution').disabled = busy;
  $('next-quiz-question').disabled = busy;
  $('new-quiz').disabled = busy;
  $('quiz-answer-form').setAttribute('aria-busy', String(busy));
}

async function startQuiz(topic) {
  if (app.busy || !app.state) return;
  navigate('quiz');
  $('quiz-topic').value = topic;
  message('quiz-error', '');
  if (!topic.trim()) { message('quiz-error', 'Choose a topic for your practice test.', true); $('quiz-topic').focus(); return; }
  if (byteLength(topic) > 4000) { message('quiz-error', 'Keep your topic within 4,000 bytes.', true); return; }
  if (!hasKey(selectedProfile())) { message('quiz-error', 'Choose a configured model in Connections before starting a test.', true); return; }
  if (!app.state.sources.length) { message('quiz-error', 'Add some notes before starting a test.', true); return; }
  let sourceScope;
  try { sourceScope = sourceSelectionRequest('quiz'); }
  catch (error) { message('quiz-error', error.message, true); $('quiz-use-all-sources').focus(); return; }
  const count = Number($('quiz-count').value);
  if (!Number.isInteger(count) || count < 1 || count > 30) { message('quiz-error', 'Choose between 1 and 30 questions.', true); $('quiz-count').focus(); return; }
  const type = $('quiz-type').value;
  const instructions = $('quiz-instructions').value.trim();
  if (type === 'custom' && !instructions) { message('quiz-error', 'Describe your custom question type first.', true); $('quiz-instructions').focus(); return; }
  setQuizBusy(true);
  $('quiz-loading').hidden = false;
  $('quiz-summary').hidden = true;
  try {
    const session = await api('/api/quiz', { method: 'POST', body: JSON.stringify({ topic: topic.trim(), profile: $('profile').value, count, type, instructions, ...sourceScope }) });
    if (!session.id || !Array.isArray(session.questions) || !session.questions.length || session.questions.some(question => !question.id || typeof question.question !== 'string')) throw new Error('The model did not return a usable test. Try another topic.');
    app.quiz = { session, index: 0, results: [] };
    $('quiz-setup').hidden = true;
    $('quiz-model').textContent = `${session.model || selectedProfile()?.model || ''} · Feedback checks your answer against the notes. Review the source passages when in doubt.`;
    document.body.classList.add('exam-mode');
    renderQuizQuestion();
  } catch (error) { message('quiz-error', error.message, true); }
  finally { $('quiz-loading').hidden = true; setQuizBusy(false); }
}

function renderQuizQuestion() {
  const { session, index } = app.quiz;
  const question = session.questions[index];
  $('quiz-exam').hidden = false;
  $('quiz-exam').dataset.status = 'unanswered';
  $('quiz-topic-label').textContent = session.topic;
  $('quiz-progress-label').textContent = `${index + 1} / ${session.questions.length}`;
  $('quiz-question-number').textContent = `Question ${index + 1}`;
  $('quiz-question').textContent = question.question;
  const progress = document.querySelector('.exam-progress');
  progress.setAttribute('aria-valuemax', String(session.questions.length));
  progress.setAttribute('aria-valuenow', String(index));
  $('quiz-progress').style.width = `${index / session.questions.length * 100}%`;
  $('quiz-answer-form').reset();
  $('quiz-choices').replaceChildren(element('legend', 'Choose your answer'));
  const multipleChoice = question.type === 'multiple-choice' && Array.isArray(question.options);
  $('quiz-choices').hidden = !multipleChoice;
  $('quiz-answer').hidden = multipleChoice;
  $('quiz-answer-label').hidden = multipleChoice;
  $('quiz-answer-help').textContent = multipleChoice ? 'Choose one option, then check your answer.' : question.type === 'fill-in-the-blank' ? 'Type the missing word or phrase.' : 'Take your time. You can retry a close answer before revealing the solution.';
  if (multipleChoice) question.options.forEach((option, index) => {
    const label = element('label', null, 'quiz-choice');
    const radio = document.createElement('input');
    radio.type = 'radio';
    radio.name = 'quiz-option';
    radio.value = option;
    label.append(radio, element('span', String.fromCharCode(65 + index), 'choice-letter'), element('span', option));
    $('quiz-choices').append(label);
  });
  $('quiz-answer').disabled = false;
  $('quiz-feedback').hidden = true;
  $('quiz-solution').hidden = true;
  $('quiz-solution-text').textContent = '';
  $('quiz-citations').replaceChildren();
  $('check-quiz-answer').hidden = false;
  $('see-quiz-solution').hidden = false;
  $('next-quiz-question').hidden = true;
  $('quiz-question').focus({ preventScroll: true });
  $('quiz-exam').scrollIntoView({ block: 'start' });
}

function renderQuizFeedback(result) {
  if (!['correct', 'close', 'incorrect'].includes(result.status) || typeof result.feedback !== 'string') throw new Error('The model returned unreadable feedback. Your answer has not been marked. Try again.');
  if (result.status === 'incorrect' && (typeof result.solution !== 'string' || !result.solution.trim())) throw new Error('The solution could not be loaded. Your answer has not been marked. Try again.');
  const close = result.status === 'close';
  $('quiz-exam').dataset.status = result.status;
  $('quiz-feedback').dataset.status = result.status;
  $('quiz-feedback-title').textContent = ({ correct: 'Correct', close: 'Close. Try again.', incorrect: 'Review this one' })[result.status];
  $('quiz-feedback-icon').textContent = ({ correct: '✓', close: '!', incorrect: '×' })[result.status];
  $('quiz-feedback-text').textContent = result.feedback;
  $('quiz-feedback').hidden = false;
  $('retry-quiz-answer').hidden = !close;
  if (!close) {
    app.quiz.results[app.quiz.index] = result.status;
    $('quiz-answer').disabled = true;
    $('quiz-choices').querySelectorAll('input').forEach(input => { input.disabled = true; });
    $('check-quiz-answer').hidden = true;
    $('see-quiz-solution').hidden = true;
    $('next-quiz-question').hidden = false;
    $('next-quiz-question').textContent = app.quiz.index + 1 === app.quiz.session.questions.length ? 'Finish test' : 'Next question ↗';
    loadQuizProgress();
  }
  $('quiz-solution').hidden = close || typeof result.solution !== 'string' || !result.solution.trim();
  if (!$('quiz-solution').hidden) {
    $('quiz-solution-text').textContent = result.solution;
    $('quiz-citations').replaceChildren();
    for (const citation of result.citations || []) {
      const figure = element('figure', null, 'citation');
      const source = app.state.sources.find(item => item.id === citation.source_id);
      figure.append(element('blockquote', citation.quote), element('figcaption', `${citation.source_id} · ${source?.title || 'Saved source'}`));
      $('quiz-citations').append(figure);
    }
  }
}

async function submitQuizAnswer(reveal = false) {
  if (app.busy || !app.quiz || app.quiz.results[app.quiz.index]) return;
  message('quiz-error', '');
  const answer = $('quiz-choices').hidden ? $('quiz-answer').value.trim() : $('quiz-choices').querySelector('input:checked')?.value || '';
  if (!reveal && !answer) { message('quiz-error', 'Write an answer first, or choose See solution.', true); $('quiz-answer').focus(); return; }
  if (byteLength(answer) > 4000) { message('quiz-error', 'Keep your answer within 4,000 bytes.', true); return; }
  setQuizBusy(true);
  $('quiz-checking').hidden = false;
  try {
    const result = await api(reveal ? '/api/quiz/solution' : '/api/quiz/grade', { method: 'POST', body: JSON.stringify({ quiz_id: app.quiz.session.id, question_id: app.quiz.session.questions[app.quiz.index].id, ...(reveal ? {} : { answer }) }) });
    renderQuizFeedback(result);
  } catch (error) { message('quiz-error', error.message, true); }
  finally { $('quiz-checking').hidden = true; setQuizBusy(false); }
}

function finishQuiz() {
  const { session, results } = app.quiz;
  const correct = results.filter(status => status === 'correct').length;
  $('quiz-exam').hidden = true;
  $('quiz-summary').hidden = false;
  $('quiz-summary-text').textContent = `${correct} of ${session.questions.length} correct. ${session.questions.length - correct} to revisit. Use your notes to work through anything that needs another look.`;
  $('quiz-summary-results').replaceChildren();
  session.questions.forEach((question, index) => {
    const row = element('div', null, `summary-result ${results[index]}`);
    row.append(element('span', results[index] === 'correct' ? '✓ Correct' : '× Review'), element('p', question.question));
    $('quiz-summary-results').append(row);
  });
  $('quiz-summary-title').setAttribute('tabindex', '-1');
  $('quiz-summary-title').focus({ preventScroll: true });
  $('quiz-summary').scrollIntoView({ block: 'start' });
}

async function loadQuizProgress() {
  if (!app.state) return;
  $('refresh-progress').disabled = true;
  try {
    const progress = await api('/api/progress');
    $('topic-progress').replaceChildren();
    if (!progress.topics?.length) $('topic-progress').append(element('p', 'Your first practice attempt will start this list. Try a topic above.', 'empty-list'));
    for (const topic of progress.topics || []) {
      const needsReview = topic.incorrect > 0 || topic.close > 0;
      const row = element('div', null, `topic-progress-row${needsReview ? ' review-next' : ''}`);
      const details = element('div');
      details.append(element('strong', topic.topic), element('p', `${topic.correct} correct · ${topic.incorrect} incorrect · ${topic.close} close · ${topic.attempts} attempts`));
      const button = element('button', needsReview ? 'Review next' : 'Answered well', 'text-button');
      button.type = 'button';
      button.addEventListener('click', () => {
        if (app.busy) return;
        app.quiz = null;
        $('quiz-exam').hidden = true;
        $('quiz-summary').hidden = true;
        $('quiz-setup').hidden = false;
        $('quiz-topic').value = topic.topic;
        document.body.classList.remove('exam-mode');
        $('quiz-topic').focus();
        $('quiz-setup').scrollIntoView({ block: 'start' });
      });
      row.append(details, button);
      $('topic-progress').append(row);
    }
    const recent = progress.recent || [];
    const seen = new Set();
    const latest = recent.filter(item => {
      const identity = item.question?.trim().toLocaleLowerCase();
      if (!identity || seen.has(identity)) return false;
      seen.add(identity);
      return true;
    });
    for (const [title, items] of [
      ['Questions to revisit', latest.filter(item => ['incorrect', 'close'].includes(item.status))],
      ['Questions answered correctly', latest.filter(item => item.status === 'correct')],
    ]) {
      if (!items.length) continue;
      const section = element('section', null, 'question-review');
      section.append(element('h3', title));
      for (const item of items.slice(0, 5)) {
        const row = element('div', null, 'topic-progress-row');
        const detail = element('div');
        detail.append(element('strong', item.question), element('p', item.topic));
        row.append(detail);
        if (item.status !== 'correct') {
          const retry = element('button', 'Practice this question', 'text-button');
          retry.type = 'button';
          retry.addEventListener('click', () => {
            if (app.busy) return;
            if (!$('quiz-exam').hidden && !window.confirm('Leave this test to practice a missed question?')) return;
            app.quiz = null;
            $('quiz-exam').hidden = true;
            $('quiz-summary').hidden = true;
            $('quiz-setup').hidden = false;
            document.body.classList.remove('exam-mode');
            $('quiz-count').value = '1';
            $('quiz-type').value = 'open-response';
            $('quiz-instructions').value = '';
            $('quiz-instructions-field').hidden = true;
            $('quiz-topic').value = item.question;
            $('start-quiz').focus();
            $('quiz-setup').scrollIntoView({ block: 'start' });
          });
          row.append(retry);
        }
        section.append(row);
      }
      $('topic-progress').append(section);
    }
    message('quiz-progress-message', '');
  } catch (error) { message('quiz-progress-message', error.message, true); }
  finally { $('refresh-progress').disabled = false; }
}
function showAnswer(result) {
  const output = result.output;
  if (!output || typeof output.answer !== 'string') throw new Error('No usable answer was returned. Try a shorter question.');
  app.answer = result;
  $('answer-empty').hidden = true;
  $('answer-card').hidden = false;
  $('answer-status').textContent = ({ answer: 'Source checked', clarify: 'A little more detail', abstain: 'Not in these notes' })[output.status] || output.status;
  $('answer-heading').textContent = output.status === 'answer' ? 'Your answer' : output.status === 'clarify' ? 'Let’s narrow it down' : 'Here’s the limit';
  $('answer-text').textContent = output.answer;
  $('citations').replaceChildren();
  if (output.citations?.length) {
    $('citations').append(element('h3', 'Check the evidence'));
    for (const citation of output.citations) {
      const sourceId = citation.source_id || citation.id;
      const source = app.state.sources.find(s => s.id === sourceId);
      const figure = element('figure', null, 'citation');
      figure.append(element('blockquote', citation.quote), element('figcaption', `${sourceId} · ${source?.title || 'Saved source'}`));
      $('citations').append(figure);
    }
  }
  const latency = Number.isFinite(result.latency_ms) ? `${(result.latency_ms / 1000).toFixed(2)} s` : '';
  $('answer-meta').textContent = [result.model, latency].filter(Boolean).join(' · ');
}
async function loadHistory() {
  const list = $('history-list');
  list.replaceChildren(element('p', 'Loading saved questions…', 'muted'));
  try {
    const { entries } = await api('/api/history');
    list.replaceChildren();
    if (!entries.length) list.append(element('p', 'Nothing saved yet. Ask your first question in Study; its response will be kept here.', 'empty-list'));
    for (const entry of [...entries].reverse()) {
      const button = element('button', null, 'history-item');
      button.type = 'button';
      const date = entry.created_at_unix ? new Date(entry.created_at_unix * 1000).toLocaleString() : 'Saved locally';
      button.append(element('strong', entry.question), element('p', entry.output.answer.slice(0, 200)), element('small', `${date} · ${entry.model}`));
      button.addEventListener('click', () => {
        $('question').value = entry.question;
        message('answer-error', '');
        showAnswer(entry);
        navigate('study');
        $('answer-card').scrollIntoView({ block: 'start' });
      });
      list.append(button);
    }
  } catch (error) { list.replaceChildren(element('p', error.message, 'notice error')); }
}
document.querySelectorAll('[data-page]').forEach(button => button.addEventListener('click', () => navigate(button.dataset.page)));
document.querySelectorAll('[data-question]').forEach(button => button.addEventListener('click', () => {
  $('question').value = button.dataset.question;
  $('question').focus();
}));
$('profile').addEventListener('change', setConnection);
$('quick-quiz').addEventListener('click', async () => {
  await navigate('quiz');
  if (!app.quiz) $('quiz-topic').focus();
});
$('quiz-setup').addEventListener('submit', event => { event.preventDefault(); startQuiz($('quiz-topic').value); });
$('quiz-type').addEventListener('change', () => { $('quiz-instructions-field').hidden = $('quiz-type').value !== 'custom'; });
$('refresh-progress').addEventListener('click', loadQuizProgress);
$('quiz-answer-form').addEventListener('submit', event => { event.preventDefault(); submitQuizAnswer(); });
$('see-quiz-solution').addEventListener('click', () => submitQuizAnswer(true));
$('quiz-answer').addEventListener('keydown', event => {
  if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') { event.preventDefault(); $('quiz-answer-form').requestSubmit(); }
});
$('retry-quiz-answer').addEventListener('click', () => {
  $('quiz-feedback').hidden = true;
  $('quiz-exam').dataset.status = 'unanswered';
  if ($('quiz-choices').hidden) $('quiz-answer').focus();
  else $('quiz-choices').querySelector('input')?.focus();
});
$('next-quiz-question').addEventListener('click', () => {
  if (app.busy || !app.quiz) return;
  if (app.quiz.index + 1 === app.quiz.session.questions.length) finishQuiz();
  else { app.quiz.index++; message('quiz-error', ''); renderQuizQuestion(); }
});
$('restart-quiz').addEventListener('click', () => {
  app.quiz = null;
  $('quiz-summary').hidden = true;
  $('quiz-setup').hidden = false;
  $('quiz-model').textContent = '';
  document.body.classList.remove('exam-mode');
  $('quiz-topic').focus();
});
$('new-quiz').addEventListener('click', () => {
  if (app.busy || !window.confirm('Start a new test? This test will close. Completed attempts stay in your review list.')) return;
  app.quiz = null;
  $('quiz-exam').hidden = true;
  $('quiz-summary').hidden = true;
  $('quiz-setup').hidden = false;
  $('quiz-model').textContent = '';
  message('quiz-error', '');
  document.body.classList.remove('exam-mode');
  $('quiz-topic').focus();
});
$('show-add-connection').addEventListener('click', () => { applyProviderPreset(); showConnectionForm(true); });
$('cancel-add-connection').addEventListener('click', () => {
  $('add-connection-form').reset();
  message('add-connection-message', '');
  showConnectionForm(false);
});
$('connection-provider').addEventListener('change', applyProviderPreset);
$('add-connection-form').addEventListener('submit', async event => {
  event.preventDefault();
  const payload = {
    provider: $('connection-provider').value,
    name: $('connection-name').value.trim(),
    model: $('connection-model').value.trim(),
    endpoint: $('connection-endpoint').value.trim(),
    api_key: $('connection-key').value.trim(),
  };
  if (!payload.name || !payload.model || !payload.endpoint) { message('add-connection-message', 'Add a connection name, model ID, and chat completions URL.', true); return; }
  $('add-connection').disabled = true;
  message('add-connection-message', '');
  try {
    const state = await api('/api/connections', { method: 'POST', body: JSON.stringify(payload) });
    $('add-connection-form').reset();
    renderState(state);
    showConnectionForm(false);
    message('connection-action-message', 'Connection saved. Choose Use model to study with it. Provider access is checked when you ask.');
  } catch (error) { message('add-connection-message', error.message, true); }
  finally { $('add-connection').disabled = false; }
});
$('question-form').addEventListener('submit', async event => {
  event.preventDefault();
  if (app.busy || !app.state) return;
  const question = $('question').value.trim();
  if (!question) { message('answer-error', 'Write a question first, or choose one of the suggestions.', true); $('question').focus(); return; }
  if (byteLength(question) > 4000) { message('answer-error', 'Keep your question within 4,000 bytes. Try asking about one idea at a time.', true); return; }
  if (quizIntent(question)) { await startQuiz(question); return; }
  if (!hasKey(selectedProfile())) { message('answer-error', 'Add a key for your selected model in Connections, then try again.', true); return; }
  app.busy = true;
  $('ask-button').disabled = true;
  $('ask-button').textContent = 'Reading your notes…';
  $('question-form').setAttribute('aria-busy', 'true');
  $('answer-loading').hidden = false;
  $('answer-card').hidden = true;
  $('answer-empty').hidden = true;
  message('answer-error', '');
  try {
    const result = await api('/api/answer', { method: 'POST', body: JSON.stringify({ question, profile: $('profile').value }) });
    showAnswer(result);
  } catch (error) { message('answer-error', error.message, true); }
  finally {
    app.busy = false;
    $('ask-button').disabled = false;
    $('ask-button').textContent = 'Ask Study Coach ↗';
    $('question-form').setAttribute('aria-busy', 'false');
    $('answer-loading').hidden = true;
  }
});
$('question').addEventListener('keydown', event => {
  if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') { event.preventDefault(); $('question-form').requestSubmit(); }
});
$('settings-form').addEventListener('submit', async event => {
  event.preventDefault();
  const payload = { profile: $('profile').value };
  if ($('groq-key').value.trim()) payload.groq_api_key = $('groq-key').value.trim();
  if ($('nvidia-key').value.trim()) payload.nvidia_api_key = $('nvidia-key').value.trim();
  $('save-settings').disabled = true;
  message('settings-message', '');
  try {
    const state = await api('/api/settings', { method: 'POST', body: JSON.stringify(payload) });
    $('groq-key').value = '';
    $('nvidia-key').value = '';
    renderState(state);
    message('settings-message', 'Connection saved locally. Try a question in Study. Provider access is checked when you ask.');
  } catch (error) { message('settings-message', error.message, true); }
  finally { $('save-settings').disabled = false; }
});
$('note-form').addEventListener('submit', async event => {
  event.preventDefault();
  const title = $('note-title').value.trim();
  const text = $('note-text').value.trim();
  if (!title || !text) { message('note-message', 'Add both a title and some notes.', true); return; }
  if (byteLength(title) > 160 || byteLength(text) > 184320) { message('note-message', 'Use a title under 160 bytes and notes under 180 KiB. Split larger documents before importing.', true); return; }
  $('save-note').disabled = true;
  try {
    const data = await api(app.importedNote || byteLength(text) > 16384 ? '/api/sources/import' : '/api/sources', { method: 'POST', body: JSON.stringify({ title, text }) });
    app.state.sources = data.sources;
    renderSources();
    $('note-form').reset();
    app.importedNote = false;
    app.pendingImportFile = null;
    $('video-import-fields').hidden = true;
    message('note-message', 'Source saved. It will be included with your next question.');
  } catch (error) { message('note-message', error.message, true); }
  finally { $('save-note').disabled = false; }
});
async function importStudyFile(file) {
  if (!file) return;
  if (!/\.(txt|md|docx|pdf|pptx|xlsx|csv|tsv|srt|vtt|png|jpe?g|bmp|mp4|webm|mov|mkv)$/i.test(file.name) || file.size > 12 * 1024 * 1024) { message('note-message', 'Choose a supported document, image, or video up to 12 MiB.', true); $('note-file').value = ''; return; }
  const video = /\.(mp4|webm|mov|mkv)$/i.test(file.name);
  const transcript = video ? $('video-transcript').value.trim() : '';
  if (byteLength(transcript) > 184320) { message('note-message', 'Keep the supplied transcript within 180 KiB.', true); return; }
  $('save-note').disabled = true;
  $('extract-video').disabled = true;
  message('note-message', 'Extracting text locally. Large documents and screenshots can take a moment…');
  try {
    const bytes = new Uint8Array(await file.arrayBuffer());
    let binary = '';
    for (let offset = 0; offset < bytes.length; offset += 32768) binary += String.fromCharCode(...bytes.subarray(offset, offset + 32768));
    const imported = await api('/api/import', { method: 'POST', body: JSON.stringify({ name: file.name, content_base64: btoa(binary), ...(transcript ? { transcript } : {}) }) });
    if (typeof imported.text !== 'string' || !imported.text.trim()) throw new Error('No readable text was found. Try a clearer image or paste the text.');
    $('note-title').value = (imported.title || file.name.replace(/\.[^.]+$/, '')).slice(0, 160);
    $('note-text').value = imported.text;
    app.importedNote = true;
    message('note-message', `File loaded for review${imported.method ? ` using ${imported.method}` : ''}. Check the text, then choose Save source.`);
  } catch (error) { message('note-message', error.message || 'Could not read this file.', true); }
  finally { $('save-note').disabled = false; $('extract-video').disabled = false; }
}
$('note-file').addEventListener('change', event => {
  const file = event.target.files[0];
  if (!file) return;
  app.pendingImportFile = file;
  const video = /\.(mp4|webm|mov|mkv)$/i.test(file.name);
  $('video-import-fields').hidden = !video;
  if (video) {
    message('note-message', 'Video selected. Paste its transcript below, or choose Load video text to extract available embedded captions.');
    $('video-transcript').focus();
  } else importStudyFile(file);
});
$('extract-video').addEventListener('click', () => importStudyFile(app.pendingImportFile));
$('copy-answer').addEventListener('click', async () => {
  if (!app.answer) return;
  const output = app.answer.output;
  const evidence = (output.citations || []).map(c => `[${c.source_id || c.id}] ${c.quote}`).join('\n');
  try {
    await navigator.clipboard.writeText([output.answer, evidence].filter(Boolean).join('\n\n'));
    $('copy-answer').textContent = 'Copied';
    setTimeout(() => { $('copy-answer').textContent = 'Copy answer'; }, 2000);
  } catch { $('copy-answer').textContent = 'Select answer text to copy'; }
});
async function closeApp() {
  if (app.closing || app.closed) return;
  const hasUnsavedNotes = $('note-title').value.trim() || $('note-text').value.trim();
  const hasUnsavedConnections = $('groq-key').value.trim() || $('nvidia-key').value.trim()
    || $('connection-name').value.trim() || $('connection-model').value.trim() || $('connection-key').value.trim()
    || (app.state && $('profile').value !== app.state.selected_profile);
  if ((hasUnsavedNotes || hasUnsavedConnections) && !window.confirm('Close Study Coach? Unsaved notes or connection edits will be lost. Saved data stays on this computer.')) return;
  app.closing = true;
  $('close-app').disabled = true;
  $('retry-close').hidden = true;
  $('return-to-app').hidden = true;
  $('close-tab').hidden = true;
  $('shutdown-title').textContent = 'Closing Study Coach…';
  $('shutdown-message').textContent = app.busy
    ? 'The current answer will finish before the local app stops. Keep this tab open until closing is confirmed.'
    : 'Stopping the local app. Your saved data stays on this computer.';
  document.querySelector('.app-shell').hidden = true;
  $('shutdown-screen').hidden = false;
  $('shutdown-screen').focus();
  app.refreshMotion?.();
  try {
    const result = await api('/api/shutdown', { method: 'POST', body: '{}' });
    if (result.shutdown !== true) throw new Error('The local app did not confirm shutdown.');
    app.closed = true;
    app.closing = false;
    $('shutdown-title').textContent = 'Study Coach is closed.';
    $('shutdown-message').textContent = 'The local service has stopped. You can close this tab and reopen Study Coach from its launcher.';
    $('close-tab').hidden = false;
    app.refreshMotion?.();
    window.close();
  } catch (error) {
    app.closing = false;
    $('close-app').disabled = false;
    $('shutdown-title').textContent = 'Closing could not be confirmed.';
    $('shutdown-message').textContent = `${error.message} Try again, or stop the launcher terminal with Ctrl+C.`;
    $('retry-close').hidden = false;
    $('return-to-app').hidden = false;
    app.refreshMotion?.();
  }
}
$('close-app').addEventListener('click', closeApp);
$('retry-close').addEventListener('click', closeApp);
$('return-to-app').addEventListener('click', () => {
  if (app.closed || app.closing) return;
  $('shutdown-screen').hidden = true;
  document.querySelector('.app-shell').hidden = false;
  $('close-app').focus();
});
$('close-tab').addEventListener('click', () => {
  window.close();
  $('shutdown-help').textContent = 'If your browser keeps this tab open, press Ctrl+W to close it. Press F11 to show hidden browser controls. Reopen Study Coach from its shortcut or StartStudyCoach.cmd.';
});
window.addEventListener('hashchange', () => { if (!app.closing && !app.closed) navigate(location.hash.slice(1), false); });

// The background projects a small 3D point field onto a high density canvas.
// No library, remote asset, tracking, or network request is used for the effect.
function initializeMotion() {
  const canvas = $('depth-field');
  const context = canvas.getContext('2d');
  const toggle = $('motion-toggle');
  const preference = matchMedia('(prefers-reduced-motion: reduce)');
  let userReduced = false;
  try { userReduced = localStorage.getItem('study-coach-reduced-motion') === 'true'; } catch { /* Storage can be disabled by the browser. */ }
  let width = innerWidth;
  let height = innerHeight;
  const points = Array.from({ length: 95 }, (_, index) => {
    const seed = (index * 137.508) * Math.PI / 180;
    return { x: Math.cos(seed) * (200 + index * 7), y: Math.sin(seed * 1.71) * 600, z: Math.sin(seed) * 650 };
  });
  const reduced = () => userReduced || preference.matches || app.closing || app.closed;
  // Keep the full-resolution constellation in one reusable surface. CSS
  // moves that surface without clearing millions of pixels on each frame.
  function render() {
    if (!context) return;
    context.clearRect(0, 0, width, height);
    const angle = .35;
    const cosine = Math.cos(angle);
    const sine = Math.sin(angle);
    const projected = points.map(point => {
      const x = point.x * cosine - point.z * sine;
      const z = point.z * cosine + point.x * sine;
      const depth = 1100 / (1500 + z);
      return { x: width * .62 + x * depth, y: height * .48 + point.y * depth, depth };
    });
    for (let index = 0; index < projected.length; index++) {
      const point = projected[index];
      context.beginPath();
      context.fillStyle = `rgba(146,228,212,${Math.min(.4, point.depth * .2)})`;
      context.arc(point.x, point.y, Math.max(.6, point.depth * 1.4), 0, Math.PI * 2);
      context.fill();
      for (let next = index + 1; next < Math.min(index + 6, projected.length); next++) {
        const neighbor = projected[next];
        const distance = Math.hypot(point.x - neighbor.x, point.y - neighbor.y);
        if (distance > 145) continue;
        context.beginPath();
        context.strokeStyle = `rgba(146,228,212,${(1 - distance / 145) * .12})`;
        context.lineWidth = .7;
        context.moveTo(point.x, point.y);
        context.lineTo(neighbor.x, neighbor.y);
        context.stroke();
      }
    }
  }
  function refresh() {
    const isReduced = reduced();
    document.body.classList.toggle('motion-off', isReduced);
    toggle.textContent = preference.matches ? 'Reduced motion' : isReduced ? 'Motion off' : 'Motion on';
    toggle.setAttribute('aria-pressed', String(isReduced));
    toggle.setAttribute('aria-label', preference.matches ? 'Reduced motion follows your device setting' : isReduced ? 'Turn motion on' : 'Turn motion off');
    toggle.disabled = preference.matches;
  }
  app.refreshMotion = refresh;
  function resize() {
    width = innerWidth;
    height = innerHeight;
    const density = Math.min(devicePixelRatio || 1, 3);
    canvas.width = Math.round(width * density);
    canvas.height = Math.round(height * density);
    if (context) context.setTransform(density, 0, 0, density, 0, 0);
    render();
    refresh();
  }
  toggle.addEventListener('click', () => {
    userReduced = !userReduced;
    try { localStorage.setItem('study-coach-reduced-motion', String(userReduced)); } catch { /* The choice still applies to this session. */ }
    refresh();
  });
  preference.addEventListener('change', refresh);
  document.addEventListener('visibilitychange', refresh);
  window.addEventListener('resize', resize, { passive: true });
  window.addEventListener('pointermove', event => {
    if (reduced() || event.pointerType === 'touch') return;
    const pointerX = (event.clientX / width - .5) * 2;
    const pointerY = (event.clientY / height - .5) * 2;
    const orbit = document.querySelector('.study-orbit');
    orbit.style.setProperty('--tilt-x', `${-pointerY * 9}deg`);
    orbit.style.setProperty('--tilt-y', `${pointerX * 12}deg`);
  }, { passive: true });
  resize();
}
initializeMotion();

(async () => {
  try { renderState(await api('/api/state')); await navigate(location.hash.slice(1) || 'study', false); }
  catch (error) { $('connection-status').textContent = 'App unavailable'; message('global-error', error.message, true); }
})();
