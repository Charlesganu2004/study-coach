'use strict';

// Material content is rendered from bounded JSON fields. Model output never
// supplies markup; the diagram below is drawn with this app's own SVG nodes.
const materials = { entries: [], selected: null, loading: false };
const svgNamespace = 'http://www.w3.org/2000/svg';

function svgElement(tag, attributes = {}, text) {
  const node = document.createElementNS(svgNamespace, tag);
  for (const [name, value] of Object.entries(attributes)) node.setAttribute(name, String(value));
  if (text !== undefined) node.textContent = text;
  return node;
}

function drawMaterialDiagram(diagram) {
  if (!diagram || !Array.isArray(diagram.nodes) || !diagram.nodes.length || diagram.nodes.length > 12) return null;
  const columns = diagram.nodes.length === 1 ? 1 : 2;
  const rows = Math.ceil(diagram.nodes.length / columns);
  const width = columns * 300;
  const height = rows * 132 + 28;
  const svg = svgElement('svg', { viewBox: `0 0 ${width} ${height}`, role: 'img', 'aria-label': diagram.title || 'Relationships between the main ideas', class: 'study-diagram' });
  const title = svgElement('title', {}, diagram.title || 'Relationships between the main ideas');
  svg.append(title);
  const defs = svgElement('defs');
  const marker = svgElement('marker', { id: 'diagram-arrow', viewBox: '0 0 10 10', refX: 9, refY: 5, markerWidth: 6, markerHeight: 6, orient: 'auto-start-reverse' });
  marker.append(svgElement('path', { d: 'M 0 0 L 10 5 L 0 10 z', fill: '#397c72' }));
  defs.append(marker);
  svg.append(defs);
  const positions = diagram.nodes.map((_, index) => ({ x: (index % columns) * 300 + 24, y: Math.floor(index / columns) * 132 + 22 }));
  for (const edge of Array.isArray(diagram.edges) ? diagram.edges.slice(0, 24) : []) {
    const from = positions[edge.from];
    const to = positions[edge.to];
    if (!from || !to || edge.from === edge.to) continue;
    const horizontal = Math.abs(from.y - to.y) < 1;
    const forward = horizontal ? from.x < to.x : from.y < to.y;
    const startX = horizontal ? from.x + (forward ? 252 : 0) : from.x + 126;
    const startY = horizontal ? from.y + 44 : from.y + (forward ? 88 : 0);
    const endX = horizontal ? to.x + (forward ? 0 : 252) : to.x + 126;
    const endY = horizontal ? to.y + 44 : to.y + (forward ? 0 : 88);
    const midpoint = horizontal ? (startX + endX) / 2 : (startY + endY) / 2;
    const path = horizontal ? `M ${startX} ${startY} H ${midpoint} V ${endY} H ${endX}` : `M ${startX} ${startY} V ${midpoint} H ${endX} V ${endY}`;
    svg.append(svgElement('path', { d: path, fill: 'none', stroke: '#397c72', 'stroke-width': 2, 'marker-end': 'url(#diagram-arrow)' }));
  }
  diagram.nodes.forEach((label, index) => {
    const { x, y } = positions[index];
    svg.append(svgElement('rect', { x, y, width: 252, height: 88, rx: 10, fill: '#eff8f5', stroke: '#8ab9ad', 'stroke-width': 1.5 }));
    const text = svgElement('text', { x: x + 126, y: y + 31, 'text-anchor': 'middle', fill: '#163b34', 'font-family': 'Segoe UI, Arial, sans-serif', 'font-size': 15 });
    const words = String(label).split(/\s+/);
    const lines = [];
    let line = '';
    for (const word of words) {
      if (`${line} ${word}`.trim().length > 26 && line) { lines.push(line); line = word; }
      else line = `${line} ${word}`.trim();
    }
    if (line) lines.push(line);
    const shown = lines.slice(0, 3);
    if (lines.length > 3) shown[2] = `${shown[2].slice(0, 23)}…`;
    shown.forEach((part, lineIndex) => text.append(svgElement('tspan', { x: x + 126, dy: lineIndex ? 20 : shown.length === 1 ? 15 : 0 }, part)));
    svg.append(text);
  });
  return svg;
}

