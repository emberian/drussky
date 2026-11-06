#!/usr/bin/env node
// Independent app-export -> real disposable relay -> SQLite/restore -> empty app acceptance.
// Inputs are explicit synthetic producer artifacts. No provider is enabled or called.
// Historical replay: set RUFREN_RELAY_QA_PROTOCOL to the frozen protocol number with its
// RUFREN_RELAY_QA_BINARY and matching RUFREN_QA_URL client; fixture event versions stay unchanged.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {randomBytes,randomUUID,createHash} from 'node:crypto';
import {spawn,execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {readFile,writeFile,mkdir,copyFile,chmod,open,unlink} from 'node:fs/promises';
import {existsSync} from 'node:fs';
import path from 'node:path';
import net from 'node:net';
import {inspectSoundHistory} from './qa-sound-consumer.mjs';
import {inspectReader} from './qa-reader-consumer.mjs';
import {inspectTiming} from './qa-timing-consumer.mjs';
const exec=promisify(execFile),require=createRequire(import.meta.url),root=process.cwd();
const mode=process.env.RUFREN_RELAY_QA_MODE||'revisit';assert(['revisit','dialogue','sound','reader','timing'].includes(mode));
const currentProtocol=(await readFile(new URL('../crates/rufren-core/src/lib.rs',import.meta.url),'utf8')).match(/^pub const SYNC_VERSION:\s*u32\s*=\s*(\d+)\s*;/m)?.[1];
const protocol=Number(process.env.RUFREN_RELAY_QA_PROTOCOL||currentProtocol);assert(Number.isSafeInteger(protocol)&&protocol>=4,'Missing current core sync version; historical replay needs an explicit override');
const source=path.resolve(process.argv[2]||(mode==='timing'?'.local/review-timing-qa/final/library.json':mode==='reader'?'.local/reader-qa/final/library.json':mode==='sound'?'.local/sound-focus-qa/final/library.json':mode==='dialogue'?'.local/teacher-dialogue-qa/final/library.json':'.local/teacher-revisit-qa/run-01-ui/library.json'));
const run=path.resolve(process.argv[3]||`.local/${mode}-relay-qa/run-01`);
const preview=process.env.RUFREN_QA_URL||'http://127.0.0.1:8091',sourceFolder=path.dirname(source);
const expectedName=mode==='timing'?'expected-timing.json':mode==='reader'?'expected-reader.json':mode==='sound'?'expected-sound.json':mode==='dialogue'?(existsSync(path.join(sourceFolder,'expected-graph.json'))?'expected-graph.json':'expected-dialogues.json'):'expected-queue.json';
const expected=JSON.parse(await readFile(path.join(sourceFolder,expectedName)));
const original=JSON.parse(await readFile(source)),events=Array.isArray(original.events)?original.events:Object.values(original.events);
assert(events.length>0&&new Set(events.map(e=>e.id)).size===events.length);
const records=existsSync(path.join(sourceFolder,'originals.json'))?JSON.parse(await readFile(path.join(sourceFolder,'originals.json'))).records:[];
assert(Array.isArray(records));const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const audioReferences=events.flatMap(({fact:{kind,data}})=>['recording_attached','dialogue_audio_attached'].includes(kind)?[{asset_id:data.asset_id,sha256:data.asset.sha256}]:['speech_transcribed','dialogue_speech_transcribed'].includes(kind)?[{asset_id:data.asset_id,sha256:data.audio_sha256}]:kind==='sound_trial_prepared'?data.references.map(ref=>({asset_id:ref.asset_id,sha256:ref.asset.sha256})):kind==='sound_reference_played'?[{asset_id:data.reference.asset_id,sha256:data.reference.asset.sha256}]:kind==='sound_trial_completed'&&data.response.kind==='speaking'?[{asset_id:data.response.asset_id,sha256:data.response.asset.sha256}]:[]);
const referenced=new Set(audioReferences.map(ref=>ref.asset_id));
if(records.length)for(const ref of audioReferences)assert(records.some(record=>record.asset_id===ref.asset_id&&record.content_hash===ref.sha256),'Original export incomplete or wrong hash for referenced take '+ref.asset_id);
await mkdir(path.dirname(run),{recursive:true,mode:0o700});await mkdir(run,{mode:0o700});
const token=randomBytes(32).toString('base64url'),redact=text=>String(text).replaceAll(token,'[redacted]');
const database=path.join(run,'library.sqlite'),restored=path.join(run,'restored.sqlite'),binary=path.join(run,'rufren-relay'),pidfile=path.join(run,'relay.pid');
let processHandle,browser,context,stage='prepare',origin,relay,uploadPages=0,port,relayExecutableSha;
const state=async next=>{stage=next;await writeFile(path.join(run,'status.json'),JSON.stringify({stage}),{mode:0o600});console.log(stage);};
const cleanEnv=Object.fromEntries(Object.entries(process.env).filter(([key])=>!/^(RUFREN_|ANTHROPIC_|OPENAI_|CLAUDE_)/.test(key)));
async function raw(method,relative,body,key=token){
 const bytes=body===undefined?undefined:Buffer.isBuffer(body)?body:Buffer.from(JSON.stringify(body));
 const response=await fetch(origin+relative,{method,redirect:'error',signal:AbortSignal.timeout(15000),headers:{Authorization:'Bearer '+key,...(relay?{'X-Rufren-Relay':relay}:{}),...(bytes?{'Content-Type':Buffer.isBuffer(body)?'application/octet-stream':'application/json'}:{})},body:bytes});
 const received=Buffer.from(await response.arrayBuffer());assert(received.length<=36*1024*1024);return{status:response.status,body:received,text:received.toString(),value:response.ok&&!relative.includes('/blobs/')?JSON.parse(received):null};
}
const request=(body,key=token)=>raw('POST','/v1/sync',body,key);
async function waitForReaderRelease(){
 const gate=process.env.RUFREN_RELAY_QA_READER_RELEASE;
 if(!gate)return;
 await state('awaiting-native-build-slot-release');
 const end=Date.now()+30*60*1000;
 while(!existsSync(gate)){assert(Date.now()<end,'Native build slot was not released within30minutes');await new Promise(resolve=>setTimeout(resolve,1000));}
}
async function stop(){
 if(!processHandle)return;
 if(processHandle.exitCode===null){const {stdout}=await exec('ps',['-p',String(processHandle.pid),'-o','command=']);assert(stdout.includes(binary),'Owned PID changed');const exited=new Promise(resolve=>processHandle.once('exit',resolve));processHandle.kill('SIGINT');let timer;try{await Promise.race([exited,new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Owned relay did not stop')),10000);})]);}finally{clearTimeout(timer);}}
 await unlink(pidfile);processHandle=null;
}
async function start(db){
 const log=await open(path.join(run,'relay.log'),'a',0o600);
 processHandle=spawn(binary,[],{cwd:run,env:{...cleanEnv,RUFREN_RELAY_BIND:'127.0.0.1:'+port,RUFREN_RELAY_DB:db,RUFREN_RELAY_ASSETS:db+'.assets',RUFREN_SYNC_TOKEN:token,RUFREN_TUTOR_PROVIDER:'disabled',RUFREN_SPEECH_PROVIDER:'disabled',RUFREN_ASR_PROVIDER:'disabled',RUFREN_ALLOWED_ORIGINS:new URL(preview).origin},stdio:['ignore',log.fd,log.fd],detached:true});await log.close();await writeFile(pidfile,String(processHandle.pid),{mode:0o600,flag:'wx'});
 for(let i=0;i<100;i++){try{const response=await fetch(origin+'/health',{signal:AbortSignal.timeout(500)});assert.equal(await response.text(),`rufren-relay/${protocol}`);return;}catch(error){if(i===99)throw error;await new Promise(resolve=>setTimeout(resolve,50));}}
}
const sameEvents=actual=>assert.deepEqual(new Map(actual.map(e=>[e.id,JSON.stringify(e)])),new Map(events.map(e=>[e.id,JSON.stringify(e)])));
async function pull(){let after=0,all=[];for(let page=0;page<100;page++){const response=await request({version:protocol,relay:page?relay:null,after,events:[]});assert.equal(response.status,200);assert.equal(response.value.relay,relay);assert(response.value.events.length<=128);all.push(...response.value.events);after=response.value.cursor;if(!response.value.has_more){sameEvents(all);assert.equal(after,events.length);return all;}}throw Error('Pagination did not finish');}
async function verifyOriginals(){
 let after='',remote=[];for(let i=0;i<100;i++){const page=(await raw('GET','/v1/assets'+(after?'?after='+encodeURIComponent(after):''))).value;assert(page);remote.push(...page.records);if(!page.next_after)break;after=page.next_after;}
 assert.equal(remote.length,records.length);
 for(const record of remote){assert.deepEqual(record,records.find(r=>r.asset_id===record.asset_id));const bytes=(await raw('GET','/v1/assets/blobs/'+record.content_hash)).body;assert.equal(hash(bytes),record.content_hash);assert.equal(bytes.length,record.byte_length);assert(bytes.equals(await readFile(path.join(sourceFolder,'originals',record.content_hash+'.blob'))));}
}
async function playVisibleOriginals(page, played){
 for(const record of records){
  if(played.has(record.asset_id))continue;
  const player=page.locator('.audio-review-player[data-asset='+JSON.stringify(record.asset_id)+'],.sound-focus-player[data-asset='+JSON.stringify(record.asset_id)+']').first();
  if(!await player.count())continue;
  await player.evaluate(root=>{for(let node=root.parentElement;node;node=node.parentElement)if(node.tagName==='DETAILS')node.open=true;});
  const button=player.locator('[data-practice-rate="1"]');
  await page.waitForFunction(id=>{const root=[...document.querySelectorAll('.audio-review-player,.sound-focus-player')].find(e=>e.dataset.asset===id);return root?.querySelector('[data-practice-rate="1"]')?.disabled===false;},record.asset_id);
  await button.click();
  await page.waitForFunction(id=>{const root=[...document.querySelectorAll('.audio-review-player,.sound-focus-player')].find(e=>e.dataset.asset===id);return root?.querySelector('[data-practice-status]')?.textContent.startsWith('Playing')&&root.querySelector('audio')?.currentTime>0;},record.asset_id);
  await player.locator('[data-practice-stop]').click();played.add(record.asset_id);
 }
}
async function inspectBrowserOriginals(page){
 const saved=await page.evaluate(()=>new Promise((resolve,reject)=>{const r=indexedDB.open('rufren-audio-v1',1);r.onsuccess=()=>{const db=r.result;const get=db.transaction('assets','readonly').objectStore('assets').getAll();get.onsuccess=async()=>{try{const list=[];for(const record of get.result){const{blob,...metadata}=record;const bytes=new Uint8Array(await blob.arrayBuffer());const sha=[...new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))].map(v=>v.toString(16).padStart(2,'0')).join('');list.push({metadata,sha,base64:btoa(Array.from(bytes,b=>String.fromCharCode(b)).join(''))});}db.close();resolve(list);}catch(error){reject(error);}};get.onerror=()=>reject(get.error);};r.onerror=()=>reject(r.error);}));
 assert.equal(saved.length,records.length);for(const item of saved){assert.deepEqual(item.metadata,records.find(r=>r.asset_id===item.metadata.asset_id));assert.equal(item.sha,item.metadata.content_hash);assert(Buffer.from(item.base64,'base64').equals(await readFile(path.join(sourceFolder,'originals',item.sha+'.blob'))));}
}
try{
 await copyFile(process.env.RUFREN_RELAY_QA_BINARY||'target/debug/rufren-relay',binary);await chmod(binary,0o700);relayExecutableSha=hash(await readFile(binary));
 port=await new Promise((resolve,reject)=>{const server=net.createServer();server.once('error',reject);server.listen(0,'127.0.0.1',()=>{const n=server.address().port;server.close(()=>resolve(n));});});origin='http://127.0.0.1:'+port;await start(database);
 await state('bounded-real-http-upload-and-retry');let cursor=0;const batches=[];
 for(const event of events){let batch=batches.at(-1);if(!batch||batch.length===16||Buffer.byteLength(JSON.stringify([...batch,event]))>1000000){batch=[];batches.push(batch);}batch.push(event);}
 for(const batch of batches){const body={version:protocol,relay:relay||null,after:cursor,events:batch};assert(Buffer.byteLength(JSON.stringify(body))<1048576);const response=await request(body);assert.equal(response.status,200,response.text.slice(0,256));relay=response.value.relay;cursor=response.value.cursor;assert.deepEqual(new Set(response.value.accepted),new Set(batch.map(e=>e.id)));uploadPages++;}
 assert.equal(cursor,events.length);const retry=await request({version:protocol,relay,after:cursor,events:batches[0]});assert.equal(retry.status,200);assert.equal(retry.value.cursor,cursor);assert.deepEqual(retry.value.events,[]);assert.equal((await request({version:protocol,relay,after:0,events:[]},'synthetic-wrong-key')).status,401);
 const old=protocol-1;const legacy=await request({version:old,relay,after:cursor,events:[{version:old,id:randomUUID(),device:randomUUID(),recorded_at_ms:1,fact:{kind:'passage_revised',data:{passage:{id:randomUUID(),title:'Rejected synthetic legacy write',russian:'Тест.',translation:'Test.',source:'Synthetic protocol boundary.',coach_reviewed:false},parents:[]}}}]});assert.equal(legacy.status,409);assert(legacy.text.includes('Unsupported data version '+old));await pull();
 const reviews=events.filter(e=>e.fact.kind==='attempt_recorded'&&e.fact.data.policy==='teacher-delayed-revisit-v1');
 const users=events.filter(e=>e.fact.kind==='teacher_dialogue_user_recorded'),replies=events.filter(e=>e.fact.kind==='teacher_dialogue_reply_recorded');
 if(mode==='revisit'){assert(reviews.length);for(const event of reviews){const ctx=events.find(e=>e.fact.kind==='attempt_context_recorded'&&e.fact.data.attempt===event.id);assert(ctx?.fact.data.reference_answer.includes('Rufren teacher revisit v1: '));}}
 else if(mode==='dialogue'){assert(users.length&&replies.length);for(const reply of replies)assert.deepEqual(reply.fact.data.request,users.find(u=>u.fact.data.request_id===reply.fact.data.request.request_id)?.fact.data);}
 else if(mode==='sound')assert(records.length&&events.some(event=>event.fact.kind==='sound_trial_prepared'),'Sound fixture needs prepared facts and linked original bytes');
 else if(mode==='timing')assert(events.some(event=>event.fact.kind==='review_started'),'Timing fixture needs immutable starts');
 else assert(events.some(event=>event.fact.kind==='reader_document_recorded'),'Reader fixture needs immutable original documents');
 if(records.length){
  await state('hash-verified-original-upload');for(const record of records){assert(/^[0-9a-f]{64}$/.test(record.content_hash));assert(Number.isSafeInteger(record.byte_length)&&record.byte_length>0&&record.byte_length<=32*1024*1024);const bytes=await readFile(path.join(sourceFolder,'originals',record.content_hash+'.blob'));assert.equal(bytes.length,record.byte_length);assert.equal(hash(bytes),record.content_hash);const blob=await raw('PUT','/v1/assets/blobs/'+record.content_hash,bytes);assert([200,201,204].includes(blob.status));const registered=await raw('PUT','/v1/assets/records/'+encodeURIComponent(record.asset_id),record);assert.equal(registered.status,200,registered.text);assert.deepEqual(registered.value,record);}await verifyOriginals();
 }
 let snapshotRestored=false,working=database;
 if((mode==='dialogue'||mode==='sound'||mode==='reader'||mode==='timing')&&(records.length||referenced.size===0)){
  await state('coherent-snapshot-and-fresh-restore');const backup=path.join(run,'backup.sqlite'),tool=path.join(root,'scripts/relay-backup.py');for(const args of [['backup',database,backup,'--assets',database+'.assets'],['verify',backup],['restore',backup,restored]])await exec('python3',[tool,...args]);
  await stop();await start(restored);await pull();if(records.length)await verifyOriginals();working=restored;snapshotRestored=true;
 }
 await state('actual-empty-app-reconstruction');const{chromium}=require(process.env.RUFREN_PLAYWRIGHT_MODULE||'playwright');browser=await chromium.launch({headless:true,...(process.env.RUFREN_CHROMIUM_EXECUTABLE?{executablePath:process.env.RUFREN_CHROMIUM_EXECUTABLE}:{})});context=await browser.newContext({serviceWorkers:'block'});
 if(expected.as_of_ms!==undefined)await context.addInitScript(asof=>{Date.now=()=>asof;},expected.as_of_ms);
 let inference=0,forbidden=0;const errors=[];await context.route('**/*',route=>{const url=new URL(route.request().url());if(url.origin!==new URL(preview).origin&&url.origin!==origin){forbidden++;return route.abort();}if(url.pathname.startsWith('/api/')&&route.request().method()==='POST'){inference++;return route.abort();}return route.continue();});
 const page=await context.newPage();page.on('pageerror',e=>errors.push(e.message));await page.goto(preview);const previewBuild=await page.evaluate(()=>fetch('./offline-build.json').then(response=>{if(!response.ok)throw Error('Missing frozen web-build receipt');return response.json();}));assert(typeof previewBuild.version==='string'&&previewBuild.version);const nav=name=>page.getByRole('navigation',{name:'Main navigation'}).getByRole('button',{name:new RegExp(name,'i')});await nav('Settings').click();await page.locator('#sync-url').fill(origin);await page.locator('#sync-key').fill(token);if(records.length)await page.locator('#sync-recordings').check();await page.getByRole('button',{name:'Sync now',exact:true}).click();await page.getByRole('button',{name:'Sync now',exact:true}).waitFor({timeout:60000});await page.waitForFunction(count=>window.rufrenLibrary.entries().filter(([key])=>key.startsWith('rufren.v1.event.')).length===count,events.length);
 if(records.length){await page.getByText(new RegExp('Notebook synced\\.')).waitFor();await inspectBrowserOriginals(page);}
 await page.getByRole('button',{name:'Prepare export',exact:true}).click();sameEvents(Object.values(JSON.parse(await page.locator('#backup-json').inputValue()).library.events));
 let queue=[],opened=[],soundTrials=[],readerInspection=null,timingInspection=null;const playedOriginals=new Set();
 if(mode==='revisit'){
  await nav('Today').click();const more=page.getByRole('button',{name:'Show all planned returns',exact:true});if(await more.count())await more.click();await page.locator('.teacher-return-row').first().waitFor();queue=await page.locator('.teacher-return-row').evaluateAll(rows=>rows.map(row=>({document_id:row.dataset.reviewDocument,practice_index:Number(row.dataset.reviewIndex),due_at_ms:Number(row.dataset.reviewDue)})).sort((a,b)=>a.practice_index-b.practice_index));assert.equal(queue.length,expected.entries.length);for(const row of queue){assert.equal(row.document_id,expected.document_id);assert.equal(row.due_at_ms,expected.entries.find(e=>e.practice_index===row.practice_index).due_at_ms);}
 }else if(mode==='dialogue'){
  await nav('Teacher').click();await page.getByRole('button',{name:'Conversations',exact:true}).click();
  const threads=expected.threads;assert(Array.isArray(threads)&&threads.length);const paths=expected.paths||[];
  for(const thread of threads)for(const leaf of thread.leaf_ids){const library=page.locator('details.dialogue-library');if(!await library.evaluate(el=>el.open))await library.locator('summary').first().click();await page.locator(`[data-open-dialogue-leaf="${leaf}"]`).click();await page.locator(`.teacher-dialogue[data-dialogue-leaf="${leaf}"]`).waitFor();assert.equal(await page.locator('.teacher-dialogue').getAttribute('data-dialogue-thread'),thread.thread_id);const ids=await page.locator('.teacher-dialogue .dialogue-message[data-message-id]').evaluateAll(nodes=>nodes.map(n=>n.dataset.messageId));const exact=paths.find(p=>p.leaf_id===leaf);if(exact)assert.deepEqual(ids,exact.message_ids);assert(ids.includes(leaf));opened.push({thread_id:thread.thread_id,leaf_id:leaf,message_ids:ids});if(records.length)await playVisibleOriginals(page,playedOriginals);}
 }else if(mode==='sound')soundTrials=await inspectSoundHistory({page,nav,events,expected,records,playVisibleOriginals,playedOriginals});
 else if(mode==='timing')timingInspection=await inspectTiming({page,nav,events,expected,records,playVisibleOriginals,playedOriginals});
 else readerInspection=await inspectReader({page,nav,events,expected,records,playedOriginals});
 if(records.length){assert(playedOriginals.size>=(mode==='reader'?1:2),'Restored linked originals must be playable in the actual app');await inspectBrowserOriginals(page);}
 const previewScripts=await page.locator('script[src]').evaluateAll(nodes=>nodes.map(n=>n.src));assert.equal(inference,0);assert.equal(forbidden,0);assert.deepEqual(errors,[]);await context.close();context=null;await browser.close();browser=null;await state('stopped-sqlite-byte-check-and-rust-reader');await stop();
 const python="import sqlite3,json,sys; data=json.load(open(sys.argv[1]))['events']; source={e['id']:e for e in data} if isinstance(data,list) else data; con=sqlite3.connect('file:'+sys.argv[2]+'?mode=ro',uri=True); rows=con.execute('select id,json from events').fetchall(); assert len(rows)==len(source); assert all(raw==json.dumps(source[i],ensure_ascii=False,separators=(',',':')) for i,raw in rows); print(con.execute('pragma user_version').fetchone()[0])";assert.equal(Number((await exec('python3',['-c',python,source,working])).stdout.trim()),1);
 await waitForReaderRelease();const reader=(await exec(binary,['--check-db',working],{cwd:run,env:cleanEnv})).stdout.trim();assert.equal(reader,`Verified ${events.length} immutable events and ${records.length} recording metadata records`);
 const result={passed:true,mode,protocol,relay_executable_sha256:relayExecutableSha,preview_build:previewBuild.version,scope:records.length?'Exact '+mode+' facts, registry and linked original audio bytes through coherent restore':'HTTP facts and SQLite; original audio bytes were not uploaded or restored',source_export:source,preview_scripts:previewScripts,events:events.length,review_attempts:reviews.length,dialogue_users:users.length,dialogue_replies:replies.length,archive_documents:events.filter(e=>e.fact.kind==='teacher_document_recorded').length,recording_links:events.filter(e=>['recording_attached','dialogue_audio_attached'].includes(e.fact.kind)).length,upload_pages:uploadPages,compact_event_json_identical:true,empty_app_queue:queue,empty_app_dialogue_paths:opened,sound_trials:soundTrials.length,sound_completed_trials:soundTrials.filter(trial=>trial.completion_ids.length).length,empty_app_sound_trials:soundTrials,reader_documents:readerInspection?.documents.length||0,empty_app_reader:readerInspection,empty_app_timing:timingInspection,timing_source_events_sha256:mode==='timing'?hash(Buffer.from(JSON.stringify(events))):undefined,original_take_records:records.length,original_hashes:new Set(records.map(r=>r.content_hash)).size,played_original_asset_ids:[...playedOriginals],coherent_snapshot_restored:snapshotRestored,as_of_ms:expected.as_of_ms,typed_reader:reader,legacy_protocol_refused:true,retry_cursor_unchanged:true,inference_calls:0,forbidden_requests:forbidden,errors,owned_service_stopped:true};await writeFile(path.join(run,'result.json'),JSON.stringify(result,null,2),{mode:0o600,flag:'wx'});console.log(JSON.stringify(result));
}catch(error){await writeFile(path.join(run,'failure.json'),JSON.stringify({stage,error:redact(error.message)},null,2),{mode:0o600});console.error(redact(error.message));process.exitCode=1;}
finally{if(context)await context.close();if(browser)await browser.close();try{await stop();}catch(error){console.error(redact(error.message));process.exitCode=1;}}
