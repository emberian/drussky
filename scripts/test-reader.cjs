// Supported Reader acceptance: actual app/IDB/media and explicitly synthetic
// Teacher/voice responses. No personal profile, inference or main service calls.
const { chromium } = require(process.env.RUFREN_PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const crypto = require('node:crypto');
const { CURRENT_PROTOCOL } = require('./qa-current-protocol.cjs');
const { replica, facts, learningEvidence, assertBounded } = require('./test-teacher-dialogue.cjs');
const { navigate, assertNavigation } = require('./qa-navigation.cjs');
const { wavFixture, generatedVoice } = require('./test-teacher-oral.cjs');
const { assetBundle } = require('./test-sound-focus.cjs');
const APP = process.env.RUFREN_QA_URL || 'http://127.0.0.1:8091';
const RELAY = 'http://127.0.0.1:18997'; // Fully intercepted; no listener.
const OUTPUT = path.resolve(process.env.RUFREN_READER_QA_OUTPUT || '.local/reader-qa');
const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const PASTE = {
  title: 'A quiet orbital garden',
  source: 'QA synthetic original, written for this browser test. No external author or edition.',
  // Deliberate combining stress, ё, an em dash and a nonbreaking space. This is
  // source data, not a request to normalize or silently replace an original.
  original_ru: 'На борту́ ста́нции был са́д. За окно́м медленно враща́лась Земля́.\n\n— Здесь ещё кто́-нибудь есть? — спроси́ла Ми́ра. В ответ мигну́ла зелёная ла́мпа.\n\nВ са́ду никого́ не бы́ло, но све́т всё ещё горе́л. На столе́ лежа́ла кни́га\u00a0о далёких мира́х.\n\nМи́ра закры́ла дверь и реши́ла верну́ться у́тром. Не всё ну́жно понима́ть сра́зу.',
  own_words: 'The garden is empty, but a light is still on. Mira will return tomorrow.',
  edited_words: 'The garden is empty, but its light is still on. Mira decides to return in the morning.',
  question: 'Why is всё ещё used here? Is the light continuing to be on?',
};
const AUDIO = wavFixture(); // Deterministic test tone, explicitly not speech.

function readerDialogueResponse(request, serial) {
  return { request: structuredClone(request), content: {
    explanation_english: 'Synthetic Reader explanation: всё ещё describes something continuing. This fixture tests the exact saved source and message binding; it is not a learner assessment.',
    utterance: null, next_question: null, correction: null, scene_state: 'open',
  }, provenance: { provider: 'qa_synthetic', model: 'synthetic-reader-context-fixture',
    response_id: `reader-qa-${serial}`, review_status: 'ai_generated_teaching',
    prompt_version: 'qa-reader-dialogue-v1', scope: 'Explicit synthetic context reply; no inference or learning judgment.' },
    usage: { input_tokens: 0, output_tokens: 0, estimated_micro_usd: 0, reserved_micro_usd: 0, accounting: 'subscription_quota' } };
}

async function fixtureRoutes(context, audit, store = replica()) {
  let replies = 0;
  await context.route('**/*', async route => {
    const req = route.request(), url = new URL(req.url());
    if (url.origin === new URL(APP).origin && !url.pathname.startsWith('/api/')) return route.continue();
    if (url.origin !== RELAY) { audit.forbidden.push(req.url()); return route.abort('blockedbyclient'); }
    if (audit.offline) return route.abort('internetdisconnected');
    const headers = { 'Access-Control-Allow-Origin': '*', 'Access-Control-Allow-Headers': '*', 'Access-Control-Allow-Methods': 'GET,POST,OPTIONS' };
    if (req.method() === 'OPTIONS') return route.fulfill({ status: 204, headers });
    const request = req.postDataJSON(); audit.requests.push({ path: url.pathname, request }); let response;
    if (url.pathname === '/v1/sync') response = store.exchange(request);
    else if (url.pathname === '/api/teacher/status' || url.pathname === '/api/tutor/status') response = {
      enabled: true, provider: 'qa_synthetic', model: 'Synthetic Reader fixture', message: 'No inference', data_sent: 'Synthetic fixture only',
      max_text_bytes: 8192, max_context_bytes: 2048, remaining_reservations: 99, remaining_budget_micro_usd: 0,
      accounting: 'subscription_quota', lesson_flow: 'guided_v1', dialogue: 'threaded_v1',
    };
    else if (url.pathname === '/api/teacher/dialogue') {
      assertBounded(request); await audit.beforeDialogue?.(request);
      response = readerDialogueResponse(request, ++replies);
      if (audit.holdReply) { audit.holdReply = false; await new Promise(resolve => { audit.releaseReply = resolve; audit.onHeldReply?.(request); }); }
    } else if (url.pathname === '/api/speech/generate') {
      await audit.beforeVoice?.(request); response = generatedVoice(request, AUDIO);
      response.provenance.model = 'deterministic-test-tone-not-russian-speech';
    } else { audit.forbidden.push(req.url()); return route.abort('blockedbyclient'); }
    return route.fulfill({ status: 200, headers, contentType: 'application/json', body: JSON.stringify(response) });
  });
  return store;
}

async function actualArtifacts(page) {
  const build = await page.evaluate(() => fetch('./offline-build.json').then(response => response.json()));
  if (process.env.RUFREN_EXPECT_WEB) assert.equal(build.version, process.env.RUFREN_EXPECT_WEB);
  return { web: build.version, current_source_protocol: CURRENT_PROTOCOL };
}
async function assertNoLearningCreated(page, baseline) {
  assert.deepEqual(learningEvidence(await facts(page)), baseline, 'Reading/import/dialogue must not fabricate graded attempts or mastery');
}
async function capture(page, name) {
  for (const [device, width, height] of [['phone', 390, 844], ['tablet', 820, 1080], ['desktop', 1200, 900]]) {
    await page.setViewportSize({ width, height });
    assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), `${name} ${device} overflow`);
    await page.screenshot({ path: path.join(OUTPUT, `${name}-${device}.png`), fullPage: true });
    await page.screenshot({ path: path.join(OUTPUT, `${name}-${device}-viewport.png`) });
  }
}
async function exportFixture(page, expected) {
  const events = await facts(page), originals = await assetBundle(page), ids = new Set();
  const walk = value => { if (!value || typeof value !== 'object') return; if (typeof value.asset_id === 'string') ids.add(value.asset_id); Object.values(value).forEach(walk); }; events.forEach(walk);
  const linked = originals.filter(original => ids.has(original.metadata.asset_id));
  for (const id of ids) assert(linked.some(original => original.metadata.asset_id === id), `Missing fact-linked original ${id}`);
  await fs.mkdir(path.join(OUTPUT, 'originals'), { recursive: true });
  for (const original of linked) { const bytes = Buffer.from(original.base64, 'base64'); assert.equal(sha(bytes), original.metadata.content_hash); assert.equal(bytes.length, original.metadata.byte_length); await fs.writeFile(path.join(OUTPUT, 'originals', `${sha(bytes)}.blob`), bytes); }
  await fs.writeFile(path.join(OUTPUT, 'library.json'), JSON.stringify({ version: CURRENT_PROTOCOL, events }, null, 2));
  await fs.writeFile(path.join(OUTPUT, 'expected-reader.json'), JSON.stringify(expected, null, 2));
  await fs.writeFile(path.join(OUTPUT, 'originals.json'), JSON.stringify({ records: linked.map(original => original.metadata) }, null, 2));
  return { events: events.length, originals: linked.length };
}


