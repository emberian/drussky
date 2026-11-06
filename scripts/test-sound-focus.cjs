// Real bundled Sound Focus UI + exact packaged originals + actual fake-mic capture.
// The single optional ASR response is explicitly synthetic. No inference/main relay.
const { chromium } = require(process.env.RUFREN_PLAYWRIGHT_MODULE || 'playwright');
const fs = require('node:fs/promises');
const path = require('node:path');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const { CURRENT_PROTOCOL, replica, facts } = require('./test-teacher-dialogue.cjs');
const { navigate } = require('./qa-navigation.cjs');
const { recognizedWords } = require('./test-teacher-oral.cjs');
const APP = process.env.RUFREN_QA_URL || 'http://127.0.0.1:8091';
const RELAY = 'http://127.0.0.1:18998'; // Intercepted, no listener.
const OUTPUT = path.resolve(process.env.RUFREN_SOUND_QA_OUTPUT || '.local/sound-focus-qa');
const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const data = event => event.fact.data;
const ofKind = (events, kind) => events.filter(event => event.fact.kind === kind);

async function assetBundle(page) {
  return page.evaluate(async () => {
    const db = await new Promise((resolve, reject) => { const r = indexedDB.open('rufren-audio-v1', 1); r.onsuccess = () => resolve(r.result); r.onerror = () => reject(r.error); });
    const records = await new Promise((resolve, reject) => { const r = db.transaction('assets').objectStore('assets').getAll(); r.onsuccess = () => resolve(r.result); r.onerror = () => reject(r.error); });
    db.close(); const assets = [];
    for (const { blob, ...metadata } of records) {
      const bytes = new Uint8Array(await blob.arrayBuffer()); let binary = '';
      for (let i = 0; i < bytes.length; i += 16384) binary += String.fromCharCode(...bytes.subarray(i, i + 16384));
      assets.push({ metadata, base64: btoa(binary) });
    }
    return assets;
  });
}

function capturedFixture(request) {
  const wav = Buffer.from(request.wav_base64, 'base64');
  assert.equal(sha(wav), request.audio_sha256);
  assert.equal(wav.toString('ascii', 0, 4), 'RIFF'); assert.equal(wav.toString('ascii', 8, 12), 'WAVE');
  let format, pcm;
  for (let offset = 12; offset + 8 <= wav.length;) {
    const size = wav.readUInt32LE(offset + 4), start = offset + 8, tag = wav.toString('ascii', offset, offset + 4);
    assert(start + size <= wav.length);
    if (tag === 'fmt ') format = { code: wav.readUInt16LE(start), channels: wav.readUInt16LE(start + 2), rate: wav.readUInt32LE(start + 4), bits: wav.readUInt16LE(start + 14) };
    if (tag === 'data') pcm = wav.subarray(start, start + size);
    offset = start + size + (size % 2);
  }
  assert(format && pcm); assert.equal(format.channels, 1);
  assert((format.code === 3 && format.bits === 32) || (format.code === 1 && format.bits === 16));
  const samples = pcm.length / (format.bits / 8), floats = Buffer.alloc(samples * 4);
  for (let i = 0; i < samples; i++) floats.writeFloatLE(format.code === 3 ? pcm.readFloatLE(i * 4) : pcm.readInt16LE(i * 2) / 32768, i * 4);
  return { wav, hash: sha(wav), frames: samples, pcmHash: sha(floats), format };
}

