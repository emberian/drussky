// Separately authorized four-case live editorial/UI acceptance. This is never
// imported by the synthetic runner and refuses to run without its explicit gate.
const { chromium } = require(process.env.RUFREN_PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const crypto = require('node:crypto');
const { APP, status, facts, learningEvidence, assertDurableUser } = require('./test-teacher-dialogue.cjs');
const { navigate } = require('./qa-navigation.cjs');
const { replyID } = require('./test-teacher-dialogue-smoke.cjs');
const RELAY = 'http://127.0.0.1:18996'; // Browser traffic intercepted; bridge owns the only real endpoint.
const REPAIR = process.env.RUFREN_LIVE_DIALOGUE_V2_REPAIR === 'approved';
const ROOT = REPAIR ? '.local/teacher-dialogue-v2-live' : '.local/teacher-dialogue-live';
const OUTPUT = path.resolve(ROOT + '/acceptance');
const MAX_CALLS = REPAIR ? 2 : 4;
const SOURCE = path.resolve('.local/teacher-v5-live-revised/lesson.json');

async function main() {
  assert.equal(REPAIR ? process.env.RUFREN_LIVE_DIALOGUE_V2_REPAIR : process.env.RUFREN_LIVE_DIALOGUE_ACCEPTANCE,
    'approved', 'Requires the separate explicit root grant for this exact live scope');
  const sourceBytes = await fs.readFile(SOURCE), original = JSON.parse(sourceBytes);
  const previous = REPAIR ? JSON.parse(await fs.readFile('.local/teacher-dialogue-live/acceptance/result.json', 'utf8')) : null;
  const { makeLiveDialogueBridge } = require(path.resolve(ROOT + '/live-bridge.cjs'));
  const bridge = await makeLiveDialogueBridge({ output: OUTPUT });
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({ viewport: { width: 1200, height: 900 } });
  context.setDefaultTimeout(130000);
  await context.tracing.start({ screenshots: true, snapshots: true, sources: true });
  const page = await context.newPage(), errors = [], forbidden = [], requests = [], replies = [], checks = [];
  page.on('pageerror', error => errors.push(error.message));
  let consumed = 0, bridgeFailure;
  await context.route('**/*', async route => {
    const req = route.request(), url = new URL(req.url());
    if (url.origin === new URL(APP).origin && !url.pathname.startsWith('/api/')) return route.continue();
    if (url.origin !== RELAY) { forbidden.push(req.url()); return route.abort('blockedbyclient'); }
    const headers = { 'Access-Control-Allow-Origin': '*', 'Access-Control-Allow-Headers': '*', 'Access-Control-Allow-Methods': 'GET,POST,OPTIONS' };
    if (req.method() === 'OPTIONS') return route.fulfill({ status: 204, headers });
    if (url.pathname === '/api/teacher/status' || url.pathname === '/api/tutor/status') return route.fulfill({ status: 200,
      headers, contentType: 'application/json', body: JSON.stringify({ ...status, model: 'Explicitly authorized isolated live dialogue', remaining_reservations: MAX_CALLS - consumed }) });
    if (url.pathname !== '/api/teacher/dialogue') { forbidden.push(req.url()); return route.abort('blockedbyclient'); }
    const request = req.postDataJSON(); requests.push(request);
    try {
      assert(consumed < MAX_CALLS, 'No extra request or retry is authorized');
      await assertDurableUser(page, request);
      const response = await bridge(req.postData()); consumed++;
      replies.push(response.value);
      return route.fulfill({ status: response.status, headers, contentType: 'application/json', body: response.body });
    } catch (error) {
      // Bridge preserves any consumed raw output before assertions. Never retry.
      bridgeFailure = String(error);
      await fs.writeFile(path.join(OUTPUT, 'ui-bridge-failure.json'), JSON.stringify({ error: String(error), requests_seen: requests.length, consumed_before_failure: consumed, retry: false }, null, 2));
      return route.fulfill({ status: 502, headers, body: 'Live acceptance stopped; inspect preserved bridge receipts.' });
    }
  });
  const nav = name => navigate(page, name);
  const connect = async () => { await nav('Settings'); await page.locator('#sync-url').fill(RELAY); await page.locator('#sync-key').fill('isolated-bridge-browser-marker-not-service-token'); };
  const library = async () => { const box = page.locator('details.dialogue-library'); if (await box.getAttribute('open') === null) await box.locator(':scope > summary').click(); };
  const sendCase = async button => {
    const before = requests.length;
    const responsePromise = page.waitForResponse(r => r.url() === RELAY + '/api/teacher/dialogue');
    await page.getByRole('button', { name: button, exact: true }).click();
    const response = await responsePromise;
    assert.equal(response.status(), 200, bridgeFailure || 'Live request failed; no automatic retry');
    assert.equal(requests.length, before + 1);
    const request = requests.at(-1);
    await page.waitForFunction(user => window.rufrenLibrary.entries().some(([key, raw]) => {
      if (!key.startsWith('rufren.v1.event.')) return false;
      const event = JSON.parse(raw); return event.fact.kind === 'teacher_dialogue_reply_recorded' && event.fact.data.request.request_id === user;
    }), request.request_id);
    await page.locator('#dialogue-message').waitFor();
    const index = requests.length;
    await page.screenshot({ path: path.join(OUTPUT, `case-${String(index).padStart(2, '0')}-ui.png`), fullPage: true });
    console.log(`LIVE case ${index} saved and rendered; no retry`);
    return request;
  };
  try {
    await page.goto(APP); await page.locator('main.main h1').waitFor(); await connect();
    let document, q2;
    if (REPAIR) {
      assert.equal(previous.actual_model_calls, 4);
      await page.evaluate(events => {
        for (const event of events) window.rufrenLibrary.write(`rufren.v1.event.${event.id}`, JSON.stringify(event));
        window.rufrenLibrary.write('rufren.v1.meta.teacher_surface', JSON.stringify('dialogues'));
      }, previous.events);
      await nav('Teacher'); await library();
      await page.locator(`[data-open-dialogue-leaf="${replyID(previous.replies.at(-1))}"]`).click();
      const parent = previous.requests[2].parent_reply_id;
      await page.locator(`article[data-message-id="${parent}"]`).getByRole('button', { name: 'Continue from this reply', exact: true }).click();
      document = previous.source_document; q2 = previous.requests[1];
      checks.push('V2 repair deliberately branches from the preserved actual v1 scene reply; old rejected output and all earlier messages remain immutable');
    } else {
    await page.evaluate(response => window.rufrenLibrary.write('rufren.v1.meta.teacher_response', JSON.stringify(response)), original);
    await page.reload(); await page.locator('main.main h1').waitFor(); await connect(); await nav('Teacher');
    const flow = page.locator('.teacher-lesson-flow'); await flow.waitFor();
    document = await flow.getAttribute('data-lesson-document');
    const archive = (await facts(page)).find(e => e.fact.kind === 'teacher_document_recorded' && e.fact.data.document_id === document);
    assert.deepEqual(archive.fact.data.response, original); assert.equal(archive.fact.data.receipt.kind, 'recovered_local_cache');
    const register = original.content.lesson_flow.steps.findIndex(step => step.id === 'register'); assert(register >= 0);
    await flow.getByRole('combobox', { name: 'Go to a step', exact: true }).selectOption(String(register));
    await page.locator('.teacher-lesson-current .ask-about-this').click();
    await page.locator('#dialogue-message').fill('Why does А у вас? use вас, but I use ты with a friend? Would у ты be correct?');
    const q1 = await sendCase('Send question');
    assert.equal(q1.anchor.document_id, document); assert.equal(q1.anchor.focus.step_id, 'register'); assert.equal(q1.correction, 'off');
    checks.push('Actual anchored register clarification uses an untouched recovered real lesson and exact source step');

    await library(); await page.locator('details.dialogue-scene-setup > summary').click();
    await page.locator('#dialogue-scene').fill('A patient AI aboard a quiet starship welcomes me and offers tea. I am a complete beginner in Russian. Begin with one short Russian turn, teach what I need, and make English help available.');
    await page.getByRole('button', { name: 'Prepare this scene', exact: true }).click();
    q2 = await sendCase('Start scene');
    assert.equal(q2.text, ''); assert.equal(q2.action, 'start_scene'); assert.equal(q2.anchor, null);
    checks.push('Actual beginner role-play starts with an explicit empty StartScene action, not an invented learner utterance');
    }

    await page.locator('#dialogue-action').selectOption('ask');
    await page.locator('#dialogue-message').fill('Could you explain your last question in English and give me one short Russian reply I can use?');
    const q3 = await sendCase('Send question');
    assert.equal(q3.thread_id, q2.thread_id); assert.equal(q3.action, 'ask'); assert.equal(q3.correction, 'off');
    console.log('PAUSED after actual English-help turn; awaiting provider-authored respond-fixture.json tied to the actual parent');
    let fixture;
    const deadline = Date.now() + 180000;
    const actualParent = await page.locator('.teacher-dialogue').getAttribute('data-dialogue-leaf');
    while (Date.now() < deadline) {
      try { const candidate = JSON.parse(await fs.readFile(ROOT + '/respond-fixture.json', 'utf8'));
        if (candidate.parent_reply_id === actualParent) { fixture = candidate; break; }
      } catch (error) { if (error.code !== 'ENOENT') throw error; }
      await new Promise(resolve => setTimeout(resolve, 500));
    }
    assert(fixture, 'No current-parent case4 fixture arrived; stop without any further request');
    assert(fixture.text.includes(fixture.deliberate_error.original));
    await page.locator('#dialogue-action').selectOption('respond');
    const correction = page.locator('details.dialogue-preferences'); await correction.locator(':scope > summary').click();
    await page.locator('#dialogue-correction').selectOption('one');
    await page.locator('#dialogue-message').fill(fixture.text);
    const q4 = await sendCase('Send reply');
    assert.equal(q4.parent_reply_id, fixture.parent_reply_id); assert.equal(q4.text, fixture.text); assert.equal(q4.correction, 'one_targeted');
    checks.push('Actual English-help turn and subsequent deliberately imperfect scene response remain bound to the actual parent, with optional one-targeted correction');
    assert.equal(requests.length, MAX_CALLS); assert.equal(consumed, MAX_CALLS); assert.equal(replies.length, MAX_CALLS);
    assert.deepEqual(learningEvidence(await facts(page)), []); assert.deepEqual(errors, []); assert.deepEqual(forbidden, []);
    if (REPAIR) { const byID = new Map((await facts(page)).map(event => [event.id, event])); for (const event of previous.events) assert.deepEqual(byID.get(event.id), event); }
    await fs.writeFile(path.join(OUTPUT, 'result.json'), JSON.stringify({ ok: true, fixture: REPAIR ? 'Two separately authorized deliberate v2 repair calls; original v1 output retained; synthetic learner/scene, no measured learning outcome' : 'Four explicitly authorized actual dialogue calls; learner questions, scene and deliberate error are synthetic; no measured learning outcome',
      actual_model_calls: consumed, tts_asr_calls: 0, source_path: SOURCE, source_sha256: crypto.createHash('sha256').update(sourceBytes).digest('hex'), source_document: document,
      checks, requests, replies, events: await facts(page), errors, forbidden }, null, 2));
    console.log(`PASS ${MAX_CALLS} actual dialogue UI cases; no TTS/ASR, no retries or fabricated practice evidence`);
  } catch (error) {
    await page.screenshot({ path: path.join(OUTPUT, 'ui-failure.png'), fullPage: true }).catch(() => {});
    await fs.writeFile(path.join(OUTPUT, 'ui-failure.txt'), await page.locator('body').innerText()).catch(() => {});
    await fs.writeFile(path.join(OUTPUT, 'ui-failure-receipt.json'), JSON.stringify({ error: String(error), consumed, requests, checks, bridgeFailure, errors, forbidden }, null, 2)).catch(() => {});
    throw error;
  } finally { await context.tracing.stop({ path: path.join(OUTPUT, 'trace.zip') }).catch(() => {}); await context.close(); await browser.close(); }
}
if (require.main === module) main().catch(error => { console.error(error); process.exitCode = 1; });