async function metadata(page, name) {
  return page.evaluate(name => { const raw = window.rufrenLibrary.get('rufren.v1.meta.' + name); return raw === null ? null : JSON.parse(raw); }, name);
}
async function documentFact(page, id) {
  return (await facts(page)).find(event => event.fact.kind === 'reader_document_recorded' && event.fact.data.document_id === id);
}
async function restoreWrites(page) {
  await page.evaluate(() => { if (window.readerQAWrite) window.rufrenLibrary.write = window.readerQAWrite; });
}
async function failWrites(page, key) {
  await page.evaluate(key => {
    window.readerQAWrite ||= window.rufrenLibrary.write;
    window.rufrenLibrary.write = function(name, value) {
      if (name.includes(key)) throw Error('Synthetic Reader persistence failure');
      return window.readerQAWrite(name, value);
    };
  }, key);
}
async function openShelf(page) {
  if (!await page.locator('.reader-shelf').isVisible()) await page.getByRole('button', { name: /^Choose (?:another )?text$/ }).click();
}
async function openReadingOptions(page, open = true) {
  const options = page.locator('details.reader-options');
  if (await options.count() && await options.evaluate(node => node.open) !== open) await options.locator(':scope > summary').click();
}
async function chooseShelf(page, title) {
  await openShelf(page);
  await page.locator('.reader-book').filter({ has: page.getByRole('heading', { name: title, exact: true }) }).getByRole('button').click();
  await page.locator('.reader-document').waitFor();
  return page.locator('.reader-document').getAttribute('data-reader-document');
}
async function choosePart(page, chunk_id) {
  const select = page.getByLabel('Choose a part', { exact: true });
  const changed = await select.inputValue() !== chunk_id;
  await select.selectOption(chunk_id);
  if (changed) await page.waitForFunction(id => document.activeElement?.id.endsWith('-' + id) && document.activeElement.id.startsWith('reader-original-'), chunk_id);
  return page.locator(`.reader-chunk[data-reader-chunk="${chunk_id}"]`);
}
async function reflection(chunk) {
  if (!await chunk.locator('textarea').count()) await chunk.getByRole('button', { name: 'In your own words / ask a question', exact: true }).click();
  return chunk.getByLabel('What did you take from this part?', { exact: true });
}
async function selectOriginal(chunk, exact) {
  await chunk.locator('.reader-original').evaluate((root, exact) => {
    const start = root.textContent.indexOf(exact); if (start < 0) throw Error('Selection is absent from original');
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT), range = document.createRange();
    let node, offset = 0, started = false;
    while ((node = walker.nextNode())) {
      const end = offset + node.data.length;
      if (!started && start >= offset && start < end) { range.setStart(node, start - offset); started = true; }
      if (started && start + exact.length <= end) { range.setEnd(node, start + exact.length - offset); break; }
      offset = end;
    }
    if (!started) throw Error('Native selection could not be established');
    const selection = getSelection(); selection.removeAllRanges(); selection.addRange(range);
    if (selection.toString() !== exact) throw Error('Native selection differs from source');
  }, exact);
}
async function waitForFact(page, predicate, description) {
  for (let i = 0; i < 300; i++) { const event = (await facts(page)).find(predicate); if (event) return event; await page.waitForTimeout(50); }
  throw Error('Missing durable Reader fact: ' + description);
}