function renderMaterial(entry) {
  materials.selected = entry;
  $('material-empty').hidden = true;
  $('material-document').hidden = false;
  $('material-kind-label').textContent = entry.kind === 'flashcards' ? 'Flashcards / Active recall' : entry.kind === 'memory' ? 'Memory aids / Original acronyms and songs' : 'Study notes / Source grounded';
  $('material-document-title').textContent = entry.title;
  $('material-content').replaceChildren();
  $('material-diagram').replaceChildren();
  $('material-citations').replaceChildren();
  message('export-message', '');
  for (const section of entry.sections || []) {
    const container = element('section', null, 'material-section');
    container.append(element('h3', section.heading), element('p', section.text));
    $('material-content').append(container);
  }
  (entry.cards || []).forEach((card, index) => {
    const detail = element('details', null, 'flashcard');
    const front = element('summary');
    front.append(element('span', String(index + 1).padStart(2, '0'), 'flashcard-number'), element('span', card.front), element('small', 'Reveal answer'));
    detail.append(front, element('p', card.back));
    $('material-content').append(detail);
  });
  const diagram = drawMaterialDiagram(entry.diagram);
  if (diagram) {
    $('material-diagram').append(element('h3', entry.diagram.title || 'How the ideas connect'), diagram);
    const relations = element('ul', null, 'diagram-relations');
    for (const edge of entry.diagram.edges || []) {
      const from = entry.diagram.nodes[edge.from];
      const to = entry.diagram.nodes[edge.to];
      if (from && to) relations.append(element('li', `${from} relates to ${to}`));
    }
    if (relations.childElementCount) $('material-diagram').append(relations);
  }
  if (entry.citations?.length) $('material-citations').append(element('h3', 'Source passages'));
  for (const citation of entry.citations || []) {
    const figure = element('figure', null, 'citation');
    const source = app.state?.sources.find(item => item.id === citation.source_id);
    figure.append(element('blockquote', citation.quote), element('figcaption', `${citation.source_id} · ${source?.title || 'Saved source'}`));
    $('material-citations').append(figure);
  }
  for (const button of document.querySelectorAll('.material-library-item')) button.classList.toggle('active', button.dataset.id === entry.id);
}

function renderMaterialLibrary() {
  const list = $('material-library-list');
  list.replaceChildren();
  if (!materials.entries.length) list.append(element('p', 'No saved materials yet. Choose a topic above to create your first set.', 'empty-list'));
  for (const entry of [...materials.entries].reverse()) {
    const button = element('button', null, 'material-library-item');
    button.type = 'button';
    button.dataset.id = entry.id;
    button.classList.toggle('active', entry.id === materials.selected?.id);
    button.append(element('small', entry.kind === 'flashcards' ? 'FLASHCARDS' : entry.kind === 'memory' ? 'MEMORY AIDS' : 'STUDY NOTES'), element('strong', entry.title));
    button.addEventListener('click', () => { renderMaterial(entry); $('material-document-title').focus({ preventScroll: true }); });
    list.append(button);
  }
}

window.loadMaterials = async function loadMaterials() {
  if (materials.loading || !app.state) return;
  materials.loading = true;
  $('refresh-materials').disabled = true;
  try {
    const response = await api('/api/materials');
    materials.entries = Array.isArray(response.entries) ? response.entries : [];
    renderMaterialLibrary();
    if (!materials.selected && materials.entries.length) renderMaterial(materials.entries.at(-1));
  } catch (error) { message('material-message', error.message, true); }
  finally { materials.loading = false; $('refresh-materials').disabled = false; }
};

function downloadMaterialBlob(blob, filename) {
  const url = URL.createObjectURL(blob);
  const link = element('a');
  link.href = url;
  link.download = filename;
  document.body.append(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 30000);
}

function materialFilename(extension) {
  return `${(materials.selected?.title || 'Study notes').replace(/[<>:"/\\|?*\x00-\x1f]/g, '').slice(0, 100).trim() || 'Study notes'}.${extension}`;
}

