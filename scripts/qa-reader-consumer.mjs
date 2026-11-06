// Actual empty-client rendering/navigation over an independently restored graph.
// Expected fields are frozen producer UI observations, not recreated local drafts.
import assert from 'node:assert/strict';

const metadata=(page,key)=>page.evaluate(key=>{const raw=window.rufrenLibrary.get('rufren.v1.meta.'+key);return raw===null?null:JSON.parse(raw);},key);

export async function inspectReader({page,nav,events,expected,records,playedOriginals}) {
 const documents=events.filter(event=>event.fact.kind==='reader_document_recorded');
 assert(documents.length&&Array.isArray(expected.documents)&&expected.documents.length);
 assert.deepEqual(documents.map(event=>event.fact.data.document_id).sort(),expected.documents.map(document=>document.document_id).sort());
 assert(Array.isArray(expected.local_only)&&expected.local_only.length,'Producer local-state observations required');
 for(const item of expected.local_only){
  assert.equal(item.consumer_before_open,null);
  assert.equal(await metadata(page,item.key),null,'Producer Reader cursor/help/draft leaked into empty client: '+item.key);
 }
 const openDocument=async id=>{
  await nav('Reader').click();
  const choose=page.getByRole('button',{name:/^Choose (?:another )?text$/});
  if(!await page.locator('.reader-shelf').isVisible()&&await choose.count())await choose.click();
  await page.locator(`.reader-book[data-reader-source="${id}"]`).getByRole('button').click();
  await page.locator(`.reader-document[data-reader-document="${id}"]`).waitFor();
  const options=page.locator('details.reader-options');if(await options.count()&&!await options.evaluate(node=>node.open))await options.locator(':scope > summary').click();
 };
 const restored=[];
 for(const observation of expected.documents){
  const doc=documents.find(event=>event.fact.data.document_id===observation.document_id)?.fact.data;
  assert(doc);const definition=doc.definition;
  assert.equal(definition.title,observation.title);assert.equal(definition.original,observation.original_ru);
  assert.equal(definition.original_sha256,observation.original_sha256);assert.deepEqual(definition.source,observation.source);
  assert.deepEqual(definition.chunks.map(chunk=>({chunk_id:chunk.id,text:chunk.text})),observation.chunks.map(chunk=>({chunk_id:chunk.chunk_id,text:chunk.text})));
  await openDocument(doc.document_id);
  const root=page.locator(`.reader-document[data-reader-document="${doc.document_id}"]`);
  assert.equal(await root.locator('.reader-continuous-original').textContent(),definition.original);
  assert.equal(await page.locator('#reader-part-select').inputValue(),definition.chunks[0].id);
  assert.equal(await root.getByRole('button',{name:'Continuous text',exact:true}).getAttribute('aria-pressed'),'true');
  assert.equal(await root.locator('.reader-stress-choice input').isChecked(),false);
  const local=await metadata(page,'reader_state:'+doc.document_id);
  assert.equal(local.chunk_id,definition.chunks[0].id);assert.equal(local.mode,'continuous');assert.equal(local.show_stress,false);
  assert.deepEqual(local.supports,{});assert.deepEqual(local.own_words,{});
  const about=root.getByRole('button',{name:'About this text & its support',exact:true});await about.click();
  const details=root.locator('.reader-source-details');assert(await details.isVisible());
  assert((await details.textContent()).includes(definition.source.title));
  assert((await details.textContent()).includes(definition.source.rights));
  for(const chunk of definition.chunks){
   assert.equal(await root.locator(`[data-reader-text-chunk="${chunk.id}"]`).textContent(),chunk.text);
   await page.locator('#reader-part-select').selectOption(chunk.id);
   await root.locator(`.reader-chunk[data-reader-chunk="${chunk.id}"]`).waitFor();
   assert.equal((await metadata(page,'reader_state:'+doc.document_id)).chunk_id,chunk.id);
  }
  const supported=definition.chunks.find(chunk=>chunk.translation||chunk.stress);
  if(supported){
   await page.locator('#reader-part-select').selectOption(supported.id);
   const part=root.locator(`.reader-chunk[data-reader-chunk="${supported.id}"]`);
   if(supported.translation){await part.getByRole('button',{name:'Meaning',exact:true}).click();assert(await part.locator('.reader-meaning').isVisible());}
   if(supported.stress){await root.locator('.reader-stress-choice input').check();assert((await part.locator('.reader-stress-support').textContent()).includes(supported.stress));await root.locator('.reader-stress-choice input').uncheck();}
   assert.equal(await root.locator('.reader-continuous-original').textContent(),definition.original,'Support changed the original');
  }
  restored.push({document_id:doc.document_id,source_kind:definition.source.kind,original_bytes:Buffer.byteLength(definition.original),original_sha256:definition.original_sha256,chunks:definition.chunks.length,empty_client_default_chunk:definition.chunks[0].id});
 }
 const excerpts=[];
 for(const observation of expected.excerpts||[]){
  const event=events.find(event=>event.id===observation.source_fact_id);assert.equal(event?.fact.kind,'reader_excerpt_linked');
  const link=event.fact.data;assert.equal(link.purpose,'kept_phrase');assert.equal(link.document_id,observation.document_id);assert.equal(link.chunk_id,observation.chunk_id);assert.equal(link.text,observation.original_ru);assert.equal(link.revision,observation.revision_id);
  const passage=events.find(event=>event.id===link.revision);assert.equal(passage?.fact.data.passage.russian,link.text);
  const definition=documents.find(event=>event.fact.data.document_id===link.document_id).fact.data.definition;
  assert.equal(Buffer.from(definition.original).subarray(link.span.start,link.span.end).toString(),link.text);
  await nav('Library').click();await page.locator('.passage-card').filter({hasText:link.text}).first().click();
  assert.equal(await page.locator('#passage-russian').inputValue(),link.text);
  await page.getByRole('button',{name:'View original source →',exact:true}).click();
  await page.locator(`.reader-document[data-reader-document="${link.document_id}"]`).waitFor();
  assert.equal(await page.locator('#reader-part-select').inputValue(),link.chunk_id);
  excerpts.push({source_fact_id:event.id,document_id:link.document_id,chunk_id:link.chunk_id,revision_id:link.revision});
 }
 const paths=[];
 for(const observation of expected.dialogues||[]){
  // Deliberately leave a different current part before opening the saved reply.
  await openDocument(observation.document_id);
  const definition=documents.find(event=>event.fact.data.document_id===observation.document_id).fact.data.definition;
  const wrong=definition.chunks.find(chunk=>chunk.id!==observation.chunk_id);assert(wrong);
  await page.locator('#reader-part-select').selectOption(wrong.id);
  await nav('Teacher').click();await page.getByRole('button',{name:'Conversations',exact:true}).click();
  const library=page.locator('details.dialogue-library');if(!await library.evaluate(node=>node.open))await library.locator('summary').first().click();
  await page.locator(`[data-open-dialogue-leaf="${observation.leaf_id}"]`).click();
  const thread=page.locator(`.teacher-dialogue[data-dialogue-leaf="${observation.leaf_id}"]`);await thread.waitFor();
  assert.equal(await thread.getAttribute('data-dialogue-thread'),observation.thread_id);
  assert.deepEqual(await thread.locator('.dialogue-message[data-message-id]').evaluateAll(nodes=>nodes.map(node=>node.dataset.messageId)),observation.message_ids);
  const request=events.find(event=>event.fact.kind==='teacher_dialogue_user_recorded'&&event.fact.data.request_id===observation.thread_id)?.fact.data;
  assert(request);assert.equal(request.text,observation.sent_text);assert.equal(request.anchor.document_id,observation.document_id);assert.equal(request.anchor.focus.kind,'reader_chunk');assert.equal(request.anchor.focus.chunk_id,observation.chunk_id);assert(request.anchor.snapshot.includes(observation.original_ru));
  await thread.getByRole('button',{name:'Return to the passage',exact:true}).click();
  const root=page.locator(`.reader-document[data-reader-document="${observation.document_id}"]`);await root.waitFor();
  assert.equal(await page.locator('#reader-part-select').inputValue(),observation.chunk_id,'Saved dialogue returned only to the document or latest cursor');
  assert.equal(await root.locator(`[data-reader-text-chunk="${observation.chunk_id}"]`).textContent(),observation.original_ru);
  paths.push({thread_id:observation.thread_id,leaf_id:observation.leaf_id,document_id:observation.document_id,chunk_id:observation.chunk_id,message_ids:observation.message_ids});
 }
 const audio=[];
 for(const observation of expected.audio||[]){
  const record=records.find(record=>record.asset_id===observation.asset_id);assert(record);
  assert.equal(record.content_hash,observation.sha256);assert.equal(record.revision_id,observation.revision_id);
  assert.equal(record.device.provenance.source_text,observation.source_text);
  await openDocument(observation.document_id);await page.locator('#reader-part-select').selectOption(observation.chunk_id);
  const part=page.locator(`.reader-chunk[data-reader-chunk="${observation.chunk_id}"]`);
  await part.getByRole('button',{name:'Listen to this part',exact:true}).click();
  const voice=part.locator(`.reader-audio[data-reader-document="${observation.document_id}"][data-reader-layer="${observation.layer}"]`);
  const player=voice.locator('.reader-audio-player');assert.equal(await player.getAttribute('data-revision'),observation.revision_id);
  assert.equal(await player.getAttribute('data-source-text-sha256'),record.device.provenance.source_text_sha256);
  await player.locator('[data-practice-rate="1"]:not([disabled])').waitFor();
  const actual=await player.locator('audio').evaluate(async media=>{const bytes=await(await fetch(media.src)).arrayBuffer();return{sha256:[...new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))].map(value=>value.toString(16).padStart(2,'0')).join(''),byte_length:bytes.byteLength};});
  assert.deepEqual(actual,{sha256:record.content_hash,byte_length:record.byte_length});
  const before=await page.evaluate(()=>window.rufrenLibrary.entries().filter(([key])=>key.startsWith('rufren.v1.event.')).map(([,raw])=>JSON.parse(raw).id));
  await player.locator('[data-practice-rate="1"]').click();
  await page.waitForFunction(({before,revision,text})=>window.rufrenLibrary.entries().some(([key,raw])=>{if(!key.startsWith('rufren.v1.event.'))return false;const event=JSON.parse(raw);return !before.includes(event.id)&&event.fact.kind==='stimulus_presented'&&event.fact.data.purpose==='audio'&&event.fact.data.revision===revision&&event.fact.data.russian===text;}),{before,revision:observation.revision_id,text:observation.source_text});
  await page.waitForFunction(revision=>{const root=[...document.querySelectorAll('.reader-audio-player')].find(root=>root.dataset.revision===revision);return root?.querySelector('audio')?.currentTime>0&&root.querySelector('[data-practice-status]')?.textContent.startsWith('Playing');},observation.revision_id);
  await player.locator('[data-practice-stop]').click();playedOriginals.add(record.asset_id);
  audio.push({document_id:observation.document_id,chunk_id:observation.chunk_id,revision_id:observation.revision_id,asset_id:record.asset_id,sha256:actual.sha256});
 }
 const current=await page.evaluate(()=>window.rufrenLibrary.entries().filter(([key])=>key.startsWith('rufren.v1.event.')).map(([,raw])=>JSON.parse(raw)));
 for(const event of events)assert.deepEqual(current.find(candidate=>candidate.id===event.id),event,'Opening sources rewrote an immutable event');
 assert.equal(current.filter(event=>event.fact.kind==='attempt_recorded').length,events.filter(event=>event.fact.kind==='attempt_recorded').length,'Reading/navigation fabricated a graded attempt');
 return {documents:restored,excerpts,dialogues:paths,audio,producer_local_keys_absent_before_open:expected.local_only.map(item=>item.key)};
}
