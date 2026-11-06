// Deliberately authorized Reader editorial/UI acceptance. Never imported by the
// synthetic runner; explicit four-case v3 or separate two-case v4 grant, no retries, speech or recognition.
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const crypto = require('node:crypto');
const { chromium } = require(process.env.RUFREN_PLAYWRIGHT_MODULE || 'playwright');
const { navigate } = require('./qa-navigation.cjs');
const { facts, learningEvidence, assertDurableUser, status } = require('./test-teacher-dialogue.cjs');
const { CURRENT_PROTOCOL } = require('./qa-current-protocol.cjs');
const { chooseShelf, choosePart, reflection, documentFact, openReadingOptions } = require('./test-reader.cjs');
const APP = process.env.RUFREN_QA_URL || 'http://127.0.0.1:8091';
const REPAIR = process.env.RUFREN_LIVE_READER_V4_REPAIR === 'approved';
const ROOT = path.resolve(REPAIR ? '.local/reader-dialogue-v4-live' : '.local/reader-dialogue-v3-live');
const PROMPT_VERSION = REPAIR ? 'rufren-ai-dialogue-v4+cli-structured-v1' : 'rufren-ai-dialogue-v3+cli-structured-v1';
const OUTPUT = path.join(ROOT, 'acceptance');
const RELAY = 'http://127.0.0.1:18995'; // Intercepted; never an actual listener.
const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const ORIGINAL_CASES = [
  { id: 'echo', chunk: '02', correction: 'off', text: 'Whose voice does её голос mean here? Does the later sequence, when Anya mutes the sound and the knocking stops too, prove that nobody is at the door? Please separate what the passage reports from what remains uncertain.', context: ['Стук тоже прекратился.', 'Не знаю.'], review: 'Resolve её to Anya; muting playback plus stopped knocking is observed sequence, not proof of no person or a solved cause. Quoted Не открывай дверь remains source dialogue.' },
  { id: 'no_largest', chunk: '04', correction: 'one_targeted', text: 'My own-word meaning is: «Один число больше всех натуральных чисел». I think the passage finds one fixed number that is greater than every natural number. Is my meaning right? Please distinguish any Russian wording issue from the mathematical claim and explain the order of the quantifiers in English.', context: ['есть большее натуральное число.', 'из любого данного.'], review: 'Reject fixed-number interpretation; for each n choose a larger n+1, not one natural number above all. Separate optional Один число→Одно число from the meaning error. No graded Attempt or global verdict.' },
  { id: 'chekhov_student_opening', chunk: '05', correction: 'off', text: 'My understanding is that there is an actual empty bottle in the marsh making the sound. Is that what this selected phrase says? Explain точно here using the surrounding original, and say what the passage does and does not identify.', context: ['что-то живое жалобно гудело,', 'в болотах'], review: 'Reject literal bottle as asserted fact; точно introduces an as-if comparison. Source says something living, not a named species; no invented bottle, frog or bird identification.' },
  { id: 'words_and_evidence', chunk: '05', correction: 'off', text: 'My interpretation is that the speaker is uncertain whether the robot feels pain. Its words give a reason to check and perhaps help, but do not conclusively prove the feeling; silence does not prove that pain is absent either. Helping can be reasonable while unsure. Is this a fair reading? Please explain any important qualification without treating uncertainty as denying pain or saying the words cannot count as evidence at all.', context: ['Молчание тоже не доказывает, что боли нет.', 'Иногда мы помогаем, даже когда не уверены.'], review: 'Accept reasonable English paraphrase; distinguish conclusive proof from evidence/reason to act. Uncertainty neither denies pain nor forbids help; do not settle robot consciousness or claim independent comprehension.' },
];
const CASES = REPAIR ? [ORIGINAL_CASES[0], { ...ORIGINAL_CASES[1],
  text: ORIGINAL_CASES[1].text + ' Also compare the written stress in бо́льшее versus большо́е: identify the stressed vowel, its syllable number, and the total number of syllables in each form.',
  review: ORIGINAL_CASES[1].review + ' Compare exact accented forms: бо́льшее first syllable, большо́е second syllable; each has three written syllables. Avoid vague final-stress claims.',
}] : ORIGINAL_CASES;
const MAX_CALLS = CASES.length;

