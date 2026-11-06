// Read-only rendering of an already captured real v7 response. No model request.
const { chromium } = require(process.env.RUFREN_PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const { navigate } = require('./qa-navigation.cjs');
const APP = process.env.RUFREN_QA_URL || 'http://127.0.0.1:8091';
const OUTPUT = path.resolve(process.env.RUFREN_SAVED_REPAIR_OUTPUT || '.local/teacher-feedback-repair-final-qa/saved-v7');
async function main() {
  const response = JSON.parse(await fs.readFile('.local/teaching-repair-live/case-03-revisit/response.json', 'utf8'));
  const acceptance = JSON.parse(await fs.readFile('.local/teacher-revisit-v6-live/acceptance/result.json', 'utf8'));
  const events = acceptance.clients[0].events;
  await fs.mkdir(OUTPUT, { recursive: true });
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({ viewport: { width: 390, height: 844 } });
  const forbidden = [], errors = [];
  await context.route('**/*', route => {
    const url = new URL(route.request().url());
    if (url.origin === new URL(APP).origin && !url.pathname.startsWith('/api/')) return route.continue();
    forbidden.push(route.request().url()); return route.abort('blockedbyclient');
  });
  const page = await context.newPage(); page.on('pageerror', error => errors.push(error.message));
  await context.tracing.start({ screenshots: true, snapshots: true, sources: true });
  try {
    await page.goto(APP); await page.locator('main.main h1').waitFor();
    const offline_build = await page.evaluate(async () => (await fetch('/offline-build.json')).json());
    await page.evaluate(({ events, response, source }) => {
      for (const event of events) window.rufrenLibrary.write(`rufren.v1.event.${event.id}`, JSON.stringify(event));
      window.rufrenLibrary.write('rufren.v1.meta.teacher_response', JSON.stringify(response));
      window.rufrenLibrary.write('rufren.v1.meta.teacher_lesson_response', JSON.stringify(source));
    }, { events, response, source: acceptance.capture.source_response });
    await navigate(page, 'Teacher');
    await page.locator('.teacher-feedback-task').waitFor();
    assert((await page.locator('.teacher-feedback-task').innerText()).includes(response.request.input.response));
    assert((await page.locator('body').innerText()).includes(response.content.explanation));
    const next = page.getByRole('button', { name: 'Try this question again', exact: true });
    assert.equal(await next.count(), 1, 'Actual captured response proves its exact archived task for repair');
    await page.locator('.teacher-feedback-task').scrollIntoViewIfNeeded();
    await page.screenshot({ path: path.join(OUTPUT, 'feedback-phone.png'), fullPage: true });
    await next.click(); await page.locator('.teacher-feedback-repair').waitFor();
    await page.screenshot({ path: path.join(OUTPUT, 'repair-phone.png'), fullPage: true });
    assert.equal(await page.locator('.teacher-feedback-repair textarea').inputValue(), '');
    assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    assert.deepEqual(errors, []); assert.deepEqual(forbidden, []);
    await fs.writeFile(path.join(OUTPUT, 'result.json'), JSON.stringify({ ok: true, offline_build, fixture: 'Previously captured v7 model response and untouched archived source/events; render-only, no new model calls. Repair start is synthetic browser use.', source: acceptance.capture.source_path, response_provenance: response.provenance, requests: forbidden, errors, body: await page.locator('body').innerText() }, null, 2));
  } catch(error) {
    await page.screenshot({ path: path.join(OUTPUT, 'failure.png'), fullPage: true });
    await fs.writeFile(path.join(OUTPUT, 'failure.json'), JSON.stringify({ error: String(error), errors, forbidden, body: await page.locator('body').innerText() }, null, 2)); throw error;
  } finally { await context.tracing.stop({ path: path.join(OUTPUT, 'trace.zip') }); await context.close(); await browser.close(); }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
