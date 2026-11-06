// Consumer acceptance: all teaching/ASR/audio is explicitly synthetic. No inference.
const { chromium } = require(process.env.RUFREN_PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const { TASKS, TONES, SEED_TIME, REVIEW_TIME, syntheticReplica, facts, fixtureRoutes,
  createDriver, seedGuided } = require('./test-teacher-revisit.cjs');
const OUTPUT = path.resolve(process.env.RUFREN_REPAIR_QA_OUTPUT || '.local/teacher-feedback-repair-qa');
const settle = page => page.evaluate(() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r))));
async function main() {
  await fs.mkdir(OUTPUT, { recursive: true });
  const audit = { requests: [], forbidden: [], errors: [], verdict: {} }, checks = [];
  const browser = await chromium.launch({ headless: true });
  const d = await createDriver(browser, 'repair', SEED_TIME, fixtureRoutes(syntheticReplica(), audit), audit);
  const { page } = d;
  audit.offline_build = await page.evaluate(async () => (await fetch('/offline-build.json')).json());
  const repair = page.locator('.teacher-feedback-repair');
  const source = await seedGuided(d, audit);
  const draft = index => page.evaluate(({ document, index }) => JSON.parse(window.rufrenLibrary.get(`rufren.v1.meta.teacher_practice:${document}:${index}`)), { document: source.document, index });
  const start = async () => {
    await page.getByRole('button', { name: 'Try this question again', exact: true }).click();
    await repair.waitFor(); await settle(page);
    assert(await repair.locator('h3').evaluate(e => document.activeElement === e), 'Repair entry focuses its heading');
  };
  const immutable = async originals => {
    const all = await facts(page);
    for (const original of originals) assert.deepEqual(all.find(e => e.id === original.id), original, 'Earlier events are immutable');
  };
  try {
    for (const index of [0, 1, 2, 3]) {
      await d.now(REVIEW_TIME + index * 1000); await d.open(index);
      if (index < 2) await d.saveText(`Synthetic wrong response ${index}`);
      else if (index === 2) {
        await d.screen.locator('textarea').fill('Synthetic wrong number');
        await page.waitForFunction(() => document.querySelector('.teacher-return-screen [data-practice-rate="1"]')?.disabled === false);
        await d.screen.locator('[data-practice-rate="1"]').first().click();
        await page.waitForFunction(() => document.querySelector('.teacher-return-screen audio')?.currentTime > .02);
        await d.screen.getByRole('button', { name: 'Discuss what I heard', exact: false }).click();
        await d.screen.locator('.teacher-return-feedback').waitFor();
      } else {
        await d.screen.getByRole('button', { name: 'Open recorder for this exercise', exact: true }).click();
        await d.screen.locator('#audio-import-attempt').setInputFiles({ name: 'synthetic-review-tone-not-speech.wav', mimeType: 'audio/wav', buffer: TONES.review.wav });
        await page.waitForFunction(() => [...document.querySelectorAll('.teacher-return-screen button')].some(b => b.textContent.includes('Keep this spoken response') && !b.disabled));
        await d.screen.getByRole('button', { name: 'Keep this spoken response', exact: false }).click();
        await d.screen.getByRole('button', { name: 'Find the words in this take', exact: true }).click();
        await d.screen.locator('.transcript-words textarea').fill('Не хотите чаю?');
        await d.screen.getByRole('button', { name: 'Use these words & discuss them', exact: false }).click();
        await d.screen.locator('.teacher-return-feedback').waitFor();
      }
      const archived = d.screen.locator('details').filter({ has: page.locator(':scope > summary').filter({ hasText: 'Saved review details' }) });
      assert.equal(await archived.count(), 1);
      assert.equal(await archived.evaluate(node => node.open), false, 'Completed original form is collapsed beside visible exact feedback context');
      assert.equal(await archived.locator('.teacher-exercise').isVisible(), false);
      const submitted = audit.requests.filter(r => r.request?.mode === 'answer_feedback').at(-1).request.input;
      const context = d.screen.locator('.teacher-feedback-task');
      assert((await context.innerText()).includes(TASKS[index].prompt));
      assert((await context.innerText()).includes(submitted.response));
      if (index === 0) assert((await context.innerText()).includes(TASKS[index].reference_russian), 'Reading feedback keeps exact Russian input beside the task');
      const originals = await facts(page), before = await draft(index);
      if (index === 0) {
        await page.evaluate(() => {
          window.repairQAWrite = window.rufrenLibrary.write;
          window.rufrenLibrary.write = (key, value) => {
            if (key.includes('meta.teacher_practice:')) throw Error('Synthetic repair draft-write rejection');
            return window.repairQAWrite(key, value);
          };
        });
        await page.getByRole('button', { name: 'Try this question again', exact: true }).click();
        await page.getByText('Browser storage is full or unavailable; your change was not saved', { exact: true }).waitFor();
        assert.equal(await repair.count(), 0, 'Failed durable write never exposes editable repair');
        assert.deepEqual(await draft(index), before, 'Failed repair keeps exact saved source draft');
        await immutable(originals);
        await page.evaluate(() => window.rufrenLibrary.write = window.repairQAWrite);
      }
      await start();
      assert.equal(await page.getByText('Browser storage is full or unavailable; your change was not saved', { exact: true }).count(), 0, 'Successful repair clears stale storage rejection');
      assert((await repair.innerText()).includes(TASKS[index].prompt), 'Repair presents the exact original question');
      const fresh = await draft(index);
      assert.notEqual(fresh.id, before.id); assert.equal(fresh.saved_attempt, null);
      assert.equal(fresh.response, '');
      assert.equal(fresh.audio?.selected_recording ?? null, null);
      assert.equal(fresh.audio?.reference_plays ?? 0, 0);
      const starts = (await facts(page)).filter(e => e.fact.kind === 'review_started' && e.fact.data.session === fresh.id);
      assert.equal(starts.length, 1, 'One durable start exists before editing');
      if (index === 3) {
        assert.equal(await repair.locator('.spoken-take').count(), 0, 'Old oral take is never selected into fresh repair');
        assert.equal(await repair.locator('.transcript-words').count(), 0, 'Old confirmed words are not a fresh response');
        await repair.getByRole('button', { name: 'Open recorder for this exercise', exact: true }).click();
        const keep = repair.getByRole('button', { name: 'Keep this spoken response', exact: false });
        assert(await keep.isDisabled(), 'Opening repair recorder cannot submit the old selected take');
        await repair.locator('#audio-import-attempt').setInputFiles({ name: 'synthetic-repair-fresh-tone-not-speech.wav', mimeType: 'audio/wav', buffer: TONES.later.wav });
        await page.waitForFunction(() => [...document.querySelectorAll('.teacher-feedback-repair button')].some(b => b.textContent.includes('Keep this spoken response') && !b.disabled));
        await keep.click(); await repair.locator('.spoken-take').waitFor();
        await repair.getByRole('button', { name: 'Find the words in this take', exact: true }).click();
        await repair.locator('.transcript-words textarea').fill('Хотите чаю?');
        await repair.getByRole('button', { name: 'Use these words & discuss them', exact: false }).click();
        await page.waitForFunction(session => {
          const events = window.rufrenLibrary.entries().filter(([k]) => k.startsWith('rufren.v1.event.')).map(([, v]) => JSON.parse(v));
          const attempt = events.find(e => e.fact.kind === 'attempt_recorded' && e.fact.data.session === session);
          return attempt && events.some(e => e.fact.kind === 'ai_assessment_recorded' && e.fact.data.attempt === attempt.id);
        }, fresh.id);
        const all = await facts(page), next = all.find(e => e.fact.kind === 'attempt_recorded' && e.fact.data.session === fresh.id);
        const attached = all.find(e => e.fact.kind === 'recording_attached' && e.fact.data.target.id === next.id);
        assert.equal(attached.fact.data.asset.sha256, TONES.later.hash, 'Repair attaches the fresh take, never the earlier recording');
        assert.notEqual(next.id, submitted.attempt_id);
        const spokenRequest = audit.requests.filter(r => r.request?.mode === 'answer_feedback').at(-1).request.input;
        assert.equal(spokenRequest.prompt, TASKS[index].prompt);
        assert.equal(spokenRequest.response, 'Хотите чаю?'); assert.equal(spokenRequest.evidence, 'transcript');
        assert.notEqual(spokenRequest.confirmed_transcript_id, submitted.confirmed_transcript_id);
      } else {
        const text = `Synthetic unfinished repair ${index}`;
        const field = repair.locator('textarea'); assert.equal(await field.inputValue(), '');
        await field.fill(text); await settle(page);
        assert(await field.evaluate(e => document.activeElement === e), 'Typing retains focus');
        await page.setViewportSize({ width: 390, height: 844 }); await settle(page);
        assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'Phone has no horizontal overflow');
        await page.screenshot({ path: path.join(OUTPUT, `repair-${index}-phone.png`), fullPage: true });
        await page.setViewportSize({ width: 820, height: 1180 }); await settle(page);
        assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'Tablet has no horizontal overflow');
        if (index === 0) {
          await page.reload(); await page.locator('main.main h1').waitFor();
          await d.nav('Practice');
          await d.screen.locator('.teacher-return-feedback').waitFor(); await start();
          assert.equal(await repair.locator('textarea').inputValue(), text, 'Reload and repeat action resumes unfinished response');
          assert.equal((await draft(index)).id, fresh.id, 'Repeat action retains the unfinished session');
          assert.equal((await facts(page)).filter(e => e.fact.kind === 'review_started' && e.fact.data.session === fresh.id).length, 1);
          await d.connect(); await d.nav('Practice');
        }
      }
      if (index === 1 || index === 2) {
        if (index === 2) {
          const discuss = repair.getByRole('button', { name: 'Discuss what I heard', exact: false });
          assert(await discuss.isDisabled(), 'Fresh listening repair requires its own reference playback');
          await page.waitForFunction(() => document.querySelector('.teacher-feedback-repair [data-practice-rate="1"]')?.disabled === false);
          await repair.locator('[data-practice-rate="1"]').first().click();
          await page.waitForFunction(() => document.querySelector('.teacher-feedback-repair audio')?.currentTime > .02);
          await discuss.click();
        } else await repair.getByRole('button', { name: 'Discuss my answer', exact: false }).click();
        await page.waitForFunction(session => window.rufrenLibrary.entries().some(([key, raw]) => {
          if (!key.startsWith('rufren.v1.event.')) return false;
          const event = JSON.parse(raw); return event.fact.kind === 'attempt_recorded' && event.fact.data.session === session;
        }), fresh.id);
        const next = (await facts(page)).find(e => e.fact.kind === 'attempt_recorded' && e.fact.data.session === fresh.id);
        assert.notEqual(next.id, submitted.attempt_id);
        assert.equal(next.fact.data.response, `Synthetic unfinished repair ${index}`);
        assert.deepEqual(next.fact.data.cues, ['reference_material'], 'Immediate correction work records supported practice');
        await page.waitForFunction(attempt => window.rufrenLibrary.entries().some(([key, raw]) => {
          if (!key.startsWith('rufren.v1.event.')) return false;
          const event = JSON.parse(raw); return event.fact.kind === 'ai_assessment_recorded' && event.fact.data.attempt === attempt;
        }), next.id);
        const nextRequest = audit.requests.filter(r => r.request?.mode === 'answer_feedback').at(-1).request.input;
        assert.equal(nextRequest.prompt, TASKS[index].prompt);
        assert.equal(nextRequest.response, next.fact.data.response);
      }
      await immutable(originals);
      checks.push({ skill: TASKS[index].skill, original_attempt: submitted.attempt_id, repair_session: fresh.id, durable_start: starts[0]?.id });
      await d.finish();
    }
    assert.deepEqual(audit.errors, []); assert.deepEqual(audit.forbidden, []);
    await fs.writeFile(path.join(OUTPUT, 'result.json'), JSON.stringify({ ok: true, fixture: 'Synthetic teaching, ASR and deterministic tones; no speech/model or learning claims', checks, ...audit, events: await facts(page) }, null, 2));
    console.log('PASS synthetic four-skill feedback repair: exact context, fresh response, oral selection cleared, durable resume and responsive focus');
  } catch (error) {
    await page.screenshot({ path: path.join(OUTPUT, 'failure.png'), fullPage: true }).catch(() => {});
    await fs.writeFile(path.join(OUTPUT, 'failure.json'), JSON.stringify({ error: String(error), checks, ...audit, events: await facts(page), body: await page.locator('body').innerText() }, null, 2));
    throw error;
  } finally {
    await d.context.tracing.stop({ path: path.join(OUTPUT, 'trace.zip') }); await d.context.close(); await browser.close();
  }
}
if (require.main === module) main().catch(error => { console.error(error); process.exitCode = 1; });
