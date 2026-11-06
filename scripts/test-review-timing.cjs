// Actual UI, isolated synthetic learner/clock. Never invokes inference or speech.
const { chromium } = require(process.env.RUFREN_PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const crypto = require('node:crypto');
const { navigate, assertNavigation } = require('./qa-navigation.cjs');
const { CURRENT_PROTOCOL } = require('./qa-current-protocol.cjs');
const APP = process.env.RUFREN_QA_URL || 'http://127.0.0.1:8091';
const OUTPUT = path.resolve(process.env.RUFREN_TIMING_QA_OUTPUT || '.local/review-timing-qa');
const SOURCE = process.env.RUFREN_TIMING_SOURCE || '.local/teacher-revisit-qa/dialogue-v5-regression-01/result.json';
const NOW = 1795176004000, DAY = 86400000;
async function facts(page) { return page.evaluate(() => window.rufrenLibrary.entries().filter(([k]) => k.startsWith('rufren.v1.event.')).map(([,v]) => JSON.parse(v))); }
async function metadata(page, name) { return page.evaluate(name => JSON.parse(window.rufrenLibrary.get(`rufren.v1.meta.${name}`)), name); }
async function seed(page, events) { await page.evaluate(events => { for (const event of events) window.rufrenLibrary.write(`rufren.v1.event.${event.id}`, JSON.stringify(event)); }, events); await page.reload(); await page.locator('main.main h1').waitFor(); }
async function openTeacher(page) {const summary=page.getByText('Browse your saved Teacher returns',{exact:true});if(!await summary.evaluate(el=>el.closest('details').open))await summary.click();const all=page.getByRole('button',{name:'Show all planned returns',exact:true});if(await all.count())await all.click();}
const attempts = events => events.filter(e => e.fact.kind === 'attempt_recorded');
async function main() {
 await fs.mkdir(OUTPUT,{recursive:true});
 const build=await fetch(`${APP}/offline-build.json`).then(r=>r.json());
 if(process.env.RUFREN_QA_BUILD)assert.equal(build.version,process.env.RUFREN_QA_BUILD,'Actual served preview build');
 await fs.writeFile(path.join(OUTPUT,'served-build.json'),JSON.stringify(build,null,2));
 const source = JSON.parse(await fs.readFile(SOURCE,'utf8')), sourceEvents=source.clients[0].events;
 // Derive a separate complete text-task fixture. The archived four-skill source
 // remains byte-for-byte untouched; oral attempts and their evidence are excluded.
 const selectedAttempts=sourceEvents.filter(e=>e.fact.kind==='attempt_recorded'&&['reading','writing'].includes(e.fact.data.skill));
 const attemptIDs=new Set(selectedAttempts.map(e=>e.id)), sessions=new Set(selectedAttempts.map(e=>e.fact.data.session)), revisions=new Set(selectedAttempts.map(e=>e.fact.data.revision));
 const original=sourceEvents.filter(e=>{const d=e.fact.data;switch(e.fact.kind){
  case 'attempt_recorded':return attemptIDs.has(e.id);
  case 'passage_revised':return revisions.has(e.id);
  case 'attempt_context_recorded':return attemptIDs.has(d.attempt);
  case 'ai_assessment_recorded':return attemptIDs.has(d.attempt)&&['reading','writing'].includes(d.skill);
  case 'stimulus_presented':return sessions.has(d.session)&&d.purpose!=='audio';
  case 'teacher_document_recorded':return d.response.request.mode==='teach_lesson'||(d.response.request.mode==='answer_feedback'&&attemptIDs.has(d.response.request.input.attempt_id));
  default:return false;
 }});
 const seedSelection={source:SOURCE,source_events:sourceEvents.length,selected_ids:original.map(e=>e.id),omitted_ids:sourceEvents.filter(e=>!original.includes(e)).map(e=>e.id),scope:'Synthetic text-only reading/writing exact-task graphs; omitted oral attempts, original recordings, ASR, confirmations and related oral feedback. Archived source unchanged.'};
 const serializedSeed=JSON.stringify(original); for(const omitted of seedSelection.omitted_ids)assert(!serializedSeed.includes(omitted),'Selected graph references omitted oral fact '+omitted); for(const forbidden of ['recording_attached','speech_transcribed','transcript_confirmed','asset_id','recording_id','audio_asset'])assert(!serializedSeed.includes('"'+forbidden+'"'),'Text-only fixture retains '+forbidden);
 await fs.writeFile(path.join(OUTPUT,'seed-selection.json'),JSON.stringify(seedSelection,null,2));
 await fs.writeFile(path.join(OUTPUT,'text-seed-library.json'),JSON.stringify({version:CURRENT_PROTOCOL,events:original},null,2));
 if(process.env.RUFREN_TIMING_SEED_ONLY==='1'){console.log(JSON.stringify(seedSelection,null,2));return;}
 const revision=crypto.randomUUID(),genericDevice=crypto.randomUUID();
 original.push({version:CURRENT_PROTOCOL,id:revision,device:genericDevice,recorded_at_ms:NOW-4*DAY,fact:{kind:'passage_revised',data:{passage:{id:crypto.randomUUID(),title:'Synthetic advanced passage return',russian:'Хотя первоначальные доводы казались убедительными, последующие события заставили пересмотреть выводы.',translation:'Although the initial arguments seemed convincing, subsequent events forced a reconsideration.',source:'Synthetic timing QA',coach_reviewed:false},parents:[]}}});
 original.push({version:CURRENT_PROTOCOL,id:crypto.randomUUID(),device:genericDevice,recorded_at_ms:NOW-3*DAY,fact:{kind:'attempt_recorded',data:{session:crypto.randomUUID(),revision,skill:'reading',response:'Synthetic earlier interpretation',cues:[],self_rating:'recalled',policy:'synthetic-timing-seed'}}});
 const browser = await chromium.launch({headless:true}), contexts=[], checks=[], errors=[], forbidden=[], layouts=[];
 const checkpoint = name => {checks.push(name); console.log(`CHECK ${name}`);};
 const create = async (label, events) => {
  const context = await browser.newContext({viewport:{width:1200,height:900}}); contexts.push({label,context});
  await context.addInitScript(now => {Date.now=()=>now;}, NOW); await context.tracing.start({screenshots:true,snapshots:true,sources:true});
  const page=await context.newPage(); page.on('pageerror',e=>errors.push({label,error:e.message}));
  await context.route('**/*',route=> { const url=new URL(route.request().url()); if(url.origin===new URL(APP).origin&&!url.pathname.startsWith('/api/')) return route.continue(); forbidden.push({label,url:url.href}); return route.abort('blockedbyclient'); });
  await page.goto(APP); await page.locator('main.main h1').waitFor(); await seed(page,events); await navigate(page,'Today');
  await openTeacher(page);
  const row=page.locator('.teacher-return-row').first(); await row.waitFor();
  const key={kind:'teacher',document_id:await row.getAttribute('data-review-document'),practice_index:Number(await row.getAttribute('data-review-index'))};
  return {page,context,key,row:page.locator(`.teacher-return-row[data-review-document="${key.document_id}"][data-review-index="${key.practice_index}"]`)};
 };
 let a,b;
 try {
  a=await create('A',original); b=await create('B',original);
  assert.deepEqual(a.key,b.key); const baseline=attempts(await facts(a.page));
  const control=a.row.locator('.review-timing'); await control.locator(':scope > summary').click();
  await a.page.evaluate(()=>{window.timingQAWrite=window.rufrenLibrary.write;window.rufrenLibrary.write=(key,value)=>{if(key.startsWith('rufren.v1.event.')&&JSON.parse(value).fact.kind==='review_timing_changed')throw Error('Synthetic timing write unavailable');return window.timingQAWrite(key,value);};});
  await control.getByRole('button',{name:'Tomorrow',exact:true}).click(); await a.page.getByText('Could not save all events.',{exact:false}).waitFor();
  assert.deepEqual(attempts(await facts(a.page)),baseline); assert.equal((await facts(a.page)).filter(e=>e.fact.kind==='review_timing_changed').length,0);
  await a.page.evaluate(()=>{window.rufrenLibrary.write=window.timingQAWrite;});
  checkpoint('Failed timing save surfaces an error and creates neither a choice nor an Attempt; same control remains retryable');
  const unseen={version:CURRENT_PROTOCOL,id:crypto.randomUUID(),device:crypto.randomUUID(),recorded_at_ms:NOW,fact:{kind:'review_timing_changed',data:{target:a.key,parents:[],action:{kind:'pause'}}}};
  await a.page.evaluate(event=>window.rufrenLibrary.write(`rufren.v1.event.${event.id}`,JSON.stringify(event)),unseen);
  await control.getByRole('button',{name:'Tomorrow',exact:true}).click();
  await a.page.getByText('Another choice for this task arrived.',{exact:false}).waitFor();
  assert.equal((await facts(a.page)).filter(e=>e.fact.kind==='review_timing_changed').length,1);
  checkpoint('A stale rendered expected-head choice fails visibly and preserves the incoming task choice');
  for(const [label,kind,delta] of [['Tomorrow','available_at',DAY],['Later · one day','available_at',2*DAY],['Sooner · now','available_at',0],['Pause this task','pause',null],['Resume returns','resume',null]]) {
   if(!await control.evaluate(el=>el.open))await control.locator(':scope > summary').click();
   const beforeOrder=await a.page.locator('.teacher-return-row').evaluateAll(rows=>rows.map(el=>el.dataset.reviewDocument+':'+el.dataset.reviewIndex));
   await control.getByRole('button',{name:label,exact:true}).click();
   if(process.env.RUFREN_TIMING_STABLE_UI==='1'){assert.deepEqual(await a.page.locator('.teacher-return-row').evaluateAll(rows=>rows.map(el=>el.dataset.reviewDocument+':'+el.dataset.reviewIndex)),beforeOrder);assert(await control.evaluate(el=>el.open),'Timing remains open after choice');const focusedName=label==='Pause this task'?'Resume returns':label==='Resume returns'?'Pause this task':label;assert(await control.getByRole('button',{name:focusedName,exact:true}).evaluate(el=>el===document.activeElement),'Chosen action retains keyboard focus');}
   const events=await facts(a.page), event=events.filter(e=>e.fact.kind==='review_timing_changed').at(-1);
   // UUID ordering is unrelated to append order; select the sole new causal head.
   const timing=events.filter(e=>e.fact.kind==='review_timing_changed'), parents=new Set(timing.flatMap(e=>e.fact.data.parents));
   const head=timing.find(e=>!parents.has(e.id)); assert(head); assert.equal(head.fact.data.action.kind,kind); assert.deepEqual(head.fact.data.target,a.key);
   if(delta!==null) assert.equal(head.fact.data.action.at_ms,NOW+delta);
   assert.deepEqual(attempts(events),baseline,'Timing controls never change saved answers or their raw evidence');
   if(kind==='pause') {if(!await control.evaluate(el=>el.open))await control.locator(':scope > summary').click();assert.equal(await a.row.getAttribute('data-review-due'),''); await control.getByText('Paused',{exact:false}).waitFor();}
  }
  checkpoint('Exact Teacher task sooner/tomorrow/later/pause/resume append timing facts without changing any Attempt');
  // Both devices choose without observing the other. Merge is event union, never clock winner.
  if(!await control.evaluate(el=>el.open))await control.locator(':scope > summary').click();
  await control.getByRole('button',{name:'Sooner · now',exact:true}).click();
  const bc=b.row.locator('.review-timing'); await bc.locator(':scope > summary').click(); await bc.getByRole('button',{name:'Pause this task',exact:true}).click();
  await navigate(a.page,'Settings'); await a.page.locator('#review-cadence').selectOption('more-often');
  await navigate(b.page,'Settings'); await b.page.locator('#review-cadence').selectOption('more-space');
  const branchA=await facts(a.page), branchB=await facts(b.page), merged=[...new Map([...branchA,...branchB].map(e=>[e.id,e])).values()];
  await seed(a.page,merged); await navigate(a.page,'Today'); await openTeacher(a.page);
  const exact=a.page.locator(`.teacher-return-row[data-review-document="${a.key.document_id}"][data-review-index="${a.key.practice_index}"]`);
  const conflict=exact.locator('.review-timing'); await conflict.locator(':scope > summary').click();
  await conflict.getByText('Different timing choices were synced.',{exact:false}).waitFor(); assert.equal(await exact.getAttribute('data-review-due'),'');
  checkpoint('Independent device choices preserve concurrent heads and expose a paused conflict rather than choose by clock');
  await conflict.getByRole('button',{name:'Sooner · now',exact:true}).click();
  await exact.getByRole('button',{name:'Try this again',exact:false}).click();
  const screen=a.page.locator('.teacher-return-screen'); await screen.waitFor();
  assert.equal(await screen.getAttribute('data-active-review-document'),a.key.document_id);
  assert.equal(Number(await screen.getAttribute('data-active-review-index')),a.key.practice_index);
  const draft='Synthetic advanced learner response: preserve this exact unfinished analysis.';
  await screen.locator('textarea').fill(draft); const active=await metadata(a.page,'teacher_revisit_active');
  const session=await screen.getAttribute('data-active-review-session');
  const starts=(await facts(a.page)).filter(e=>e.fact.kind==='review_started'); assert(starts.length>0);
  await a.page.getByRole('button',{name:'Pause & return to Today',exact:true}).click();
  await navigate(a.page,'Settings'); await a.page.locator('#review-cadence').selectOption('more-space');
  assert.deepEqual((await facts(a.page)).filter(e=>e.fact.kind==='review_started'),starts);
  await navigate(a.page,'Today'); await openTeacher(a.page);
  await exact.getByRole('button',{name:'Try this again',exact:false}).click(); await screen.waitFor(); assert.equal(await screen.locator('textarea').inputValue(),draft);
  await a.page.waitForFunction(()=>!!navigator.serviceWorker.controller); await a.context.setOffline(true); await a.page.reload(); await a.page.locator('main.main h1').waitFor();
  await navigate(a.page,'Today'); await openTeacher(a.page); await exact.getByRole('button',{name:'Try this again',exact:false}).click();
  assert.equal(await screen.locator('textarea').inputValue(),draft); assert.deepEqual((await facts(a.page)).filter(e=>e.fact.kind==='review_started'),starts); await a.context.setOffline(false);
  checkpoint('Cadence changes preserve original start and exact unfinished Teacher draft through offline reload');
  const late={version:CURRENT_PROTOCOL,id:crypto.randomUUID(),device:crypto.randomUUID(),recorded_at_ms:NOW-DAY,
   fact:{kind:'passage_revised',data:{passage:{id:crypto.randomUUID(),title:'Synthetic late source arrival',russian:'После долгого разговора решение всё ещё казалось неоднозначным.',translation:'After a long conversation the decision still seemed ambiguous.',source:'Synthetic late-arrival fixture',coach_reviewed:false},parents:[]}}};
  await seed(a.page,[late]);
  await navigate(a.page,'Settings');
  const cadence=await a.page.locator('.review-cadence').evaluate(el=>({selected:document.querySelector('#review-cadence').value,heads:JSON.parse(el.dataset.reviewHeads),conflicted:el.dataset.reviewConflicted==='true',pending:el.dataset.reviewPending==='true'}));
  await navigate(a.page,'Today');
  const selector=`.teacher-return-row[data-review-document="${a.key.document_id}"][data-review-index="${a.key.practice_index}"] .review-timing`;
  const observation=await a.page.locator(selector).evaluate(el=>({target:JSON.parse(el.dataset.reviewTarget),heads:JSON.parse(el.dataset.reviewHeads),paused:el.dataset.reviewPaused==='true',conflicted:el.dataset.reviewConflicted==='true',pending:el.dataset.reviewPending==='true',effective_due_at_ms:el.dataset.reviewDue===''?null:Number(el.dataset.reviewDue)}));
  const expected={as_of_ms:NOW,cadence,tasks:[{navigation:'Today',selector,...observation}],starts:starts.map(e=>({event_id:e.id,snapshot:e.fact.data})),producer_checks:{prior_start_event_ids:starts.map(e=>e.id),divergent_cadence_ids:[branchA,branchB].map(events=>events.find(e=>e.fact.kind==='review_cadence_changed').id),divergent_timing_ids:[branchA,branchB].map(events=>{const timing=events.filter(e=>e.fact.kind==='review_timing_changed'),parents=new Set(timing.flatMap(e=>e.fact.data.parents));return timing.find(e=>!parents.has(e.id)).id;}),paused_draft_resumed:{session,before:draft,after:draft},late_arrival_ids:[late.id]}};
  await fs.writeFile(path.join(OUTPUT,'expected-timing.json'),JSON.stringify(expected,null,2));
  await fs.writeFile(path.join(OUTPUT,'originals.json'),JSON.stringify({records:[]},null,2));
  await fs.mkdir(path.join(OUTPUT,'originals'),{recursive:true});
  for(const [name,width,height] of [['desktop',1200,900],['phone',390,844],['tablet',820,1180]]) {
   await a.page.setViewportSize({width,height}); await assertNavigation(a.page,layouts); await navigate(a.page,'Today'); await openTeacher(a.page); const timing=a.page.locator(selector); if(!await timing.evaluate(el=>el.open))await timing.locator(':scope > summary').click(); await timing.getByRole('button',{name:'Tomorrow',exact:true}).focus(); await a.page.keyboard.press('Tab'); assert(await timing.evaluate(el=>el.contains(document.activeElement)),'Timing controls retain keyboard focus');
   await a.page.screenshot({path:path.join(OUTPUT,`${name}-today.png`),fullPage:true});
   assert(await a.page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
  }
  await a.page.setViewportSize({width:1200,height:900}); await navigate(a.page,'Today');
  await a.page.getByText('Browse your saved passage returns',{exact:true}).click();
  const generic=a.page.locator('.passage-return-row').filter({has:a.page.locator(`.review-timing[data-review-target*="${revision}"]`)});
  assert.equal(await generic.count(),1); const gc=generic.locator('.review-timing'); await gc.locator(':scope > summary').click();
  for(const label of ['Tomorrow','Sooner · now']){const order=await a.page.locator('.passage-return-row .review-timing').evaluateAll(els=>els.map(el=>el.dataset.reviewTarget));await gc.getByRole('button',{name:label,exact:true}).click();if(process.env.RUFREN_TIMING_STABLE_UI==='1'){assert.deepEqual(await a.page.locator('.passage-return-row .review-timing').evaluateAll(els=>els.map(el=>el.dataset.reviewTarget)),order);assert(await gc.evaluate(el=>el.open));assert(await gc.getByRole('button',{name:label,exact:true}).evaluate(el=>el===document.activeElement));}}
  const gheads=JSON.parse(await gc.getAttribute('data-review-heads'));
  await generic.getByRole('button',{name:'Practice now',exact:false}).click(); await a.page.locator('#practice-response').fill('Synthetic close reading: the initial claim is qualified by later evidence.');
  const genericSession=(await metadata(a.page,'session')).id;
  const gstart=(await facts(a.page)).find(e=>e.fact.kind==='review_started'&&e.fact.data.session===genericSession); assert(gstart); assert(gstart.fact.data.evidence_ids.includes(late.id),'Later generic start includes the source already known then'); assert.equal(gstart.fact.data.target.revision,revision); assert.deepEqual(gstart.fact.data.timing_heads,gheads);
  const unseenPause={version:CURRENT_PROTOCOL,id:crypto.randomUUID(),device:crypto.randomUUID(),recorded_at_ms:NOW,fact:{kind:'review_timing_changed',data:{target:{kind:'passage',revision,skill:'reading'},parents:gheads,action:{kind:'pause'}}}};
  await a.page.evaluate(event=>window.rufrenLibrary.write(`rufren.v1.event.${event.id}`,JSON.stringify(event)),unseenPause);
  await a.page.getByRole('button',{name:'Compare my response',exact:false}).click(); await a.page.getByRole('button',{name:'I recalled it',exact:true}).click();
  await navigate(a.page,'Today'); await a.page.getByText('Browse your saved passage returns',{exact:true}).click();
  assert.equal(await gc.getAttribute('data-review-due'),'','Completion may not consume a pause not observed by its start');
  if(!await gc.evaluate(el=>el.open))await gc.locator(':scope > summary').click();
  await gc.getByRole('button',{name:'Resume returns',exact:true}).click();
  assert(Number(await gc.getAttribute('data-review-due'))>NOW,'Genuine completion consumes observed one-shot sooner date');
  checkpoint('Generic exact-source start records observed timing; genuine saved response consumes one-shot availability');
  expected.starts=(await facts(a.page)).filter(e=>e.fact.kind==='review_started').map(e=>({event_id:e.id,snapshot:e.fact.data}));
  const genericObservation=await gc.evaluate(el=>({target:JSON.parse(el.dataset.reviewTarget),heads:JSON.parse(el.dataset.reviewHeads),paused:el.dataset.reviewPaused==='true',conflicted:el.dataset.reviewConflicted==='true',pending:el.dataset.reviewPending==='true',effective_due_at_ms:el.dataset.reviewDue===''?null:Number(el.dataset.reviewDue)}));
  expected.tasks.push({navigation:'Today',selector:`.passage-return-row .review-timing[data-review-target*="${revision}"]`,...genericObservation});
  await fs.writeFile(path.join(OUTPUT,'expected-timing.json'),JSON.stringify(expected,null,2));
  const events=await facts(a.page); assert.equal(attempts(events).length,baseline.length+1); for(const event of baseline)assert.deepEqual(events.find(e=>e.id===event.id),event); assert.deepEqual(errors,[]); assert.deepEqual(forbidden,[]);
  await fs.writeFile(path.join(OUTPUT,'library.json'),JSON.stringify({version:CURRENT_PROTOCOL,events},null,2));
  await fs.writeFile(path.join(OUTPUT,'branches.json'),JSON.stringify({a:branchA,b:branchB},null,2));
  await fs.writeFile(path.join(OUTPUT,'result.json'),JSON.stringify({ok:true,synthetic:true,seedSelection,checks,layouts,errors,forbidden,key:a.key,active,draft,starts:events.filter(e=>e.fact.kind==='review_started'),events},null,2));
  console.log('PASS review timing actual UI');
 } catch(error) { if(a) {await a.page.screenshot({path:path.join(OUTPUT,'failure.png'),fullPage:true}).catch(()=>{}); await fs.writeFile(path.join(OUTPUT,'failure.txt'),await a.page.locator('body').innerText()).catch(()=>{});} await fs.writeFile(path.join(OUTPUT,'failure-receipt.json'),JSON.stringify({error:String(error),checks,errors,forbidden},null,2)); throw error;
 } finally {for(const {label,context} of contexts){await context.tracing.stop({path:path.join(OUTPUT,`trace-${label}.zip`)}).catch(()=>{});await context.close();}await browser.close();}
}
if(require.main===module)main().catch(e=>{console.error(e);process.exitCode=1;});
