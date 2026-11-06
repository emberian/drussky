// Exact-task delayed-return browser acceptance. The default journey uses synthetic
// teaching/ASR/voices/replication; the application, storage and UI run normally.
// Optional real-source mode imports an untouched saved lesson. Learner/time stay
// synthetic; its separate one-call live proxy requires the explicit root grant flag.
const { chromium } = require(process.env.RUFREN_PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs/promises');
const path = require('node:path');
const { CURRENT_PROTOCOL } = require('./qa-current-protocol.cjs');
const { navigate } = require('./qa-navigation.cjs');
const { teacherResponse, wavFixture, generatedVoice, recognizedWords } = require('./test-teacher-oral.cjs');
const { openLessonTools: openLegacyLessonTools } = require('./test-teacher-notebook.cjs');

async function openLessonTools(page) {
  const session = page.locator('.teacher-start > summary');
  if (!await session.count()) return openLegacyLessonTools(page);
  if (!await session.evaluate(node => node.closest('details')?.open)) await session.click();
}

const APP = process.env.RUFREN_QA_URL || 'http://127.0.0.1:8091';
const RELAY = 'http://127.0.0.1:18993'; // Intercepted; never starts or contacts a relay.
const OUTPUT = path.resolve(process.env.RUFREN_REVISIT_QA_OUTPUT || '.local/teacher-revisit-qa');
const DAY = 86400000;
const SEED_TIME = Date.UTC(2026, 10, 1, 12);
const REVIEW_TIME = SEED_TIME + DAY + 1000;
const hash = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const TASKS = [
  { skill: 'reading', prompt: 'Which speaker offers help? Answer in English.',
    reference_russian: '— Ва́м помо́чь? — Да, пожа́луйста.',
    reference_english: 'The first speaker offers help; the second accepts.',
    expected_response: 'The first speaker.', teaching_note: 'The offer belongs to the first speaker.' },
  { skill: 'writing', prompt: 'Ask politely in Russian whether this seat is free. Do not introduce yourself.',
    reference_russian: 'Скажи́те, пожа́луйста, э́то ме́сто свобо́дно?',
    reference_english: 'Excuse me, is this seat free?',
    expected_response: 'Скажите, пожалуйста, это место свободно?', teaching_note: 'Accept suitable polite alternatives.' },
  { skill: 'listening', prompt: 'Listen and answer in English: how many tickets were requested?',
    reference_russian: 'Два биле́та, пожа́луйста.', reference_english: 'Two tickets, please.',
    expected_response: 'Two.', teaching_note: 'Focus on the requested number.' },
  { skill: 'speaking', prompt: 'Offer tea politely to one person in Russian. Save your spoken response.',
    reference_russian: 'Хоти́те ча́ю?', reference_english: 'Would you like some tea?',
    expected_response: 'Хотите чаю?', teaching_note: 'Accept a suitable spoken offer; text feedback is not acoustic assessment.' },
];

function revisitResponse(request, serial = 1, verdict = 'uncertain') {
  const response = teacherResponse(request, `revisit-${serial}`);
  response.provenance.model = 'synthetic-exact-task-fixture';
  response.provenance.prompt_version = 'qa-revisit-v1';
  response.provenance.response_id = `revisit-${serial}`;
  if (request.mode === 'answer_feedback') {
    response.content.feedback.verdict = verdict;
    response.content.explanation = 'Synthetic exact-task language opinion, not measured learning or pronunciation.';
    return response;
  }
  response.content = {
    explanation: 'Four synthetic questions with distinct requests and model references.',
    examples: [{ russian: 'Здра́вствуйте!', english: 'Hello!', note: 'Synthetic orientation example.' }],
    practice: TASKS.map(task => ({ ...task })), feedback: null, caveats: [],
    next_step: 'Return to the exact questions another day, with support when wanted.',
  };
  if (request.input.lesson_flow === 'guided_v1') response.content.lesson_flow = {
    steps: TASKS.map((task, index) => ({ id: `task-${index}`, goal: `Try ${task.skill}`,
      orientation: 'Answer the current question. Open support only when you want it.',
      examples: [], practice_index: index })),
  };
  return response;
}

function freshTone(frequency) {
  const base = wavFixture(), wav = Buffer.from(base.wav), pcm = Buffer.alloc(base.frames * 4);
  for (let i = 0; i < base.frames; i++) {
    wav.writeInt16LE(Math.round(5500 * Math.sin(2 * Math.PI * frequency * i / 24000)), 44 + 2 * i);
    pcm.writeFloatLE(wav.readInt16LE(44 + 2 * i) / 32768, i * 4);
  }
  return { wav, frames: base.frames, hash: hash(wav), pcmHash: hash(pcm) };
}
const TONES = { seed: wavFixture(), review: freshTone(330), later: freshTone(440) };
assert.notEqual(TONES.seed.hash, TONES.review.hash);

function syntheticReplica() {
  const relay = crypto.randomUUID(), records = [], byID = new Map();
  const canonical = value => JSON.stringify(value, (_, v) => v && typeof v === 'object' && !Array.isArray(v)
    ? Object.fromEntries(Object.entries(v).sort(([a], [b]) => a.localeCompare(b))) : v);
  return {
    records,
    exchange(request) {
      assert.equal(request.version, CURRENT_PROTOCOL);
      assert(request.relay === null || request.relay === relay);
      assert(Number.isSafeInteger(request.after) && request.after >= 0 && request.after <= records.length);
      assert(request.events.length <= 128);
      for (const event of request.events) {
        if (byID.has(event.id)) assert.equal(canonical(byID.get(event.id)), canonical(event), 'Duplicate event identity must preserve all bytes/fields');
        else { const saved = structuredClone(event); byID.set(event.id, saved); records.push(saved); }
      }
      const page = records.slice(request.after, request.after + 128);
      return { version: CURRENT_PROTOCOL, relay, accepted: request.events.map(event => event.id), events: page,
        cursor: request.after + page.length, has_more: request.after + page.length < records.length };
    },
  };
}

async function installClock(context, time) {
  await context.addInitScript(time => {
    // Browser-only clock; Node/service time and all system settings stay unchanged.
    Date.now = () => Number(sessionStorage.getItem('qa.revisit.clock') ?? time);
    window.qaRevisitClock = value => sessionStorage.setItem('qa.revisit.clock', String(value));
  }, time);
}

async function facts(page) {
  return page.evaluate(() => window.rufrenLibrary.entries().filter(([key]) => key.startsWith('rufren.v1.event.'))
    .map(([, raw]) => JSON.parse(raw)));
}

function fixtureRoutes(replica, audit) {
  let serial = 0;
  return async (context, label) => {
    await context.route('**/*', async route => {
      const req = route.request(), url = new URL(req.url());
      if (url.origin === new URL(APP).origin && !url.pathname.startsWith('/api/')) return route.continue();
      if (url.origin !== RELAY) { audit.forbidden.push({ label, url: req.url() }); return route.abort('blockedbyclient'); }
      const headers = { 'Access-Control-Allow-Origin': '*', 'Access-Control-Allow-Headers': '*', 'Access-Control-Allow-Methods': 'GET,POST,OPTIONS' };
      if (req.method() === 'OPTIONS') return route.fulfill({ status: 204, headers });
      const request = req.postDataJSON(); audit.requests.push({ label, path: url.pathname, request });
      let body;
      if (url.pathname === '/v1/sync') body = replica.exchange(request);
      else if (url.pathname === '/api/teacher/status' || url.pathname === '/api/tutor/status') body = {
        enabled: true, provider: 'qa_synthetic', model: 'Synthetic exact-task fixture', message: 'No inference',
        data_sent: 'Synthetic only', max_text_bytes: 8192, max_context_bytes: 8192,
        remaining_reservations: 99, remaining_budget_micro_usd: 0, accounting: 'subscription_quota', lesson_flow: 'guided_v1',
      };
      else if (url.pathname === '/api/teacher/teach') {
        if (request.mode === 'answer_feedback' && request.input.exercise_id?.startsWith('teacher:revisit:v1:') && audit.captureReview) {
          await audit.captureReview(req, request);
          if (process.env.RUFREN_LIVE_REVISIT_FEEDBACK === 'approved') {
            const bridgePath=path.resolve(process.env.RUFREN_REVISIT_LIVE_BRIDGE||'.local/teacher-revisit-live/live-bridge.cjs');
            const bridge = audit.liveBridge ||= await require(bridgePath).makeLiveRevisitBridge({output:OUTPUT});
            const result = await bridge(req.postData());
            return route.fulfill({ status: result.status, contentType: 'application/json', headers, body: result.body });
          }
        }
        const verdict = audit.verdict[label] || 'uncertain'; body = revisitResponse(request, ++serial, verdict);
      }
      else if (url.pathname === '/api/speech/generate') body = generatedVoice(request);
      else if (url.pathname === '/api/speech/transcribe') {
        const tone = Object.values(TONES).find(value => value.hash === request.audio_sha256);
        assert(tone, 'ASR must receive one exact known fixture original');
        body = recognizedWords(request, tone, 'Хотите чаю?');
      }
      else { audit.forbidden.push({ label, url: req.url() }); return route.abort('blockedbyclient'); }
      if(request?.mode==='answer_feedback'&&request.input.exercise_id?.startsWith('teacher:revisit:v1:')&&audit.holdNextFeedback?.[label]){
        audit.holdNextFeedback[label]=false;
        await new Promise(resolve=>{(audit.releaseFeedback||={})[label]=resolve;audit.holdNotify?.[label]?.();});
      }
      await route.fulfill({ status: 200, contentType: 'application/json', headers, body: JSON.stringify(body) });
    });
  };
}

async function createDriver(browser, label, time, routeInstaller, audit, taskSpecs = TASKS) {
  const context = await browser.newContext({ viewport: { width: 1100, height: 900 } });
  await installClock(context, time); await routeInstaller(context, label);
  await context.tracing.start({ screenshots: true, snapshots: true, sources: true });
  const page = await context.newPage();
  page.on('pageerror', error => audit.errors.push({ label, message: error.message }));
  const nav = name => navigate(page, name);
  const connect = async () => { await nav('Settings'); await page.locator('#sync-url').fill(RELAY); await page.locator('#sync-key').fill('synthetic-revisit-no-secret'); };
  const now = async value => {
    await page.evaluate(value => window.qaRevisitClock(value), value);
    // Production Today refreshes its idle clock every 30 seconds. A synthetic
    // multi-day clock jump must remount it, rather than wait for that real timer.
    await nav('Settings'); await nav('Today');
  };
  const allRows = async () => {
    // Today keeps the full return catalogue behind one disclosure; the bounded
    // daily session is a different workflow from this exact-task regression.
    const browse = page.locator('details').filter({ has: page.locator(':scope > summary').filter({ hasText: 'Browse your saved Teacher returns' }) });
    if (await browse.count() && !await browse.evaluate(node => node.open)) await browse.locator(':scope > summary').click();
    const more = page.getByRole('button', { name: 'Show all planned returns', exact: true });
    if (await more.count()) await more.click();
    return page.locator('.teacher-return-row');
  };
  const row = index => page.locator(`.teacher-return-row[data-review-index="${index}"]`);
  const due = async index => { await nav('Today'); await allRows(); return Number(await row(index).getAttribute('data-review-due')); };
  const screen = page.locator('.teacher-return-screen');
  const open = async index => {
    await nav('Today'); await allRows(); await row(index).getByRole('button').click();
    await screen.waitFor(); assert.equal(await screen.getAttribute('data-active-review-index'), String(index));
    await page.waitForFunction(()=>{
      const heading=[...document.querySelectorAll('.page-heading h1')].find(e=>e.textContent==='A fresh try');
      if(!heading||document.activeElement!==heading)return false;
      const r=heading.getBoundingClientRect();return r.top>=0&&r.bottom<=innerHeight;
    });
    assert((await screen.locator('.teacher-exercise').innerText()).includes(taskSpecs[index].prompt));
    return screen.getAttribute('data-active-review-session');
  };
  const currentAttempt = async () => {
    const session = await screen.getAttribute('data-active-review-session');
    const all = await facts(page), event = all.find(e => e.fact.kind === 'attempt_recorded' && e.fact.data.session === session);
    assert(event, 'Current return must have one durable attempt');
    const contexts = all.filter(e => e.fact.kind === 'attempt_context_recorded' && e.fact.data.attempt === event.id);
    assert.equal(contexts.length, 1); return { event, context: contexts[0], all };
  };
  const saveText = async (text, verdict = 'uncertain') => {
    audit.verdict[label] = verdict;
    // A resume intentionally focuses its heading on the mount frame. Settle that
    // entry before testing that a later ordinary input rerender preserves focus.
    await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
    const field=screen.locator('.teacher-exercise textarea');await field.fill(text);
    await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
    assert(await field.evaluate(node=>document.activeElement===node),'Ordinary review typing must not steal input focus');
    await screen.getByRole('button', { name: 'Save & discuss this return', exact: false }).click();
    await screen.locator('.teacher-return-feedback').waitFor();
    const result = await currentAttempt(); assert.equal(result.event.fact.data.self_rating, 'not_reported');
    assert.equal(result.event.fact.data.response, text); return result;
  };
  const finish = () => screen.getByRole('button', { name: 'Finish & return to Today', exact: true }).click();
  const sync = async () => {
    await connect();
    const response = page.waitForResponse(r => r.url().endsWith('/v1/sync') && r.request().method() === 'POST');
    await page.getByRole('button', { name: 'Sync now', exact: true }).click(); await response;
    await page.waitForFunction(() => [...document.querySelectorAll('button')].some(b => b.textContent === 'Sync now' && !b.disabled));
    await nav('Today');
  };
  await page.goto(APP); await page.locator('main.main h1').waitFor(); await connect();
  return { context, page, label, nav, connect, now, allRows, row, due, screen, open, saveText, currentAttempt, finish, sync };
}

async function seedGuided(driver, audit, indices = [0, 1, 2, 3]) {
  const { page, nav } = driver;
  await nav('Teacher');
  await openLessonTools(page);
  await page.getByText('Ready · Synthetic exact-task fixture', { exact: true }).waitFor({ state: 'attached' });
  await page.getByRole('button', { name: 'Teach me this lesson', exact: false }).click();
  const flow = page.locator('.teacher-lesson-flow'); await flow.waitFor();
  const document = await flow.getAttribute('data-lesson-document');
  const answers = ['The first speaker.', TASKS[1].expected_response, 'Two.'];
  const jump = async index => {
    const legacySelect = flow.getByRole('combobox', { name: 'Go to a step', exact: true });
    if (await legacySelect.count()) await legacySelect.selectOption(String(index));
    else {
      const toggle=flow.getByRole('button',{name:'Lesson steps',exact:false});
      if(await toggle.count() && await toggle.getAttribute('aria-expanded')!=='true') await toggle.click();
      await flow.getByRole('navigation', { name: 'Lesson outline', exact: true }).locator('.teacher-step-row').nth(index).click();
    }
    await page.waitForFunction(index => document.querySelector('.teacher-lesson-flow')?.dataset.lessonStep === String(index), index); };
  for (const index of indices) {
    await jump(index); const exercise = page.locator('.teacher-lesson-current .teacher-exercise');
    if (index === 3) {
      await exercise.getByRole('button', { name: 'Open recorder for this exercise', exact: true }).click();
      await exercise.locator('#audio-import-attempt').setInputFiles({ name: 'synthetic-guided-tone.wav', mimeType: 'audio/wav', buffer: TONES.seed.wav });
      const keep = exercise.getByRole('button', { name: 'Keep this spoken response', exact: false });
      await page.waitForFunction(() => [...document.querySelectorAll('.teacher-lesson-current button')].some(b => b.textContent.includes('Keep this spoken response') && !b.disabled));
      await keep.click(); await exercise.locator('.spoken-take').waitFor();
    } else {
      if (index === 2) {
        await exercise.getByRole('button', { name: 'Generate reference audio', exact: true }).click();
        await page.waitForFunction(() => document.querySelector('.teacher-lesson-current [data-practice-rate="1"]')?.disabled === false);
        await exercise.locator('[data-practice-rate="1"]').click();
        await page.waitForFunction(() => document.querySelector('.teacher-lesson-current audio')?.currentTime > .02);
      }
      await exercise.locator('textarea').fill(answers[index]); audit.verdict[driver.label] = 'uncertain';
      await exercise.getByRole('button', { name: index === 2 ? 'Discuss what I heard' : 'Discuss my answer', exact: false }).click();
      await page.getByRole('heading', { name: 'Working with your response', exact: true }).waitFor();
      await page.getByRole('button', { name: 'Return to this lesson & exercise', exact: false }).click(); await flow.waitFor();
    }
  }
  const seeded = (await facts(page)).filter(e => e.fact.kind === 'attempt_recorded' && e.fact.data.policy.startsWith('teacher-guided-'));
  assert.equal(seeded.length, indices.length); assert(seeded.every(e => e.fact.data.self_rating === 'not_reported'));
  return { document, seeded, jump, flow };
}

async function captureScreen(driver, audit, name) {
  const previous=driver.page.viewportSize();audit.layouts ||= [];
  for(const [form,width,height]of[['tablet',820,1180],['phone',390,844]]){
    await driver.page.setViewportSize({width,height});
    await driver.page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
    assert(await driver.page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
    await driver.page.screenshot({path:path.join(OUTPUT,`${name}-${form}.png`)});
    await driver.page.screenshot({path:path.join(OUTPUT,`${name}-${form}-full.png`),fullPage:true});
    audit.layouts.push(await driver.page.evaluate(({name,form})=>({name,form,width:innerWidth,height:innerHeight,scrollY,
      heading:document.querySelector('.page-heading h1')?.getBoundingClientRect().toJSON(),
      screen:document.querySelector('.teacher-return-screen')?.getBoundingClientRect().toJSON(),
      response:document.querySelector('.teacher-return-screen textarea')?.getBoundingClientRect().toJSON()}),{name,form}));
  }
  await driver.page.setViewportSize(previous);
}

async function consumerJourney(browser, audit, checks, drivers) {
  const replica = syntheticReplica(), routes = fixtureRoutes(replica, audit);
  const a = await createDriver(browser, 'A', SEED_TIME, routes, audit); drivers.push(a);
  const source = await seedGuided(a, audit);
  await source.jump(0); await a.page.locator('.teacher-lesson-current textarea').fill('Original unfinished lesson draft');
  const originalDraft = await a.page.evaluate(document => JSON.parse(window.rufrenLibrary.get(`rufren.v1.meta.teacher_practice:${document}:0`)), source.document);
  await a.now(SEED_TIME);
  await a.page.getByRole('button',{name:'Choose my next step',exact:false}).first().click();
  assert.equal(await a.page.locator('.practice-panel').count(),0,'Generated exercise references must not become generic wrong-skill passage practice');
  assert.equal(await a.screen.count(),0,'Completed guided work is scheduled later, not an immediately due review');
  await a.now(SEED_TIME + DAY - 1); await a.allRows();
  assert.equal(await a.page.locator('.teacher-return-row').count(), 4);
  for (let i = 0; i < 4; i++) assert.equal(await a.due(i), SEED_TIME + DAY);
  assert.equal(await a.page.getByRole('button', { name: 'Try this again', exact: false }).count(), 0);
  await a.now(REVIEW_TIME); await a.sync();
  const b = await createDriver(browser, 'B', REVIEW_TIME, routes, audit); drivers.push(b); await b.sync(); await b.allRows();
  assert.equal(await b.page.locator('.teacher-return-row').count(), 4);
  checks.push('Four completed NotReported guided tasks seed exact-skill delayed rows; no immediate due or wrong generic skill');

  await a.open(0);
  assert.equal(await a.screen.getAttribute('data-active-review-index'),'0');
  assert.equal(await a.screen.getAttribute('data-active-review-document'),source.document);
  await a.page.waitForFunction(()=>document.activeElement?.textContent==='A fresh try');
  const firstSession = await a.screen.getAttribute('data-active-review-session'); assert.equal(await a.screen.locator('textarea').first().inputValue(), '');
  assert.notEqual(firstSession, source.seeded.find(e => e.fact.data.skill === 'reading').fact.data.session);
  await captureScreen(a,audit,'fresh-reading');
  audit.holdNextFeedback={A:true};audit.holdNotify={};
  const heldReady=new Promise(resolve=>audit.holdNotify.A=resolve);
  await a.screen.locator('.teacher-exercise textarea').fill('The first speaker.');
  await a.screen.getByRole('button',{name:'Save & discuss this return',exact:false}).click();
  let readyTimer;try{await Promise.race([heldReady,new Promise((_,reject)=>readyTimer=setTimeout(()=>reject(Error('Saved review did not issue its bounded synthetic feedback request')),15000))]);}finally{clearTimeout(readyTimer);}
  const read = await a.currentAttempt();
  assert.equal(read.event.fact.data.skill, 'reading'); assert(read.context.fact.data.prompt.startsWith(TASKS[0].prompt));
  assert.equal(read.context.fact.data.task, 'meaning_recall'); assert.deepEqual(read.event.fact.data.cues, []);
  assert.equal(read.all.some(e=>e.fact.kind==='ai_assessment_recorded'&&e.fact.data.attempt===read.event.id),false);
  await a.finish();assert.equal(await a.due(0),REVIEW_TIME+DAY);
  const pendingWriteSession=await a.open(1), pendingWriteText='Draft while earlier feedback is pending';
  assert(await a.screen.locator('textarea').first().isEnabled());await a.screen.locator('textarea').first().fill(pendingWriteText);
  assert(await a.screen.locator('.teacher-exercise button.primary').isDisabled());
  audit.releaseFeedback.A();
  await a.page.waitForFunction(attempt=>window.rufrenLibrary.entries().some(([key,raw])=>key.startsWith('rufren.v1.event.')&&(()=>{const e=JSON.parse(raw);return e.fact.kind==='ai_assessment_recorded'&&e.fact.data.attempt===attempt;})()),read.event.id);
  assert.equal(await a.screen.getAttribute('data-active-review-session'),pendingWriteSession);
  assert.equal(await a.screen.locator('textarea').first().inputValue(),pendingWriteText);
  await a.screen.getByRole('button',{name:'Pause & return to Today',exact:true}).click();
  checks.push('Saved review can finish while feedback is pending; another draft stays editable, submission serialized, and late feedback cannot navigate or overwrite it');
  await b.open(0); const concurrent = await b.saveText('The first speaker.', 'correct'); await b.finish();
  assert.notEqual(concurrent.event.id, read.event.id); assert.notEqual(concurrent.event.fact.data.session, firstSession);
  await a.sync(); await b.sync(); await a.sync();
  assert.equal(await a.due(0), REVIEW_TIME + DAY); assert.equal(await b.due(0), REVIEW_TIME + DAY);
  const size = replica.records.length; await a.sync(); assert.equal(replica.records.length, size);
  const kept = await a.page.evaluate(document => JSON.parse(window.rufrenLibrary.get(`rufren.v1.meta.teacher_practice:${document}:0`)), source.document);
  assert.deepEqual(kept, originalDraft);
  checks.push('Same literal response uses a fresh attempt; concurrent/duplicate sync converges without doubling cadence or overwriting lesson draft');

  const writeTime = REVIEW_TIME + 1000; await a.now(writeTime); const writeSession = await a.open(1);
  assert.equal(writeSession,pendingWriteSession);assert.equal(await a.screen.locator('textarea').first().inputValue(),pendingWriteText);
  assert.equal((await a.screen.innerText()).includes(TASKS[1].reference_russian), false);
  await a.screen.locator('textarea').first().fill(TASKS[1].expected_response);
  await a.screen.getByRole('button', { name: 'Pause & return to Today', exact: true }).click();
  await a.page.evaluate(async () => { await navigator.serviceWorker.ready; }); await a.page.waitForFunction(() => navigator.serviceWorker.controller !== null);
  const posts = audit.requests.length; await a.context.setOffline(true); await a.page.reload(); await a.page.locator('main.main h1').waitFor();
  await a.allRows();
  await a.page.getByRole('button', { name: 'Continue this return', exact: false }).click(); await a.screen.waitFor();
  assert.equal(await a.screen.getAttribute('data-active-review-session'), writeSession);
  assert.equal(await a.screen.locator('textarea').first().inputValue(), TASKS[1].expected_response);
  assert.equal(audit.requests.length, posts, 'Offline review recovery must not regenerate teaching');
  await a.context.setOffline(false); await a.connect(); await a.nav('Practice');
  const written = await a.saveText(TASKS[1].expected_response, 'correct'); assert.equal(written.context.fact.data.task, 'composition');
  assert(written.context.fact.data.prompt === TASKS[1].prompt); assert.deepEqual(written.event.fact.data.cues, []);
  await captureScreen(a,audit,'saved-writing-feedback');
  await a.finish(); const writeDue = writeTime + 3 * DAY; assert.equal(await a.due(1), writeDue);
  await a.now(writeTime + 1000); const earlySession = await a.open(1); assert.notEqual(earlySession, writeSession);
  assert.equal(await a.screen.locator('textarea').first().inputValue(), '');
  await a.saveText(TASKS[1].expected_response, 'correct'); await a.finish(); assert.equal(await a.due(1), writeDue);
  checks.push('Writing preserves Composition/prompt, offline pause/session/draft, neutral self-rating with separate AI-correct cadence, and non-postponing early same-answer repeat');

  await a.now(REVIEW_TIME + 3000); const listenSession = await a.open(2);
  assert.equal(await a.screen.locator('textarea').first().inputValue(), '');
  assert.equal((await a.screen.innerText()).includes(TASKS[2].reference_russian), false);
  await a.screen.locator('textarea').first().fill('Two.');
  const discuss = a.screen.getByRole('button', { name: 'Discuss what I heard', exact: false }); assert(await discuss.isDisabled());
  await a.page.waitForFunction(() => document.querySelector('.teacher-return-screen [data-practice-rate="1"]')?.disabled === false);
  await a.screen.locator('[data-practice-rate="1"]').first().click();
  await a.page.waitForFunction(() => document.querySelector('.teacher-return-screen audio')?.currentTime > .02);
  await a.page.waitForFunction(session => window.rufrenLibrary.entries().some(([key,raw]) => key.startsWith('rufren.v1.event.') && (() => {const e=JSON.parse(raw);return e.fact.kind==='stimulus_presented'&&e.fact.data.session===session&&e.fact.data.purpose==='task_input';})()), listenSession);
  const firstPlay = (await facts(a.page)).find(e => e.fact.kind === 'stimulus_presented' && e.fact.data.session === listenSession && e.fact.data.purpose === 'task_input');
  assert.equal(firstPlay.fact.data.russian, TASKS[2].reference_russian);
  await a.screen.locator('[data-practice-rate="1"]').first().click();
  await a.screen.getByRole('button', { name: 'Show a model & explanation', exact: true }).click();
  audit.verdict.A = 'correct'; await discuss.click(); await a.screen.locator('.teacher-return-feedback').waitFor();
  const listened = await a.currentAttempt(); assert(listened.event.fact.data.cues.includes('replay'));
  assert(listened.event.fact.data.cues.includes('original_text')); assert(listened.event.fact.data.cues.includes('translation'));
  assert.equal(listened.context.fact.data.task, 'meaning_recall'); await a.finish();
  assert.equal(await a.due(2), REVIEW_TIME + 3000 + DAY);
  checks.push('Listening demands new-session TaskInput play; first input is distinct from Replay and model/meaning assistance, with exact question/audio retained');

  const speakTime = REVIEW_TIME + 4000; await a.now(speakTime); const speakSession = await a.open(3);
  await a.screen.getByRole('button', { name: 'Open recorder for this exercise', exact: true }).click();
  await a.screen.locator('#audio-import-attempt').setInputFiles({ name: 'new-review-tone.wav', mimeType: 'audio/wav', buffer: TONES.review.wav });
  await a.page.waitForFunction(() => [...document.querySelectorAll('.teacher-return-screen button')].some(b => b.textContent.includes('Keep this spoken response') && !b.disabled));
  audit.verdict.A = 'uncertain'; await a.screen.getByRole('button', { name: 'Keep this spoken response', exact: false }).click();
  await a.screen.locator('.spoken-take').waitFor(); const spoken = await a.currentAttempt(); assert.deepEqual(spoken.event.fact.data.cues, []);
  const attachment = spoken.all.find(e => e.fact.kind === 'recording_attached' && e.fact.data.target.target === 'attempt' && e.fact.data.target.id === spoken.event.id);
  assert.equal(attachment.fact.data.asset.sha256, TONES.review.hash); assert.equal(spoken.event.fact.data.session, speakSession);
  await a.screen.getByRole('button', { name: 'Find the words in this take', exact: true }).click(); await a.screen.locator('.transcript-words textarea').waitFor();
  await a.screen.locator('.transcript-words textarea').fill('Не хотите чаю?');
  await a.screen.getByRole('button', { name: 'Use these words & discuss them', exact: false }).click(); await a.screen.locator('.teacher-return-feedback').waitFor();
  const speechFeedback = audit.requests.filter(r => r.label==='A' && r.request?.mode==='answer_feedback').at(-1).request.input;
  assert.equal(speechFeedback.response, 'Не хотите чаю?'); assert.equal(speechFeedback.evidence, 'transcript'); assert(speechFeedback.confirmed_transcript_id);
  await a.finish(); assert.equal(await a.due(3), speakTime + DAY);
  await a.now(speakTime + 1000); await a.open(3);
  await a.screen.getByRole('button', { name: 'Open recorder for this exercise', exact: true }).click();
  await a.screen.locator('#audio-import-attempt').setInputFiles({ name: 'known-prior-response.wav', mimeType: 'audio/wav', buffer: TONES.seed.wav });
  await a.page.getByText(/Make a new take for this revisit/).first().waitFor();
  assert(await a.screen.getByRole('button', { name: 'Keep this spoken response', exact: false }).isDisabled());
  await a.screen.locator('#audio-import-attempt').setInputFiles({ name: 'fresh-after-help.wav', mimeType: 'audio/wav', buffer: TONES.later.wav });
  await a.page.waitForFunction(() => [...document.querySelectorAll('.teacher-return-screen button')].some(b => b.textContent.includes('Keep this spoken response') && !b.disabled));
  await a.screen.getByRole('button', { name: 'Keep this spoken response', exact: false }).click(); await a.screen.locator('.spoken-take').waitFor();
  assert((await a.currentAttempt()).event.fact.data.cues.includes('reference_material')); await a.finish();
  checks.push('Fresh speaking original/session/attachment and explicitly confirmed-word feedback pass; known prior bytes cannot masquerade as production and their support stays monotone');

  await a.sync(); await b.sync(); const currentTime = speakTime + 1000;
  const futureTime = writeDue + 1000; await b.now(futureTime); await b.open(1); await b.saveText(TASKS[1].expected_response, 'correct'); await b.finish(); await b.sync();
  await a.now(currentTime); await a.sync(); assert.equal(await a.due(1), writeDue, 'Future return/judgment must not affect present queue');
  await a.now(futureTime); assert.equal(await a.due(1), futureTime + 7 * DAY);
  const heldDue = futureTime + 7 * DAY, heldStart = heldDue - 1;
  await a.now(heldStart); await a.open(1);
  const heldSession = await a.screen.getAttribute('data-active-review-session');
  await a.screen.locator('textarea').first().fill(TASKS[1].expected_response);
  await a.page.evaluate(time => window.qaRevisitClock(time), heldDue + 1000);
  const held = await a.saveText(TASKS[1].expected_response, 'correct');
  assert.equal(held.event.fact.data.session, heldSession);
  assert(held.event.recorded_at_ms > heldDue);
  const heldProof = JSON.parse(held.context.fact.data.reference_answer.split('\nRufren teacher revisit v1: ')[1]);
  assert.equal(heldProof.started_at_ms, heldStart);
  await a.finish(); assert.equal(await a.due(1), heldDue, 'Opening early then submitting after due cannot manufacture delayed retrieval');
  checks.push('An early-opened response held across its due boundary retains original start proof and earns no delayed advance');
  const lateTime=heldDue+1000+8*DAY;
  await a.now(lateTime);await a.open(1);const beforeForeign=await a.saveText(TASKS[1].expected_response,'correct');await a.finish();
  assert.equal(await a.due(1),lateTime+14*DAY);
  const immutableBefore=JSON.stringify(beforeForeign.event);
  await b.now(lateTime);await b.nav('Teacher');await openLessonTools(b.page);
  await b.page.getByRole('button',{name:'Your saved lessons',exact:true}).click();
  await b.page.locator(`.teacher-notebook-entry[data-document-id="${source.document}"]`).click();
  const foreignFlow=b.page.locator('.teacher-lesson-flow');await foreignFlow.waitFor();
  await foreignFlow.getByRole('combobox',{name:'Go to a step',exact:true}).selectOption('1');
  await b.page.locator('.teacher-lesson-current').getByText('Show an example and explanation',{exact:true}).click();
  await foreignFlow.locator('.teacher-lesson-current').getByText(TASKS[1].expected_response,{exact:true}).waitFor();
  await b.page.waitForFunction(({text,time})=>window.rufrenLibrary.entries().some(([key,raw])=>key.startsWith('rufren.v1.event.')&&(()=>{const e=JSON.parse(raw);return e.fact.kind==='stimulus_presented'&&e.fact.data.russian===text&&e.recorded_at_ms===time;})()),{text:TASKS[1].expected_response,time:lateTime});
  await b.sync();await a.sync();
  assert.equal(await a.due(1),lateTime+DAY,'Late-arriving matching support in the attempt window revises cadence conservatively');
  assert.equal(JSON.stringify((await facts(a.page)).find(e=>e.id===beforeForeign.event.id)),immutableBefore,'Derived support must not rewrite the original attempt/cues');
  await b.sync();assert.equal(await b.due(1),lateTime+DAY);
  checks.push('Late-synced foreign answer exposure inside a completed attempt window shortens AI cadence without rewriting immutable attempt cues');
  for (const [name,width,height] of [['tablet',820,1180],['phone',390,844]]) {
    await a.page.setViewportSize({width,height}); assert(await a.page.evaluate(() => document.documentElement.scrollWidth<=innerWidth));
    await a.page.screenshot({path:path.join(OUTPUT,`${name}.png`),fullPage:true});
  }
  checks.push('Actual two-client synthetic-replica sync converges; future completion/opinion is ignored until its occurrence, then admitted without arrival-time anchoring');
  return { source_document: source.document, replica_events: replica.records.length,
    clients: await Promise.all(drivers.map(async d => ({label:d.label,events:await facts(d.page)}))) };
}

async function realSourceCapture(browser, audit, checks, drivers) {
  const sourcePath = path.resolve('.local/teacher-v5-live-revised/lesson.json');
  const original = JSON.parse(await fs.readFile(sourcePath,'utf8'));
  assert.equal(original.request.mode,'teach_lesson'); assert.equal(original.content.practice[0].skill,'reading');
  const routes=fixtureRoutes(syntheticReplica(),audit);
  const d=await createDriver(browser,'capture',SEED_TIME,routes,audit,original.content.practice);drivers.push(d);
  if(process.env.RUFREN_LIVE_REVISIT_FEEDBACK==='approved')d.page.setDefaultTimeout(130000);
  await d.page.evaluate(response => window.rufrenLibrary.write('rufren.v1.meta.teacher_response',JSON.stringify(response)),original);
  await d.page.reload();await d.page.locator('main.main h1').waitFor();await d.connect();await d.nav('Teacher');
  const flow=d.page.locator('.teacher-lesson-flow');await flow.waitFor();
  const document=await flow.getAttribute('data-lesson-document');
  const archive=(await facts(d.page)).find(e=>e.fact.kind==='teacher_document_recorded'&&e.fact.data.document_id===document);
  assert.deepEqual(archive.fact.data.response,original);assert.equal(archive.fact.data.receipt.kind,'recovered_local_cache');
  const step=original.content.lesson_flow?.steps.findIndex(s=>s.practice_index===0)??1;
  await flow.getByRole('combobox',{name:'Go to a step',exact:true}).selectOption(String(step));
  await d.page.locator('.teacher-lesson-current textarea').fill('They say they are well, thank the other person, and ask how they are.');
  await d.page.getByRole('button',{name:'Discuss my answer',exact:false}).click();
  await d.page.getByRole('heading',{name:'Working with your response',exact:true}).waitFor();
  await d.now(REVIEW_TIME);
  const session=await d.open(0);
  assert.equal(await d.screen.locator('textarea').first().inputValue(),'');
  let captured;
  audit.captureReview=async(raw,request)=>{
    assert.equal(captured,undefined,'Capture/proxy permits one exact review feedback request');
    captured=structuredClone(request);
    await fs.writeFile(path.join(OUTPUT,'request.json'),JSON.stringify(request,null,2));
    await fs.writeFile(path.join(OUTPUT,'request-raw.json'),raw.postData());
  };
  const saved=await d.saveText('The speaker says goodbye and asks where you live.','uncertain');
  assert(captured);assert.equal(captured.input.skill,'reading');assert.equal(saved.event.fact.data.self_rating,'not_reported');
  assert(captured.input.prompt.startsWith(original.content.practice[0].prompt));
  assert(captured.input.reference.includes(original.content.practice[0].reference_russian));
  assert.equal(saved.event.fact.data.session,session);assert.deepEqual(saved.event.fact.data.cues,[]);
  checks.push('Untouched genuine-v5 lesson recovered with unknown receipt; synthetic source practice and delayed Reading return yield exact captured request before fulfillment');
  const receipt={source_path:sourcePath,source_sha256:hash(await fs.readFile(sourcePath)),source_response:original.provenance,
    document_id:document,practice_index:0,seed_time:SEED_TIME,review_time:REVIEW_TIME,
    time_provenance:'Browser-only synthetic clock; not an empirical delayed-learning observation',
    response_provenance:'Synthetic learner response',self_rating:'not_reported',request_path:path.join(OUTPUT,'request.json'),
    actual_proxy_enabled:process.env.RUFREN_LIVE_REVISIT_FEEDBACK==='approved'};
  await fs.writeFile(path.join(OUTPUT,'capture-receipt.json'),JSON.stringify(receipt,null,2));
  return {capture:receipt,clients:[{label:d.label,events:await facts(d.page)}]};
}

async function main() {
  if(process.env.RUFREN_LIVE_REVISIT_FEEDBACK==='approved'&&process.env.RUFREN_REVISIT_REAL_SOURCE!=='1')throw Error('Live proxy is restricted to the explicitly approved single real-source capture mode');
  await fs.mkdir(OUTPUT,{recursive:true});
  const audit={requests:[],forbidden:[],errors:[],verdict:{}},checks=[],drivers=[];
  const browser=await chromium.launch({headless:true});
  try {
    const result=process.env.RUFREN_REVISIT_REAL_SOURCE==='1'
      ?await realSourceCapture(browser,audit,checks,drivers)
      :await consumerJourney(browser,audit,checks,drivers);
    assert.deepEqual(audit.errors,[]);assert.deepEqual(audit.forbidden,[]);
    await fs.writeFile(path.join(OUTPUT,'result.json'),JSON.stringify({ok:true,fixture:process.env.RUFREN_LIVE_REVISIT_FEEDBACK==='approved'?'Exactly one explicitly approved isolated feedback proxy; learner response/time remain synthetic':'Synthetic teaching/audio/ASR/replica; browser-only fake time; no inference or main service calls',checks,...result,...audit},null,2));
    console.log(process.env.RUFREN_REVISIT_REAL_SOURCE==='1'?'PASS exact real-source review request capture; response/time provenance explicit':'PASS exact-task four-skill revisit, cadence, fresh identity/audio, pause/offline and two-client queue');
  } catch(error) {
    for(const d of drivers){await d.page.screenshot({path:path.join(OUTPUT,`failure-${d.label}.png`),fullPage:true}).catch(()=>{});await fs.writeFile(path.join(OUTPUT,`failure-${d.label}.txt`),await d.page.locator('body').innerText()).catch(()=>{});}
    const clients=await Promise.all(drivers.map(async d=>({label:d.label,now:await d.page.evaluate(()=>Date.now()),events:await facts(d.page)})));
    await fs.writeFile(path.join(OUTPUT,'failure.json'),JSON.stringify({error:String(error),checks,clients,...audit},null,2));throw error;
  } finally {
    for(const release of Object.values(audit.releaseFeedback||{}))release();
    for(const d of drivers){await d.context.tracing.stop({path:path.join(OUTPUT,`trace-${d.label}.zip`)}).catch(()=>{});await d.context.close();}
    await browser.close();
  }
}
if (require.main === module) main().catch(error => { console.error(error); process.exitCode = 1; });
module.exports = { TASKS, TONES, SEED_TIME, REVIEW_TIME, DAY, revisitResponse, syntheticReplica,
  installClock, facts, fixtureRoutes, openLessonTools, createDriver, seedGuided };
