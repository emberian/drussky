// Narrow final-bundle UI check using archived actual outputs. No live bridge,
// model/speech/ASR request or synthetic relabelling of the saved provenance.
const { chromium } = require(process.env.RUFREN_PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const crypto = require('node:crypto');
const { APP, status, facts, learningEvidence } = require('./test-teacher-dialogue.cjs');
const { navigate, assertNavigation } = require('./qa-navigation.cjs');
const RELAY = 'http://127.0.0.1:18997'; // Status only, entirely intercepted.
const OUTPUT = path.resolve(process.env.RUFREN_DIALOGUE_SMOKE_OUTPUT || '.local/teacher-dialogue-qa/final-actual-output-smoke');
const SOURCE = path.resolve(process.env.RUFREN_DIALOGUE_SMOKE_SOURCE || '.local/teacher-dialogue-live/acceptance/result.json');
function replyID(response) {
  const p = response.provenance;
  const ns = Buffer.from('6ba7b8119dad11d180b400c04fd430c8', 'hex');
  const b = crypto.createHash('sha1').update(ns).update(`rufren:teacher-dialogue-reply:${p.provider}:${p.model}:${p.response_id}`).digest().subarray(0, 16);
  b[6] = (b[6] & 15) | 80; b[8] = (b[8] & 63) | 128;
  const h = b.toString('hex'); return `${h.slice(0, 8)}-${h.slice(8, 12)}-${h.slice(12, 16)}-${h.slice(16, 20)}-${h.slice(20)}`;
}

async function main() {
  const sourceBytes = await fs.readFile(SOURCE), source = JSON.parse(sourceBytes);
  assert.equal(source.ok, true); assert([2, 4].includes(source.actual_model_calls));
  assert.equal(source.replies.length, source.actual_model_calls);
  await fs.mkdir(OUTPUT, { recursive: true });
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({ viewport: { width: 1200, height: 900 } });
  await context.tracing.start({ screenshots: true, snapshots: true, sources: true });
  const page = await context.newPage(), errors = [], forbidden = [], requests = [], layouts = [], checks = [];
  const exposure = { withheld_help_ids: [], russian_before_help: [], visible_help_presentations: [] };
  page.on('pageerror', error => errors.push(error.message));
  await context.route('**/*', async route => {
    const req = route.request(), url = new URL(req.url());
    if (url.origin === new URL(APP).origin && !url.pathname.startsWith('/api/')) return route.continue();
    const headers = { 'Access-Control-Allow-Origin': '*', 'Access-Control-Allow-Headers': '*', 'Access-Control-Allow-Methods': 'GET,POST,OPTIONS' };
    if (url.origin === RELAY && req.method() === 'OPTIONS') return route.fulfill({ status: 204, headers });
    if (url.origin === RELAY && url.pathname === '/api/teacher/status') {
      requests.push({ path: url.pathname, method: req.method() });
      return route.fulfill({ status: 200, headers, contentType: 'application/json', body: JSON.stringify(status) });
    }
    forbidden.push({ url: req.url(), method: req.method() }); return route.abort('blockedbyclient');
  });
  const library = async () => { const box = page.locator('details.dialogue-library'); if (await box.getAttribute('open') === null) await box.locator(':scope > summary').click(); };
  const open = async id => { await library(); await page.locator(`[data-open-dialogue-leaf="${id}"]`).click();
    await page.waitForFunction(id => document.querySelector('.teacher-dialogue')?.dataset.dialogueLeaf === id, id); };
  const latestResponse = source.replies.at(-1), latest = replyID(latestResponse);
  const clarificationResponse = source.events.find(e => e.fact.kind === 'teacher_dialogue_reply_recorded'
    && e.fact.data.request.config.intent === 'clarification').fact.data;
  const clarification = replyID(clarificationResponse);
  const sceneStart = source.events.find(e => e.fact.kind === 'teacher_dialogue_user_recorded'
    && e.fact.data.action === 'start_scene' && e.fact.data.thread_id === latestResponse.request.thread_id).fact.data;
  try {
    await page.goto(APP); await page.locator('main.main h1').waitFor();
    await page.evaluate(events => {
      for (const event of events) window.rufrenLibrary.write(`rufren.v1.event.${event.id}`, JSON.stringify(event));
      window.rufrenLibrary.write('rufren.v1.meta.teacher_surface', JSON.stringify('dialogues'));
    }, source.events);
    await navigate(page, 'Settings'); await page.locator('#sync-url').fill(RELAY); await page.locator('#sync-key').fill('read-only-archive-status-fixture');
    await navigate(page, 'Teacher'); await open(latest);
    const root = page.locator('.teacher-dialogue');
    assert.equal(await root.locator('.dialogue-heading h2').innerText(), 'Conversation');
    const scene = root.locator('details.dialogue-scene-context');
    assert.equal(await scene.getAttribute('open'), null);
    assert.equal(await scene.locator('p').isVisible(), false);
    await scene.locator(':scope > summary').click();
    assert.equal(await scene.locator('p').innerText(), sceneStart.config.topic);
    await scene.locator(':scope > summary').click();
    const start = page.locator(`article[data-message-id="${sceneStart.request_id}"]`);
    assert.equal(await start.innerText(), 'Scene started');
    assert.equal(await start.locator('.dialogue-speaker').count(), 0);
    const reply = page.locator(`article[data-message-id="${latest}"]`);
    const help = reply.locator('details.dialogue-reply-help');
    assert.equal(await help.getAttribute('open'), null);
    assert.equal(await help.locator('.teacher-prose').isVisible(), false);
    const helpObservers = await help.locator('[id^="russian-presentation-"]').evaluateAll(nodes => nodes
      .filter(node => /[\u0400-\u052f]/u.test(node.textContent)).map(node => node.id.slice('russian-presentation-'.length)));
    const beforeHelp = new Set((await facts(page)).map(event => event.id));
    for (const id of helpObservers) assert(!beforeHelp.has(id), 'Collapsed current help observer cannot claim presentation');
    exposure.withheld_help_ids = helpObservers;
    const chunks = reply.locator('.dialogue-russian[id^="russian-presentation-"]');
    for (let index = 0; index < await chunks.count(); index++) {
      const chunk = chunks.nth(index); await chunk.scrollIntoViewIfNeeded();
      assert(await chunk.evaluate(node => !!(node.compareDocumentPosition(node.closest('article').querySelector('.dialogue-reply-help')) & Node.DOCUMENT_POSITION_FOLLOWING)), 'Spoken Russian precedes optional English explanation');
      const id = (await chunk.getAttribute('id')).slice('russian-presentation-'.length);
      await page.waitForFunction(id => window.rufrenLibrary.entries().some(([key]) => key === `rufren.v1.event.${id}`), id);
      exposure.russian_before_help.push((await facts(page)).find(event => event.id === id));
    }
    await help.locator(':scope > summary').click();
    for (const id of helpObservers) {
      await page.locator(`#russian-presentation-${id}`).scrollIntoViewIfNeeded();
      await page.waitForFunction(id => window.rufrenLibrary.entries().some(([key]) => key === `rufren.v1.event.${id}`), id);
      const event = (await facts(page)).find(event => event.id === id);
      assert.equal(event.fact.kind, 'stimulus_presented'); assert.equal(event.fact.data.purpose, 'instruction');
      exposure.visible_help_presentations.push(event);
    }
    await help.locator(':scope > summary').click();
    const correction = latestResponse.content.correction; assert(correction);
    assert((await reply.locator('.dialogue-small-correction').innerText()).includes(`${correction.original} → ${correction.replacement}`));
    for (const chunk of [latestResponse.content.utterance, latestResponse.content.next_question].filter(Boolean))
      assert((await reply.locator('.dialogue-russian').allTextContents()).includes(chunk.russian));
    checks.push('Archived actual outputs render short scene heading/full preserved context, quiet StartScene, actual Russian turn and targeted correction; non-Ask help remains optional');

    for (const [name, width, height] of [['phone', 390, 844], ['tablet', 820, 1180]]) {
      await page.setViewportSize({ width, height }); await navigate(page, 'Teacher'); await assertNavigation(page, layouts);
      await page.locator('#dialogue-message').fill(`Unsent final ${name} message`);
      await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
      assert(await page.locator('#dialogue-message').evaluate(node => document.activeElement === node));
      await page.evaluate(() => scrollTo(0, 0));
      const position = await root.locator('.dialogue-heading h2').boundingBox();
      const header = await page.locator('.navigation-heading').boundingBox();
      layouts.push({ name, scene_heading: position, shell_header: header });
      assert(position.height <= 70, 'Short scene heading cannot consume six phone lines');
      await page.screenshot({ path: path.join(OUTPUT, `${name}-top.png`) });
      await reply.scrollIntoViewIfNeeded();
      await page.screenshot({ path: path.join(OUTPUT, `${name}-latest-reply.png`) });
      await page.screenshot({ path: path.join(OUTPUT, `${name}-full.png`), fullPage: true });
    }
    checks.push('Final phone/tablet compact heading, Menu, keyboard focus and ordinary typing pass; current hidden-help observers have no fact until visible opening');
    await open(clarification);
    assert.equal(await page.locator('.dialogue-source').getAttribute('data-dialogue-source-document'), source.source_document);
    const sourceBox = page.locator('details.dialogue-source'); assert.equal(await sourceBox.getAttribute('open'), null);
    await sourceBox.locator(':scope > summary').click();
    assert((await sourceBox.innerText()).includes('register'));
    await page.getByRole('button', { name: 'Return to the source lesson or feedback', exact: true }).click();
    assert.equal(await page.locator('.teacher-lesson-flow').getAttribute('data-lesson-document'), source.source_document);
    assert.equal(await page.locator('.teacher-lesson-current').getAttribute('data-step-id'), 'register');
    const after = await facts(page), actual = new Map(after.map(event => [event.id, event]));
    for (const event of source.events) assert.deepEqual(actual.get(event.id), event, 'Rendering preserves exact archived source/user/reply bytes');
    assert.deepEqual(learningEvidence(after), []); assert.deepEqual(errors, []); assert.deepEqual(forbidden, []);
    checks.push('Final phone/tablet Menu, focus and compact scene geometry pass; actual clarification retains its exact register-step source; no new model/audio or practice evidence');
    await fs.writeFile(path.join(OUTPUT, 'result.json'), JSON.stringify({ ok: true, source_path: SOURCE,
      source_sha256: crypto.createHash('sha256').update(sourceBytes).digest('hex'), scope: 'Archived actual output rendered on final bundle; zero new inference, TTS or ASR',
      new_model_calls: 0, checks, layouts, exposure, requests, errors, forbidden }, null, 2));
    console.log('PASS final archived-actual-output phone/tablet/source/focus smoke; zero inference');
  } catch (error) {
    await page.screenshot({ path: path.join(OUTPUT, 'failure.png'), fullPage: true }).catch(() => {});
    await fs.writeFile(path.join(OUTPUT, 'failure.txt'), await page.locator('body').innerText()).catch(() => {});
    await fs.writeFile(path.join(OUTPUT, 'failure-receipt.json'), JSON.stringify({ error: String(error), checks, layouts, exposure, requests, errors, forbidden }, null, 2));
    throw error;
  } finally { await context.tracing.stop({ path: path.join(OUTPUT, 'trace.zip') }).catch(() => {}); await context.close(); await browser.close(); }
}
if (require.main === module) main().catch(error => { console.error(error); process.exitCode = 1; });
module.exports = { replyID };