async function routes(context, audit) {
  const store = replica(), headers = { 'Access-Control-Allow-Origin': '*', 'Access-Control-Allow-Headers': '*', 'Access-Control-Allow-Methods': 'GET,POST,OPTIONS' };
  await context.route('**/*', async route => {
    const request = route.request(), url = new URL(request.url());
    if (url.origin === new URL(APP).origin && !url.pathname.startsWith('/api/')) return route.continue();
    if (url.origin !== RELAY) { audit.forbidden.push(request.url()); return route.abort('blockedbyclient'); }
    if (audit.offline) return route.abort('internetdisconnected');
    if (request.method() === 'OPTIONS') return route.fulfill({ status: 204, headers });
    const body = request.postDataJSON(); audit.requests.push({ path: url.pathname, request: body }); let response;
    if (url.pathname === '/v1/sync') response = store.exchange(body);
    else if (url.pathname === '/api/teacher/status' || url.pathname === '/api/tutor/status') response = { enabled: false, provider: 'qa_synthetic', model: 'No teacher calls', message: 'Offline sound fixture', data_sent: 'None', max_text_bytes: 8192, max_context_bytes: 2048, remaining_reservations: 0, remaining_budget_micro_usd: 0, accounting: 'subscription_quota' };
    else if (url.pathname === '/api/speech/transcribe') {
      assert.equal(audit.requests.filter(r => r.path === url.pathname).length, 1, 'Only one explicit synthetic ASR request');
      const fixture = capturedFixture(body); response = recognizedWords(body, fixture, 'Лук.');
      response.original_audio = { sample_rate: fixture.format.rate, channels: fixture.format.channels, frames: fixture.frames, bits: fixture.format.bits, encoding: fixture.format.code === 3 ? 'ieee_f32le' : 'pcm_s16le' };
      response.derivative.frames = Math.ceil(fixture.frames * 16000 / fixture.format.rate);
      audit.synthetic_asr = { input_hash: fixture.hash, transcript: response.transcript, target_conditioned: response.target_conditioned };
    } else { audit.forbidden.push(request.url()); return route.abort('blockedbyclient'); }
    return route.fulfill({ status: 200, headers, contentType: 'application/json', body: JSON.stringify(response) });
  });
  return store;
}