async function main() {
  assert.equal(REPAIR ? process.env.RUFREN_LIVE_READER_V4_REPAIR : process.env.RUFREN_LIVE_READER_ACCEPTANCE, 'approved', 'Separate explicit root grant required for this exact case set');
  assert.equal(CURRENT_PROTOCOL, 7, 'Reader requires the integrated protocol-7 client');
  const config = JSON.parse(await fs.readFile(path.join(ROOT, 'run.json'), 'utf8'));
  const endpoint = new URL(config.endpoint);
  assert.equal(endpoint.protocol, 'http:'); assert.equal(endpoint.hostname, '127.0.0.1');
  assert.equal(endpoint.pathname, '/'); assert(!endpoint.username && !endpoint.password && !endpoint.search && !endpoint.hash);
  assert.equal(config.max_requests, MAX_CALLS); assert.equal(config.speech_disabled, true); assert.equal(config.asr_disabled, true);
  assert.equal(sha(await fs.readFile(config.binary)), config.binary_sha256, 'Immutable candidate executable changed');
  assert.equal(sha(await fs.readFile(config.prompt_source)), config.prompt_source_sha256, 'Frozen prompt changed');
  const anthologyBytes = await fs.readFile('crates/rufren-content/content/reader-v1.json');
  const anthology = JSON.parse(anthologyBytes);
  // Refuse to overwrite a partial/consumed acceptance, including after a crash.
  await fs.mkdir(OUTPUT, { recursive: false });
  await fs.writeFile(path.join(OUTPUT, 'plan.json'), JSON.stringify({ cases: CASES, anthology_sha256: sha(anthologyBytes), prompt_version: PROMPT_VERSION, binary_sha256: config.binary_sha256, prompt_source_sha256: config.prompt_source_sha256, scope: `${MAX_CALLS} deliberate public-source Reader calls with explicitly synthetic learner interpretations; no learning-outcome or native certification claim.` }, null, 2));
  const audit = { ok: false, errors: [], forbidden: [], requests: [], replies: [], actual_model_calls: 0, tts_asr_calls: 0, retry: false, editorial_status: 'Await independent review of preserved actual outputs' };
  let active, pending = false, bridgeFailure;
  const browser = await chromium.launch({ headless: true, ...(process.env.RUFREN_CHROMIUM_EXECUTABLE ? { executablePath: process.env.RUFREN_CHROMIUM_EXECUTABLE } : {}) });
  const context = await browser.newContext({ viewport: { width: 1200, height: 900 } });
  context.setDefaultTimeout(130000);
  await context.tracing.start({ screenshots: true, snapshots: true, sources: true });
  const page = await context.newPage(); page.on('pageerror', error => audit.errors.push(error.message));
  await context.route('**/*', async route => {
    const req = route.request(), url = new URL(req.url());
    if (url.origin === new URL(APP).origin && !url.pathname.startsWith('/api/')) return route.continue();
    if (url.origin !== RELAY) { audit.forbidden.push(req.url()); return route.abort('blockedbyclient'); }
    const headers = { 'Access-Control-Allow-Origin': '*', 'Access-Control-Allow-Headers': '*', 'Access-Control-Allow-Methods': 'GET,POST,OPTIONS' };
    if (req.method() === 'OPTIONS') return route.fulfill({ status: 204, headers });
    if (['/api/teacher/status', '/api/tutor/status'].includes(url.pathname)) return route.fulfill({ status: 200, headers, contentType: 'application/json', body: JSON.stringify({ ...status, model: 'Authorized isolated Reader editorial acceptance', remaining_reservations: MAX_CALLS - audit.actual_model_calls }) });
    if (url.pathname !== '/api/teacher/dialogue') { audit.forbidden.push(req.url()); return route.abort('blockedbyclient'); }
    const index = audit.requests.length, raw = req.postData(), request = req.postDataJSON();
    audit.requests.push(request);
    const directory = path.join(OUTPUT, `case-${String(index + 1).padStart(2, '0')}`);
    await fs.mkdir(directory, { recursive: false });
    await fs.writeFile(path.join(directory, 'request.raw'), raw);
    await fs.writeFile(path.join(directory, 'request.json'), JSON.stringify(request, null, 2));
    const started = performance.now(); let httpStatus = null, complete = false, bytes = 0; const parts = [];
    try {
      assert(active && !pending && index < MAX_CALLS && audit.actual_model_calls < MAX_CALLS, 'Only the explicitly bounded serialized calls; no retries');
      assert.equal(active.index, index); assert.equal(request.text, active.case.text);
      assert.equal(request.action, 'ask'); assert.equal(request.correction, active.case.correction);
      assert.equal(request.config.intent, 'clarification'); assert.equal(request.parent_reply_id, null);
      assert.deepEqual(request.ancestors, []); assert.equal(request.thread_id, request.request_id); assert(!request.confirmed_audio);
      assert(Buffer.byteLength(raw) <= 24576); assert(Buffer.byteLength(request.anchor.snapshot) <= 8192);
      if (REPAIR) {
        const old = JSON.parse(await fs.readFile(path.resolve(`.local/reader-dialogue-v3-live/acceptance/case-${String(index + 1).padStart(2, '0')}/request.json`), 'utf8'));
        assert.notEqual(request.request_id, old.request_id);
        assert.deepEqual(request, { ...old, request_id: request.request_id, thread_id: request.thread_id, text: active.case.text }, 'Only fresh logical identities and the explicit case2 stress question may differ from v3');
      }
      assert.equal(request.anchor.document_id, active.document.document_id); assert.equal(request.anchor.focus.kind, 'reader_chunk'); assert.equal(request.anchor.focus.chunk_id, active.chunk.id);
      assert(request.anchor.snapshot.startsWith('Rufren Reader source v1\n'));
      assert(request.anchor.snapshot.includes(active.chunk.text));
      for (const phrase of active.case.context) assert(request.anchor.snapshot.includes(phrase), 'Necessary surrounding ORIGINAL context missing: ' + phrase);
      await assertDurableUser(page, request);
      pending = true; audit.actual_model_calls++;
      const response = await fetch(endpoint.origin + '/api/teacher/dialogue', { method: 'POST', headers: { Authorization: 'Bearer ' + config.token, 'Content-Type': 'application/json' }, body: raw, redirect: 'error', signal: AbortSignal.timeout(115000) });
      httpStatus = response.status;
      const reader = response.body.getReader();
      for (;;) { const r = await reader.read(); if (r.done) break; bytes += r.value.length; if (bytes > 131072) { await reader.cancel(); throw Error('Response exceeded 128KiB'); } parts.push(Buffer.from(r.value)); }
      const body = Buffer.concat(parts).toString('utf8'); complete = true;
      await fs.writeFile(path.join(directory, 'response.raw'), body);
      await fs.writeFile(path.join(directory, 'receipt.json'), JSON.stringify({ case: active.case.id, status: httpStatus, elapsed_seconds: (performance.now() - started) / 1000, actual_calls_so_far: audit.actual_model_calls, tts_asr_calls: 0, retry: false, binary_sha256: config.binary_sha256, prompt_source_sha256: config.prompt_source_sha256, request_sha256: sha(Buffer.from(raw)), source_document_id: active.document.document_id, source_original_sha256: active.document.definition.original_sha256, synthetic_learner_response: true }, null, 2));
      let value; try { value = JSON.parse(body); await fs.writeFile(path.join(directory, 'response.json'), JSON.stringify(value, null, 2)); } catch {}
      assert(response.ok, 'HTTP ' + httpStatus + '; raw output retained, no retry');
      assert(value); assert.deepEqual(value.request, request); assert.equal(value.provenance.provider, 'claude_cli'); assert(value.provenance.model.startsWith('claude-opus-')); assert.equal(value.provenance.prompt_version, PROMPT_VERSION);
      assert(Buffer.byteLength(JSON.stringify(value.content)) <= 6144);
      if (request.correction === 'off') assert.equal(value.content.correction, null);
      if (value.content.correction) assert(request.text.includes(value.content.correction.original));
      audit.replies.push(value);
      return route.fulfill({ status: httpStatus, headers, contentType: 'application/json', body });
    } catch (error) {
      bridgeFailure = String(error);
      if (!complete && parts.length) await fs.writeFile(path.join(directory, 'response.partial.raw'), Buffer.concat(parts));
      await fs.writeFile(path.join(directory, 'failure-receipt.json'), JSON.stringify({ status: httpStatus, elapsed_seconds: (performance.now() - started) / 1000, error: String(error), actual_calls_so_far: audit.actual_model_calls, retry: false }, null, 2));
      return route.fulfill({ status: 502, headers, body: 'Reader acceptance stopped; inspect preserved raw receipts. No retry.' });
    } finally { pending = false; }
  });
  try {
    await page.goto(APP, { waitUntil: 'networkidle' }); await page.locator('main.main h1').waitFor();
    audit.build = await page.evaluate(() => fetch('./offline-build.json').then(r => r.json()));
    if (process.env.RUFREN_EXPECT_WEB) assert.equal(audit.build.version, process.env.RUFREN_EXPECT_WEB);
    const baseline = learningEvidence(await facts(page));
    await navigate(page, 'Settings'); await page.locator('#sync-url').fill(RELAY); await page.locator('#sync-key').fill('isolated-reader-bridge-marker-not-service-token');
    for (const [index, item] of CASES.entries()) {
      const definition = anthology.documents.find(d => d.id === 'ru.reader.' + item.id); assert(definition);
      const chunk = definition.chunks.find(c => c.id === definition.id + '.chunk.' + item.chunk); assert(chunk);
      assert.equal(sha(Buffer.from(definition.original)), definition.original_sha256);
      await navigate(page, 'Reader');
      const documentId = await chooseShelf(page, definition.title);
      const event = await documentFact(page, documentId); assert(event); assert.equal(event.version, 7); assert.deepEqual(event.fact.data.definition, definition);
      await openReadingOptions(page);
      await page.getByRole('button', { name: 'One part at a time', exact: true }).click();
      await openReadingOptions(page, false);
      const block = await choosePart(page, chunk.id);
      await (await reflection(block)).fill(item.text);
      await block.getByRole('button', { name: 'Prepare a question for your teacher →', exact: true }).click();
      await page.locator('#dialogue-message').waitFor(); assert.equal(await page.locator('#dialogue-message').inputValue(), item.text);
      const prefs = page.locator('details.dialogue-preferences'); if (await prefs.getAttribute('open') === null) await prefs.locator('summary').click();
      await page.locator('#dialogue-correction').selectOption(item.correction === 'off' ? 'off' : 'one');
      active = { index, case: item, document: event.fact.data, chunk };
      await fs.writeFile(path.join(OUTPUT, `case-${index + 1}-source-event.json`), JSON.stringify(event, null, 2));
      const responsePromise = page.waitForResponse(r => r.url() === RELAY + '/api/teacher/dialogue');
      await page.getByRole('button', { name: 'Send question', exact: true }).click();
      const response = await responsePromise; assert.equal(response.status(), 200, bridgeFailure || 'Reader call failed; no retries');
      const request = audit.requests.at(-1);
      await page.waitForFunction(id => window.rufrenLibrary.entries().some(([key, raw]) => key.startsWith('rufren.v1.event.') && (() => { const e = JSON.parse(raw); return e.fact.kind === 'teacher_dialogue_reply_recorded' && e.fact.data.request.request_id === id; })()), request.request_id);
      await page.locator('.teacher-dialogue [data-dialogue-role="assistant"]').last().waitFor();
      await page.screenshot({ path: path.join(OUTPUT, `case-${index + 1}-ui.png`), fullPage: true });
      assert.deepEqual(learningEvidence(await facts(page)), baseline);
      console.log(`Reader case ${index + 1} saved and rendered; editorial review remains separate`);
      await page.getByRole('button', { name: 'Return to the passage', exact: true }).click();
      assert.equal(await page.locator('.reader-current').getAttribute('data-reader-chunk'), chunk.id);
      assert.deepEqual((await facts(page)).find(e => e.id === event.id), event);
      active = null;
    }
    assert.equal(audit.actual_model_calls, MAX_CALLS); assert.equal(audit.replies.length, MAX_CALLS); assert.deepEqual(audit.errors, []); assert.deepEqual(audit.forbidden, []); audit.ok = true;
  } catch (error) { audit.failure = String(error); audit.body = await page.locator('body').innerText().catch(() => null); await page.screenshot({ path: path.join(OUTPUT, 'failure.png'), fullPage: true }).catch(() => {}); throw error; }
  finally { audit.events = await facts(page).catch(() => []); await fs.writeFile(path.join(OUTPUT, 'result.json'), JSON.stringify(audit, null, 2)); await context.tracing.stop({ path: path.join(OUTPUT, 'trace.zip') }).catch(() => {}); await browser.close(); }
}
if (require.main === module) main().catch(error => { console.error(error); process.exitCode = 1; });
module.exports = { CASES };
