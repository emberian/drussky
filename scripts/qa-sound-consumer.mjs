// Empty-client Sound history inspection; expected observations come from the
// actual producer UI. Immutable event fields are compared directly, not scored.
import assert from 'node:assert/strict';

export async function inspectSoundHistory({page,nav,events,expected,records,playVisibleOriginals,playedOriginals}) {
 const prepared=events.filter(event=>event.fact.kind==='sound_trial_prepared');
 const completed=events.filter(event=>event.fact.kind==='sound_trial_completed');
 assert(prepared.length&&completed.length,'Sound recovery fixture needs both prepared and completed trials');
 assert(records.length,'Sound recovery requires actual original audio');
 assert(Array.isArray(expected.blocks)&&Array.isArray(expected.trials),'Expected Sound producer observations are missing');
 const blocks=[...new Set(prepared.map(event=>event.fact.data.block_id))].sort();
 assert.deepEqual(expected.blocks.map(block=>block.block_id).sort(),blocks);
 assert.deepEqual(expected.trials.map(trial=>trial.prepared_id).sort(),prepared.map(event=>event.id).sort());
 const restored=[];
 for(const block of blocks){
  await nav('Progress').click();
  const link=page.locator(`.progress-sound-history [data-sound-block="${block}"]`).getByRole('button',{name:'See this sound practice →',exact:true});
  await link.click();
  const history=page.locator(`.sound-focus-history[data-sound-history="${block}"]`);await history.waitFor();
  const packets=prepared.filter(event=>event.fact.data.block_id===block);
  assert.equal(await history.locator('.sound-focus-history-trial').count(),packets.length);
  for(const packet of packets){
   const data=packet.fact.data,observation=expected.trials.find(trial=>trial.prepared_id===packet.id);
   assert.equal(observation.trial_id,data.trial_id);assert.equal(observation.block_id,block);
   assert.equal(observation.position,data.position);assert.equal(observation.mode,data.mode);
   const trial=history.locator(`[data-sound-prepared="${packet.id}"]`);
   assert.equal(await trial.getAttribute('data-sound-trial'),data.trial_id);
   assert.equal(Number(await trial.getAttribute('data-sound-position')),data.position);
   assert.equal(await trial.getAttribute('data-sound-mode'),data.mode);
   if(data.mode==='listening'){
    assert.deepEqual(observation.option_member_ids,data.option_member_ids);
    assert.deepEqual(await trial.locator('[data-sound-option-order]').evaluateAll(nodes=>nodes.map(node=>node.dataset.soundOptionOrder)),data.option_member_ids);
   }
   const responses=completed.filter(event=>event.fact.data.prepared===packet.id);
   assert.equal(observation.completed,responses.length>0);
   assert.equal(await trial.locator('[data-sound-completion]').count(),responses.length);
   const distinctResponses=new Set(responses.map(event=>event.fact.data.response.kind==='listening'?event.fact.data.response.chosen_member_id:event.fact.data.response.asset_id));
   assert.equal(await trial.getAttribute('data-sound-conflict'),String(distinctResponses.size>1));
   const incomplete=responses.length===0||responses.some(event=>!events.some(candidate=>candidate.id===event.fact.data.attempt&&candidate.fact.kind==='attempt_recorded')||!events.some(candidate=>candidate.id===event.fact.data.context&&candidate.fact.kind==='attempt_context_recorded'));
   assert.equal(await trial.getAttribute('data-sound-incomplete'),String(incomplete));
   const member=data.contrast.members.find(member=>member.id===data.target_member_id);
   const reference=data.references.find(reference=>reference.member_id===data.target_member_id);
   assert(member&&reference);assert((await trial.textContent()).includes(member.utterance.text));
   assert.equal(await trial.locator(`.sound-focus-player[data-asset="${reference.asset_id}"]`).getAttribute('data-expected-hash'),reference.asset.sha256);
   for(const response of responses){
    const completion=response.fact.data,row=trial.locator(`[data-sound-completion="${response.id}"]`);
    assert.equal(await row.getAttribute('data-sound-attempt'),completion.attempt);
    if(completion.response.kind==='listening'){
     assert.equal(await row.locator('[data-sound-chosen]').getAttribute('data-sound-chosen'),completion.response.chosen_member_id);
     if(responses.length===1)assert.equal(observation.chosen_member_id,completion.response.chosen_member_id);
    }else{
     const saved=row.locator(`[data-sound-saved-asset="${completion.response.asset_id}"]`);
     assert.equal(await saved.locator('.sound-focus-player').getAttribute('data-expected-hash'),completion.response.asset.sha256);
    }
    const starts=completion.known_playback_ids.filter(id=>events.some(event=>event.id===id&&event.fact.kind==='sound_reference_played'&&event.fact.data.stage.kind==='started')).length;
    assert((await row.textContent()).includes(`Model starts before this response: ${starts}.`));
   }
   restored.push({block_id:block,prepared_id:packet.id,trial_id:data.trial_id,mode:data.mode,position:data.position,completion_ids:responses.map(event=>event.id),incomplete});
  }
  await playVisibleOriginals(page,playedOriginals);
 }
 assert(restored.some(trial=>trial.incomplete),'Unfinished prepared work must survive empty-client reconstruction');
 const sourceAssets=new Set(prepared.flatMap(event=>event.fact.data.references.map(reference=>reference.asset_id)));
 const spokenAssets=new Set(completed.filter(event=>event.fact.data.response.kind==='speaking').map(event=>event.fact.data.response.asset_id));
 assert(spokenAssets.size,'Fixture must contain a genuinely saved spoken original');
 for(const asset of [...sourceAssets,...spokenAssets])assert(playedOriginals.has(asset),'Restored exact model or spoken take was not actually played: '+asset);
 return restored;
}
