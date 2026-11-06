// Real shared-origin storage + Web Locks, isolated from the user's browser data.
async (page)=>{
 const{readFile}=await import('node:fs/promises');const source=await readFile('crates/rufren-app/src/browser_store.js','utf8');
 const context=await page.context().browser().newContext();await context.route('https://rufren-workspace.test/**',route=>route.fulfill({contentType:'text/html',body:'<!doctype html><title>Saved window test</title>'}));
 const initialize=target=>target.evaluate(async source=>new(Object.getPrototypeOf(async function(){}).constructor)(source)(),source);
 const open=async()=>{const target=await context.newPage();await target.goto('https://rufren-workspace.test/');await initialize(target);return target;};
 const flush=target=>target.waitForFunction(()=>!Object.keys(localStorage).some(key=>key.startsWith('rufren.')));
 const id=target=>target.evaluate(()=>sessionStorage.getItem('rufren.workspace.v1'));
 const get=(target,name)=>target.evaluate(name=>{const value=window.rufrenLibrary.get('rufren.v1.meta.'+name);return value===null?null:JSON.parse(value);},name);
 const write=(target,values)=>target.evaluate(values=>{for(const[name,value]of Object.entries(values))window.rufrenLibrary.write('rufren.v1.meta.'+name,JSON.stringify(value));},values);
 const session=(id,draft)=>({id,revision:'22222222-2222-4222-8222-222222222222',skill:'reading',phase:'reflect',draft,cues:[]});
 const dailyA={daily_plan_active:{id:'synthetic-plan-a',items:['reading-a']},daily_plan_settings:{max_items:2,include_new:true}};
 const dailyB={daily_plan_active:{id:'synthetic-plan-b',items:['listening-b','writing-b']},daily_plan_settings:{max_items:4,include_new:false}};
 const dailyD={daily_plan_active:{id:'synthetic-plan-duplicate',items:['speaking-duplicate']},daily_plan_settings:{max_items:1,include_new:true}};
 const checkDaily=async(target,expected)=>{for(const key of ['daily_plan_active','daily_plan_settings'])if(JSON.stringify(await get(target,key))!==JSON.stringify(expected?.[key]??null))throw Error('Daily plan or settings crossed workspace boundaries: '+key);};
 try{
  const migration=await page.context().browser().newContext();
  try{
    await migration.route('https://rufren-workspace.test/**',route=>route.fulfill({contentType:'text/html',body:'<!doctype html><title>Legacy migration</title>'}));
    const [first,second]=await Promise.all([migration.newPage(),migration.newPage()]);await Promise.all([first.goto('https://rufren-workspace.test/'),second.goto('https://rufren-workspace.test/')]);
    await first.evaluate(()=>{localStorage.setItem('rufren.v1.meta.session',JSON.stringify({id:'55555555-5555-4555-8555-555555555555',draft:'legacy answer',phase:'reflect'}));localStorage.setItem('rufren.v1.meta.session_plan',JSON.stringify({session:'55555555-5555-4555-8555-555555555555'}));});
    await Promise.all([initialize(first),initialize(second)]);
    const sessions=await Promise.all([get(first,'session'),get(second,'session')]);if(sessions.filter(Boolean).length!==1||sessions.find(Boolean).draft!=='legacy answer')throw Error('Concurrent legacy migration copied an answered session into two workspaces');
  }finally{await migration.close();}
  const a=await open(),wa=await id(a);const sa=session('11111111-1111-4111-8111-111111111111','answer A');
  await write(a,{session:sa,session_plan:{session:sa.id,policy:'A'},passage_draft_ids:['draft-a'],'passage_draft.draft-a':{editor:{title:'Passage A'}},active_passage_draft:'draft-a',paused_practice_ids:['paused-a'],selected_lesson:'lesson-a',teacher_request:{text:'Question A'},teacher_response:{text:'Explanation A'},teacher_lesson_response:{text:'Lesson A'},teacher_notebook_selection:'document-a','teacher_notebook_cursor:shared':1,teacher_revisit_active:{draft_id:'review-a'},'teacher_revisit:shared:0':{id:'review-a',response:'Return A'},teacher_surface:'dialogues',teacher_dialogue_state:{composer:'Question A'},teacher_dialogue_new_scene:'Scene A','dialogue_audio_draft:shared:input':{take:'Audio A'},'dialogue_transcript_draft:shared':'Draft words A',teacher_context_prepared:{request_id:'Request A',summary:{text:'Relevant work A'}},'teacher_practice:source:0':{response:'Practice A'},'transcript_draft:take':'Words A'});await flush(a);
  await write(a,dailyA);await flush(a);
  const b=await open(),wb=await id(b);if(wa===wb||await get(b,'session')!==null)throw Error('Second live tab inherited an active answered practice');await checkDaily(b,null);
  const sb=session('33333333-3333-4333-8333-333333333333','answer B');await write(b,{session:sb,session_plan:{session:sb.id,policy:'B'},passage_draft_ids:['draft-b'],'passage_draft.draft-b':{editor:{title:'Passage B'}},active_passage_draft:'draft-b',paused_practice_ids:['paused-b'],selected_lesson:'lesson-b',teacher_request:{text:'Question B'},teacher_response:{text:'Explanation B'},teacher_lesson_response:{text:'Lesson B'},teacher_notebook_selection:'document-b','teacher_notebook_cursor:shared':2,teacher_revisit_active:{draft_id:'review-b'},'teacher_revisit:shared:0':{id:'review-b',response:'Return B'},teacher_surface:'lessons',teacher_dialogue_state:{composer:'Question B'},teacher_dialogue_new_scene:'Scene B','dialogue_audio_draft:shared:input':{take:'Audio B'},'dialogue_transcript_draft:shared':'Draft words B',teacher_context_prepared:{request_id:'Request B',summary:{text:'Relevant work B'}},'teacher_practice:source:0':{response:'Practice B'},'transcript_draft:take':'Words B'});await flush(b);
  await write(b,dailyB);await flush(b);await checkDaily(a,dailyA);await checkDaily(b,dailyB);
  if((await get(a,'session')).draft!=='answer A'||(await get(b,'session')).draft!=='answer B')throw Error('Concurrent responses overwrote each other');
  if((await get(a,'passage_draft_ids'))[0]!=='draft-a'||(await get(b,'paused_practice_ids'))[0]!=='paused-b')throw Error('Draft index read/replace crossed workspace boundaries');
  if((await get(a,'teacher_request')).text!=='Question A'||(await get(b,'teacher_response')).text!=='Explanation B')throw Error('Teacher state crossed workspace boundaries');
  if((await get(a,'teacher_lesson_response')).text!=='Lesson A'||(await get(b,'teacher_lesson_response')).text!=='Lesson B')throw Error('Retained teacher lesson crossed workspace boundaries');
  if(await get(a,'teacher_notebook_selection')!=='document-a'||await get(b,'teacher_notebook_selection')!=='document-b')throw Error('Notebook selection crossed workspace boundaries');
  if(await get(a,'teacher_notebook_cursor:shared')!==1||await get(b,'teacher_notebook_cursor:shared')!==2)throw Error('Notebook cursor crossed workspace boundaries');
  if((await get(a,'teacher_revisit_active')).draft_id!=='review-a'||(await get(b,'teacher_revisit:shared:0')).response!=='Return B')throw Error('Scheduled revisit drafts crossed workspace boundaries');
  if(await get(a,'teacher_surface')!=='dialogues'||await get(b,'teacher_dialogue_new_scene')!=='Scene B'||(await get(a,'teacher_dialogue_state')).composer!=='Question A'||(await get(b,'dialogue_audio_draft:shared:input')).take!=='Audio B')throw Error('Dialogue drafts, scene or audio input crossed workspace boundaries');
  if(await get(a,'dialogue_transcript_draft:shared')!=='Draft words A'||await get(b,'dialogue_transcript_draft:shared')!=='Draft words B')throw Error('Dialogue transcript edits crossed workspace boundaries');
  const liveCaches=await b.evaluate(()=>window.rufrenLibrary.teacherCaches());if(liveCaches.length!==2||liveCaches.some(c=>!['Explanation B','Lesson B'].includes(c.response.text)))throw Error('Legacy recovery scanned another actively owned workspace');
  if((await get(a,'teacher_context_prepared')).request_id!=='Request A'||(await get(b,'teacher_context_prepared')).summary.text!=='Relevant work B')throw Error('Prepared teacher context crossed workspace boundaries');
  if((await get(a,'teacher_practice:source:0')).response!=='Practice A'||(await get(b,'teacher_practice:source:0')).response!=='Practice B'||await get(a,'transcript_draft:take')!=='Words A'||await get(b,'transcript_draft:take')!=='Words B')throw Error('Teacher exercise or transcript drafts crossed workspace boundaries');
  const old=await a.evaluate(wa=>window.rufrenLibrary.entries().filter(([key])=>key.startsWith('rufren.v1.workspace.'+wa+'.')),wa);
  await write(a,{session:{...sa,draft:'newer answer A'}});await flush(a);
  await b.evaluate(old=>{const channel=new BroadcastChannel('rufren-library');channel.postMessage(old);channel.close();},old);await a.waitForTimeout(30);
  if((await get(a,'session')).draft!=='newer answer A')throw Error('Delayed cross-tab snapshot rolled back newer durable draft');
  const popupEvent=a.waitForEvent('popup');await a.evaluate(()=>window.open(location.href,'_blank'));const duplicate=await popupEvent;await duplicate.waitForLoadState('domcontentloaded');
  if(await id(duplicate)!==wa)throw Error('Duplicate-tab test did not copy sessionStorage token');await initialize(duplicate);const wd=await id(duplicate);
  if(wd===wa||wd===wb||await get(duplicate,'session')!==null)throw Error('Copied tab token obtained a live writable workspace');await checkDaily(duplicate,null);
  const sd=session('44444444-4444-4444-8444-444444444444','answer duplicate');await write(duplicate,{session:sd,session_plan:{session:sd.id,policy:'D'}});await flush(duplicate);
  await write(duplicate,dailyD);await flush(duplicate);await checkDaily(a,dailyA);await checkDaily(b,dailyB);
  await a.reload();await initialize(a);if(await id(a)!==wa||(await get(a,'session')).draft!=='newer answer A'||(await get(a,'session_plan')).session!==sa.id)throw Error('Reload did not retain exact practice identity, draft and plan');await checkDaily(a,dailyA);
  if((await get(a,'teacher_lesson_response')).text!=='Lesson A')throw Error('Reload lost the retained teacher lesson');
  await b.close();const recovered=await open();if(await id(recovered)!==wb||(await get(recovered,'session')).draft!=='answer B'||(await get(recovered,'active_passage_draft'))!=='draft-b')throw Error('New window did not recover the most recent inactive workspace');
  if((await get(recovered,'teacher_lesson_response')).text!=='Lesson B')throw Error('Inactive-window recovery lost the retained teacher lesson');await checkDaily(recovered,dailyB);
  const list=await recovered.evaluate(()=>window.rufrenLibrary.workspaces());if(!list.find(w=>w.id===wa)?.active||!list.find(w=>w.id===wb)?.current)throw Error('Recovery list did not identify protected live owners');
  const refused=await recovered.evaluate(async wa=>{try{await window.rufrenLibrary.switchWorkspace(wa);return false;}catch{return true;}},wa);if(!refused||await id(recovered)!==wb)throw Error('Recovery stole another live window');await checkDaily(recovered,dailyB);await checkDaily(a,dailyA);
  await recovered.close();await duplicate.evaluate(wb=>window.rufrenLibrary.switchWorkspace(wb),wb);await duplicate.reload();await initialize(duplicate);
  if(await id(duplicate)!==wb||(await get(duplicate,'session')).draft!=='answer B')throw Error('Explicit workspace recovery did not restore all local state');await checkDaily(duplicate,dailyB);
  await duplicate.evaluate(wd=>window.rufrenLibrary.switchWorkspace(wd),wd);await duplicate.reload();await initialize(duplicate);
  if((await get(duplicate,'session')).id!==sd.id||(await get(duplicate,'session_plan')).policy!=='D')throw Error('Switching windows lost the previous active response');await checkDaily(duplicate,dailyD);
  await flush(duplicate);await a.close();await duplicate.close();const reopened=await open();
  if(await id(reopened)!==wd||(await get(reopened,'session')).draft!=='answer duplicate')throw Error('All-tabs-closed recovery did not retain the most recent saved window');await checkDaily(reopened,dailyD);
  const recoveryCaches=await reopened.evaluate(()=>window.rufrenLibrary.teacherCaches());if(!['Explanation A','Lesson A','Explanation B','Lesson B'].every(text=>recoveryCaches.some(c=>c.response.text===text)))throw Error('Legacy recovery missed inactive saved-window teacher caches');
  if(!recoveryCaches.some(c=>c.response.text==='Lesson A'&&c.prepared_context?.request_id==='Request A'))throw Error('Legacy recovery mixed prepared context between windows');
  const malformed=await reopened.evaluate(wa=>{const key='rufren.v1.workspace.'+wa+'.meta.teacher_response';window.rufrenLibrary.write(key,'[broken');const caches=window.rufrenLibrary.teacherCaches();return{caches,raw:window.rufrenLibrary.get(key)};},wa);if(malformed.raw!=='[broken'||!malformed.caches.some(c=>c.response===null)||!malformed.caches.some(c=>c.response?.text==='Lesson B'))throw Error('One malformed legacy cache blocked valid recovery or lost original bytes');
  return{passed:['independent tab practice IDs and responses','isolated daily plan and settings across live tabs, reload, inactive recovery and switching','isolated passage/paused indexes','isolated retained teacher lessons across reload and recovery','isolated teacher exercise and transcript drafts','isolated prepared teacher context','legacy response recovery from current and inactive windows only','isolated notebook selection and cursors','isolated scheduled revisit sessions and drafts','isolated dialogue state, scene and audio input drafts','malformed legacy cache isolated without raw-data loss','stale broadcast rejection','duplicate-tab ownership claim','reload preserves identity and plan','automatic inactive-window recovery','live owner cannot be stolen','explicit whole-window switching','all-tabs-closed recovery','serialized legacy migration without copied sessions']};
 }finally{await context.close();}
}
