'use strict';
const $ = id => document.getElementById(id);
const app = { state: null, page: 'study', answer: null, busy: false, closing: false, closed: false };
const pages = ['study', 'notes', 'history', 'settings'];
const labels = { study: 'Your study workspace', notes: 'Your source library', history: 'Your learning history', settings: 'Your model connections' };
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
function hasKey(profile) { return Boolean(profile && app.state?.keys[profile.provider]); }
function setConnection() {
  const profile = selectedProfile();
  $('connection-status').textContent = hasKey(profile) ? `${profile.provider === 'groq' ? 'Groq' : 'NVIDIA'} key ready` : 'Connection needed';
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
}
function renderState(state) {
  app.state = state;
  const selection = state.selected_profile || 'groq-120b';
  $('profile').replaceChildren();
  for (const profile of state.profiles) {
    const option = element('option', `${profile.provider === 'groq' ? 'Groq' : 'NVIDIA'} · ${profile.model}`);
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
  history.replaceState(null, '', `#${page}`);
  if (focus) $('main').focus({ preventScroll: true });
  if (page === 'history') await loadHistory();
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
$('question-form').addEventListener('submit', async event => {
  event.preventDefault();
  if (app.busy || !app.state) return;
  const question = $('question').value.trim();
  if (!question) { message('answer-error', 'Write a question first, or choose one of the suggestions.', true); $('question').focus(); return; }
  if (byteLength(question) > 4000) { message('answer-error', 'Keep your question within 4,000 bytes. Try asking about one idea at a time.', true); return; }
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
  if (byteLength(title) > 160 || byteLength(text) > 16384) { message('note-message', 'Use a title under 160 bytes and notes under 16 KiB. Split long sources into smaller passages.', true); return; }
  $('save-note').disabled = true;
  try {
    const data = await api('/api/sources', { method: 'POST', body: JSON.stringify({ title, text }) });
    app.state.sources = data.sources;
    renderSources();
    $('note-form').reset();
    message('note-message', 'Source saved. It will be included with your next question.');
  } catch (error) { message('note-message', error.message, true); }
  finally { $('save-note').disabled = false; }
});
$('note-file').addEventListener('change', async event => {
  const file = event.target.files[0];
  if (!file) return;
  if (!/\.(txt|md)$/i.test(file.name) || file.size > 16384) { message('note-message', 'Choose a .txt or .md file under 16 KiB.', true); event.target.value = ''; return; }
  try {
    const text = await file.text();
    if (text.includes('\0') || byteLength(text) > 16384) throw new Error('Choose a plain text file under 16 KiB.');
    $('note-title').value = file.name.replace(/\.[^.]+$/, '').slice(0, 160);
    $('note-text').value = text;
    message('note-message', 'File loaded for review. Choose Save source to add it.');
  } catch (error) { message('note-message', error.message || 'Could not read this file.', true); }
});
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
  let frame = 0;
  let pointerX = 0;
  let pointerY = 0;
  const points = Array.from({ length: 95 }, (_, index) => {
    const seed = (index * 137.508) * Math.PI / 180;
    return { x: Math.cos(seed) * (200 + index * 7), y: Math.sin(seed * 1.71) * 600, z: Math.sin(seed) * 650 };
  });
  const reduced = () => userReduced || preference.matches || app.closing || app.closed;
  function render(time = 0) {
    frame = 0;
    if (!context) return;
    context.clearRect(0, 0, width, height);
    const angle = reduced() ? .35 : time * .000035;
    const cosine = Math.cos(angle);
    const sine = Math.sin(angle);
    const projected = points.map(point => {
      const x = point.x * cosine - point.z * sine;
      const z = point.z * cosine + point.x * sine;
      const depth = 1100 / (1500 + z);
      return { x: width * .62 + x * depth + pointerX * depth * 12, y: height * .48 + point.y * depth + pointerY * depth * 10, depth };
    });
    for (let index = 0; index < projected.length; index++) {
      const point = projected[index];
      context.beginPath();
      context.fillStyle = `rgba(169,245,195,${Math.min(.4, point.depth * .2)})`;
      context.arc(point.x, point.y, Math.max(.6, point.depth * 1.4), 0, Math.PI * 2);
      context.fill();
      for (let next = index + 1; next < Math.min(index + 6, projected.length); next++) {
        const neighbor = projected[next];
        const distance = Math.hypot(point.x - neighbor.x, point.y - neighbor.y);
        if (distance > 145) continue;
        context.beginPath();
        context.strokeStyle = `rgba(169,245,195,${(1 - distance / 145) * .12})`;
        context.lineWidth = .7;
        context.moveTo(point.x, point.y);
        context.lineTo(neighbor.x, neighbor.y);
        context.stroke();
      }
    }
    if (!reduced() && !document.hidden) frame = requestAnimationFrame(render);
  }
  function refresh() {
    cancelAnimationFrame(frame);
    const isReduced = reduced();
    document.body.classList.toggle('motion-off', isReduced);
    toggle.textContent = preference.matches ? 'Reduced motion' : isReduced ? 'Motion off' : 'Motion on';
    toggle.setAttribute('aria-pressed', String(isReduced));
    toggle.setAttribute('aria-label', preference.matches ? 'Reduced motion follows your device setting' : isReduced ? 'Turn motion on' : 'Turn motion off');
    toggle.disabled = preference.matches;
    render(performance.now());
  }
  app.refreshMotion = refresh;
  function resize() {
    width = innerWidth;
    height = innerHeight;
    const density = Math.min(devicePixelRatio || 1, 2.5);
    canvas.width = Math.round(width * density);
    canvas.height = Math.round(height * density);
    if (context) context.setTransform(density, 0, 0, density, 0, 0);
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
    pointerX = (event.clientX / width - .5) * 2;
    pointerY = (event.clientY / height - .5) * 2;
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
