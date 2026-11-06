// Actual app acceptance with explicitly synthetic dialogue/teaching and isolated
// browser storage. No model, main relay, personal profile or injected CSS.
const { chromium } = require(process.env.RUFREN_PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const crypto = require('node:crypto');
const { notebookResponse, openLessonTools } = require('./test-teacher-notebook.cjs');
const { navigate, assertNavigation } = require('./qa-navigation.cjs');

const APP = process.env.RUFREN_QA_URL || 'http://127.0.0.1:8091';
const RELAY = 'http://127.0.0.1:18995'; // Every request intercepted; no listener.
const OUTPUT = path.resolve(process.env.RUFREN_DIALOGUE_QA_OUTPUT || '.local/teacher-dialogue-qa');
const QUESTIONS = {
  first: 'Why does this use вас rather than ты?',
  follow: 'What if there are two people?',
  branch: 'Could you show this in a short starship conversation?',
};
const status = { enabled: true, provider: 'qa_synthetic', model: 'Synthetic dialogue fixture',
  message: 'No inference', data_sent: 'Synthetic only', max_text_bytes: 8192, max_context_bytes: 2048,
  remaining_reservations: 99, remaining_budget_micro_usd: 0, accounting: 'subscription_quota',
  lesson_flow: 'guided_v1', dialogue: 'threaded_v1' };

function dialogueLesson(request, sequence = 1) {
  const response = notebookResponse(request, sequence);
  if (request.mode !== 'teach_lesson') return response;
  response.provenance.response_id = `dialogue-lesson-${request.input.request_id}`;
  const name = sequence === 1 ? 'Alpha' : 'Beta';
  response.content.explanation = `${name}: return a polite question. Вас can address one person politely or several people; ты addresses one person informally.`;
  response.content.examples[0] = { russian: 'Хорошо́, спаси́бо. А у вас?', english: 'Good, thank you. And you?', note: 'Polite or plural address.' };
  response.content.practice[0] = { prompt: `Write a polite returned question for ${name}: And you?`, skill: 'writing',
    reference_russian: 'А у вас?', reference_english: 'And you? (polite or plural)',
    expected_response: 'А у вас?', teaching_note: 'Compare polite/plural вас with informal singular тебя in this construction.' };
  if (response.content.lesson_flow) {
    response.content.lesson_flow.steps[0].goal = `Return a polite question in ${name}`;
    response.content.lesson_flow.steps[0].orientation = 'Notice whom you are addressing before choosing a form.';
  }
  return response;
}

function dialogueResponse(request, serial) {
  const roleplay = request.config.intent === 'roleplay';
  const followup = request.text === QUESTIONS.follow;
  const replied = roleplay && request.text === 'Хорошо, спасибо. А у вас?';
  return { request: structuredClone(request), content: {
    explanation_english: roleplay ? (replied ? 'Synthetic scene: the teacher answers your returned question and closes the exchange.'
      : 'Synthetic beginner scene: greet the teacher. Meaning is available when wanted.')
      : followup ? 'Synthetic follow-up: вас can address several people as well as one person politely.'
        : 'Synthetic anchored explanation: вас is appropriate for polite address; ты is informal singular.',
    utterance: roleplay ? { russian: replied ? 'Хорошо́, спаси́бо. До свида́ния!' : 'Здра́вствуйте!',
      english: replied ? 'Good, thank you. Goodbye!' : 'Hello!', note: 'Synthetic scene, not recorded native speech.' } : null,
    next_question: roleplay && !replied ? { russian: 'Как дела́?', english: 'How are things?', note: 'One short previously taught question.' } : null,
    correction: null, scene_state: replied ? 'closed' : 'open',
  }, provenance: { provider: 'qa_synthetic', model: 'synthetic-dialogue-fixture',
    response_id: `dialogue-${serial}`, review_status: 'ai_generated_teaching',
    prompt_version: 'qa-dialogue-v1', scope: 'Synthetic browser fixture; no learner or acoustic judgment.' },
    usage: { input_tokens: 0, output_tokens: 0, estimated_micro_usd: 0, reserved_micro_usd: 0, accounting: 'subscription_quota' } };
}

function replica() {
  const relay = crypto.randomUUID(), records = [], ids = new Map();
  const canonical = value => JSON.stringify(value, (_, v) => v && typeof v === 'object' && !Array.isArray(v)
    ? Object.fromEntries(Object.entries(v).sort(([a], [b]) => a.localeCompare(b))) : v);
  return { records, exchange(request) {
    assert.equal(request.version, 5);
    assert(request.relay === null || request.relay === relay);
    assert(Number.isSafeInteger(request.after) && request.after >= 0 && request.after <= records.length);
    assert(request.events.length <= 128);
    for (const event of request.events) {
      if (ids.has(event.id)) assert.equal(canonical(ids.get(event.id)), canonical(event), 'Sync retries retain immutable content');
      else { const copy = structuredClone(event); ids.set(copy.id, copy); records.push(copy); }
    }
    const page = records.slice(request.after, request.after + 128);
    return { version: 5, relay, accepted: request.events.map(e => e.id), events: page,
      cursor: request.after + page.length, has_more: request.after + page.length < records.length };
  } };
}

async function facts(page) {
  return page.evaluate(() => window.rufrenLibrary.entries().filter(([key]) => key.startsWith('rufren.v1.event.'))
    .map(([, raw]) => JSON.parse(raw)));
}

function learningEvidence(events) {
  // Dialogue may create real presentation opportunities, but neither ordinary
  // turns nor their model replies may masquerade as completed graded practice.
  const kinds = new Set(['attempt_recorded', 'attempt_context_recorded', 'ai_assessment_recorded',
    'self_rating_recorded', 'grammar_response_recorded']);
  return events.filter(event => kinds.has(event.fact.kind)).sort((a, b) => a.id.localeCompare(b.id));
}

async function assertDurableUser(page, request) {
  const matching = (await facts(page)).filter(event => event.fact.kind === 'teacher_dialogue_user_recorded'
    && event.fact.data.request_id === request.request_id);
  assert.equal(matching.length, 1, 'A single immutable authored message exists before provider transport');
  assert.deepEqual(matching[0].fact.data, request, 'Transport uses the exact durably prepared message/context');
  assert.equal(matching[0].version, 5);
}

function assertBounded(request) {
  assert(Buffer.byteLength(request.text) <= 2048);
  assert(Buffer.byteLength(JSON.stringify(request)) <= 24576);
  assert(request.ancestors.length <= 8 && request.ancestors.length % 2 === 0);
  assert(Buffer.byteLength(JSON.stringify(request.ancestors)) <= 10240);
  if (request.anchor) assert(Buffer.byteLength(request.anchor.snapshot) <= 8192);
  assert.equal(request.omitted_count === 0, request.omitted_parent_id === null);
}

// Route setup is independent of UI selectors so companion exposure/audio checks
// can reuse the same explicit fixtures without contacting a provider.
async function fixtureRoutes(context, audit, store = replica()) {
  let lessons = 0, replies = 0;
  const namespace = audit.label || crypto.randomUUID();
  await context.route('**/*', async route => {
    const req = route.request(), url = new URL(req.url());
    if (url.origin === new URL(APP).origin && !url.pathname.startsWith('/api/')) return route.continue();
    if (url.origin !== RELAY) { audit.forbidden.push(req.url()); return route.abort('blockedbyclient'); }
    const headers = { 'Access-Control-Allow-Origin': '*', 'Access-Control-Allow-Headers': '*', 'Access-Control-Allow-Methods': 'GET,POST,OPTIONS' };
    if (audit.offline) return route.abort('internetdisconnected');
    if (req.method() === 'OPTIONS') return route.fulfill({ status: 204, headers });
    const request = req.postDataJSON(); audit.requests.push({ path: url.pathname, request });
    let body;
    if (url.pathname === '/api/teacher/status' || url.pathname === '/api/tutor/status') body = status;
    else if (url.pathname === '/v1/sync') body = store.exchange(request);
    else if (url.pathname === '/api/teacher/teach') body = dialogueLesson(request, ++lessons);
    else if (url.pathname === '/api/teacher/dialogue') {
      assertBounded(request);
      if (audit.beforeDialogue) await audit.beforeDialogue(request);
      body = dialogueResponse(request, `${namespace}-${++replies}`);
      if (audit.wrongEchoNext) { audit.wrongEchoNext = false; body.request.text = 'A different synthetic request'; }
      if (audit.holdNext) {
        audit.holdNext = false;
        await new Promise(resolve => { audit.release = resolve; audit.held?.(request); });
      }
      if (audit.failNext) { audit.failNext = false; return route.fulfill({ status: 503, headers, body: 'Synthetic unavailable provider' }); }
    } else { audit.forbidden.push(req.url()); return route.abort('blockedbyclient'); }
    return route.fulfill({ status: 200, headers, contentType: 'application/json', body: JSON.stringify(body) });
  });
  return store;
}

async function main() {
  await fs.mkdir(OUTPUT, { recursive: true });
  const browser = await chromium.launch({ headless: true });
  const store = replica(), contexts = [], checks = [], layouts = [];
  const checkpoint = message => { checks.push(message); console.log(`CHECK ${message}`); };
  const audit = { label: 'A', requests: [], forbidden: [], errors: [], offline: false };
  const create = async (label, sharedAudit) => {
    const context = await browser.newContext({ viewport: { width: 1200, height: 900 } });
    contexts.push({ label, context });
    await context.tracing.start({ screenshots: true, snapshots: true, sources: true });
    const page = await context.newPage();
    page.on('pageerror', error => sharedAudit.errors.push({ label, message: error.message }));
    await fixtureRoutes(context, sharedAudit, store);
    sharedAudit.beforeDialogue = request => assertDurableUser(page, request);
    const nav = name => navigate(page, name);
    const connect = async () => { await nav('Settings'); await page.locator('#sync-url').fill(RELAY); await page.locator('#sync-key').fill('synthetic-dialogue-no-secret'); };
    const mode = name => page.getByRole('group', { name: 'Teacher activities' }).getByRole('button', { name, exact: true }).click();
    const library = async () => { const box = page.locator('details.dialogue-library'); if (await box.getAttribute('open') === null) await box.locator(':scope > summary').click(); };
    const open = async id => { await mode('Conversations'); await library(); await page.locator(`[data-open-dialogue-leaf="${id}"]`).click();
      await page.waitForFunction(id => document.querySelector('.teacher-dialogue')?.dataset.dialogueLeaf === id, id); };
    const newQuestion = async () => { await mode('Conversations'); await library(); await page.getByRole('button', { name: 'New question', exact: true }).click(); };
    const startLesson = async index => {
      await mode('Lessons'); await openLessonTools(page);
      const panel = page.locator('details.teacher-start'); if (await panel.getAttribute('open') === null) await panel.locator(':scope > summary').click();
      await page.getByText('Ready · Synthetic dialogue fixture', { exact: true }).waitFor();
      await page.locator('#teacher-lesson').selectOption({ index });
      await page.getByRole('button', { name: 'Teach me this lesson', exact: false }).click();
      await page.locator('.teacher-lesson-current textarea').waitFor();
      return page.locator('.teacher-lesson-flow').getAttribute('data-lesson-document');
    };
    const send = async text => { await page.locator('#dialogue-message').fill(text);
      await page.getByRole('button', { name: /^Send (question|reply)$/ }).click(); };
    const replies = async () => (await facts(page)).filter(e => e.fact.kind === 'teacher_dialogue_reply_recorded');
    const waitReply = async user => {
      await page.waitForFunction(user => window.rufrenLibrary.entries().some(([key, raw]) => {
        if (!key.startsWith('rufren.v1.event.')) return false;
        const e = JSON.parse(raw); return e.fact.kind === 'teacher_dialogue_reply_recorded' && e.fact.data.request.request_id === user;
      }), user);
      return (await replies()).find(e => e.fact.data.request.request_id === user);
    };
    const sync = async () => { await nav('Settings'); const before = sharedAudit.requests.filter(r => r.path === '/v1/sync').length;
      await page.getByRole('button', { name: 'Sync now', exact: true }).click();
      await page.waitForFunction(() => document.body.innerText.includes('Your notebook is up to date with the sync library.'));
      assert(sharedAudit.requests.filter(r => r.path === '/v1/sync').length > before); };
    await page.goto(APP); await page.locator('main.main h1').waitFor(); await connect(); await nav('Teacher');
    return { context, page, nav, connect, mode, library, open, newQuestion, startLesson, send, replies, waitReply, sync };
  };
  let a, b;
  try {
    a = await create('A', audit);
    const { page } = a;
    const alpha = await a.startLesson(0);
    await page.locator('.teacher-lesson-current textarea').fill('Unfinished Alpha lesson response');
    const baseline = learningEvidence(await facts(page)); assert.equal(baseline.length, 0);
    await page.locator('.ask-about-this:visible').click();
    await page.locator('#dialogue-message').fill(QUESTIONS.first);
    const firstThread = await page.locator('.teacher-dialogue').getAttribute('data-dialogue-thread');
    await page.waitForFunction(() => !!navigator.serviceWorker.controller);
    audit.offline = true; await a.context.setOffline(true); await page.reload(); await page.locator('main.main h1').waitFor(); await a.nav('Teacher');
    assert.equal(await page.locator('#dialogue-message').inputValue(), QUESTIONS.first);
    assert.equal(await page.locator('.teacher-dialogue').getAttribute('data-dialogue-thread'), firstThread);
    const callsBeforeSave = audit.requests.filter(r => r.path === '/api/teacher/dialogue').length;
    await page.getByRole('button', { name: 'Save message for later', exact: true }).click();
    const firstUser = (await facts(page)).find(e => e.fact.kind === 'teacher_dialogue_user_recorded');
    assert(firstUser); assert.equal(firstUser.fact.data.request_id, firstThread);
    assert.equal(firstUser.fact.data.anchor.document_id, alpha);
    assert.equal(firstUser.fact.data.anchor.focus.kind, 'step');
    assert(firstUser.fact.data.anchor.snapshot.includes('вас'));
    assert.equal(audit.requests.filter(r => r.path === '/api/teacher/dialogue').length, callsBeforeSave);
    await page.reload(); await page.locator('main.main h1').waitFor(); await a.nav('Teacher');
    await page.getByRole('button', { name: 'Retry reply', exact: true }).waitFor();
    assert.equal((await facts(page)).filter(e => e.fact.kind === 'teacher_dialogue_user_recorded').length, 1);
    audit.offline = false; await a.context.setOffline(false); await a.connect(); await a.nav('Teacher');
    await page.getByRole('button', { name: 'Retry reply', exact: true }).click(); await a.waitReply(firstThread);
    await page.locator('#dialogue-message').waitFor();
    const firstReply = await page.locator('article[data-dialogue-role="assistant"]').getAttribute('data-message-id');
    checkpoint('Anchored authored question/draft and exact source survive offline reload; save commits once without transport; explicit retry uses the same immutable message');

    await page.getByRole('button', { name: 'Return to the source lesson or feedback', exact: true }).click();
    assert.equal(await page.locator('.teacher-lesson-flow').getAttribute('data-lesson-document'), alpha);
    assert.equal(await page.locator('.teacher-lesson-current textarea').inputValue(), 'Unfinished Alpha lesson response');
    const beta = await a.startLesson(1); assert.notEqual(alpha, beta);
    await page.locator('.teacher-lesson-current textarea').fill('Unfinished Beta lesson response');
    await a.open(firstReply); await page.locator('#dialogue-message').waitFor();
    let heldRequest;
    const held = new Promise(resolve => { audit.held = request => { heldRequest = request; resolve(); }; });
    audit.holdNext = true; await a.send(QUESTIONS.follow);
    let heldTimer;
    try { await Promise.race([held, new Promise((_, reject) => { heldTimer = setTimeout(() => reject(Error('Expected synthetic held dialogue request did not arrive')), 15000); })]); }
    finally { clearTimeout(heldTimer); }
    assert.equal(heldRequest.anchor.document_id, alpha);
    assert.equal(heldRequest.parent_reply_id, firstReply);
    assert.equal(heldRequest.ancestors.length, 2);
    assert.equal(heldRequest.ancestors[0].text, QUESTIONS.first);
    assert(!JSON.stringify(heldRequest).includes('Unfinished Beta lesson response'));
    await a.newQuestion(); await page.locator('#dialogue-message').fill('Unsent unrelated Beta question');
    const unrelatedThread = await page.locator('.teacher-dialogue').getAttribute('data-dialogue-thread');
    audit.release(); await a.waitReply(heldRequest.request_id);
    assert.equal(await page.locator('.teacher-dialogue').getAttribute('data-dialogue-thread'), unrelatedThread);
    assert.equal(await page.locator('#dialogue-message').inputValue(), 'Unsent unrelated Beta question');
    assert(await page.locator('#dialogue-message').evaluate(node => document.activeElement === node), 'Late reply must not steal focus from another conversation draft');
    checkpoint('Follow-up contains exact parent/source and no sibling draft; held reply archives without stealing the newer conversation or its unsent message');

    // Locate immutable reply identity through the actual rendered saved branch.
    await a.mode('Lessons');
    assert.equal(await page.locator('.teacher-lesson-flow').getAttribute('data-lesson-document'), beta);
    assert.equal(await page.locator('.teacher-lesson-current textarea').inputValue(), 'Unfinished Beta lesson response');
    await a.mode('Conversations'); await a.library();
    const userID = heldRequest.request_id;
    const replyIDs = await page.locator('[data-open-dialogue-leaf]').evaluateAll(nodes => nodes.map(n => n.dataset.openDialogueLeaf));
    let followReply;
    for (const id of replyIDs) { await a.open(id); if (await page.locator(`article[data-message-id="${userID}"]`).count()) { followReply = id; break; } }
    assert(followReply, 'Saved branch has the exact held user message');
    await page.locator(`article[data-message-id="${firstReply}"]`).getByRole('button', { name: 'Continue from this reply', exact: true }).click();
    await a.send(QUESTIONS.branch);
    const branchRequest = audit.requests.filter(r => r.path === '/api/teacher/dialogue').at(-1).request;
    await a.waitReply(branchRequest.request_id); await page.locator('#dialogue-message').waitFor();
    assert.equal(branchRequest.parent_reply_id, firstReply);
    assert.equal(branchRequest.ancestors.length, 2);
    assert(!JSON.stringify(branchRequest.ancestors).includes(QUESTIONS.follow));
    for (let i = 0; i < 5; i++) {
      await a.send(`Explain one more precise contrast, turn ${i + 1}.`);
      const req = audit.requests.filter(r => r.path === '/api/teacher/dialogue').at(-1).request;
      await a.waitReply(req.request_id); await page.locator('#dialogue-message').waitFor();
      assert.equal(req.anchor.document_id, alpha); assertBounded(req);
      assert(!JSON.stringify(req.ancestors).includes(QUESTIONS.follow));
    }
    const bounded = audit.requests.filter(r => r.path === '/api/teacher/dialogue').at(-1).request;
    assert.equal(bounded.ancestors.length, 8); assert(bounded.omitted_count > 0 && bounded.omitted_parent_id);
    checkpoint('Explicit branch from an older reply excludes the sibling; long context sends at most four complete pairs and names omitted ancestry while all messages remain archived');

    await a.newQuestion();
    const postsBeforeFault = audit.requests.filter(r => r.path === '/api/teacher/dialogue').length;
    await page.evaluate(() => {
      const original = window.rufrenLibrary.write;
      window.qaDialogueWriteOriginal = original;
      window.qaDialogueWriteFailure = true;
      window.rufrenLibrary.write = (key, value) => {
        if (window.qaDialogueWriteFailure && key.endsWith('meta.teacher_dialogue_state')) throw Error('Synthetic isolated metadata write failure');
        return original(key, value);
      };
    });
    const visibleEdit = 'Keep this visible edit through a synthetic save failure.';
    await page.locator('#dialogue-message').fill(visibleEdit);
    await page.locator('.dialogue-save-problem').waitFor();
    assert.equal(await page.locator('#dialogue-message').inputValue(), visibleEdit);
    assert(await page.locator('.dialogue-composer .actions .primary').isDisabled());
    assert.equal(audit.requests.filter(r => r.path === '/api/teacher/dialogue').length, postsBeforeFault);
    await page.evaluate(() => { window.qaDialogueWriteFailure = false; });
    await page.getByRole('button', { name: 'Retry saving this edit', exact: true }).click();
    await page.locator('.dialogue-save-problem').waitFor({ state: 'detached' });
    assert.equal(await page.locator('#dialogue-message').inputValue(), visibleEdit);
    const savedEdit = await page.evaluate(() => { const local = JSON.parse(window.rufrenLibrary.get('rufren.v1.meta.teacher_dialogue_state')); return local.drafts[local.active].text; });
    assert.equal(savedEdit, visibleEdit);
    await page.evaluate(() => { window.rufrenLibrary.write = window.qaDialogueWriteOriginal; });
    const oversized = 'я'.repeat(1025); // 1,025 chars but 2,050 UTF-8 bytes: above the actual byte limit.
    await page.locator('#dialogue-message').fill(oversized);
    await page.locator('.dialogue-save-problem').waitFor();
    assert.equal(await page.locator('#dialogue-message').inputValue(), oversized);
    assert(await page.locator('.dialogue-composer .actions .primary').isDisabled());
    assert.equal(audit.requests.filter(r => r.path === '/api/teacher/dialogue').length, postsBeforeFault);
    await page.locator('#dialogue-message').fill('Shortened Russian question: почему?');
    await page.locator('.dialogue-save-problem').waitFor({ state: 'detached' });
    assert(await page.locator('.dialogue-composer .actions .primary').isEnabled());
    checkpoint('Failed metadata writes and UTF8 byte overflow retain visible typing, block Send and make no POST; explicit retry/shortening recovers the exact saved edit');
    audit.wrongEchoNext = true; const replyCount = (await a.replies()).length;
    await a.send('Keep this exact question through a wrong-echo response.');
    await page.getByRole('button', { name: 'Retry reply', exact: true }).waitFor();
    assert.equal((await a.replies()).length, replyCount, 'Wrong-request response cannot enter the dialogue graph');
    const rejectedRequest = audit.requests.filter(r => r.path === '/api/teacher/dialogue').at(-1).request;
    await page.getByRole('button', { name: 'Retry reply', exact: true }).click(); await a.waitReply(rejectedRequest.request_id);
    const retriedRequest = audit.requests.filter(r => r.path === '/api/teacher/dialogue').at(-1).request;
    assert.deepEqual(retriedRequest, rejectedRequest);
    assert.equal((await facts(page)).filter(e => e.fact.kind === 'teacher_dialogue_user_recorded' && e.fact.data.request_id === rejectedRequest.request_id).length, 1);
    checkpoint('Wrong-request echo is rejected; explicit retry preserves authored identity/context without duplicating the learner message');

    await a.newQuestion(); await page.locator('#dialogue-message').fill('An expendable local draft, never sent.');
    const committedBeforeDiscard = (await facts(page)).filter(e => e.fact.kind.startsWith('teacher_dialogue_'));
    const callsBeforeDiscard = audit.requests.filter(r => r.path === '/api/teacher/dialogue').length;
    await page.getByRole('button', { name: 'Discard draft…', exact: true }).click();
    await page.getByRole('button', { name: 'Keep writing', exact: true }).click();
    assert.equal(await page.locator('#dialogue-message').inputValue(), 'An expendable local draft, never sent.');
    await page.getByRole('button', { name: 'Discard draft…', exact: true }).click();
    await page.getByRole('button', { name: 'Discard this draft', exact: true }).click();
    await page.locator('#dialogue-message').waitFor({ state: 'detached' });
    assert.deepEqual((await facts(page)).filter(e => e.fact.kind.startsWith('teacher_dialogue_')), committedBeforeDiscard);
    assert.equal(audit.requests.filter(r => r.path === '/api/teacher/dialogue').length, callsBeforeDiscard);
    checkpoint('Discard requires an explicit choice; Keep writing preserves typing and confirmed discard removes only the unsent draft, not any committed message');

    await a.library(); const scene = page.locator('details.dialogue-scene-setup'); await scene.locator(':scope > summary').click();
    await page.locator('#dialogue-scene').fill('A beginner greets a patient Russian-speaking starship librarian. Use the taught greeting and How are things? exchange, with English help.');
    await page.getByRole('button', { name: 'Prepare this scene', exact: true }).click();
    await page.getByRole('button', { name: 'Start scene', exact: true }).click();
    const sceneRequest = audit.requests.filter(r => r.path === '/api/teacher/dialogue').at(-1).request;
    await a.waitReply(sceneRequest.request_id); await page.locator('#dialogue-action').waitFor();
    assert.equal(sceneRequest.action, 'start_scene'); assert.equal(sceneRequest.text, '');
    assert.equal(sceneRequest.config.intent, 'roleplay'); assert.equal(sceneRequest.anchor, null);
    await page.locator('#dialogue-action').selectOption('ask'); await a.send('What does Как дела mean? Please give me an English meaning.');
    let sceneHelp = audit.requests.filter(r => r.path === '/api/teacher/dialogue').at(-1).request;
    await a.waitReply(sceneHelp.request_id); await page.locator('#dialogue-action').waitFor();
    assert.equal(sceneHelp.action, 'ask'); assert.equal(sceneHelp.thread_id, sceneRequest.thread_id);
    await page.locator('#dialogue-action').selectOption('respond'); await a.send('Хорошо, спасибо. А у вас?');
    const spokenText = audit.requests.filter(r => r.path === '/api/teacher/dialogue').at(-1).request;
    const closedReply = await a.waitReply(spokenText.request_id); await page.locator('#dialogue-message').waitFor();
    assert.equal(closedReply.fact.data.content.scene_state, 'closed');
    await page.getByText('A natural place to pause. You can continue if you want to.', { exact: true }).waitFor();
    assert.equal(spokenText.action, 'respond'); assert.equal(spokenText.config.topic, sceneRequest.config.topic);
    checkpoint('Beginner role-play starts without a fictional learner utterance, permits English clarification, keeps scene roles through replies and reaches an explicit natural pause');

    for (const [name, width, height] of [['phone', 390, 844], ['tablet', 820, 1180], ['desktop', 1200, 900]]) {
      await page.setViewportSize({ width, height }); await a.nav('Teacher'); await assertNavigation(page, layouts);
      await page.locator('#dialogue-message').fill(`Unsent ${name} conversation draft`);
      await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
      assert(await page.locator('#dialogue-message').evaluate(node => document.activeElement === node), 'Ordinary input rerenders retain typing focus');
      if (width <= 760) { assert.equal(await page.locator('#navigation-toggle').getAttribute('aria-expanded'), 'false');
        const header = await page.locator('.navigation-heading').boundingBox(); assert(header.height <= 84); }
      await page.screenshot({ path: path.join(OUTPUT, `${name}-conversation.png`), fullPage: true });
      await page.evaluate(() => scrollTo(0, 0)); await page.screenshot({ path: path.join(OUTPUT, `${name}-viewport.png`) });
    }
    assert.deepEqual(learningEvidence(await facts(page)), baseline);
    checkpoint('Actual 390/820/desktop shell has no overflow; compact Menu keyboard Escape restores focus, destination focuses heading, input rerenders do not alter navigation or steal focus; dialogue creates no Attempt/context/assessment');
    const sceneLeaf = await page.locator('.teacher-dialogue').getAttribute('data-dialogue-leaf');
    await a.library();
    const expectedDialogues = {
      protocol: 5, scope: 'Synthetic authored turns and exact UI branch/source reconstruction; no audio originals',
      source_document: alpha, other_lesson: beta, selected_thread: sceneRequest.thread_id, selected_leaf: sceneLeaf,
      selected_messages: await page.locator('.teacher-dialogue article[data-message-id]').evaluateAll(nodes => nodes.map(node => ({ id: node.dataset.messageId, role: node.dataset.dialogueRole, text: node.innerText }))),
      visible_saved_leaves: await page.locator('[data-open-dialogue-leaf]').evaluateAll(nodes => nodes.map(node => node.dataset.openDialogueLeaf).sort()),
      first_user: firstThread, first_reply: firstReply, followup_user: heldRequest.request_id, followup_reply: followReply,
    };
    await a.sync();
    const auditB = { label: 'B', requests: [], forbidden: [], errors: [] };
    b = await create('B', auditB); await b.sync(); await b.nav('Teacher'); await b.mode('Conversations');
    await b.open(sceneLeaf); await b.page.locator(`article[data-message-id="${sceneRequest.request_id}"]`).waitFor();
    const bEvents = new Map((await facts(b.page)).map(e => [e.id, e]));
    for (const event of await facts(page)) assert.deepEqual(bEvents.get(event.id), event, 'Empty app retains every exact synced source/message fact');
    assert.deepEqual(learningEvidence(await facts(b.page)), baseline);
    assert.equal(await b.page.locator('#dialogue-message').inputValue(), '', 'Unsent local draft is not silently copied between workspaces/devices');
    await b.page.waitForFunction(() => !!navigator.serviceWorker.controller);
    auditB.offline = true; await b.context.setOffline(true); await b.page.reload(); await b.page.locator('main.main h1').waitFor(); await b.nav('Teacher');
    await b.page.locator(`article[data-message-id="${sceneRequest.request_id}"]`).waitFor();
    checkpoint('Synced immutable conversations reconstruct in an independent empty app; local unsent draft stays local and saved branch remains readable after offline reload');
    assert.deepEqual(audit.errors, []); assert.deepEqual(audit.forbidden, []); assert.deepEqual(auditB.errors, []); assert.deepEqual(auditB.forbidden, []);
    const result = { ok: true, fixture: 'Synthetic Teacher/dialogue and in-memory protocol5 replica; no inference or main services', checks, layouts,
      requests: [...audit.requests, ...auditB.requests], events: await facts(page), replica_events: store.records.length, errors: [...audit.errors, ...auditB.errors], forbidden: [...audit.forbidden, ...auditB.forbidden] };
    await fs.writeFile(path.join(OUTPUT, 'result.json'), JSON.stringify(result, null, 2));
    await fs.writeFile(path.join(OUTPUT, 'library.json'), JSON.stringify({ events: Object.fromEntries(result.events.map(event => [event.id, event])) }, null, 2));
    await fs.writeFile(path.join(OUTPUT, 'expected-dialogues.json'), JSON.stringify(expectedDialogues, null, 2));
    console.log('PASS anchored durable dialogue, exact context/branches, offline retry, beginner scene, evidence neutrality and actual responsive navigation');
  } catch (error) {
    if (a) { await a.page.screenshot({ path: path.join(OUTPUT, 'failure.png'), fullPage: true }).catch(() => {});
      await fs.writeFile(path.join(OUTPUT, 'failure.txt'), await a.page.locator('body').innerText()).catch(() => {}); }
    await fs.writeFile(path.join(OUTPUT, 'failure-receipt.json'), JSON.stringify({ error: String(error), checks, audit,
      events: a ? await facts(a.page).catch(() => []) : [] }, (_, value) => typeof value === 'function' ? undefined : value, 2)).catch(() => {});
    throw error;
  } finally {
    for (const { label, context } of contexts) { await context.tracing.stop({ path: path.join(OUTPUT, `trace-${label}.zip`) }).catch(() => {}); await context.close(); }
    await browser.close();
  }
}

module.exports = { APP, RELAY, OUTPUT, QUESTIONS, status, dialogueResponse, replica, facts, assertBounded,
  fixtureRoutes, navigate, assertNavigation, dialogueLesson, notebookResponse, openLessonTools,
  learningEvidence, assertDurableUser };

if (require.main === module) main().catch(error => { console.error(error); process.exitCode = 1; });