function exportMaterialDocument() {
  const doc = document.implementation.createHTMLDocument(materials.selected.title);
  const charset = doc.createElement('meta');
  charset.setAttribute('charset', 'utf-8');
  doc.studyPrintStyles = '@page{size:A4;margin:18mm}body{font:12pt/1.6 "Segoe UI",Arial,sans-serif;color:#16342d;max-width:760px;margin:32px auto;padding:0 20px}h1{font-size:28pt;line-height:1.15}h2,h3{break-after:avoid;line-height:1.3}p,blockquote{white-space:pre-wrap;overflow-wrap:anywhere}section,figure,details{break-inside:avoid}figure{border-left:3px solid #397c72;margin:20px 0;padding-left:16px}figcaption,small{font-size:10pt;color:#435d56}svg{display:block;width:100%;max-height:650px}details{padding:12px 0;border-bottom:1px solid #b0c8c0}summary{font-weight:600}summary small{display:none}.flashcard-number{margin-right:12px}.diagram-relations{font-size:10pt}@media print{body{margin:0;padding:0;max-width:none}button{display:none}}';
  doc.head.append(charset);
  doc.body.append(element('p', 'STUDY COACH / SOURCE GROUNDED MATERIAL'), element('h1', materials.selected.title));
  for (const id of ['material-content', 'material-diagram', 'material-citations']) {
    const copy = $(id).cloneNode(true);
    copy.removeAttribute('id');
    copy.querySelectorAll('details').forEach(details => { details.open = true; });
    doc.body.append(copy);
  }
  doc.body.append(element('p', 'Generated from saved study sources. Check the source passages and revise any uncertain claims.'));
  return doc;
}

