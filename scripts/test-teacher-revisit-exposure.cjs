// Actual-app support attribution checks; all service responses and audio are synthetic.
const {chromium}=require(process.env.RUFREN_PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');
const {TASKS,SEED_TIME,REVIEW_TIME,installClock,facts,fixtureRoutes,syntheticReplica,openLessonTools}=require('./test-teacher-revisit.cjs');
(async()=>{
 const browser=await chromium.launch({headless:true});
 const context=await browser.newContext({viewport:{width:1200,height:900}});
 const audit={requests:[],forbidden:[],verdict:{E:'correct'}},errors=[];
 await installClock(context,SEED_TIME);await fixtureRoutes(syntheticReplica(),audit)(context,'E');
 const page=await context.newPage();page.on('pageerror',e=>errors.push(e.message));
 const nav=name=>page.getByRole('navigation',{name:'Main navigation'}).getByRole('button',{name:new RegExp(name)}).click();
 const draft=()=>page.evaluate(()=>{const active=JSON.parse(window.rufrenLibrary.get('rufren.v1.meta.teacher_revisit_active'));return active?JSON.parse(window.rufrenLibrary.get(`rufren.v1.meta.teacher_revisit:${active.document_id}:${active.practice_index}`)):null;});
 const readRow=index=>page.locator(`.teacher-return-row[data-review-index="${index}"]`);
 const start=async index=>{await nav('Today');const all=page.getByRole('button',{name:'Show all planned returns',exact:true});if(await all.count())await all.click();await readRow(index).getByRole('button').click();await page.locator(`.teacher-return-screen[data-active-review-index="${index}"]`).waitFor();};
 const persistedCue=async cue=>{assert((await draft()).review.cues.includes(cue));};
 try {
  await page.goto(process.env.RUFREN_QA_URL||'http://127.0.0.1:8091');await page.getByRole('navigation').waitFor();
  await nav('Settings');await page.locator('#sync-url').fill('http://127.0.0.1:18993');await page.locator('#sync-key').fill('synthetic-exposure-not-secret');
  await nav('Teacher');await page.getByText('Ready · Synthetic exact-task fixture',{exact:true}).waitFor();
  await openLessonTools(page);await page.getByRole('button',{name:'Teach me this lesson',exact:false}).click();
  await page.locator('.teacher-lesson-flow[data-lesson-step="0"]').waitFor();
  for(const index of [0,1,2]) {
   await page.getByLabel('Go to a step').selectOption(String(index));
   const exercise=page.locator('.teacher-lesson-current .teacher-exercise');await exercise.locator('textarea').fill(index===0?'The first speaker.':index===1?TASKS[1].expected_response:'Two.');
   if(index===2) {await exercise.getByRole('button',{name:'Generate reference audio',exact:true}).click();await exercise.locator('[data-practice-rate="1"]').click();await page.waitForFunction(()=>document.querySelector('.teacher-oral-exercise audio')?.currentTime>0.02);}
   await exercise.getByRole('button',{name:index===2?'Discuss what I heard':'Discuss my answer',exact:false}).click();
   const back=page.getByRole('button',{name:'Return to this lesson & exercise',exact:false});await back.waitFor();await back.click();await page.locator('.teacher-lesson-flow').waitFor();
  }
  await page.evaluate(time=>window.qaRevisitClock(time),REVIEW_TIME);
  await start(0);let initial=await draft();assert.deepEqual(initial.review.cues,[]);
  await page.getByText(TASKS[0].reference_russian,{exact:true}).scrollIntoViewIfNeeded();
  await page.waitForFunction(({id,text})=>window.rufrenLibrary.entries().some(([k,v])=>{if(!k.startsWith('rufren.v1.event.'))return false;const e=JSON.parse(v);return e.fact.kind==='stimulus_presented'&&e.fact.data.session===id&&e.fact.data.russian===text&&e.fact.data.purpose==='task_input';}),{id:initial.id,text:TASKS[0].reference_russian});
  assert.deepEqual((await draft()).review.cues,[],'required reading input is not help');
  // A simulated storage fault must prevent the model from entering the DOM.
  await page.evaluate(()=>{window.qaOriginalWrite=window.rufrenLibrary.write;window.rufrenLibrary.write=(key,value)=>{if(key.includes('meta.teacher_revisit:'))throw Error('Synthetic support-write rejection');return window.qaOriginalWrite(key,value);};});
  const model=page.getByRole('button',{name:'Show an example and explanation',exact:true});await model.click();
  assert.equal(await page.locator('.teacher-revisit-model').count(),0,'failed support persistence must keep model hidden');
  assert.deepEqual((await draft()).review.cues,[]);
  await page.evaluate(()=>window.rufrenLibrary.write=window.qaOriginalWrite);await model.click();await page.locator('.teacher-revisit-model').waitFor();await persistedCue('reference_material');
  await page.locator('.teacher-return-screen textarea').fill('A saved local review draft');
  await page.getByRole('button',{name:'Pause & return to Today',exact:true}).click();
  await page.reload();await page.getByRole('button',{name:'Continue this return',exact:false}).click();
  assert.equal((await draft()).id,initial.id);assert.equal(await page.locator('.teacher-return-screen textarea').inputValue(),'A saved local review draft');
  await nav('Library');await persistedCue('reference_material');await nav('Practice');
  await page.locator('.teacher-return-screen textarea').fill('Edited after opening Library');await persistedCue('reference_material');
  // A late visible-exposure guard can update durable support while the editor
  // still has an earlier signal snapshot. Exercise the actual edit merge path.
  await page.evaluate(()=>{const a=JSON.parse(window.rufrenLibrary.get('rufren.v1.meta.teacher_revisit_active')),key=`rufren.v1.meta.teacher_revisit:${a.document_id}:${a.practice_index}`,d=JSON.parse(window.rufrenLibrary.get(key));d.review.cues.push('coach');window.rufrenLibrary.write(key,JSON.stringify(d));});
  await page.locator('.teacher-return-screen textarea').fill('Edited after late support');await persistedCue('coach');
  await page.evaluate(time=>window.qaRevisitClock(time),REVIEW_TIME+500);
  await start(2);const listening=page.locator('.teacher-return-screen .teacher-oral-exercise');
  assert.deepEqual((await draft()).review.cues,[]);assert.equal(await listening.locator('.teacher-oral-model').count(),0);
  await listening.locator('[data-practice-rate="1"]').click();
  await page.waitForFunction(()=>{const a=JSON.parse(window.rufrenLibrary.get('rufren.v1.meta.teacher_revisit_active'));return JSON.parse(window.rufrenLibrary.get(`rufren.v1.meta.teacher_revisit:${a.document_id}:${a.practice_index}`))?.audio?.reference_plays===1;});
  const heard=await draft();assert.deepEqual(heard.review.cues,[]);assert.deepEqual(heard.audio.cues,[],'first required listening input is not help');
  const ownInput=(await facts(page)).filter(e=>e.fact.kind==='stimulus_presented'&&e.fact.data.session===heard.id&&e.fact.data.russian===TASKS[2].reference_russian);
  assert.deepEqual(ownInput.map(e=>e.fact.data.purpose),['task_input'],'first listening playback must be required input, not Audio replay');
  assert.equal(await listening.locator('.teacher-oral-model').count(),0);
  // Browser adapter fault: playback appends an immutable Audio fact before
  // updating the workspace draft. Reject only that later metadata save.
  await page.evaluate(()=>{window.qaOriginalWrite=window.rufrenLibrary.write;window.rufrenLibrary.write=(key,value)=>{if(key.includes('meta.teacher_revisit:')&&JSON.parse(value)?.audio?.reference_plays>=2)throw Error('Synthetic replay draft-save rejection');return window.qaOriginalWrite(key,value);};});
  await page.waitForFunction(()=>document.querySelector('.teacher-return-screen audio')?.ended===true);
  await listening.locator('[data-practice-rate="1"]').click();
  await page.waitForFunction(id=>window.rufrenLibrary.entries().some(([key,value])=>{if(!key.startsWith('rufren.v1.event.'))return false;const e=JSON.parse(value);return e.fact.kind==='stimulus_presented'&&e.fact.data.session===id&&e.fact.data.purpose==='audio';}),heard.id);
  assert.equal((await draft()).audio.reference_plays,1,'failed save leaves original local count');
  await page.evaluate(()=>window.rufrenLibrary.write=window.qaOriginalWrite);
  await page.reload();await nav('Practice');await listening.waitFor();
  assert.equal((await draft()).id,heard.id);
  await listening.locator('textarea').fill('Two.');
  await page.getByText('Your own impression (optional)',{exact:true}).click();await page.getByLabel('How did that response feel?').selectOption('recalled');
  await listening.getByRole('button',{name:'Discuss what I heard',exact:false}).click();
  await page.getByRole('button',{name:'Finish & return to Today',exact:true}).waitFor();
  const replayed=(await facts(page)).find(e=>e.fact.kind==='attempt_recorded'&&e.fact.data.session===heard.id);
  assert.deepEqual(replayed?.fact.data.cues,['replay'],'surviving own Audio recovers Replay only, not invented ReferenceMaterial');
  await page.getByRole('button',{name:'Finish & return to Today',exact:true}).click();
  assert.equal(Number(await readRow(2).getAttribute('data-review-due')),REVIEW_TIME+500+86400000,'replay-supported recalled/correct response cannot advance interval');
  await start(2);assert.deepEqual((await draft()).review.cues,[]);
  // This review is still cue-free, so this transition proves attribution adds
  // support rather than merely preserving a previously revealed model's cue.
  await nav('Library');await persistedCue('reference_material');await nav('Practice');
  await page.locator('.teacher-return-screen textarea').fill('Two after opening a reference page.');await persistedCue('reference_material');
  // Actual model restudy BEFORE a new writing review must qualify the delay,
  // without inventing an assistance cue inside that subsequent review.
  await nav('Teacher');await page.locator('.teacher-lesson-flow').waitFor();
  await page.getByLabel('Go to a step').selectOption('1');
  await page.locator('.teacher-lesson-current').getByText('Show an example and explanation',{exact:true}).click();
  await page.locator('.teacher-lesson-current').getByText(TASKS[1].expected_response,{exact:true}).scrollIntoViewIfNeeded();
  await page.waitForFunction(text=>window.rufrenLibrary.entries().some(([key,value])=>{if(!key.startsWith('rufren.v1.event.'))return false;const e=JSON.parse(value);return e.fact.kind==='stimulus_presented'&&e.fact.data.russian===text&&e.fact.data.purpose==='answer';}),TASKS[1].expected_response);
  await page.evaluate(time=>window.qaRevisitClock(time),REVIEW_TIME+1000);
  await start(1);const writing=await draft();assert.deepEqual(writing.review.cues,[]);
  await page.locator('.teacher-return-screen textarea').fill(TASKS[1].expected_response);
  await page.getByText('Your own impression (optional)',{exact:true}).click();await page.getByLabel('How did that response feel?').selectOption('recalled');
  await page.getByRole('button',{name:'Save & discuss this return',exact:false}).click();
  await page.getByRole('button',{name:'Finish & return to Today',exact:true}).waitFor();
  const savedWriting=(await facts(page)).find(e=>e.fact.kind==='attempt_recorded'&&e.fact.data.session===writing.id);
  assert(savedWriting);assert.deepEqual(savedWriting.fact.data.cues,[],'prestart restudy is not a made-up in-task assistance cue');
  await page.getByRole('button',{name:'Finish & return to Today',exact:true}).click();
  const due=Number(await readRow(1).getAttribute('data-review-due'));
  assert.equal(due,REVIEW_TIME+1000+86400000,'recent actual target restudy must prevent the nominal delayed success from growing the interval');
  assert.deepEqual(errors,[]);assert.deepEqual(audit.forbidden,[]);
  console.log('PASS required reading/first listening are not help; model withheld on support-write failure; durable reveal/navigation cues survive edits; paused exact review draft resumes; prestart actual restudy stays cue-free but prevents interval growth; surviving own Audio after browser draft-save failure recovers Replay on reload/submit and prevents growth. Synthetic services only.');
 }catch(error){console.error(await page.locator('body').innerText(),errors,audit.forbidden);throw error;}
 finally{await context.close();await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
