// Actual app journey with synthetic teaching only. No real relay/model/profile is used.
const { chromium } = require(process.env.RUFREN_PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const { CURRENT_PROTOCOL } = require('./qa-current-protocol.cjs');
const { navigate } = require('./qa-navigation.cjs');
const { teacherResponse } = require('./test-teacher-oral.cjs');

const APP = process.env.RUFREN_QA_URL || 'http://127.0.0.1:8091';
const RELAY = 'http://127.0.0.1:18992'; // All requests intercepted, no server required.
const OUTPUT = path.resolve(process.env.RUFREN_NOTEBOOK_QA_OUTPUT || '.local/teacher-notebook-qa');
const MARKERS = {
  detail: 'Подро́бность: форма зави́сит от говоря́щего.',
  support: 'Мы чита́ем.',
  goal0: 'Express one action', orientation0: 'Use a short message to say what you are doing.',
  goal1: 'Understand another message', goal2: 'Carry the idea into a story',
};

function notebookResponse(request, sequence = 1) {
  const response = teacherResponse(request, `notebook-${sequence}`);
  response.provenance.model = 'synthetic-notebook-fixture';
  response.provenance.prompt_version = 'qa-guided-notebook-v1';
  response.provenance.response_id = `notebook-${sequence}`;
  if (request.mode !== 'teach_lesson') return response;
  const name = ['Alpha', 'Beta', 'Legacy'][sequence - 1] || `Fixture${sequence}`;
  response.content = {
    explanation: `${name} notebook topic. Full original explanation: русская форма.`,
    examples: [
      { russian: 'Я чита́ю.', english: 'I am reading.', note: 'Synthetic model.' },
      { russian: MARKERS.support, english: 'We are reading.', note: 'Synthetic optional support.' },
      { russian: 'Звёзды свети́ли.', english: 'The stars were shining.', note: 'Synthetic story model.' },
    ],
    practice: [
      { prompt: `Write a short Russian message for ${name}: I am reading.`, skill: 'writing',
        reference_russian: 'Я чита́ю.', reference_english: 'I am reading.', expected_response: 'Я читаю.',
        teaching_note: 'Synthetic writing explanation.' },
      { prompt: `Explain the Russian message for ${name} in English.`, skill: 'reading',
        reference_russian: 'Здесь ма́як.', reference_english: 'There is a lighthouse here.',
        expected_response: 'There is a lighthouse here.', teaching_note: 'Synthetic reading explanation.' },
    ],
    feedback: null, next_step: 'Bring the message into another short exchange.', caveats: [],
  };
  if (request.input.lesson_flow === 'guided_v1') {
    response.content.lesson_flow = { steps: [
      { id: 'action', goal: MARKERS.goal0, orientation: MARKERS.orientation0, detail: MARKERS.detail,
        examples: [{ example_index: 0, presentation: 'model' }, { example_index: 1, presentation: 'optional_support' }], practice_index: 0 },
      { id: 'understand', goal: MARKERS.goal1, orientation: 'Explain the message in your own words.',
        detail: 'Необяза́тельная подро́бность второго́ ша́га.', examples: [], practice_index: 1 },
      { id: 'story', goal: MARKERS.goal2, orientation: 'Notice a small scene before you finish.',
        examples: [{ example_index: 2, presentation: 'model' }] },
    ] };
  }
  return response;
}

const status = { enabled: true, provider: 'qa_synthetic', model: 'Synthetic notebook fixture',
  message: 'No inference', data_sent: 'Synthetic only', max_text_bytes: 8192, max_context_bytes: 8192,
  remaining_reservations: 99, remaining_budget_micro_usd: 0, accounting: 'subscription_quota', lesson_flow: 'guided_v1' };

async function events(page) {
  return page.evaluate(() => window.rufrenLibrary.entries().filter(([key]) => key.startsWith('rufren.v1.event.'))
    .map(([, raw]) => JSON.parse(raw)));
}

async function openLessonTools(page) {
  const summary = page.getByText('Lessons & history', { exact: true });
  if (await summary.count() && !await summary.evaluate(node => node.closest('details')?.open)) await summary.click();
}

async function main() {
  await fs.mkdir(OUTPUT, { recursive: true });
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({ viewport: { width: 1200, height: 900 } });
  await context.tracing.start({ screenshots: true, snapshots: true, sources: true });
  const page = await context.newPage(), errors = [], forbidden = [], requests = [], checks = [], layouts = [];
  let lessonCount = 0, legacy = false;
  page.on('pageerror', error => errors.push(error.message));
  await context.route('**/*', async route => {
    const req = route.request(), url = new URL(req.url());
    if (url.origin === new URL(APP).origin && !url.pathname.startsWith('/api/')) return route.continue();
    if (url.origin !== RELAY) { forbidden.push(req.url()); return route.abort('blockedbyclient'); }
    if (req.method() === 'OPTIONS') return route.fulfill({ status: 204, headers: {
      'Access-Control-Allow-Origin': '*', 'Access-Control-Allow-Headers': '*', 'Access-Control-Allow-Methods': 'GET,POST,OPTIONS' } });
    const request = req.postDataJSON(); requests.push({ path: url.pathname, method: req.method(), request });
    let body;
    if (url.pathname === '/api/teacher/status' || url.pathname === '/api/tutor/status') body = {
      ...status, model: legacy ? 'Synthetic legacy fixture' : status.model, lesson_flow: legacy ? null : 'guided_v1' };
    else if (url.pathname === '/api/teacher/teach') {
      if (request.mode === 'teach_lesson') {
        lessonCount++; assert.equal(request.input.lesson_flow ?? null, legacy ? null : 'guided_v1');
        body = notebookResponse(request, lessonCount);
      } else body = notebookResponse(request, `feedback-${requests.length}`);
    } else { forbidden.push(req.url()); return route.abort('blockedbyclient'); }
    await route.fulfill({ status: 200, contentType: 'application/json', headers: { 'Access-Control-Allow-Origin': '*' }, body: JSON.stringify(body) });
  });
  const nav = name => navigate(page, name);
  const connect = async () => { await nav('Settings'); await page.locator('#sync-url').fill(RELAY); await page.locator('#sync-key').fill('synthetic-notebook-no-secret'); };
  const flow = page.locator('.teacher-lesson-flow');
  const input = page.locator('.teacher-lesson-current .teacher-exercise textarea');
  const documents = async () => (await events(page)).filter(e => e.fact.kind === 'teacher_document_recorded');
  const step = async position => { await page.waitForFunction(position => document.querySelector('.teacher-lesson-flow')?.dataset.lessonStep === String(position), position); };
  const focusedStep = async () => page.waitForFunction(() => {
    const heading = document.querySelector('.teacher-lesson-current .teacher-lesson-goal');
    if (document.activeElement !== heading || !heading) return false;
    const rect = heading.getBoundingClientRect(); return rect.top >= 0 && rect.bottom <= innerHeight;
  });
  const jump = async position => {
    const row = flow.locator('.teacher-step-row').nth(position);
    await row.click(); await step(position); await focusedStep();
    assert.equal(await row.getAttribute('aria-current'), 'step');
    assert.equal(await flow.getByRole('combobox').count(), 0, 'Lesson outline remains visible without opening a select');
  };
  const start = async index => {
    await nav('Lessons');
    assert.equal(await page.locator('#teacher-lesson').count(), 0, 'Catalog is the single lesson browser');
    if (index === 0) {
      await page.setViewportSize({width:390,height:844});
      assert(await page.locator('.lesson-index').isVisible(), 'Compact Learn starts with the catalog');
      assert.equal(await page.locator('.lesson-body').isVisible(), false);
      assert(await page.locator('.lesson-index').evaluate(node=>node.scrollHeight<=node.clientHeight+1), 'Catalog has no nested scroll region');
      await page.locator('.lesson-link').first().click();
      assert.equal(await page.locator('.lesson-index').isVisible(),false);
      await page.waitForFunction(()=>{const back=document.querySelector('.lesson-catalog-back')?.getBoundingClientRect();return document.activeElement?.id==='lesson-detail-heading'&&back?.top>=0&&back.bottom<=innerHeight;});
      await page.getByRole('button',{name:'Back to lessons',exact:false}).click();
      assert(await page.locator('.lesson-index').isVisible());
      await page.waitForFunction(()=>document.activeElement?.id==='lesson-search');
      await nav('Settings'); await nav('Lessons');
      assert(await page.locator('.lesson-index').isVisible(), 'A persisted selection does not hide the catalog on return');
      assert.equal(await page.locator('.lesson-body').isVisible(),false);
      await page.screenshot({path:path.join(OUTPUT,'phone-catalog-list.png'),fullPage:true});
      await page.setViewportSize({width:1200,height:900});
      await page.locator('#lesson-search').fill('mathematical');
      assert.equal(await page.locator('.lesson-link').count(), 1, 'Catalog search finds a meaningful topic');
      await page.locator('#lesson-search').fill('');
    }
    await page.locator('.lesson-link').nth(index).click();
    await page.getByRole('button', { name: 'Learn with the AI teacher', exact: false }).click();
    const panel = page.locator('details.teacher-start');
    if (await panel.getAttribute('open') === null) await panel.locator(':scope > summary').click();
    await page.getByRole('button', { name: 'Teach me this lesson', exact: false }).click(); await step(0);
    assert.equal(await flow.locator('.teacher-step-row[aria-current="step"]').count(), 1);
  };
  const openSaved = async query => {
    await openLessonTools(page);
    await page.getByRole('button', { name: 'Your saved lessons', exact: true }).click();
    await page.locator('#teacher-notebook-search').fill(query);
    assert.equal(await page.locator('.teacher-notebook-entry').count(), 1, 'Search finds exactly one archived lesson');
    await page.locator('.teacher-notebook-entry').click();
  };
  try {
    await page.goto(APP); await page.locator('main.main h1').waitFor(); await connect(); await nav('Teacher');
    await page.getByText('Ready · Synthetic notebook fixture', { exact: true }).waitFor({ state: 'attached' });
    await start(0);
    const alpha = await flow.getAttribute('data-lesson-document');
    assert.equal(await page.locator('.teacher-exercise').count(), 1);
    assert.equal(await page.getByText(MARKERS.detail, { exact: true }).count(), 0);
    assert.equal(await page.getByText(MARKERS.support, { exact: true }).count(), 0);
    assert.equal((await events(page)).some(e => e.fact.kind === 'stimulus_presented' && [MARKERS.detail, MARKERS.support].includes(e.fact.data.russian)), false);
    await flow.getByRole('button', { name: 'Why', exact: false }).click();
    await page.getByText(MARKERS.detail, { exact: true }).waitFor();
    await flow.getByRole('button', { name: 'See optional examples', exact: false }).click();
    await page.getByText(MARKERS.support, { exact: true }).waitFor();
    checks.push('Guided detail and optional model absent before explicit opening');
    await input.fill('Черновик Alpha: я читаю…');
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    assert(await input.evaluate(node => document.activeElement === node), 'Draft rerender must not steal input focus');
    await flow.getByRole('button', { name: 'Next step', exact: false }).click(); await step(1); await focusedStep();
    await flow.getByRole('button', { name: 'Back', exact: false }).click(); await step(0); await focusedStep();
    assert.equal(await input.inputValue(), 'Черновик Alpha: я читаю…');
    await flow.getByRole('button', { name: 'Next step', exact: false }).click(); await step(1); await focusedStep();
    assert.equal(await page.locator('.teacher-exercise').count(), 1);
    assert.equal(await input.inputValue(), ''); await input.fill('Alpha unfinished lighthouse meaning');
    const alphaField = await input.getAttribute('id');
    await start(1);
    const beta = await flow.getAttribute('data-lesson-document'); assert.notEqual(beta, alpha);
    await input.fill('Черновик Beta: отдельный ответ');
    const lessons = (await documents()).filter(e => e.fact.data.response.request.mode === 'teach_lesson');
    assert.equal(lessons.length, 2); assert(lessons.every(e => e.version === CURRENT_PROTOCOL));
    assert.equal((await events(page)).filter(e => e.fact.kind === 'attempt_recorded').length, 0, 'Navigation and saving documents are not completed practice');
    await openSaved('alpha notebook'); await step(1);
    assert.equal(await flow.getAttribute('data-lesson-document'), alpha); assert.equal(await input.getAttribute('id'), alphaField);
    assert.equal(await input.inputValue(), 'Alpha unfinished lighthouse meaning');
    checks.push('Two immutable archived lessons retain separate unfinished drafts and exact per-document cursor');
    await page.getByRole('button', { name: 'Discuss my answer', exact: false }).click();
    await page.getByRole('heading', { name: 'Working with your response', exact: true }).waitFor();
    const critique = requests.find(r => r.request?.mode === 'answer_feedback').request.input;
    assert.equal(critique.response, 'Alpha unfinished lighthouse meaning'); assert(critique.prompt.includes('Alpha'));
    assert.equal(critique.skill, 'reading'); assert(critique.exercise_id.endsWith(':1'));
    await page.getByRole('button', { name: 'Return to this lesson & exercise', exact: false }).click(); await step(1);
    assert.equal(await flow.getAttribute('data-lesson-document'), alpha); assert.equal(await input.getAttribute('id'), alphaField);
    await jump(0); assert.equal(await input.inputValue(), 'Черновик Alpha: я читаю…');
    assert.equal(await page.getByText(MARKERS.detail, { exact: true }).count(), 0, 'Disclosures reset when changing steps');
    await jump(1);
    checks.push('Feedback returns to exact source document/practice, not the latest other lesson');
    await page.getByRole('button', { name: 'Review saved practice', exact: true }).click();
    await page.locator('#teacher-attempt-search').fill('reading');
    assert(await page.locator('.teacher-history-row').count() > 0, 'Saved practice can be found by skill');
    assert.equal(await page.locator('#teacher-attempt').count(), 0, 'Saved practice is a searchable list, not a capped select');
    await page.locator('.teacher-history-row').first().click();
    assert.equal(await page.locator('.teacher-history-row').first().getAttribute('aria-pressed'), 'true');
    await page.getByRole('button', { name: 'Return to current teaching', exact: true }).click(); await step(1);

    await flow.getByRole('button', { name: 'Pause & return to Today', exact: true }).click();
    await page.locator('.teacher-resume').waitFor();
    const beforeReload = (await events(page)).map(e => e.id);
    await page.reload(); await page.locator('main.main h1').waitFor(); await connect(); await nav('Today');
    await page.getByRole('button', { name: 'Continue this lesson', exact: false }).click(); await step(1);
    assert.equal(await flow.getAttribute('data-lesson-document'), alpha);
    assert.equal(await input.inputValue(), 'Alpha unfinished lighthouse meaning');
    const recoveredIDs = new Set((await events(page)).map(e => e.id)); assert(beforeReload.every(id => recoveredIDs.has(id)));
    await openSaved('Beta notebook'); await step(0); assert.equal(await flow.getAttribute('data-lesson-document'), beta);
    assert.equal(await input.inputValue(), 'Черновик Beta: отдельный ответ');
    await openSaved('alpha notebook'); await step(1);
    checks.push('Pause/reload/Today resume and searchable archive preserve both lessons and drafts');
    for (const [name, width, height] of [['tablet', 820, 1180], ['phone', 390, 844]]) {
      await page.setViewportSize({ width, height }); await jump(1);
      assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
      layouts.push(await page.evaluate(name => ({ name, viewport: { width: innerWidth, height: innerHeight },
        scrollY, goal: document.querySelector('.teacher-lesson-goal')?.getBoundingClientRect().toJSON(),
        response: document.querySelector('.teacher-lesson-current textarea')?.getBoundingClientRect().toJSON(),
        active: document.activeElement?.className }), name));
      await page.screenshot({ path: path.join(OUTPUT, `${name}.png`), fullPage: true });
      await page.screenshot({ path: path.join(OUTPUT, `${name}-viewport.png`) });
    }
    checks.push('Next/Back/jump focus a visible current-step heading; ordinary draft rerenders retain input focus');
    // A relay that does not advertise guided_v1 receives the old request and schema.
    legacy = true;
    await openLessonTools(page);
    const panel = page.locator('details.teacher-start'); if (await panel.getAttribute('open') === null) await panel.locator(':scope > summary').click();
    await page.getByText('Connection and lesson context', { exact: true }).click();
    const ready = page.waitForResponse(r => r.url().endsWith('/api/teacher/status'));
    await page.getByRole('button', { name: 'Check connection again', exact: true }).click(); await ready;
    await page.getByText('Ready · Synthetic legacy fixture', { exact: true }).waitFor();
    await start(2); assert.equal(await page.locator('.teacher-exercise').count(), 0);
    assert.equal(await flow.locator('.teacher-lesson-current').getAttribute('data-step-id'), 'orientation');
    await flow.getByRole('button', { name: 'Read the full saved lesson', exact: false }).click();
    await page.getByText('Legacy notebook topic. Full original explanation: русская форма.', { exact: true }).waitFor();
      await flow.getByRole('button', { name: 'Next step', exact: false }).click(); await step(1); await focusedStep();
    assert.equal(await page.locator('.teacher-exercise').count(), 1);
    assert.equal(await flow.locator('.teacher-lesson-current').getAttribute('data-step-id'), 'practice-0');
    checks.push('Unadvertised guided capability uses legacy schema; original disclosure and original practice order remain readable');
    // Search must include old attempts beyond the former eighty-row select cap.
    await page.evaluate(() => {
      const base=window.rufrenLibrary.entries().filter(([key])=>key.startsWith('rufren.v1.event.')).map(([,raw])=>JSON.parse(raw)).find(event=>event.fact.kind==='attempt_recorded');
      for(let index=0;index<81;index++) {
        const event={...base,id:crypto.randomUUID(),recorded_at_ms:base.recorded_at_ms-1000-index,fact:{kind:'attempt_recorded',data:{...base.fact.data,response:'uncapped history fixture '+index}}};
        window.rufrenLibrary.write('rufren.v1.event.'+event.id,JSON.stringify(event));
      }
    });
    await page.reload(); await page.locator('main.main h1').waitFor(); await connect(); await nav('Teacher');
    await page.getByRole('button',{name:'Review saved practice',exact:true}).click();
    assert.equal(await page.locator('.teacher-history-row').count(),12);
    await page.getByRole('button',{name:'Show more saved practice',exact:true}).click();
    assert.equal(await page.locator('.teacher-history-row').count(),24);
    await page.locator('#teacher-attempt-search').fill('uncapped history fixture 80');
    assert.equal(await page.locator('.teacher-history-row').count(),1);
    await page.locator('.teacher-history-row').click();
    assert.equal(await page.locator('blockquote').innerText(),'uncapped history fixture 80');
    checks.push('Search includes attempts older than eighty rows; Show more expands twelve to twenty-four and selection opens the exact response');
    assert.deepEqual(errors, []); assert.deepEqual(forbidden, []);
    await fs.writeFile(path.join(OUTPUT, 'result.json'), JSON.stringify({ ok: true, fixture: 'All teaching is synthetic; no inference or main service mutation', checks, layouts, requests, events: await events(page), errors, forbidden }, null, 2));
    console.log('PASS synthetic notebook/guided lesson archive, drafts, exact return, resume, search, legacy fallback and responsive layout');
  } catch (error) {
    await page.screenshot({ path: path.join(OUTPUT, 'failure.png'), fullPage: true }).catch(() => {});
    await fs.writeFile(path.join(OUTPUT, 'failure.txt'), await page.locator('body').innerText()).catch(() => {});
    await fs.writeFile(path.join(OUTPUT, 'failure-receipt.json'), JSON.stringify({ error: String(error), checks, requests, errors, forbidden, events: await events(page) }, null, 2)).catch(() => {});
    throw error;
  } finally {
    await context.tracing.stop({ path: path.join(OUTPUT, 'trace.zip') }).catch(() => {}); await context.close(); await browser.close();
  }
}

if (require.main === module) main().catch(error => { console.error(error); process.exitCode = 1; });
module.exports = { notebookResponse, MARKERS, openLessonTools };