async function waitFact(page, kind, predicate = () => true) {
  // Poll the actual durable library; do not synthesize media lifecycle events.
  for (let i = 0; i < 300; i++) { const result = ofKind(await facts(page), kind).filter(predicate); if (result.length) return result; await page.waitForTimeout(50); }
  throw Error(`Timed out waiting for durable ${kind}`);
}
async function playable(player) { await player.locator('[data-practice-rate="1"]:not([disabled])').waitFor(); }
async function play(player) { await playable(player); await player.locator('[data-practice-rate="1"]').click(); }
async function naturalTrial(page, trial) {
  const before = new Set(ofKind(await facts(page), 'sound_reference_played').map(e => e.id));
  await play(trial.locator('.sound-focus-player'));
  return (await waitFact(page, 'sound_reference_played', e => !before.has(e.id) && data(e).stage.kind === 'completed'))[0];
}
async function openContrast(page) {
  await navigate(page, 'Lessons');
  await page.locator('.lesson-link').filter({ hasText: 'Cyrillic through things' }).click();
  await page.locator('button.exercise-choice').filter({ hasText: 'Onion or hatch?' }).click();
  await page.locator('.sound-focus[data-sound-focus-phase="examples"]').waitFor();
}
async function teach(page) {
  assert(await page.getByRole('button', { name: 'Try listening →', exact: true }).isDisabled());
  for (const model of await page.locator('.sound-focus-model').all()) {
    await play(model.locator('.sound-focus-player'));
    await model.getByText('Example played through', { exact: true }).waitFor();
  }
  await page.getByRole('button', { name: 'Try listening →', exact: true }).click();
  await page.locator('[data-trial-token]').waitFor();
}
async function record(page, duration) {
  const before = new Set((await assetBundle(page)).map(a => a.metadata.asset_id));
  await page.locator('[data-action="record"][data-role="attempt"]').click();
  await page.waitForFunction(() => document.getElementById('audio-mic-status')?.dataset.active === 'true');
  await page.waitForTimeout(duration); await page.locator('#audio-stop-recording').click();
  for (let i = 0; i < 300; i++) {
    const created = (await assetBundle(page)).filter(a => !before.has(a.metadata.asset_id) && a.metadata.device?.sound_focus_scope?.origin === 'microphone_capture');
    if (created.length) { await page.waitForFunction(() => document.getElementById('audio-mic-status')?.dataset.active !== 'true'); return created.at(-1); }
    await page.waitForTimeout(50);
  }
  throw Error('Actual microphone original was not durably saved');
}
async function screenshot(page, output, label) {
  for (const [device, width, height] of [['phone', 390, 844], ['tablet', 820, 1080], ['desktop', 1200, 900]]) {
    await page.setViewportSize({ width, height });
    assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), `${label} ${device} overflow`);
    await page.screenshot({ path: path.join(output, `${label}-${device}.png`), fullPage: true });
    await page.screenshot({ path: path.join(output, `${label}-${device}-viewport.png`) });
  }
}
async function exportFixture(page, output, observations) {
  const events = await facts(page), ids = new Set();
  const walk = value => { if (!value || typeof value !== 'object') return; if (typeof value.asset_id === 'string') ids.add(value.asset_id); for (const child of Object.values(value)) walk(child); };
  events.forEach(walk);
  const originals = await assetBundle(page), linked = originals.filter(a => ids.has(a.metadata.asset_id));
  for (const id of ids) assert(linked.some(a => a.metadata.asset_id === id), `Fact-linked original missing: ${id}`);
  await fs.mkdir(path.join(output, 'originals'), { recursive: true });
  for (const original of linked) { const bytes = Buffer.from(original.base64, 'base64'); assert.equal(sha(bytes), original.metadata.content_hash); assert.equal(bytes.length, original.metadata.byte_length); await fs.writeFile(path.join(output, 'originals', `${sha(bytes)}.blob`), bytes); }
  const prepared = ofKind(events, 'sound_trial_prepared'), completed = ofKind(events, 'sound_trial_completed');
  const expected = { scope: 'Frozen source identities from actual facts; order and choices verified in producer UI. Locally unsent state is not a synced fact.', blocks: [...new Set(prepared.map(e => data(e).block_id))].map(block_id => ({ block_id })),
    trials: prepared.map(e => { const p = data(e), c = completed.find(c => data(c).prepared === e.id); return { prepared_id: e.id, trial_id: p.trial_id, block_id: p.block_id, position: p.position, mode: p.mode, ...(p.mode === 'listening' ? { option_member_ids: p.option_member_ids, chosen_member_id: c ? data(c).response.chosen_member_id : null } : {}), completed: !!c }; }), ui_observations: observations };
  await fs.writeFile(path.join(output, 'library.json'), JSON.stringify({ version: CURRENT_PROTOCOL, events }, null, 2));
  await fs.writeFile(path.join(output, 'originals.json'), JSON.stringify({ records: linked.map(a => a.metadata) }, null, 2));
  await fs.writeFile(path.join(output, 'expected-sound.json'), JSON.stringify(expected, null, 2));
  await fs.writeFile(path.join(output, 'local-orphans.json'), JSON.stringify({ scope: 'Locally preserved sources without immutable fact pointers; outside linked-original restore proof.', records: originals.filter(a => !ids.has(a.metadata.asset_id)).map(a => a.metadata) }, null, 2));
  return { events: events.length, prepared: prepared.length, completed: completed.length, originals: linked.length, blocks: expected.blocks };
}

