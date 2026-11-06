// Focused actual-app visibility checks. All speech history below is fabricated
// metadata (no audio/model/provider calls); the isolated browser has no user data.
const {chromium} = require(process.env.RUFREN_PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const pack = require('../crates/rufren-content/data/beginner-v1.json');
const speechEvents = [
  {
    "version": 3,
    "id": "bdf47052-a116-4084-999a-0c4fcfb85d2d",
    "device": "b4e4c046-bab3-490c-8df8-f7f721cebdb5",
    "recorded_at_ms": 1793951383164,
    "fact": {
      "kind": "passage_revised",
      "data": {
        "passage": {
          "id": "286ce416-b90e-431f-ab76-290df35be8bb",
          "title": "Shell update story — synthetic QA",
          "russian": "Ёжик смо́трит на звёзды.",
          "translation": "The hedgehog looks at the stars.",
          "source": "Synthetic browser update QA; not a personal notebook.",
          "coach_reviewed": false
        },
        "parents": []
      }
    }
  },
  {
    "version": 3,
    "id": "1443b4b6-c303-4721-86b0-307e1d05c92f",
    "device": "b4e4c046-bab3-490c-8df8-f7f721cebdb5",
    "recorded_at_ms": 1793951383302,
    "fact": {
      "kind": "attempt_recorded",
      "data": {
        "session": "baed001c-19cb-4ab1-bdbd-519ff0a80f37",
        "revision": "bdf47052-a116-4084-999a-0c4fcfb85d2d",
        "skill": "speaking",
        "response": "Synthetic QA recording label; no learner speech was recorded",
        "cues": [],
        "self_rating": "not_reported",
        "policy": "shell-update-synthetic-fixture"
      }
    }
  },
  {
    "version": 3,
    "id": "16486ae9-b3de-4f4e-b7ed-df816fd9f65e",
    "device": "b4e4c046-bab3-490c-8df8-f7f721cebdb5",
    "recorded_at_ms": 1793951383302,
    "fact": {
      "kind": "recording_attached",
      "data": {
        "asset_id": "shell-update-fixture-6c128d3a-d13c-4b96-a4ba-a6266bb16f43",
        "asset": {
          "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
          "media_type": "audio/wav",
          "sample_rate_hz": 16000,
          "channels": 1,
          "frames": 16000
        },
        "target": {
          "target": "attempt",
          "id": "1443b4b6-c303-4721-86b0-307e1d05c92f"
        },
        "role": "attempt",
        "note": "Synthetic metadata-only shell update fixture; no original learner audio exists."
      }
    }
  },
  {
    "version": 3,
    "id": "d05de45f-0c9a-4287-b83e-fa41b12e958e",
    "device": "b4e4c046-bab3-490c-8df8-f7f721cebdb5",
    "recorded_at_ms": 1793951383302,
    "fact": {
      "kind": "speech_transcribed",
      "data": {
        "recording": "16486ae9-b3de-4f4e-b7ed-df816fd9f65e",
        "attempt": "1443b4b6-c303-4721-86b0-307e1d05c92f",
        "revision": "bdf47052-a116-4084-999a-0c4fcfb85d2d",
        "asset_id": "shell-update-fixture-6c128d3a-d13c-4b96-a4ba-a6266bb16f43",
        "audio_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "span": {
          "start_frame": 0,
          "end_frame": 16000
        },
        "original_audio": {
          "sample_rate_hz": 16000,
          "channels": 1,
          "frames": 16000,
          "encoding": "ieee_f32le",
          "bits": 32
        },
        "transcript": "Гипотеза: ёжик читает.",
        "language": "ru",
        "provenance": {
          "schema_version": "rufren-speech-transcript-v1",
          "request_id": "f2d7ac4b-aa67-409c-b262-5377f77eb1f6",
          "provider": "qa-shell-update-fixture",
          "model": "synthetic-no-recognizer",
          "model_revision": "fixture-v1",
          "runtime": "fixture-only",
          "manifest_sha256": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
          "worker_sha256": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
        },
        "audit_json": "{\"schema_version\":\"rufren-speech-transcript-v1\",\"request_id\":\"f2d7ac4b-aa67-409c-b262-5377f77eb1f6\",\"attempt_id\":\"1443b4b6-c303-4721-86b0-307e1d05c92f\",\"revision_id\":\"bdf47052-a116-4084-999a-0c4fcfb85d2d\",\"asset_id\":\"shell-update-fixture-6c128d3a-d13c-4b96-a4ba-a6266bb16f43\",\"audio_sha256\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"original_audio\":{\"sample_rate\":16000,\"channels\":1,\"frames\":16000,\"encoding\":\"ieee_f32le\",\"bits\":32},\"source_pcm_sha256\":\"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\",\"source_pcm_hash_format\":\"interleaved-little-endian-f32-no-header-v1\",\"span\":{\"start_frame\":0,\"end_frame\":16000},\"span_coordinate_system\":\"original_frames_half_open\",\"derivative\":{\"sample_rate\":16000,\"channels\":1,\"frames\":16000,\"pcm_sha256\":\"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc\",\"pcm_hash_format\":\"little-endian-f32-no-header-v1\",\"method\":\"synthetic persistence fixture; no DSP executed\",\"scipy_version\":\"fixture\",\"amplitude_normalization\":false},\"evidence_kind\":\"transcript_hypothesis\",\"transcript\":\"Синтетическая запись для проверки обновления: ёжик.\",\"language\":\"ru\",\"language_selection\":\"explicit_not_detected\",\"target_conditioned\":false,\"word_timestamps\":null,\"word_alignment_status\":\"not_run\",\"confidence\":null,\"acoustic_assessment\":null,\"provenance\":{\"provider\":\"qa-shell-update-fixture\",\"model\":\"synthetic-no-recognizer\",\"revision\":\"fixture-v1\",\"runtime\":\"fixture-only\",\"mlx_version\":\"fixture\",\"manifest_sha256\":\"dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd\",\"worker_sha256\":\"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee\",\"decoding\":{}},\"timing\":{},\"usage\":{}}"
      }
    }
  },
  {
    "version": 3,
    "id": "24d95fbb-cd76-49c8-8102-d0f1bff24520",
    "device": "b4e4c046-bab3-490c-8df8-f7f721cebdb5",
    "recorded_at_ms": 1793951383302,
    "fact": {
      "kind": "transcript_confirmed",
      "data": {
        "transcription": "d05de45f-0c9a-4287-b83e-fa41b12e958e",
        "text": "Подтверждённые слова: ёжик пишет.",
        "operation": "edit",
        "supersedes": []
      }
    }
  }
];
// Keep the synthetic audit receipt consistent with its displayed hypothesis.
for (const event of speechEvents) if(event.fact.kind==='speech_transcribed') {
  const data=event.fact.data,audit=JSON.parse(data.audit_json);
  audit.transcript=data.transcript;data.audit_json=JSON.stringify(audit);
}
const tutor = {
  task:'explain',
  provenance:{provider:'qa_synthetic',model:'no-model-called',review_status:'unreviewed_ai_suggestion',scope:'Synthetic DOM check',prompt_version:'fixture-v1',response_id:'coverage-fixture'},
  usage:{accounting:'subscription_quota',input_tokens:0,output_tokens:0,estimated_micro_usd:0,reserved_micro_usd:0},
  content:{explanation:'**Синтетический пример.** Visible tutor prose.',examples:[{russian:'Черепаха читает.',english:'The tortoise reads.',note:'Only a synthetic fixture.'}],next_step:'A rendering check only.',caveats:['**Скрытый нюанс.** Collapsed tutor fixture.']},
};
(async()=>{
  const browser=await chromium.launch({headless:true});
  const context=await browser.newContext({viewport:{width:1100,height:800}});
  const page=await context.newPage(); const errors=[],unexpected=[];
  page.on('pageerror',e=>errors.push(e.message));
  const origin=new URL(process.env.RUFREN_QA_URL || 'http://127.0.0.1:8091').origin;
  await context.route('**/*',route=>{
    const url=new URL(route.request().url());
    if(url.origin===origin && !url.pathname.startsWith('/api/'))return route.continue();
    // The teacher mounts a status check; answer entirely in-process.
    if(url.pathname==='/api/teacher/status')return route.fulfill({status:200,contentType:'application/json',body:JSON.stringify({enabled:false,provider:null,model:null,message:'Synthetic offline check',data_sent:'None',max_text_bytes:8192,max_context_bytes:8192,remaining_reservations:0,remaining_budget_micro_usd:0})});
    unexpected.push(route.request().url());return route.abort();
  });
  const facts=()=>page.evaluate(()=>window.rufrenLibrary.entries().filter(([k])=>k.startsWith('rufren.v1.event.')).map(([,v])=>JSON.parse(v)).filter(e=>e.fact.kind==='stimulus_presented'));
  const seen=async text=>(await facts()).filter(e=>e.fact.data.russian===text);
  const waitSeen=text=>page.waitForFunction(text=>window.rufrenLibrary.entries().filter(([k])=>k.startsWith('rufren.v1.event.')).map(([,v])=>JSON.parse(v)).some(e=>e.fact.kind==='stimulus_presented'&&e.fact.data.russian===text),text);
  const nav=name=>page.getByRole('navigation').getByRole('button',{name:new RegExp(name)}).click();
  const settle=()=>page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
  try {
    await page.goto(origin);await page.getByRole('navigation').waitFor();
    await page.evaluate(({events,tutor})=>{
      for(const e of events)window.rufrenLibrary.write('rufren.v1.event.'+e.id,JSON.stringify(e));
      window.rufrenLibrary.write('rufren.v1.meta.tutor_response',JSON.stringify(tutor));
    },{events:speechEvents,tutor});
    await page.reload();await page.getByRole('navigation').waitFor();
    await nav('Lessons');await page.locator('.lesson-link').first().click();
    const sound=page.locator('.sound-note');await sound.scrollIntoViewIfNeeded();await waitSeen(pack.lessons[0].sound_note);
    const oldId=await sound.getAttribute('id');
    await page.locator('.lesson-link').nth(1).click();
    await sound.filter({hasText:pack.lessons[1].sound_note}).waitFor();
    await sound.scrollIntoViewIfNeeded();await waitSeen(pack.lessons[1].sound_note);
    assert.notEqual(await sound.getAttribute('id'),oldId,'changing lesson must replace observer identity');
    assert.notEqual((await seen(pack.lessons[0].sound_note))[0].id,(await seen(pack.lessons[1].sound_note))[0].id);
    const chunks=page.locator('.phrase-chunks [id^="russian-presentation-"]');
    await chunks.first().waitFor();await chunks.first().scrollIntoViewIfNeeded();await waitSeen(await chunks.first().innerText());
    const alphabet=page.locator('details.alphabet');
    const letterText=pack.script[0].upper+' '+pack.script[0].lower;
    const last=pack.script.at(-1),lastText=last.upper+' '+last.lower;
    assert.equal((await seen(letterText)).length,0,'collapsed alphabet is not exposure');
    await alphabet.locator('summary').click();
    await alphabet.locator('article').first().scrollIntoViewIfNeeded();await waitSeen(letterText);await settle();
    assert.equal((await seen(lastText)).length,0,'offscreen alphabet rows remain withheld');
    await alphabet.locator('article').last().scrollIntoViewIfNeeded();await waitSeen(lastText);
    const lastExample=last.examples[0];await waitSeen(lastExample);
    await nav('Teacher');
    await page.getByRole('button',{name:'Review saved practice',exact:true}).click();
    const hypothesis='Гипотеза: ёжик читает.',confirmed='Подтверждённые слова: ёжик пишет.';
    await page.getByText(hypothesis,{exact:true}).scrollIntoViewIfNeeded();await waitSeen(hypothesis);
    await page.getByText(confirmed,{exact:true}).scrollIntoViewIfNeeded();await waitSeen(confirmed);
    const attempt=speechEvents.find(e=>e.fact.kind==='attempt_recorded');
    for(const text of [hypothesis,confirmed]) {
      const event=(await seen(text))[0];assert.equal(event.fact.data.purpose,'answer');
      assert.equal(event.fact.data.session,attempt.fact.data.session);assert.equal(event.fact.data.revision,attempt.fact.data.revision);
    }
    await page.getByRole('button',{name:'Return to current teaching',exact:true}).click();
    assert.equal((await seen('Синтетический пример. Visible tutor prose.')).length,0,'collapsed tutor is not exposure');
    await page.getByText('Ask a different question',{exact:true}).click();
    await page.locator('.tutor-prose').scrollIntoViewIfNeeded();await waitSeen('Синтетический пример. Visible tutor prose.');
    await page.getByText('Черепаха читает.',{exact:true}).scrollIntoViewIfNeeded();await waitSeen('Черепаха читает.');
    assert.equal((await seen('Скрытый нюанс. Collapsed tutor fixture.')).length,0);
    await page.getByText('Things to check',{exact:true}).click();
    await page.locator('.tutor-caveats').scrollIntoViewIfNeeded();await waitSeen('Скрытый нюанс. Collapsed tutor fixture.');
    assert.deepEqual(errors,[]);assert.deepEqual(unexpected,[]);
    console.log('PASS changed lesson observer identity/facts; chunks; collapsed/offscreen alphabet and examples; ASR/confirmed text exact attribution; collapsed tutor and rich caveat withholding. No model/provider requests.');
  } catch(error) {console.error("Browser diagnostic:",await page.locator("body").innerText(),errors,unexpected);throw error;}
  finally {await context.close();await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
