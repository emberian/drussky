// Independent reconstruction of observed timing intent, never a progress estimator.
import assert from 'node:assert/strict';

const sorted = ids => [...ids].sort();
const fact = (events, id, kind) => {
 const event=events.find(event=>event.id===id);
 assert.equal(event?.fact.kind,kind,'Missing exact '+kind+' fact '+id);
 return event;
};

export async function inspectTiming({page,nav,events,expected,records,playVisibleOriginals,playedOriginals}) {
 assert(Array.isArray(expected.starts)&&expected.starts.length,'Actual producer start snapshots required');
 assert(Array.isArray(expected.tasks)&&expected.tasks.length,'Producer task timing observations required');
 const proof=expected.producer_checks;
 assert(proof,'Producer divergent-client and draft-resume observations required');
 const preferences=proof.divergent_cadence_ids.map(id=>fact(events,id,'review_cadence_changed'));
 assert.equal(preferences.length,2);
 assert.equal(new Set(preferences.map(event=>event.device)).size,2,'Preference branches must originate on two clients');
 assert.equal(new Set(preferences.map(event=>event.fact.data.cadence)).size,2);
 assert(!preferences[0].fact.data.parents.includes(preferences[1].id));
 assert(!preferences[1].fact.data.parents.includes(preferences[0].id));
 const timings=proof.divergent_timing_ids.map(id=>fact(events,id,'review_timing_changed'));
 assert.equal(timings.length,2);
 assert.equal(new Set(timings.map(event=>event.device)).size,2,'Task timing branches must originate on two clients');
 assert.deepEqual(timings[0].fact.data.target,timings[1].fact.data.target);
 assert.notDeepEqual(timings[0].fact.data.action,timings[1].fact.data.action);
 for(const [a,b] of [[0,1],[1,0]])assert(!timings[a].fact.data.parents.includes(timings[b].id));
 const resumed=proof.paused_draft_resumed;
 assert(resumed?.session&&typeof resumed.before==='string'&&resumed.before.length);
 assert.equal(resumed.after,resumed.before,'Resuming must retain the actual original unfinished response');
 assert.deepEqual(sorted(expected.starts.map(start=>start.event_id)),sorted(events.filter(event=>event.fact.kind==='review_started').map(event=>event.id)),'Every issued start must survive restore');
 const starts=[];
 for(const observation of expected.starts){
  const event=fact(events,observation.event_id,'review_started');
  assert.deepEqual(event.fact.data,observation.snapshot,'A saved start changed after later sync/support');
  assert(Array.isArray(event.fact.data.evidence_ids));
  if(proof.prior_start_event_ids?.includes(event.id))for(const id of proof.late_arrival_ids||[])assert(!event.fact.data.evidence_ids.includes(id),'Late arrivals were retroactively attributed to this earlier start');
  starts.push({event_id:event.id,...event.fact.data});
 }
 assert(starts.some(start=>start.session===resumed.session),'Original resumed session has no pinned start');
 assert(Array.isArray(proof.late_arrival_ids)&&proof.late_arrival_ids.length);
 assert(Array.isArray(proof.prior_start_event_ids)&&proof.prior_start_event_ids.length);
 for(const id of proof.prior_start_event_ids)assert(expected.starts.some(start=>start.event_id===id),'Earlier pinned start missing from restoration');
 for(const id of proof.late_arrival_ids)assert(events.some(event=>event.id===id),'Late source/support never arrived');
 await nav('Settings').click();
 const cadence=page.locator('.review-cadence');await cadence.waitFor();
 assert.equal(await page.locator('#review-cadence').inputValue(),expected.cadence.selected);
 assert.equal(await cadence.getAttribute('data-review-conflicted'),String(expected.cadence.conflicted));
 assert.equal(await cadence.getAttribute('data-review-pending'),String(expected.cadence.pending));
 assert.deepEqual(sorted(JSON.parse(await cadence.getAttribute('data-review-heads'))),sorted(expected.cadence.heads));
 const tasks=[];
 for(const observation of expected.tasks){
  await nav(observation.navigation||'Today').click();
  if(observation.open_button)await page.getByRole('button',{name:observation.open_button,exact:true}).click();
  if((observation.navigation||'Today')==='Today'){
   const label=observation.target.kind==='teacher'?'Browse your saved Teacher returns':'Browse your saved passage returns';
   const summary=page.getByText(label,{exact:true});
   if(await summary.count()&&!await summary.evaluate(node=>node.parentElement.open))await summary.click();
   const more=page.getByRole('button',{name:'Show all planned returns',exact:true});
   if(await more.count()&&await more.isVisible())await more.click();
  }
  // Selector identifies the exact source/task observed by the producer, not row order.
  assert.equal(typeof observation.selector,'string');
  const control=page.locator(observation.selector);assert.equal(await control.count(),1);
  await control.waitFor({state:'visible'});
  if(!await control.evaluate(node=>node.open))await control.locator(':scope > summary').click();
  assert.deepEqual(JSON.parse(await control.getAttribute('data-review-target')),observation.target);
  for(const key of ['paused','conflicted','pending'])assert.equal(await control.getAttribute('data-review-'+key),String(observation[key]));
  assert.deepEqual(sorted(JSON.parse(await control.getAttribute('data-review-heads'))),sorted(observation.heads));
  const due=await control.getAttribute('data-review-due');
  assert.equal(due,observation.effective_due_at_ms===null?'':String(observation.effective_due_at_ms));
  tasks.push(observation);
  await playVisibleOriginals(page,playedOriginals);
 }
 for(const leaf of expected.audio_dialogue_leaf_ids||[]){
  await nav('Teacher').click();
  await page.getByRole('button',{name:'Conversations',exact:true}).click();
  const library=page.locator('details.dialogue-library');
  if(!await library.evaluate(node=>node.open))await library.locator('summary').first().click();
  await page.locator(`[data-open-dialogue-leaf="${leaf}"]`).click();
  await page.locator(`.teacher-dialogue[data-dialogue-leaf="${leaf}"]`).waitFor();
  await playVisibleOriginals(page,playedOriginals);
 }
 // Reopening timing screens may not grade or rewrite the producer's immutable facts.
 const current=await page.evaluate(()=>window.rufrenLibrary.entries().filter(([key])=>key.startsWith('rufren.v1.event.')).map(([,raw])=>JSON.parse(raw)));
 for(const event of events)assert.deepEqual(current.find(candidate=>candidate.id===event.id),event);
 assert.equal(current.filter(event=>event.fact.kind==='attempt_recorded').length,events.filter(event=>event.fact.kind==='attempt_recorded').length);
 return {cadence:expected.cadence,tasks,starts,producer_checks:proof};
}