async function main() {
  await fs.mkdir(OUTPUT, { recursive: true });
  const audit = { checks: [], errors: [], forbidden: [], requests: [], observations: [], offline: false, inference_calls: 0 };
  const launch = { headless: true, args: ['--autoplay-policy=no-user-gesture-required', '--use-fake-device-for-media-stream', '--use-fake-ui-for-media-stream'] };
  if (process.env.RUFREN_CHROMIUM_EXECUTABLE) launch.executablePath = process.env.RUFREN_CHROMIUM_EXECUTABLE;
  const browser = await chromium.launch(launch), context = await browser.newContext({ viewport: { width: 1200, height: 900 }, permissions: ['microphone'] });
  await context.tracing.start({ screenshots: true, snapshots: true, sources: true });
  const page = await context.newPage(); page.setDefaultTimeout(30000); page.on('pageerror', error => audit.errors.push(error.message)); await routes(context, audit);
  const check = text => { audit.checks.push(text); console.log(`CHECK ${text}`); };
  try {
    await page.goto(APP, { waitUntil: 'networkidle' }); await page.locator('main.main h1').waitFor();
    audit.artifact = await page.evaluate(() => fetch('./offline-build.json').then(r => r.json()).then(r => r.version));
    if (process.env.RUFREN_EXPECT_WEB) assert.equal(audit.artifact, process.env.RUFREN_EXPECT_WEB);
    await navigate(page, 'Settings'); await page.locator('#sync-url').fill(RELAY); await page.locator('#sync-key').fill('synthetic-sound-no-secret');
    await openContrast(page);
    const block = await page.locator('.sound-focus').getAttribute('data-sound-focus-session');
    const packets = ofKind(await facts(page), 'sound_trial_prepared').filter(e => data(e).block_id === block).sort((a, b) => data(a).position - data(b).position);
    assert.equal(packets.length, 4); assert(packets.every(e => e.version === CURRENT_PROTOCOL));
    assert.equal(new Set(packets.map(e => data(e).trial_id)).size, 4); assert.equal(new Set(packets.map(e => data(e).token)).size, 4);
    const cells = packets.map(e => `${data(e).target_member_id}|${data(e).option_member_ids.join(',')}`); assert.equal(new Set(cells).size, 4);
    const members = data(packets[0]).contrast.members.map(m => m.id);
    for (const member of members) assert.equal(packets.filter(e => data(e).target_member_id === member).length, 2);
    assert.equal(packets.filter(e => data(e).option_member_ids[0] === data(e).target_member_id).length, 2);
    await screenshot(page, OUTPUT, 'labelled-teaching');
    await teach(page);
    assert.equal(ofKind(await facts(page), 'attempt_recorded').length, 0);
    check('Four frozen balanced trials; labelled natural-end teaching creates no completed answer');
    for (let i = 0; i < 4; i++) {
      const packet = packets[i], p = data(packet), trial = page.locator(`[data-trial-token="${p.token}"]`); await trial.waitFor();
      const player = trial.locator('.sound-focus-player'), reference = p.references.find(r => r.member_id === p.target_member_id);
      assert.equal(await player.getAttribute('data-asset'), reference.asset_id); assert.equal(await player.getAttribute('data-expected-hash'), reference.asset.sha256);
      assert.deepEqual(await trial.locator('[data-sound-choice]').evaluateAll(nodes => nodes.map(n => n.dataset.soundChoice)), p.option_member_ids);
      assert(await trial.locator('[data-sound-choice]').first().isDisabled());
      const blind = await trial.innerText(); assert(!/[А-Яа-яЁё]/u.test(blind), 'Blind text contains no Russian answer');
      const accessibleLabels = await trial.locator('[aria-label],[title]').evaluateAll(nodes => nodes.map(n => [n.getAttribute('aria-label'), n.getAttribute('title')]).flat().filter(Boolean).join(' '));
      assert(!/[А-Яа-яЁё]/u.test(accessibleLabels), 'Blind accessible labels contain no Russian answer');
      assert.equal(await trial.locator('select,input,textarea,[data-practice-rate="0.7"],.audio-provenance').count(), 0);
      if (i === 0) {
        await screenshot(page, OUTPUT, 'blind-listening');
        await play(player); await waitFact(page, 'sound_reference_played', e => data(e).scope.prepared === packet.id && data(e).stage.kind === 'started');
        await player.locator('[data-practice-stop]').click(); await page.waitForTimeout(800);
        assert.equal(ofKind(await facts(page), 'sound_reference_played').filter(e => data(e).scope.prepared === packet.id && data(e).stage.kind === 'completed').length, 0);
        assert(await trial.locator('[data-sound-choice]').first().isDisabled()); check('Started then stopped sound cannot unlock a blind answer');
      }
      if (i === 1) {
        await page.evaluate(() => { window.soundQAWrite = window.rufrenLibrary.write; window.rufrenLibrary.write = function(key, value) { if (key.includes('sound_focus_active')) throw Error('Synthetic reveal draft-write failure'); return window.soundQAWrite(key, value); }; });
        await trial.getByRole('button', { name: 'Show me the word · use help', exact: true }).click();
        assert.equal(await trial.locator('.sound-focus-feedback').count(), 0, 'Model stays hidden after failed support persistence');
        await page.evaluate(() => { window.rufrenLibrary.write = window.soundQAWrite; });
        await trial.getByRole('button', { name: 'Show me the word · use help', exact: true }).click(); await trial.locator('.sound-focus-feedback').waitFor();
        assert(await trial.locator('[data-sound-choice]').first().isDisabled());
        await page.getByRole('button', { name: 'Pause & return to Today', exact: true }).click();
        await page.waitForFunction(() => !!navigator.serviceWorker.controller); audit.offline = true; await context.setOffline(true); await page.reload({ waitUntil: 'networkidle' }); await page.locator('main.main h1').waitFor();
        await navigate(page, 'Today'); await page.locator(`[data-sound-continue="${block}"]`).getByRole('button', { name: 'Continue Sound Focus →', exact: true }).click(); await trial.waitFor();
        assert.equal(await trial.locator('.sound-focus-player').getAttribute('data-asset'), reference.asset_id); await trial.locator('.sound-focus-feedback').waitFor();
        assert.deepEqual(await trial.locator('[data-sound-choice]').evaluateAll(nodes => nodes.map(n => n.dataset.soundChoice)), p.option_member_ids);
        audit.offline = false; await context.setOffline(false);
        // The private connection key is intentionally memory-only and is gone
        // after reload. Re-enter the synthetic key before the later ASR request.
        await navigate(page, 'Settings'); await page.locator('#sync-url').fill(RELAY); await page.locator('#sync-key').fill('synthetic-sound-no-secret'); await navigate(page, 'Practice'); await trial.waitFor();
        check('Failed reveal stays hidden; successful help, frozen order and exact trial survive pause/offline reload');
      }
      const ended = await naturalTrial(page, trial); const started = (await facts(page)).find(e => e.id === data(ended).stage.started_event);
      assert(started); assert.equal(data(started).token, p.token); assert.equal(data(ended).natural_speed, true); assert.equal(data(ended).reference.asset.sha256, reference.asset.sha256);
      const chosen = i === 0 ? members.find(m => m !== p.target_member_id) : p.target_member_id;
      if (i === 0) { const previousStarts = new Set(ofKind(await facts(page), 'sound_reference_played').filter(e => data(e).scope.prepared === packet.id && data(e).stage.kind === 'started').map(e => e.id)); await play(player); await waitFact(page, 'sound_reference_played', e => data(e).scope.prepared === packet.id && data(e).stage.kind === 'started' && !previousStarts.has(e.id)); }
      await trial.locator(`[data-sound-choice="${chosen}"]`).click(); await trial.locator('.sound-focus-feedback').waitFor();
      const completion = (await waitFact(page, 'sound_trial_completed', e => data(e).prepared === packet.id))[0];
      assert.equal(data(completion).response.chosen_member_id, chosen); assert(data(completion).known_playback_ids.includes(ended.id));
      const attempt = (await facts(page)).find(e => e.id === data(completion).attempt); assert(attempt); assert.equal(data(attempt).self_rating, 'not_reported');
      if (i === 0) assert(data(attempt).cues.includes('replay'), 'Repeated starts remain recorded as replay help');
      if (i === 1) assert(data(attempt).cues.includes('original_text') && data(attempt).cues.includes('translation'));
      if (i >= 2) assert.deepEqual(data(attempt).cues, [], 'First required natural playback is task input, not a replay cue');
      audit.observations.push({ trial_id: p.trial_id, option_member_ids: p.option_member_ids, asset_id: reference.asset_id, sha256: reference.asset.sha256, chosen_member_id: chosen, completed_event: completion.id, ended_event: ended.id });
      await trial.getByRole('button', { name: i === 3 ? 'Choose a meaning to say →' : 'Next sound →', exact: true }).click();
      if (i === 0) { const next = page.locator(`[data-trial-token="${data(packets[1]).token}"]`); await next.waitFor(); await page.waitForTimeout(800); assert(await next.locator('[data-sound-choice]').first().isDisabled(), 'Late old playback cannot unlock next token'); }
    }
    check('Four exact choices retain their own natural-end chain, order, answer and support; stale playback cannot unlock next trial');
    await page.locator(`fieldset [data-sound-intent="${members[0]}"]`).click();
    const firstIntent = await page.locator('.sound-focus-speaking').getAttribute('data-sound-intent');
    const abandoned = await record(page, 400); assert.equal(abandoned.metadata.device.sound_focus_scope.intent_id, firstIntent);
    await page.locator(`fieldset [data-sound-intent="${members[1]}"]`).click();
    const intent = await page.locator('.sound-focus-speaking').getAttribute('data-sound-intent'); assert.notEqual(intent, firstIntent);
    assert(await page.getByRole('button', { name: 'Keep this fresh take', exact: true }).isDisabled());
    await page.locator('#audio-import-attempt').setInputFiles({ name: 'prior-intent-copy.wav', mimeType: 'audio/wav', buffer: Buffer.from(abandoned.base64, 'base64') });
    await page.waitForFunction(() => document.getElementById('audio-name-attempt')?.textContent.includes('prior-intent-copy.wav'));
    assert(await page.getByRole('button', { name: 'Keep this fresh take', exact: true }).isDisabled(), 'Import cannot become fresh production');
    const fresh = await record(page, 950); assert.equal(fresh.metadata.device.sound_focus_scope.intent_id, intent); assert.equal(fresh.metadata.device.sound_focus_scope.member_id, members[1]);
    assert.notEqual(fresh.metadata.content_hash, abandoned.metadata.content_hash);
    await page.getByRole('button', { name: 'Keep this fresh take', exact: true }).click(); await page.locator('.sound-focus-speaking[data-sound-kept="true"]').waitFor();
    const spoken = (await waitFact(page, 'sound_trial_completed', e => data(e).response.kind === 'speaking'))[0], preservedSpoken = JSON.stringify(spoken);
    assert.equal(data(spoken).response.intended_member_id, members[1]); assert.equal(data(spoken).response.asset_id, fresh.metadata.asset_id); assert.equal(data(spoken).response.asset.sha256, fresh.metadata.content_hash);
    assert.equal(data(spoken).response.capture_scope.intent_id, intent);
    await screenshot(page, OUTPUT, 'kept-speaking');
    await page.getByRole('button', { name: 'Find the words in my take', exact: true }).click(); await page.locator('.sound-focus-transcript textarea').waitFor(); assert.equal(await page.locator('.sound-focus-transcript textarea').inputValue(), 'Лук.');
    await page.getByRole('button', { name: 'Keep these confirmed words', exact: true }).click(); await page.getByRole('button', { name: 'Words confirmed', exact: true }).waitFor();
    assert.equal(JSON.stringify((await facts(page)).find(e => e.id === spoken.id)), preservedSpoken, 'Wrong ASR/confirmation cannot redefine chosen intention or original');
    assert.equal(ofKind(await facts(page), 'ai_assessment_recorded').length, 0);
    check('Changing meaning discards selection, preserves originals; only fresh scoped microphone take counts; wrong independent ASR leaves intention immutable');
    await page.getByRole('button', { name: 'Finish for now', exact: true }).click(); await page.locator('.sound-focus-history').waitFor();
    const history = page.locator('.sound-focus-history'); assert.equal(await history.locator('[data-sound-completion]').count(), 5);
    for (const observation of audit.observations) { const row = history.locator(`[data-sound-trial="${observation.trial_id}"]`); assert.deepEqual(await row.locator('[data-sound-option-order]').evaluateAll(nodes => nodes.map(n => n.dataset.soundOptionOrder)), observation.option_member_ids); assert.equal(await row.locator('[data-sound-chosen]').getAttribute('data-sound-chosen'), observation.chosen_member_id); }
    const attempts = ofKind(await facts(page), 'attempt_recorded'); assert.equal(attempts.length, 5); assert.equal(attempts.filter(e => data(e).skill === 'listening').length, 4); assert.equal(attempts.filter(e => data(e).skill === 'speaking').length, 1);
    await navigate(page, 'Progress'); await page.locator(`.progress-sound-history [data-sound-block="${block}"]`).getByRole('button', { name: 'See this sound practice →', exact: true }).click(); await history.waitFor();
    await screenshot(page, OUTPUT, 'saved-history'); check('Progress opens exact immutable Sound history with five responses, never a generic wrong-question fallback');
    await openContrast(page); const unfinished = await page.locator('.sound-focus').getAttribute('data-sound-focus-session'); assert.notEqual(unfinished, block); await teach(page);
    const pendingTrial = page.locator('[data-trial-token]'), pendingId = await pendingTrial.getAttribute('data-sound-trial');
    await play(pendingTrial.locator('.sound-focus-player'));
    // Stop based on an actual durable start, not a synthetic media callback.
    const pendingPacket = ofKind(await facts(page), 'sound_trial_prepared').find(e => data(e).trial_id === pendingId);
    await waitFact(page, 'sound_reference_played', e => data(e).scope.prepared === pendingPacket.id && data(e).stage.kind === 'started');
    await pendingTrial.locator('[data-practice-stop]').click();
    assert(await pendingTrial.locator('[data-sound-choice]').first().isDisabled()); await page.getByRole('button', { name: 'Pause & return to Today', exact: true }).click();
    audit.export = await exportFixture(page, OUTPUT, audit.observations); assert.equal(audit.export.completed, 5); assert(audit.export.prepared >= 9);
    check('Second unfinished frozen block and every graph-linked original exported for independent protocol6 HTTP/SQLite/empty-client restore');
    assert.deepEqual(audit.errors, []); assert.deepEqual(audit.forbidden, []); audit.ok = true;
  } catch (error) { audit.ok = false; audit.error = error.stack; audit.body = await page.locator('body').innerText().catch(() => null); audit.events = await facts(page).catch(() => []); audit.assets = (await assetBundle(page).catch(() => [])).map(a => a.metadata); await page.screenshot({ path: path.join(OUTPUT, 'failure.png'), fullPage: true }).catch(() => {}); throw error;
  } finally { await fs.writeFile(path.join(OUTPUT, 'results.json'), JSON.stringify(audit, null, 2)); await context.tracing.stop({ path: path.join(OUTPUT, 'trace.zip') }).catch(() => {}); await browser.close(); }
  console.log(JSON.stringify({ ok: audit.ok, artifact: audit.artifact, checks: audit.checks, export: audit.export }, null, 2));
}
if (require.main === module) main().catch(error => { console.error(error); process.exitCode = 1; });
module.exports = { assetBundle, capturedFixture, routes, exportFixture, ofKind };