async function main() {
  assert.equal(CURRENT_PROTOCOL, 7, 'Reader acceptance requires the new core contract; do not run against an unfinished/old bundle');
  await fs.mkdir(OUTPUT, { recursive: true });
  const audit = { checks: [], errors: [], forbidden: [], requests: [], inference_calls: 0, observations: [] }, store = replica();
  const browser = await chromium.launch({ headless: true, args: ['--autoplay-policy=no-user-gesture-required'], ...(process.env.RUFREN_CHROMIUM_EXECUTABLE ? { executablePath: process.env.RUFREN_CHROMIUM_EXECUTABLE } : {}) });
  const context = await browser.newContext({ viewport: { width: 1200, height: 900 } });
  await context.tracing.start({ screenshots: true, snapshots: true, sources: true });
  const page = await context.newPage(); page.setDefaultTimeout(30000); page.on('pageerror', error => audit.errors.push(error.message));
  await fixtureRoutes(context, audit, store);
  const check = text => { audit.checks.push(text); console.log('CHECK ' + text); };
  const connect = async () => { await navigate(page, 'Settings'); await page.locator('#sync-url').fill(RELAY); await page.locator('#sync-key').fill('synthetic-reader-no-secret'); };
  const expected = { documents: [], excerpts: [], dialogues: [], local_only: [], provenance: 'Actual Reader UI observations plus exact immutable source fields; all provider replies/audio are synthetic.' };
  try {
    await page.goto(APP, { waitUntil: 'networkidle' }); await page.locator('main.main h1').waitFor(); audit.artifact = await actualArtifacts(page);
    const baseline = learningEvidence(await facts(page)); await connect(); await navigate(page, 'Reader');
    const anthology = JSON.parse(await fs.readFile('crates/rufren-content/content/reader-v1.json', 'utf8'));
    assert.equal(anthology.documents.length, 4); assert.equal(await page.locator('.reader-shelf [data-reader-source]').count(), 4);
    for (const definition of anthology.documents) {
      const id = await chooseShelf(page, definition.title), event = await documentFact(page, id); assert(event);
      assert.equal(event.version, CURRENT_PROTOCOL); assert.deepEqual(event.fact.data.definition, definition);
      const doc = page.locator('.reader-document');
      assert.equal(await doc.locator('.reader-continuous-original').textContent(), definition.original);
      assert.equal(await doc.locator('.reader-continuous-original .reader-text-chunk').count(), definition.chunks.length);
      await doc.getByRole('button', { name: 'About this text & its support', exact: true }).click();
      const attribution = await doc.locator('.reader-source-details').innerText();
      for (const field of ['edition', 'rights', 'provenance']) assert(attribution.includes(definition.source[field]), 'Missing source ' + field);
      assert(attribution.includes(definition.help_provenance), 'AI assistance has separate attribution');
      assert.equal(await doc.locator('.reader-byline').textContent(), definition.source.author);
      if (definition.source.url) assert.equal(await doc.getByRole('link', { name: 'View the source ↗', exact: true }).getAttribute('href'), definition.source.url);
      expected.documents.push({ document_id: id, title: definition.title, original_ru: definition.original, original_sha256: definition.original_sha256, source: definition.source, chunks: definition.chunks.map((chunk, index) => ({ chunk_id: chunk.id, index, text: chunk.text })) });
    }
    await assertNoLearningCreated(page, baseline); check('All four exact anthology originals preserve source/edition and separately attributed AI support without attempts');
    const supported = anthology.documents.find(definition => definition.chunks[0].stress && definition.chunks[0].translation && definition.chunks[0].glosses.length);
    const supportedId = await chooseShelf(page, supported.title), supportedEvent = await documentFact(page, supportedId), frozenSupported = JSON.stringify(supportedEvent);
    await openReadingOptions(page); await page.getByRole('button', { name: 'One part at a time', exact: true }).click();
    let chunk = page.locator(`.reader-chunk[data-reader-chunk="${supported.chunks[0].id}"]`);
    assert.equal(await page.locator('.reader-chunk').count(), 1); assert.equal(await chunk.locator('.reader-help,.reader-stress-support').count(), 0);
    assert(!(await facts(page)).some(event => event.fact.kind === 'stimulus_presented' && event.fact.data.russian === supported.chunks[0].stress), 'Collapsed stress is not exposure');
    await page.getByRole('checkbox', { name: 'Show stress support alongside the original', exact: false }).check();
    await openReadingOptions(page, false); await chunk.locator('.reader-stress-support').scrollIntoViewIfNeeded();
    await waitForFact(page, event => event.fact.kind === 'stimulus_presented' && event.fact.data.russian === supported.chunks[0].stress, 'visible stress scaffold');
    for (const name of ['Meaning', 'Words in context', 'How the sentence works']) { const button = chunk.getByRole('button', { name, exact: true }); if (await button.count()) await button.click(); }
    assert.equal(await chunk.locator('.reader-original').textContent(), supported.chunks[0].text);
    assert.equal(JSON.stringify(await documentFact(page, supportedId)), frozenSupported, 'Support cannot replace the source original');
    await capture(page, 'supported-anthology'); check('Focus shows one part; collapsed support is absent, visible stress is observed, help never changes original bytes');
    await page.waitForFunction(() => !!navigator.serviceWorker.controller); audit.offline = true; await context.setOffline(true);
    await page.reload({ waitUntil: 'networkidle' }); await page.locator('main.main h1').waitFor(); await navigate(page, 'Reader');
    chunk = page.locator(`.reader-chunk[data-reader-chunk="${supported.chunks[0].id}"]`);
    for (const selector of ['.reader-meaning', '.reader-glosses', '.reader-grammar', '.reader-stress-support']) await chunk.locator(selector).waitFor();
    await chunk.getByRole('button', { name: 'Meaning', exact: true }).click(); assert.equal(await chunk.locator('.reader-meaning').count(), 0);
    await page.reload({ waitUntil: 'networkidle' }); await page.locator('main.main h1').waitFor(); await navigate(page, 'Reader');
    chunk = page.locator(`.reader-chunk[data-reader-chunk="${supported.chunks[0].id}"]`);
    assert.equal(await chunk.locator('.reader-meaning').count(), 0); await chunk.locator('.reader-glosses').waitFor();
    assert.equal(JSON.stringify(await documentFact(page, supportedId)), frozenSupported);
    audit.offline = false; await context.setOffline(false); await connect(); await navigate(page, 'Reader');
    check('Open support and stress survive offline reload; closing one support stays closed while other help remains open');


    await openShelf(page);
    await page.getByRole('button', { name: 'Or bring your own Russian text', exact: true }).click();
    await page.locator('#reader-import-title').fill(PASTE.title); await page.locator('#reader-import-original').fill(PASTE.original_ru); await page.locator('#reader-import-source').fill(PASTE.source);
    assert.equal(await page.locator('#reader-import-original').inputValue(), PASTE.original_ru);
    const previousSelection = await metadata(page, 'reader_selection');
    await failWrites(page, 'meta.reader_selection'); await page.getByRole('button', { name: 'Open my text →', exact: true }).click();
    await page.locator('.reader-error').filter({ hasText: /storage is full or unavailable|Synthetic Reader persistence failure/ }).first().waitFor();
    assert.equal(await metadata(page, 'reader_selection'), previousSelection, 'Failed selection keeps earlier reading');
    assert.equal((await metadata(page, 'reader_import_draft')).original, PASTE.original_ru, 'Exact paste stays recoverable');
    await restoreWrites(page); await page.getByRole('button', { name: 'Open my text →', exact: true }).click();
    await page.getByRole('heading', { name: PASTE.title, exact: true }).waitFor();
    const customId = await page.locator('.reader-document').getAttribute('data-reader-document'), customEvent = await documentFact(page, customId), custom = customEvent.fact.data.definition;
    assert.equal(custom.original, PASTE.original_ru); assert.equal(custom.original_sha256, sha(Buffer.from(PASTE.original_ru))); assert.equal(custom.source.kind, 'learner_provided'); assert(JSON.stringify(custom.source).includes(PASTE.source), 'Supplied source note is preserved separately from original'); assert.equal(custom.chunks.length, 4);
    assert.equal(custom.chunks.map(part => part.text).join(''), PASTE.original_ru);
    assert.equal(await page.locator('.reader-continuous-original').textContent(), PASTE.original_ru);
    assert(!(await facts(page)).some(event => event.fact.kind === 'stimulus_presented' && event.fact.data.russian === PASTE.original_ru), 'Import does not fabricate exposure to an unrendered whole-text string');
    expected.documents.push({ document_id: customId, title: custom.title, original_ru: custom.original, original_sha256: custom.original_sha256, source: custom.source, chunks: custom.chunks.map((part, index) => ({ chunk_id: part.id, index, text: part.text })) });
    const frozenCustom = JSON.stringify(customEvent);
    await openReadingOptions(page); await page.getByRole('button', { name: 'One part at a time', exact: true }).click();
    await openReadingOptions(page, false);
    const late = custom.chunks[2], last = custom.chunks[3]; chunk = await choosePart(page, late.id); let field = await reflection(chunk);
    await field.fill(PASTE.own_words); await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    assert(await field.evaluate(node => document.activeElement === node), 'Typing does not steal focus');
    const beforeFailure = await metadata(page, 'reader_state:' + customId);
    await failWrites(page, 'reader_state:' + customId); await field.fill(PASTE.edited_words);
    await chunk.locator('.reader-error').filter({ hasText: /storage is full or unavailable|Synthetic Reader persistence failure/ }).waitFor(); assert.equal(await field.inputValue(), PASTE.edited_words);
    assert.deepEqual(await metadata(page, 'reader_state:' + customId), beforeFailure);
    if (await page.getByRole('button', { name: /^Next part/ }).isEnabled()) await page.getByRole('button', { name: /^Next part/ }).click(); assert.equal((await metadata(page, 'reader_state:' + customId)).chunk_id, late.id);
    await restoreWrites(page);
    // Navigating after recovery must save or retain the visible failed edit,
    // rather than unmount it and silently recover only an older durable note.
    const retry = chunk.getByRole('button', { name: /Try saving again|Retry.*sav|Sav.*again/i }); if (await retry.count()) await retry.first().click();
    await page.getByRole('button', { name: /^Next part/ }).click();
    chunk = await choosePart(page, late.id); field = await reflection(chunk); assert.equal(await field.inputValue(), PASTE.edited_words);
    await page.waitForFunction(() => !!navigator.serviceWorker.controller); audit.offline = true; await context.setOffline(true); await page.reload({ waitUntil: 'networkidle' }); await page.locator('main.main h1').waitFor(); await navigate(page, 'Reader');
    assert.equal(await page.locator('.reader-document').getAttribute('data-reader-document'), customId); assert.equal(await page.locator('.reader-current').getAttribute('data-reader-chunk'), late.id); assert.equal(await page.locator('.reader-reflection textarea').inputValue(), PASTE.edited_words);
    audit.offline = false; await context.setOffline(false); await connect(); await navigate(page, 'Reader');
    check('Exact pasted Unicode/paragraphs survive failed selection; place and edited own words recover offline after a failed draft write');

    chunk = page.locator(`.reader-chunk[data-reader-chunk="${late.id}"]`);
    const phrase = 'све́т всё ещё горе́л'; assert(late.text.includes(phrase));
    for (let i = 0; i < 2; i++) { await selectOriginal(chunk, phrase); await chunk.getByRole('button', { name: 'Keep selected words for later', exact: true }).click(); await chunk.getByText(/^(?:Phrase saved|Saved) in your library\.$/).waitFor(); }
    const kept = (await facts(page)).filter(event => event.fact.kind === 'reader_excerpt_linked' && event.fact.data.document_id === customId && event.fact.data.purpose === 'kept_phrase'); assert.equal(kept.length, 1);
    const link = kept[0].fact.data, revision = (await facts(page)).find(event => event.id === link.revision); assert.equal(link.text, phrase); assert.equal(revision.fact.data.passage.russian, phrase);
    assert.equal(Buffer.from(custom.original).subarray(link.span.start, link.span.end).toString(), phrase);
    expected.excerpts.push({ source_fact_id: kept[0].id, revision_id: link.revision, document_id: customId, chunk_id: late.id, chunk_index: 2, original_ru: phrase });
    await assertNoLearningCreated(page, baseline); check('Native selected exact phrase is kept once with immutable source span; keeping it never starts practice');
    await choosePart(page, last.id); await navigate(page, 'Library');
    await page.locator('.passage-card').filter({ hasText: phrase }).click();
    assert.equal(await page.locator('#passage-russian').inputValue(), phrase);
    await page.locator('#passage-meaning').fill('the light was still on'); await page.getByRole('button', { name: 'Save passage', exact: true }).click();
    const editedExcerpt = (await facts(page)).find(event => event.fact.kind === 'passage_revised' && event.fact.data.parents.includes(link.revision)); assert(editedExcerpt);
    assert.equal(editedExcerpt.fact.data.passage.russian, phrase); assert.equal(editedExcerpt.fact.data.passage.translation, 'the light was still on');
    expected.excerpts[0].edited_revision_id = editedExcerpt.id;
    await page.getByRole('button', { name: 'Read for meaning', exact: true }).click(); await page.locator('.practice-panel').waitFor();
    await assertNoLearningCreated(page, baseline);
    await navigate(page, 'Library'); await page.locator('.passage-card').filter({ hasText: phrase }).click();
    await page.getByRole('button', { name: 'View original source →', exact: true }).click();
    assert.equal(await page.locator('.reader-document').getAttribute('data-reader-document'), customId); assert.equal(await page.locator('.reader-current').getAttribute('data-reader-chunk'), late.id);
    assert.equal(JSON.stringify(await documentFact(page, customId)), frozenCustom);
    chunk = page.locator(`.reader-chunk[data-reader-chunk="${late.id}"]`);
    check('Adding a meaning creates an ordinary phrase child; practice starts only by explicit choice, and its source link returns the original late part');

    await chunk.getByRole('button', { name: 'Listen to this part', exact: true }).click();
    const audio = chunk.locator('.reader-audio[data-reader-layer="original"]'); audit.beforeVoice = request => { assert.equal(request.text, late.text); assert.equal(request.revision_id, audioRevision); };
    const audioRevision = await audio.locator('.reader-audio-player').getAttribute('data-revision');
    const beforeAudio = (await facts(page)).filter(event => event.fact.kind === 'stimulus_presented' && event.fact.data.purpose === 'audio').length;
    await audio.getByRole('button', { name: 'Create spoken original', exact: true }).click(); await audio.locator('[data-practice-rate="1"]:not([disabled])').waitFor();
    assert.equal((await facts(page)).filter(event => event.fact.kind === 'stimulus_presented' && event.fact.data.purpose === 'audio').length, beforeAudio);
    await audio.locator('[data-practice-rate="1"]').click();
    await waitForFact(page, event => event.fact.kind === 'stimulus_presented' && event.fact.data.purpose === 'audio' && event.fact.data.revision === audioRevision && event.fact.data.russian === late.text, 'actual exact Reader audio playback');
    const original = (await assetBundle(page)).find(item => item.metadata.revision_id === audioRevision); assert(original); assert.equal(original.metadata.content_hash, AUDIO.hash); assert.equal(original.metadata.device.provenance.source_text, late.text); assert.equal(original.metadata.device.provenance.source_text_sha256, sha(Buffer.from(late.text)));
    expected.audio = [{ document_id: customId, chunk_id: late.id, layer: 'original', revision_id: audioRevision, asset_id: original.metadata.asset_id, sha256: AUDIO.hash, source_text: late.text }];
    check('One explicitly generated synthetic tone stores exact Reader original/revision/hash; only actual Listen logs audio exposure');

    for (const [part, index] of [[late, 2], [last, 3]]) {
      chunk = await choosePart(page, part.id); field = await reflection(chunk);
      const unsent = index === 2 ? PASTE.edited_words : 'Mira decides to return in the morning. Why does the final sentence use сразу?'; await field.fill(unsent);
      const posts = audit.requests.filter(request => request.path === '/api/teacher/dialogue').length;
      await chunk.getByRole('button', { name: 'Prepare a question for your teacher →', exact: true }).click(); await page.locator('#dialogue-message').waitFor();
      assert.equal(audit.requests.filter(request => request.path === '/api/teacher/dialogue').length, posts, 'Preparing opens editable composer without Send'); assert.equal(await page.locator('#dialogue-message').inputValue(), unsent);
      const sent = index === 2 ? PASTE.edited_words + ' ' + PASTE.question : unsent + ' Please explain just this part.';
      await page.locator('#dialogue-message').fill(sent);
      audit.beforeDialogue = async request => {
        assert.equal(request.text, sent); assert.equal(request.anchor.document_id, customId); assert.equal(request.anchor.focus.kind, 'reader_chunk'); assert.equal(request.anchor.focus.chunk_id, part.id); assert(request.anchor.snapshot.includes(part.text));
        const authored = (await facts(page)).find(event => event.fact.kind === 'teacher_dialogue_user_recorded' && event.fact.data.request_id === request.request_id);
        assert(authored, 'Authored request is durable before transport'); assert.deepEqual(authored.fact.data, request);
      };
      const posted = page.waitForRequest(request => request.url() === RELAY + '/api/teacher/dialogue' && request.method() === 'POST');
      await page.getByRole('button', { name: 'Send question', exact: true }).click();
      const request = (await posted).postDataJSON();
      assert(request); await waitForFact(page, event => event.fact.kind === 'teacher_dialogue_reply_recorded' && event.fact.data.request.request_id === request.request_id, 'synthetic Reader reply');
      const root = page.locator('.teacher-dialogue'); await root.locator('[data-dialogue-role="assistant"]').last().waitFor();
      expected.dialogues.push({ thread_id: request.thread_id, leaf_id: await root.getAttribute('data-dialogue-leaf'), message_ids: await root.locator('[data-message-id]').evaluateAll(nodes => nodes.map(node => node.dataset.messageId)), document_id: customId, chunk_id: part.id, chunk_index: index, original_ru: part.text, sent_text: sent });
      await page.getByRole('button', { name: 'Return to the passage', exact: true }).click(); await page.locator('.reader-document').waitFor();
      assert.equal(await page.locator('.reader-current').getAttribute('data-reader-chunk'), part.id, 'Saved source return uses exact frozen chunk, not document default');
      assert.equal(JSON.stringify(await documentFact(page, customId)), frozenCustom);
    }
    chunk = page.locator(`.reader-chunk[data-reader-chunk="${last.id}"]`);
    for (let i = 0; i < 2; i++) await chunk.getByRole('button', { name: 'Keep this part', exact: true }).click();
    const wholePart = (await facts(page)).filter(event => event.fact.kind === 'reader_excerpt_linked' && event.fact.data.document_id === customId && event.fact.data.purpose === 'kept_phrase' && event.fact.data.span.start === last.span.start && event.fact.data.span.end === last.span.end);
    assert.equal(wholePart.length, 1, 'Accessible full-part keep is idempotent'); assert.equal(wholePart[0].fact.data.text, last.text);
    expected.excerpts.push({ source_fact_id: wholePart[0].id, revision_id: wholePart[0].fact.data.revision, document_id: customId, chunk_id: last.id, chunk_index: 3, original_ru: last.text });
    await assertNoLearningCreated(page, baseline); await capture(page, 'reader-source-return');
    audit.navigation = [];
    for (const width of [390, 820, 1200]) { await page.setViewportSize({ width, height: 900 }); await assertNavigation(page, audit.navigation); await navigate(page, 'Reader'); }
    await choosePart(page, custom.chunks[1].id); const next = page.getByRole('button', { name: /^Next part/ }); await next.focus(); await page.keyboard.press('Enter');
    await page.waitForFunction(id => document.activeElement?.id.endsWith('-' + id) && document.activeElement.id.startsWith('reader-original-'), late.id);
    check('Real compact/desktop navigation and keyboard part movement keep the exact original focused without typing focus theft');
    check('Two explicit edited questions freeze different late chunks of one source; exact replies and returns cannot fabricate attempts or retarget original');
    for (const name of ['reader_selection', 'reader_import_draft', ...expected.documents.map(document => 'reader_state:' + document.document_id)]) expected.local_only.push({ key: name, producer_value: await metadata(page, name), consumer_before_open: null });
    audit.export = await exportFixture(page, expected); assert.equal(audit.requests.filter(request => request.path === '/api/speech/generate').length, 1); assert.equal(audit.requests.filter(request => request.path === '/api/teacher/dialogue').length, 2); assert.deepEqual(audit.errors, []); assert.deepEqual(audit.forbidden, []); audit.ok = true;
  } catch (error) { audit.ok = false; audit.error = error.stack; audit.body = await page.locator('body').innerText().catch(() => null); audit.events = await facts(page).catch(() => []); await page.screenshot({ path: path.join(OUTPUT, 'failure.png'), fullPage: true }).catch(() => {}); throw error;
  } finally { audit.releaseReply?.(); await restoreWrites(page).catch(() => {}); await fs.writeFile(path.join(OUTPUT, 'result.json'), JSON.stringify(audit, (_, value) => typeof value === 'function' ? undefined : value, 2)); await context.tracing.stop({ path: path.join(OUTPUT, 'trace.zip') }).catch(() => {}); await browser.close(); }
  console.log(JSON.stringify({ ok: audit.ok, artifact: audit.artifact, checks: audit.checks, export: audit.export }, null, 2));
}
if (require.main === module) main().catch(error => { console.error(error); process.exitCode = 1; });
module.exports = { PASTE, AUDIO, APP, RELAY, OUTPUT, readerDialogueResponse, fixtureRoutes, actualArtifacts, assertNoLearningCreated, capture, exportFixture, chooseShelf, choosePart, reflection, documentFact, metadata, openShelf, openReadingOptions };
