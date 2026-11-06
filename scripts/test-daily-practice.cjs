// Actual daily-plan/Progress composition with synthetic saved facts. No model,
// speech, ASR or relay routes are enabled; no personal browser profile is used.
const { chromium } = require(process.env.RUFREN_PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const crypto = require('node:crypto');
const { navigate } = require('./qa-navigation.cjs');
const APP = process.env.RUFREN_QA_URL || 'http://127.0.0.1:8091';
const OUTPUT = path.resolve(process.env.RUFREN_DAILY_QA_OUTPUT || '.local/daily-practice-qa');
const SOURCE = process.env.RUFREN_DAILY_SOURCE || '.local/teacher-revisit-qa/dialogue-v5-regression-01/result.json';
const NOW = 1795176004000, DAY = 86400000;

function passage(title, skill, practiced = true) {
  const revision = crypto.randomUUID(), device = crypto.randomUUID();
  const definition = { version: 5, id: revision, device, recorded_at_ms: NOW - 3 * DAY,
    fact: { kind: 'passage_revised', data: { passage: { id: crypto.randomUUID(), title,
      russian: 'Я чита́ю.', translation: 'I am reading.', source: 'Synthetic daily-plan fixture', coach_reviewed: false }, parents: [] } } };
  const events = [definition];
  if (practiced) events.push({ version: 5, id: crypto.randomUUID(), device, recorded_at_ms: NOW - 2 * DAY,
    fact: { kind: 'attempt_recorded', data: { session: crypto.randomUUID(), revision, skill, response: 'Synthetic prior response',
      cues: [], self_rating: 'recalled', policy: 'synthetic-passage-practice' } } });
  return { revision, events };
}
async function facts(page) {
  return page.evaluate(() => window.rufrenLibrary.entries().filter(([key]) => key.startsWith('rufren.v1.event.')).map(([, raw]) => JSON.parse(raw)));
}
async function metadata(page, name) {
  return page.evaluate(name => JSON.parse(window.rufrenLibrary.get(`rufren.v1.meta.${name}`)), name);
}

async function main() {
  await fs.mkdir(OUTPUT, { recursive: true });
  const source = JSON.parse(await fs.readFile(SOURCE, 'utf8'));
  const original = source.clients[0].events;
  const read = passage('A practiced reading passage', 'reading'), write = passage('A practiced writing passage', 'writing');
  const unpracticed = passage('Available new material, not a review obligation', 'reading', false);
  const browser = await chromium.launch({ headless: true });
  const contexts = [], checks = [], layouts = [], errors = [], forbidden = [];
  let a, b;
  const create = async (label, now, seed) => {
    const context = await browser.newContext({ viewport: { width: 1200, height: 900 } }); contexts.push({ label, context });
    await context.addInitScript(now => { Date.now = () => now; }, now);
    await context.tracing.start({ screenshots: true, snapshots: true, sources: true });
    const page = await context.newPage(); page.on('pageerror', error => errors.push({ label, message: error.message }));
    await context.route('**/*', route => { const url = new URL(route.request().url());
      if (url.origin === new URL(APP).origin && !url.pathname.startsWith('/api/')) return route.continue();
      forbidden.push({ label, url: route.request().url() }); return route.abort('blockedbyclient'); });
    await page.goto(APP); await page.locator('main.main h1').waitFor();
    await page.evaluate(seed => { for (const event of seed) window.rufrenLibrary.write(`rufren.v1.event.${event.id}`, JSON.stringify(event)); }, seed);
    const nav = name => navigate(page, name);
    await nav('Today'); return { context, page, nav };
  };
  const checkpoint = text => { checks.push(text); console.log(`CHECK ${text}`); };
  const capture = async (page, stage) => {
    const previous = page.viewportSize();
    for (const [name, width, height] of [['desktop', 1200, 900], ['phone', 390, 844], ['tablet', 820, 1180]]) {
      await page.setViewportSize({ width, height }); await page.evaluate(() => scrollTo(0, 0));
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth);
      assert.equal(overflow, false); layouts.push({ stage, name, width, height, overflow });
      await page.screenshot({ path: path.join(OUTPUT, `${name}-${stage}-full.png`), fullPage: true });
      await page.screenshot({ path: path.join(OUTPUT, `${name}-${stage}-viewport.png`) });
    }
    await page.setViewportSize(previous);
  };
  try {
    a = await create('A', NOW, [...original, ...read.events, ...write.events, ...unpracticed.events]);
    const { page } = a, panel = page.locator('.daily-plan');
    const controls = panel.locator('details').filter({ has: page.locator('#daily-plan-budget') });
    await controls.locator(':scope > summary').click();
    await page.locator('#daily-plan-budget').selectOption('3'); await page.locator('#daily-plan-quiet').check();
    const attemptsBefore = (await facts(page)).filter(e => e.fact.kind === 'attempt_recorded').length;
    await page.evaluate(() => {
      const write = window.rufrenLibrary.write;
      window.rufrenLibrary.write = (key, value) => {
        if (key.endsWith('meta.daily_plan_active') && value) {
          const plan = JSON.parse(value), pending = plan.items.find(item => item.start_pending);
          if (pending) { window.qaDailyReservedSession = Object.keys(pending.started_sessions)[0]; window.qaDailySawReservation = true; }
          else if (window.qaDailySawReservation) { window.qaDailyAckFailed = true; throw Error('Synthetic final start acknowledgment write failure'); }
        }
        return write(key, value);
      };
    });
    await panel.getByRole('button', { name: 'Choose my next step', exact: false }).click();
    await page.waitForFunction(() => window.qaDailyAckFailed === true);
    const reservedID = await page.evaluate(() => window.qaDailyReservedSession);
    const interrupted = await metadata(page, 'daily_plan_active');
    assert(interrupted.items.some(item => item.start_pending && Object.hasOwn(item.started_sessions, reservedID)));
    assert.equal((await metadata(page, 'session')).id, reservedID);
    assert.equal((await facts(page)).filter(e => e.fact.kind === 'attempt_recorded').length, attemptsBefore);
    await page.screenshot({ path: path.join(OUTPUT, 'failed-start-ack.png'), fullPage: true });
    // Reload removes the isolated failure hook. Recovery must acknowledge the
    // existing reserved response, not create another response identity.
    await page.reload(); await page.locator('main.main h1').waitFor(); await a.nav('Today');
    assert.equal((await metadata(page, 'session')).id, reservedID);
    await panel.getByRole('button', { name: 'Continue this session', exact: false }).click();
    await page.locator('#practice-response').waitFor();
    const recovered = await metadata(page, 'daily_plan_active');
    assert(recovered.items.every(item => !item.start_pending));
    assert.equal((await metadata(page, 'session')).id, reservedID);
    assert.equal(recovered.items.reduce((sum, item) => sum + Object.keys(item.started_sessions).length, 0), 1);
    const plan = await metadata(page, 'daily_plan_active');
    assert.equal(plan.items.length, 3); assert.equal(plan.settings.quiet, true);
    assert(plan.items.some(item => item.key.kind === 'teacher')); assert(plan.items.some(item => item.key.kind === 'passage'));
    assert(plan.items.every(item => ['reading', 'writing'].includes(item.skill)));
    assert(plan.items.every(item => item.key.revision !== unpracticed.revision));
    assert.equal((await facts(page)).filter(e => e.fact.kind === 'attempt_recorded').length, attemptsBefore);
    const first = await metadata(page, 'session');
    const draft = 'This exact quiet-session draft survives offline pause.';
    await page.locator('#practice-response').fill(draft); await a.nav('Today');
    await capture(page, 'today-active');
    await page.waitForFunction(() => !!navigator.serviceWorker.controller);
    await a.context.setOffline(true); await page.reload(); await page.locator('main.main h1').waitFor(); await a.nav('Today');
    await panel.getByRole('button', { name: 'Continue this session', exact: false }).click();
    await page.locator('#practice-response').waitFor();
    assert.equal(await page.locator('#practice-response').inputValue(), draft);
    assert.equal((await metadata(page, 'session')).id, first.id);
    assert.equal((await metadata(page, 'daily_plan_active')).id, plan.id);
    await a.context.setOffline(false);
    checkpoint('Reserved start survives failed final acknowledgment with the same session and no completion fact; quiet bounded mixed plan excludes unattempted material and preserves exact draft through offline reload');

    await page.getByRole('button', { name: 'Compare my response', exact: false }).click();
    await page.getByRole('button', { name: 'I recalled it', exact: true }).click();
    await page.getByRole('button', { name: 'Next in this session', exact: false }).click();
    await page.locator('.teacher-return-screen').waitFor();
    const skippedSession = await page.locator('.teacher-return-screen').getAttribute('data-active-review-session');
    const skippedIndex = await page.locator('.teacher-return-screen').getAttribute('data-active-review-index');
    const skippedText = 'Keep this unfinished Teacher response when I leave it for later.';
    await page.locator('.teacher-return-screen textarea').fill(skippedText); await a.nav('Today');
    const teacherIndex = plan.items.findIndex(item => item.key.kind === 'teacher');
    await page.locator(`[data-daily-plan-index="${teacherIndex}"]`).getByRole('button', { name: 'Leave for later', exact: true }).click();
    assert.equal(await metadata(page, 'teacher_revisit_active'), null);
    assert.equal((await facts(page)).filter(e => e.fact.kind === 'attempt_recorded').length, attemptsBefore + 1);
    const nextControls = panel.locator('details').filter({ has: page.locator('#daily-plan-budget') });
    await nextControls.locator(':scope > summary').click();
    await page.locator('#daily-plan-budget').selectOption('1'); await page.locator('#daily-plan-quiet').uncheck();
    const currentPlan = await metadata(page, 'daily_plan_active');
    assert.equal(currentPlan.items.length, 3); assert.equal(currentPlan.settings.quiet, true);
    assert.equal((await metadata(page, 'daily_plan_settings')).response_budget, 1);
    await panel.getByRole('button', { name: 'Continue this session', exact: false }).click();
    await page.locator('#practice-response').waitFor();
    assert.notEqual((await metadata(page, 'session')).id, first.id);
    assert.equal(await page.locator('#practice-response').inputValue(), '');
    await page.waitForFunction(() => document.activeElement === document.querySelector('main.main h1'));
    await page.locator('#practice-response').fill('A second saved generic response.');
    await page.getByRole('button', { name: 'Compare my response', exact: false }).click();
    await page.getByRole('button', { name: 'I recalled it', exact: true }).click();
    await page.getByText('Your short session is finished. You can stop here.', { exact: true }).waitFor();
    await a.nav('Today'); await panel.getByRole('heading', { name: 'A good place to stop', exact: true }).waitFor();
    assert((await panel.innerText()).includes('2 of 3 responses saved'));
    assert.equal(await panel.getByRole('button', { name: 'Continue this session', exact: false }).count(), 0);
    await capture(page, 'today-finished');
    checkpoint('Active Teacher skip preserves its unfinished response without counting it; next item is a fresh keyed passage; settings affect only a future plan and two saved plus one skipped reaches an explicit stop');

    await a.nav('Progress');
    await page.getByRole('heading', { name: 'Keep all four skills in view', exact: true }).waitFor();
    await page.getByText('Later recorded support also applies', { exact: false }).waitFor();
    assert.equal(await page.locator('[data-progress-skill]').count(), 4);
    const recentID = await page.locator('.progress-practice-row').filter({ hasText: 'Later recorded support also applies' }).getAttribute('data-progress-attempt');
    const unchanged = (await facts(page)).find(e => e.id === recentID); assert.deepEqual(unchanged.fact.data.cues, []);
    for (const [name, width, height] of [['desktop', 1200, 900], ['phone', 390, 844], ['tablet', 820, 1180]]) {
      await page.setViewportSize({ width, height });
      assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
      layouts.push({ name, width, height });
      await page.screenshot({ path: path.join(OUTPUT, `${name}-progress-full.png`), fullPage: true });
      await page.evaluate(() => scrollTo(0, 0)); await page.screenshot({ path: path.join(OUTPUT, `${name}-progress-viewport.png`) });
    }
    await page.locator(`[data-progress-attempt="${recentID}"]`).getByRole('button', { name: 'Return to this question', exact: false }).click();
    await page.locator('.teacher-return-screen').waitFor();
    assert.equal(await page.locator('.teacher-return-screen').getAttribute('data-active-review-index'), '1');
    await page.getByRole('button', { name: 'Pause & return to Today', exact: true }).click();
    await panel.getByRole('heading', { name: 'A good place to stop', exact: true }).waitFor();
    const browse = page.locator('details').filter({ has: page.getByText('Browse your saved Teacher returns', { exact: true }) });
    await browse.locator(':scope > summary').click();
    await page.locator(`.teacher-return-row[data-review-index="${skippedIndex}"]`).getByRole('button', { name: 'Try this again', exact: false }).click();
    assert.equal(await page.locator('.teacher-return-screen').getAttribute('data-active-review-session'), skippedSession);
    assert.equal(await page.locator('.teacher-return-screen textarea').inputValue(), skippedText);
    await page.getByRole('button', { name: 'Pause & return to Today', exact: true }).click();
    checkpoint('Progress uses seven-day four-skill activity, shows late support without rewriting raw cues and links to the exact Teacher task; skipped work remains independently resumable outside the finished plan');

    const previousPlanID = (await metadata(page, 'daily_plan_active')).id;
    const attemptsBeforeNewPlan = (await facts(page)).filter(e => e.fact.kind === 'attempt_recorded').length;
    await panel.getByRole('button', { name: 'Choose another short session', exact: true }).click();
    await page.waitForFunction(previous => {
      const plan = JSON.parse(window.rufrenLibrary.get('rufren.v1.meta.daily_plan_active'));
      return plan?.id && plan.id !== previous;
    }, previousPlanID);
    const nextPlan = await metadata(page, 'daily_plan_active');
    assert.equal(nextPlan.settings.response_budget, 1); assert.equal(nextPlan.settings.quiet, false);
    assert(nextPlan.items.length <= 1);
    assert.equal((await facts(page)).filter(e => e.fact.kind === 'attempt_recorded').length, attemptsBeforeNewPlan);
    const oldKey = plan.items[teacherIndex].key;
    const oldDraft = await metadata(page, `teacher_revisit:${oldKey.document_id}:${oldKey.practice_index}`);
    assert.equal(oldDraft.id, skippedSession); assert.equal(oldDraft.response, skippedText);
    await a.nav('Today');
    const reopenedBrowse = page.getByText('Browse your saved Teacher returns', { exact: true });
    if (!await reopenedBrowse.evaluate(node => node.closest('details').open)) await reopenedBrowse.click();
    await page.locator(`.teacher-return-row[data-review-index="${skippedIndex}"]`).getByRole('button', { name: 'Try this again', exact: false }).click();
    assert.equal(await page.locator('.teacher-return-screen').getAttribute('data-active-review-session'), skippedSession);
    assert.equal(await page.locator('.teacher-return-screen textarea').inputValue(), skippedText);
    assert.equal((await metadata(page, 'daily_plan_active')).id, nextPlan.id);
    checkpoint('Explicit new session detaches optional paused work, applies the new one-response/nonquiet settings and creates no completion; the exact old Teacher draft remains resumable without replacing the new plan');

    const early = 1793620801000;
    b = await create('B', early, original); await b.nav('Progress');
    const expectedPast = original.filter(e => e.fact.kind === 'attempt_recorded' && e.recorded_at_ms <= early && e.recorded_at_ms >= early - 7 * DAY);
    const expectedFuture = original.filter(e => e.fact.kind === 'attempt_recorded' && e.recorded_at_ms > early);
    for (const skill of ['listening', 'speaking', 'reading', 'writing']) {
      const value = Number(await b.page.locator(`[data-progress-skill="${skill}"] > strong`).innerText());
      assert.equal(value, expectedPast.filter(e => e.fact.data.skill === skill).length);
    }
    const clock = b.page.locator('details.progress-clock'); await clock.locator(':scope > summary').click();
    assert((await clock.innerText()).includes(String(expectedFuture.length)));
    assert((await clock.innerText()).includes('ahead'));
    assert.equal((await facts(b.page)).filter(e => e.fact.kind === 'attempt_recorded').length, 14);
    await b.page.screenshot({ path: path.join(OUTPUT, 'future-qualified-progress.png'), fullPage: true });
    checkpoint('Independent earlier-clock profile excludes eight future attempts from current/recent counts while preserving and exposing all original records for inspection');
    assert.deepEqual(errors, []); assert.deepEqual(forbidden, []);
    await fs.writeFile(path.join(OUTPUT, 'result.json'), JSON.stringify({ ok: true, fixture: 'Synthetic activity/clock; no model/speech/relay routes',
      checks, layouts, plan, nextPlan, skippedSession, expected_past: expectedPast.length, expected_future: expectedFuture.length,
      reservation: { reserved_session: reservedID, interrupted, recovered, attempts_before_start: attemptsBefore,
        attempts_after_finish: (await facts(page)).filter(e => e.fact.kind === 'attempt_recorded').length },
      events: await facts(page), errors, forbidden }, null, 2));
    console.log('PASS bounded mixed daily plan, quiet/budget, skip/pause/offline/stop and qualified linked Progress');
  } catch (error) {
    if (a) { await a.page.screenshot({ path: path.join(OUTPUT, 'failure.png'), fullPage: true }).catch(() => {});
      await fs.writeFile(path.join(OUTPUT, 'failure.txt'), await a.page.locator('body').innerText()).catch(() => {}); }
    await fs.writeFile(path.join(OUTPUT, 'failure-receipt.json'), JSON.stringify({ error: String(error), checks, errors, forbidden,
      stored_plan: a ? await metadata(a.page, 'daily_plan_active').catch(() => null) : null,
      session: a ? await metadata(a.page, 'session').catch(() => null) : null,
      events: a ? await facts(a.page).catch(() => []) : [],
    }, null, 2));
    throw error;
  } finally { for (const { label, context } of contexts) { await context.tracing.stop({ path: path.join(OUTPUT, `trace-${label}.zip`) }).catch(() => {}); await context.close(); } await browser.close(); }
}
if (require.main === module) main().catch(error => { console.error(error); process.exitCode = 1; });
