// Actual browser composition test. Every teacher/ASR/speech response is synthetic.
// The WAV is a deterministic tone, not speech or a learner pronunciation sample.
// Run against an explicitly prepared app build; this never contacts a real relay.
const { chromium } = require(process.env.RUFREN_PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs/promises');
const path = require('node:path');
const { navigate } = require('./qa-navigation.cjs');

const APP = process.env.RUFREN_QA_URL || 'http://127.0.0.1:8091';
const RELAY = 'http://127.0.0.1:18991'; // Intercepted; no process listens here.
const OUTPUT = path.resolve(process.env.RUFREN_ORAL_QA_OUTPUT || '.local/teacher-oral-qa');
const LISTEN = 'Ко́шка спи́т.';
const SPEAK = 'Я чита́ю.';
const WRITTEN = 'Я пишу́.';
const sha = bytes => crypto.createHash('sha256').update(bytes).digest('hex');

function wavFixture() {
  const frames = 24000, wav = Buffer.alloc(44 + frames * 2), decoded = Buffer.alloc(frames * 4);
  wav.write('RIFF'); wav.writeUInt32LE(wav.length - 8, 4); wav.write('WAVEfmt ', 8);
  wav.writeUInt32LE(16, 16); wav.writeUInt16LE(1, 20); wav.writeUInt16LE(1, 22);
  wav.writeUInt32LE(24000, 24); wav.writeUInt32LE(48000, 28);
  wav.writeUInt16LE(2, 32); wav.writeUInt16LE(16, 34); wav.write('data', 36);
  wav.writeUInt32LE(frames * 2, 40);
  for (let frame = 0; frame < frames; frame++) {
    const sample = Math.round(5000 * Math.sin(2 * Math.PI * 220 * frame / 24000));
    wav.writeInt16LE(sample, 44 + frame * 2);
    // Hash decoded encoded samples: Math.round can produce -0, PCM16 cannot.
    decoded.writeFloatLE(wav.readInt16LE(44 + frame * 2) / 32768, frame * 4);
  }
  return { wav, frames, hash: sha(wav), pcmHash: sha(decoded) };
}
const audio = wavFixture();

function teacherResponse(request, count) {
  const lesson = request.mode === 'teach_lesson';
  return {
    request,
    provenance: { provider: 'qa_synthetic', model: 'synthetic-oral-fixture',
      review_status: 'ai_generated_teaching', scope: 'Synthetic browser integration fixture; no inference or language assessment.',
      prompt_version: 'qa-oral-v1', response_id: `oral-fixture-${count}` },
    usage: { accounting: 'subscription_quota', input_tokens: 0, output_tokens: 0,
      estimated_micro_usd: 0, reserved_micro_usd: 0 },
    content: lesson ? {
      explanation: 'Synthetic oral practice. Listen for meaning, then save one short spoken response.',
      examples: [{ russian: 'Приве́т!', english: 'Hello!', note: 'Synthetic fixture example.' }],
      practice: [
        { skill: 'listening', prompt: 'Listen, then describe what the animal is doing in English.',
          reference_russian: LISTEN, reference_english: 'The cat is sleeping.',
          expected_response: 'The cat is sleeping.', teaching_note: 'Synthetic listening fixture explanation.' },
        { skill: 'speaking', prompt: 'Say in Russian that you are reading.',
          reference_russian: SPEAK, reference_english: 'I am reading.',
          expected_response: SPEAK, teaching_note: 'Synthetic speaking fixture explanation.' },
        { skill: 'writing', prompt: 'Write in Russian that you are writing.',
          reference_russian: WRITTEN, reference_english: 'I am writing.',
          expected_response: WRITTEN, teaching_note: 'Synthetic written draft fixture.' },
      ], next_step: 'Keep the remaining draft and return when ready.', caveats: [], feedback: null,
    } : {
      explanation: 'Synthetic feedback on the selected words only; this fixture does not judge pronunciation.',
      examples: [], practice: [], next_step: 'Return to the remaining exercises.', caveats: [],
      feedback: { verdict: 'uncertain', suggested_response: null, corrections: [] },
    },
  };
}

function generatedVoice(request, fixture = audio) {
  return { request_id: request.request_id, revision_id: request.revision_id,
    source_text: request.text, wav_base64: fixture.wav.toString('base64'), sha256: fixture.hash,
    sample_rate: 24000, channels: 1, frames: fixture.frames,
    provenance: { kind: 'ai_generated_speech', provider: 'qa_synthetic',
      model: 'deterministic-tone-not-speech', voice: 'Synthetic test tone' } };
}

function recognizedWords(request, fixture = audio, transcript = 'Я читаю.') {
  assert.equal(request.audio_sha256, fixture.hash);
  assert.deepEqual(Buffer.from(request.wav_base64, 'base64'), fixture.wav);
  // Production recognition contract fixes provider/model identifiers. The route,
  // timing and audit receipt below explicitly identify this as fabricated QA data.
  return {
    schema_version: 'rufren-speech-transcript-v1', request_id: request.request_id,
    attempt_id: request.attempt_id, revision_id: request.revision_id,
    asset_id: request.asset_id, audio_sha256: request.audio_sha256,
    original_audio: { sample_rate: 24000, channels: 1, frames: fixture.frames, encoding: 'pcm_s16le', bits: 16 },
    source_pcm_sha256: fixture.pcmHash,
    source_pcm_hash_format: 'interleaved-little-endian-f32-no-header-v1',
    span: { start_frame: 0, end_frame: fixture.frames }, span_coordinate_system: 'original_frames_half_open',
    derivative: { sample_rate: 16000, channels: 1, frames: Math.ceil(fixture.frames * 16000 / 24000), pcm_sha256: '1'.repeat(64),
      pcm_hash_format: 'little-endian-f32-no-header-v1',
      method: 'arithmetic-channel-mean; scipy.signal.resample_poly default Kaiser5',
      scipy_version: 'synthetic-fixture-not-run', amplitude_normalization: false },
    evidence_kind: 'transcript_hypothesis', transcript, language: 'ru',
    language_selection: 'explicit_not_detected', target_conditioned: false,
    word_timestamps: null, word_alignment_status: 'not_run', confidence: null, acoustic_assessment: null,
    provenance: { provider: 'local_qwen3_asr', model: 'mlx-community/Qwen3-ASR-1.7B-bf16',
      revision: 'e1f6c266914abc5a46e8756e02580f834a6cf8a7', runtime: 'mlx-audio==0.6.4',
      mlx_version: 'synthetic-fixture-not-run', manifest_sha256: '1'.repeat(64), worker_sha256: '2'.repeat(64),
      decoding: { temperature: 0, max_tokens: 1024, seed: 42, language: 'Russian', hotwords: [] } },
    timing: { started_at: 'synthetic-fixture', completed_at: 'synthetic-fixture', elapsed_seconds: 0,
      process_peak_rss_bytes: 0, metal_peak_bytes: 0 }, usage: { prompt_tokens: 0, generation_tokens: 0 },
  };
}

const status = { enabled: true, provider: 'qa_synthetic', model: 'Synthetic browser fixture',
  message: 'No inference', data_sent: 'Synthetic fixture only', max_text_bytes: 8192,
  max_context_bytes: 8192, remaining_reservations: 99, remaining_budget_micro_usd: 0,
  accounting: 'subscription_quota' };

async function events(page) {
  return page.evaluate(() => window.rufrenLibrary.entries()
    .filter(([key]) => key.startsWith('rufren.v1.event.')).map(([, value]) => JSON.parse(value)));
}

async function assets(page) {
  return page.evaluate(async () => {
    const db = await new Promise((resolve, reject) => { const r = indexedDB.open('rufren-audio-v1', 1);
      r.onsuccess = () => resolve(r.result); r.onerror = () => reject(r.error); });
    try {
      const records = await new Promise((resolve, reject) => {
        const r = db.transaction('assets').objectStore('assets').getAll();
        r.onsuccess = () => resolve(r.result); r.onerror = () => reject(r.error);
      });
      return await Promise.all(records.map(async ({ blob, ...record }) => ({ ...record,
        actual_sha256: [...new Uint8Array(await crypto.subtle.digest('SHA-256', await blob.arrayBuffer()))]
          .map(n => n.toString(16).padStart(2, '0')).join('') })));
    } finally { db.close(); }
  });
}

async function openLessonTools(page) {
  const summary = page.getByText('Lessons & history', { exact: true });
  if (await summary.count() && !await summary.evaluate(node => node.closest('details')?.open)) await summary.click();
}

async function main() {
  await fs.mkdir(OUTPUT, { recursive: true });
  await fs.writeFile(path.join(OUTPUT, 'deterministic-tone.wav'), audio.wav);
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({ viewport: { width: 1100, height: 900 }, serviceWorkers: 'allow' });
  await context.tracing.start({ screenshots: true, snapshots: true, sources: true });
  const page = await context.newPage();
  const errors = [], forbidden = [], requests = [], shellAssets = [];
  page.on('pageerror', error => errors.push(error.message));
  await context.route('**/*', async route => {
    const req = route.request(), url = new URL(req.url());
    if (url.origin === new URL(APP).origin && !url.pathname.startsWith('/api/')) {
      if (/\.(?:js|wasm)$/.test(url.pathname)) shellAssets.push(req.url());
      return route.continue();
    }
    if (url.origin !== RELAY) { forbidden.push(req.url()); return route.abort('blockedbyclient'); }
    if (req.method() === 'OPTIONS') return route.fulfill({ status: 204,
      headers: { 'Access-Control-Allow-Origin': '*', 'Access-Control-Allow-Headers': '*', 'Access-Control-Allow-Methods': 'GET,POST,OPTIONS' } });
    const request = req.postDataJSON();
    requests.push({ path: url.pathname, method: req.method(), request });
    let body;
    if (url.pathname === '/api/teacher/status' || url.pathname === '/api/tutor/status') body = status;
    else if (url.pathname === '/api/teacher/teach') body = teacherResponse(request, requests.length);
    else if (url.pathname === '/api/speech/generate') body = generatedVoice(request);
    else if (url.pathname === '/api/speech/transcribe') body = recognizedWords(request);
    else { forbidden.push(req.url()); return route.abort('blockedbyclient'); }
    await route.fulfill({ status: 200, contentType: 'application/json',
      headers: { 'Access-Control-Allow-Origin': '*' }, body: JSON.stringify(body) });
  });
  const nav = name => navigate(page, name);
  try {
    await page.goto(APP); await page.locator('main.main h1').waitFor();
    await nav('Settings'); await page.locator('#sync-url').fill(RELAY);
    await page.locator('#sync-key').fill('synthetic-oral-fixture-no-secret');
    await nav('Teacher');
    await page.getByRole('button', { name: 'Teach me this lesson', exact: false }).click();
    await page.locator('.teacher-lesson-flow').waitFor();
    await journey({ page, context, nav, requests });
    assert.deepEqual(forbidden, [], 'No real relay/external service may be contacted');
    assert.deepEqual(errors, []);
    await fs.writeFile(path.join(OUTPUT, 'result.json'), JSON.stringify({ ok: true,
      fixture: 'Entire Teacher/ASR/voice route synthetic; deterministic WAV is a tone, not speech',
      shell_assets: [...new Set(shellAssets)], requests, events: await events(page), assets: await assets(page), errors, forbidden }, null, 2));
    console.log('PASS synthetic Teacher oral journey; exact original, confirmed words and offline recovery');
  } catch (error) {
    await page.screenshot({ path: path.join(OUTPUT, 'failure.png'), fullPage: true }).catch(() => {});
    await fs.writeFile(path.join(OUTPUT, 'failure.txt'), await page.locator('body').innerText()).catch(() => {});
    const overflow = await page.evaluate(() => ({ viewport: innerWidth, document: document.documentElement.scrollWidth,
      elements: [...document.querySelectorAll('body *')].map(e => ({ tag: e.tagName, id: e.id, class: e.className,
        right: e.getBoundingClientRect().right, text: e.textContent.slice(0, 200) }))
        .filter(e => e.right > innerWidth + 1).slice(-30) })).catch(() => null);
    await fs.writeFile(path.join(OUTPUT, 'failure-receipt.json'), JSON.stringify({ fixture: 'Synthetic only; no inference',
      error: String(error), shell_assets: [...new Set(shellAssets)], requests, errors, forbidden, overflow, events: await events(page), assets: await assets(page) }, null, 2)).catch(() => {});
    throw error;
  } finally {
    await context.tracing.stop({ path: path.join(OUTPUT, 'trace.zip') }).catch(() => {});
    await context.close(); await browser.close();
  }
}

async function journey({ page, context, nav, requests }) {
  const oral = skill => page.locator(`.teacher-oral-exercise[data-teacher-skill="${skill}"]`);
  const listening = oral('listening'), speaking = oral('speaking');
  const written = page.locator('.teacher-exercise').filter({ has: page.getByText('Write in Russian that you are writing.', { exact: true }) });
  const returnToLesson = () => page.getByRole('button', { name: 'Return to this lesson & exercise', exact: false }).click();
  // This fixture deliberately omits guided capability: legacy response order is
  // orientation, listening, speaking, writing. Every practice is still one step.
  const jump = async index => {
    await page.locator('.teacher-lesson-flow').getByRole('combobox', { name: 'Go to a step', exact: true }).selectOption(String(index + 1));
    await page.waitForFunction(position => document.querySelector('.teacher-lesson-flow')?.dataset.lessonStep === String(position), index + 1);
  };
  const waitFact = (kind, field, value) => page.waitForFunction(({ kind, field, value }) =>
    window.rufrenLibrary.entries().some(([key, raw]) => key.startsWith('rufren.v1.event.') &&
      (() => { const e = JSON.parse(raw); return e.fact.kind === kind && e.fact.data[field] === value; })()), { kind, field, value });
  const feedbackRequest = () => requests.filter(r => r.path === '/api/teacher/teach' && r.request.mode === 'answer_feedback').at(-1)?.request.input;
  await jump(2);
  await written.locator('textarea').fill('Незаконченный ответ — ёжик.');
  await jump(0); await listening.waitFor();
  assert.equal(await listening.locator('.teacher-oral-model').count(), 0);
  assert.equal((await page.locator('body').innerText()).includes(LISTEN), false, 'Listening reference must be concealed');
  assert.equal((await events(page)).some(e => e.fact.kind === 'stimulus_presented' && e.fact.data.russian === LISTEN), false,
    'Defining a hidden reference is not presentation');
  await listening.locator('textarea').fill('The cat is sleeping.');
  assert.equal(await listening.getByRole('button', { name: 'Discuss what I heard', exact: false }).isDisabled(), true);
  await listening.getByRole('button', { name: 'Generate reference audio', exact: true }).click();
  const listenButton = listening.locator('[data-practice-rate="1"]');
  await page.waitForFunction(() => document.querySelector('.teacher-oral-exercise[data-teacher-skill="listening"] [data-practice-rate="1"]')?.disabled === false);
  assert.equal(await listening.getByRole('button', { name: 'Discuss what I heard', exact: false }).isDisabled(), true,
    'Generated/saved audio alone must not count as played');
  await page.getByRole('button', { name: 'Read the full saved lesson', exact: false }).click();
  await page.getByRole('button', { name: 'Create spoken example', exact: true }).click();
  await page.waitForFunction(() => document.querySelector('.teacher-example-player [data-practice-rate="1"]')?.disabled === false);
  const players = await page.evaluate(() => [...document.querySelectorAll('[data-practice-media]')]
    .map(media => ({ id: media.parentElement.id, revision: media.parentElement.dataset.revision })));
  assert(players.length >= 2);
  assert.equal(new Set(players.map(p => p.id)).size, players.length, 'Every inline player needs its own DOM root');
  await listenButton.click();
  await page.waitForFunction(() => document.querySelector('.teacher-oral-exercise[data-teacher-skill="listening"] audio')?.currentTime > 0.02);
  await waitFact('stimulus_presented', 'russian', LISTEN);
  const played = (await events(page)).find(e => e.fact.kind === 'stimulus_presented' && e.fact.data.russian === LISTEN && e.fact.data.purpose === 'audio');
  assert(played?.fact.data.revision && played.fact.data.session, 'Actual playback records exact revision/session');
  assert.equal(played.fact.data.revision, await listening.locator('[data-revision]').getAttribute('data-revision'));
  assert.equal(await page.locator('.teacher-example-player audio').evaluate(media => media.currentTime), 0,
    'The listening control must play its own inline player');
  assert.equal(await listening.locator('.teacher-oral-model').count(), 0, 'Playing must not reveal the transcript');
  await listening.getByRole('button', { name: 'Show a model & explanation', exact: true }).click();
  await listening.locator('.teacher-oral-model').waitFor();
  assert((await listening.locator('.teacher-oral-model').innerText()).includes(LISTEN));
  await listening.getByRole('button', { name: 'Discuss what I heard', exact: false }).click();
  await page.getByRole('heading', { name: 'Working with your response', exact: true }).waitFor();
  const heard = feedbackRequest();
  assert.equal(heard.skill, 'listening'); assert.equal(heard.response, 'The cat is sleeping.');
  assert(heard.reference.includes(LISTEN));
  assert.deepEqual([...heard.cues].sort(), ['original_text', 'reference_material', 'translation'],
    'One played reference plus explicit model reveal has exact assisted cues');
  const heardEvidence = JSON.parse(heard.reference.split('\nRufren oral evidence v1: ')[1]);
  assert.equal(heardEvidence.reference_plays, 1);
  assert.equal(heardEvidence.played_reference.presentation, played.id);
  assert.equal(heardEvidence.played_reference.sha256, audio.hash);
  assert.equal(heardEvidence.binding.revision_id, played.fact.data.revision);
  const listeningAttempt = (await events(page)).find(e => e.id === heard.attempt_id);
  assert.equal(listeningAttempt.fact.data.self_rating, 'not_reported');
  assert.equal(listeningAttempt.fact.data.revision, played.fact.data.revision);
  await returnToLesson();
  await jump(2);
  assert.equal(await written.locator('textarea').inputValue(), 'Незаконченный ответ — ёжик.');
  await jump(0);
  assert.equal(await listening.locator('textarea').inputValue(), 'The cat is sleeping.');
  await listening.locator('.teacher-oral-model').waitFor();

  await jump(1);
  await speaking.getByRole('button', { name: 'Open recorder for this exercise', exact: true }).click();
  assert.equal(await page.locator('.audio-speaking').count(), 1, 'Only one speaking editor mounts');
  await speaking.locator('#audio-import-attempt').setInputFiles({ name: 'synthetic-tone-not-speech.wav', mimeType: 'audio/wav', buffer: audio.wav });
  const keep = speaking.getByRole('button', { name: 'Keep this spoken response', exact: false });
  await page.waitForFunction(() => [...document.querySelectorAll('.teacher-oral-exercise[data-teacher-skill="speaking"] button')]
    .some(button => button.textContent.includes('Keep this spoken response') && !button.disabled));
  const original = (await assets(page)).find(record => record.role === 'attempt');
  assert(original); assert.equal(original.content_hash, audio.hash); assert.equal(original.actual_sha256, audio.hash);
  const wrongID = await page.evaluate(async id => {
    const db = await new Promise((resolve, reject) => { const r = indexedDB.open('rufren-audio-v1', 1); r.onsuccess = () => resolve(r.result); r.onerror = () => reject(r.error); });
    const badID = `wrong-revision-${crypto.randomUUID()}`;
    try { await new Promise((resolve, reject) => {
      const tx = db.transaction('assets', 'readwrite'), store = tx.objectStore('assets'), get = store.get(id);
      get.onsuccess = () => store.put({ ...get.result, asset_id: badID, revision_id: crypto.randomUUID(), name: 'Synthetic wrong-revision adversarial fixture' });
      tx.oncomplete = resolve; tx.onerror = () => reject(tx.error);
    }); } finally { db.close(); }
    return badID;
  }, original.asset_id);
  const picker = speaking.locator('#audio-library-attempt');
  assert.equal(await picker.locator(`option[value="${wrongID}"]`).count(), 0, 'Wrong-revision takes are not offered');
  // Adversarial stale DOM choice exercises the actual JS guard, beyond filtering.
  await picker.evaluate((select, id) => { select.add(new Option('Synthetic stale option', id)); select.value = id; select.dispatchEvent(new Event('change', { bubbles: true })); }, wrongID);
  await speaking.getByText('This recording does not belong to the current phrase. Choose its exact saved take.', { exact: true }).waitFor();
  assert.equal(await keep.isDisabled(), true, 'Wrong-revision selection clears submit readiness');
  await picker.selectOption(original.asset_id);
  await page.waitForFunction(() => [...document.querySelectorAll('.teacher-oral-exercise[data-teacher-skill="speaking"] button')]
    .some(button => button.textContent.includes('Keep this spoken response') && !button.disabled));
  await keep.click();
  await speaking.locator('.spoken-take').waitFor();
  const saved = await events(page);
  const attempt = saved.find(e => e.fact.kind === 'attempt_recorded' && e.fact.data.skill === 'speaking');
  assert(attempt); assert.equal(attempt.fact.data.revision, original.revision_id);
  assert.equal(attempt.fact.data.self_rating, 'not_reported');
  const attachment = saved.find(e => e.fact.kind === 'recording_attached' && e.fact.data.target.target === 'attempt' && e.fact.data.target.id === attempt.id);
  assert(attachment); assert.equal(attachment.fact.data.asset_id, original.asset_id);
  assert.equal(attachment.fact.data.asset.sha256, audio.hash);
  assert.equal(saved.some(e => e.fact.kind === 'recording_attached' && e.fact.data.asset_id === wrongID), false);
  await speaking.getByRole('button', { name: 'Find the words in this take', exact: true }).click();
  const words = speaking.locator('.transcript-words textarea');
  await words.waitFor(); assert.equal(await words.inputValue(), 'Я читаю.');
  assert.equal(requests.filter(r => r.path === '/api/teacher/teach' && r.request.mode === 'answer_feedback').length, 1,
    'An ASR hypothesis cannot automatically request teacher feedback');
  await words.fill('Я читаю книгу.');
  await speaking.getByRole('button', { name: 'Use these words & discuss them', exact: false }).click();
  await page.getByRole('heading', { name: 'Working with your response', exact: true }).waitFor();
  const spoken = feedbackRequest();
  assert.equal(spoken.skill, 'speaking'); assert.equal(spoken.evidence, 'transcript');
  assert.equal(spoken.response, 'Я читаю книгу.'); assert.equal(spoken.attempt_id, attempt.id);
  assert(spoken.confirmed_transcript_id);
  const after = await events(page);
  const confirmation = after.find(e => e.id === spoken.confirmed_transcript_id);
  assert.equal(confirmation.fact.kind, 'transcript_confirmed'); assert.equal(confirmation.fact.data.operation, 'edit');
  await waitFact('ai_assessment_recorded', 'confirmed_transcript', confirmation.id);
  await returnToLesson();
  await jump(2);
  assert.equal(await written.locator('textarea').inputValue(), 'Незаконченный ответ — ёжик.');
  await jump(1);
  assert.equal(await page.locator('.transcript-words textarea').count(), 1,
    'Current lesson must not duplicate its inline transcript editor in saved history');
  const transcriptID = await speaking.locator('.transcript-words textarea').getAttribute('id');
  await openLessonTools(page);
  await page.getByRole('button', { name: 'Review saved practice', exact: true }).click();
  assert.equal(await page.locator('.teacher-answer').count(), 0, 'Saved practice is an alternate view');
  await page.getByRole('heading', { name: 'Work with your answer', exact: true }).waitFor();
  assert.equal(await page.locator('.transcript-words textarea').count(), 1,
    'Saved practice mounts exactly one editor for the selected transcription');
  assert.equal(await page.locator('.transcript-words textarea').getAttribute('id'), transcriptID);
  assert.equal(await page.locator('.transcript-words textarea').inputValue(), 'Я читаю книгу.');
  await openLessonTools(page);
  await page.getByRole('button', { name: 'Return to current teaching', exact: true }).click();
  assert.equal(await page.locator('.transcript-words textarea').count(), 1);
  await jump(2);
  assert.equal(await written.locator('textarea').inputValue(), 'Незаконченный ответ — ёжик.');
  await jump(1);
  await page.screenshot({ path: path.join(OUTPUT, 'lesson-desktop.png'), fullPage: true });
  await page.setViewportSize({ width: 390, height: 844 });
  assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'Phone layout must fit');
  await page.screenshot({ path: path.join(OUTPUT, 'lesson-phone.png'), fullPage: true });
  await page.evaluate(async () => { await navigator.serviceWorker.ready; });
  await page.waitForFunction(() => navigator.serviceWorker.controller !== null);
  const durableIDs = (await events(page)).map(event => event.id);
  const postsBeforeOffline = requests.filter(request => request.method === 'POST').length;
  await context.setOffline(true);
  await page.reload(); await page.locator('main.main h1').waitFor();
  // Web connection credentials are deliberately memory-only across reloads.
  await nav('Settings'); await page.locator('#sync-url').fill(RELAY);
  await page.locator('#sync-key').fill('synthetic-oral-fixture-no-secret');
  await nav('Teacher');
  await page.locator('.teacher-lesson-flow').waitFor();
  await jump(2);
  assert.equal(await written.locator('textarea').inputValue(), 'Незаконченный ответ — ёжик.');
  await jump(0);
  assert.equal(await listening.locator('textarea').inputValue(), 'The cat is sleeping.');
  await jump(1);
  const recovered = (await assets(page)).find(record => record.asset_id === original.asset_id);
  assert.equal(recovered.actual_sha256, audio.hash); assert.equal(recovered.revision_id, attempt.fact.data.revision);
  const offlineIDs = new Set((await events(page)).map(event => event.id));
  assert(durableIDs.every(id => offlineIDs.has(id)), 'Reload keeps every original event identity');
  assert(offlineIDs.has(attachment.id));
  const review = speaking.locator('[data-asset]').filter({ has: page.locator('[data-practice-rate="1"]') }).first();
  await review.waitFor();
  await page.waitForFunction(id => {
    const root = [...document.querySelectorAll('[data-asset]')].find(e => e.dataset.asset === id);
    return root?.querySelector('[data-practice-rate="1"]')?.disabled === false;
  }, original.asset_id);
  await review.locator('[data-practice-rate="1"]').click();
  await page.waitForFunction(id => [...document.querySelectorAll('[data-asset]')]
    .some(root => root.dataset.asset === id && root.querySelector('audio')?.currentTime > 0.02), original.asset_id);
  assert.equal(requests.filter(request => request.method === 'POST').length, postsBeforeOffline,
    'Offline take recovery and replay must not regenerate or resubmit work');
}

if (require.main === module) main().catch(error => { console.error(error); process.exitCode = 1; });
module.exports = { wavFixture, teacherResponse, generatedVoice, recognizedWords };