async function diagramPng() {
  const svg = $('material-diagram').querySelector('svg');
  if (!svg) return null;
  const copy = svg.cloneNode(true);
  copy.setAttribute('xmlns', svgNamespace);
  const image = new Image();
  image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(new XMLSerializer().serializeToString(copy))}`;
  await image.decode();
  const canvas = document.createElement('canvas');
  canvas.width = 1200;
  canvas.height = Math.round(1200 * svg.viewBox.baseVal.height / svg.viewBox.baseVal.width);
  const context = canvas.getContext('2d');
  if (!context) throw new Error('The browser could not prepare the diagram.');
  context.fillStyle = '#fff';
  context.fillRect(0, 0, canvas.width, canvas.height);
  context.drawImage(image, 0, 0, canvas.width, canvas.height);
  return await new Promise((resolve, reject) => canvas.toBlob(blob => blob ? resolve(blob) : reject(new Error('The diagram could not be copied.')), 'image/png'));
}

$('material-form').addEventListener('submit', async event => {
  event.preventDefault();
  if (app.busy || !app.state) return;
  const topic = $('material-topic').value.trim();
  if (!topic || byteLength(topic) > 4000) { message('material-message', 'Choose a topic within 4,000 bytes.', true); $('material-topic').focus(); return; }
  if (!hasKey(selectedProfile())) { message('material-message', 'Choose a configured model in Connections before creating materials.', true); return; }
  if (!app.state.sources.length) { message('material-message', 'Add some source notes before creating materials.', true); return; }
  let sourceScope;
  try { sourceScope = sourceSelectionRequest('material'); }
  catch (error) { message('material-message', error.message, true); $('material-use-all-sources').focus(); return; }
  app.busy = true;
  $('generate-material').disabled = true;
  $('material-loading').hidden = false;
  message('material-message', '');
  try {
    const entry = await api('/api/materials', { method: 'POST', body: JSON.stringify({ kind: $('material-kind').value, topic, profile: $('profile').value, style: $('material-kind').value === 'memory' ? $('memory-style').value : $('material-style').value, document_type: $('material-document-type').value.trim(), instructions: $('material-instructions').value.trim(), ...sourceScope }) });
    if (!entry.id || typeof entry.title !== 'string') throw new Error('The model did not return usable materials. Try a narrower topic.');
    materials.entries = [...materials.entries.filter(item => item.id !== entry.id), entry];
    renderMaterialLibrary();
    renderMaterial(entry);
    $('material-document-title').focus({ preventScroll: true });
    $('material-document').scrollIntoView({ block: 'start' });
    message('material-message', 'Study material saved to your library.');
  } catch (error) { message('material-message', error.message, true); }
  finally { app.busy = false; $('generate-material').disabled = false; $('material-loading').hidden = true; }
});
$('refresh-materials').addEventListener('click', () => window.loadMaterials());
$('material-kind').addEventListener('change', () => {
  const memory = $('material-kind').value === 'memory';
  $('memory-style-field').hidden = !memory;
  $('material-style-field').hidden = memory;
  $('material-document-type').value = memory ? 'Memory aids' : $('material-kind').value === 'flashcards' ? 'Flashcard set' : 'Study notes';
});
$('remove-material').addEventListener('click', async () => {
  if (!materials.selected || !window.confirm(`Remove “${materials.selected.title}” from your saved materials?`)) return;
  $('remove-material').disabled = true;
  try {
    const response = await api(`/api/materials/${encodeURIComponent(materials.selected.id)}`, { method: 'DELETE' });
    materials.entries = response.entries;
    materials.selected = null;
    $('material-document').hidden = true;
    $('material-empty').hidden = false;
    renderMaterialLibrary();
    if (materials.entries.length) renderMaterial(materials.entries.at(-1));
    message('material-message', 'Study material removed.');
  } catch (error) { message('export-message', error.message, true); }
  finally { $('remove-material').disabled = false; }
});
$('export-word').addEventListener('click', async () => {
  if (!materials.selected) return;
  $('export-word').disabled = true;
  message('export-message', 'Preparing your Word document…');
  try {
    const response = await fetch('/api/export/docx', { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Coach-Token': app.state.csrf_token }, body: JSON.stringify(materials.selected), credentials: 'same-origin' });
    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || 'The Word document could not be prepared.');
    }
    downloadMaterialBlob(await response.blob(), materialFilename('docx'));
    message('export-message', 'Word document downloaded, including source passages and the diagram when available.');
  } catch (error) { message('export-message', error.message, true); }
  finally { $('export-word').disabled = false; }
});
$('export-pdf').addEventListener('click', async () => {
  if (!materials.selected) return;
  const printWindow = window.open('', '_blank');
  if (!printWindow) { message('export-message', 'Allow a print window for this local app, then choose Save as PDF again.', true); return; }
  const doc = exportMaterialDocument();
  const stylesheet = doc.createElement('link');
  stylesheet.rel = 'stylesheet';
  stylesheet.href = new URL('/print.css', location.href).href;
  doc.head.append(stylesheet);
  printWindow.document.replaceChild(printWindow.document.importNode(doc.documentElement, true), printWindow.document.documentElement);
  printWindow.opener = null;
  try {
    const loaded = printWindow.document.querySelector('link');
    if (!loaded.sheet) await new Promise((resolve, reject) => {
      loaded.addEventListener('load', resolve, { once: true });
      loaded.addEventListener('error', () => reject(new Error('The print stylesheet could not be loaded. Keep the local app running, then try again.')), { once: true });
    });
    message('export-message', 'In the print dialog, choose Save as PDF. The print view includes all flashcard answers and diagrams.');
    printWindow.focus();
    printWindow.print();
  } catch (error) { message('export-message', error.message, true); }
});
$('copy-onenote').addEventListener('click', async () => {
  if (!materials.selected) return;
  $('copy-onenote').disabled = true;
  const doc = exportMaterialDocument();
  try {
    if (!navigator.clipboard?.write || typeof ClipboardItem === 'undefined') throw new Error('Rich clipboard access is unavailable.');
    const image = await diagramPng();
    if (image) {
      const bytes = new Uint8Array(await image.arrayBuffer());
      let binary = '';
      for (let offset = 0; offset < bytes.length; offset += 32768) binary += String.fromCharCode(...bytes.subarray(offset, offset + 32768));
      const picture = doc.createElement('img');
      picture.src = `data:image/png;base64,${btoa(binary)}`;
      picture.alt = materials.selected.diagram?.title || 'Study diagram';
      picture.width = 600;
      doc.querySelector('svg')?.replaceWith(picture);
    }
    await navigator.clipboard.write([new ClipboardItem({ 'text/html': new Blob([doc.body.innerHTML], { type: 'text/html' }), 'text/plain': new Blob([doc.body.textContent], { type: 'text/plain' }) })]);
    message('export-message', 'Copied. Open a OneNote page and paste (Ctrl+V). Notes, answers, source passages, and the diagram are included.');
  } catch {
    downloadMaterialBlob(new Blob(['<!doctype html>', doc.documentElement.outerHTML.replace('</head>', '<style>' + doc.studyPrintStyles + '</style></head>')], { type: 'text/html;charset=utf-8' }), materialFilename('html'));
    message('export-message', 'Rich copy was blocked. Open the downloaded HTML, select its contents (Ctrl+A), copy (Ctrl+C), then paste into a OneNote page (Ctrl+V).');
  } finally { $('copy-onenote').disabled = false; }
});

if (app.page === 'materials' && app.state) window.loadMaterials();
