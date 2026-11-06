#!/usr/bin/env node
// Consume an actual synthetic app export through a disposable real relay and empty app.
// Fact scope only: audio attachment links are retained, original binary sync is not claimed.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {randomBytes,randomUUID} from 'node:crypto';
import {spawn,execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {readFile,writeFile,mkdir,copyFile,chmod,open,unlink} from 'node:fs/promises';
import path from 'node:path';
import net from 'node:net';
const exec=promisify(execFile),require=createRequire(import.meta.url),root=process.cwd();
const source=path.resolve(process.argv[2]||'.local/teacher-revisit-qa/run-01-ui/library.json');
const run=path.resolve(process.argv[3]||'.local/teacher-revisit-relay-qa/run-01');
const preview=process.env.RUFREN_QA_URL||'http://127.0.0.1:8091';
const expected=JSON.parse(await readFile(path.join(path.dirname(source),'expected-queue.json')));
const original=JSON.parse(await readFile(source)),events=Object.values(original.events);
assert(events.length>0&&events.every(event=>original.events[event.id]));
await mkdir(path.dirname(run),{recursive:true,mode:0o700});await mkdir(run,{mode:0o700});
const token=randomBytes(32).toString('base64url'),redact=text=>String(text).replaceAll(token,'[redacted]');
const database=path.join(run,'library.sqlite'),binary=path.join(run,'rufren-relay'),pidfile=path.join(run,'relay.pid');
let processHandle,browser,context,stage='prepare';let origin,relay,uploadPages=0,requests=0;
const state=async next=>{stage=next;await writeFile(path.join(run,'status.json'),JSON.stringify({stage}),{mode:0o600});console.log(stage);};
const cleanEnv=Object.fromEntries(Object.entries(process.env).filter(([key])=>!/^(RUFREN_|ANTHROPIC_|OPENAI_|CLAUDE_)/.test(key)));
async function request(body,key=token){
 requests++;const response=await fetch(origin+'/v1/sync',{method:'POST',redirect:'error',signal:AbortSignal.timeout(15000),headers:{Authorization:'Bearer '+key,'Content-Type':'application/json'},body:JSON.stringify(body)});
 const text=await response.text();assert(text.length<=1100000);return{status:response.status,text,value:response.ok?JSON.parse(text):null};
}
async function stop(){
 if(!processHandle)return;
 if(processHandle.exitCode===null){const {stdout}=await exec('ps',['-p',String(processHandle.pid),'-o','command=']);assert(stdout.includes(binary),'Owned PID changed');
  const exited=new Promise(resolve=>processHandle.once('exit',resolve));processHandle.kill('SIGINT');let timer;try{await Promise.race([exited,new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Owned relay did not stop')),10000);})]);}finally{clearTimeout(timer);}}
 await unlink(pidfile);processHandle=null;
}
const sameEvents=(actual)=>assert.deepEqual(new Map(actual.map(event=>[event.id,JSON.stringify(event)])),new Map(events.map(event=>[event.id,JSON.stringify(event)])));
try{
 await copyFile('target/debug/rufren-relay',binary);await chmod(binary,0o700);
 const port=await new Promise((resolve,reject)=>{const server=net.createServer();server.once('error',reject);server.listen(0,'127.0.0.1',()=>{const port=server.address().port;server.close(()=>resolve(port));});});origin='http://127.0.0.1:'+port;
 const log=await open(path.join(run,'relay.log'),'wx',0o600);processHandle=spawn(binary,[],{cwd:run,env:{...cleanEnv,RUFREN_RELAY_BIND:'127.0.0.1:'+port,RUFREN_RELAY_DB:database,RUFREN_RELAY_ASSETS:database+'.assets',RUFREN_SYNC_TOKEN:token,RUFREN_TUTOR_PROVIDER:'disabled',RUFREN_SPEECH_PROVIDER:'disabled',RUFREN_ASR_PROVIDER:'disabled',RUFREN_ALLOWED_ORIGINS:new URL(preview).origin},stdio:['ignore',log.fd,log.fd],detached:true});await log.close();await writeFile(pidfile,String(processHandle.pid),{mode:0o600,flag:'wx'});
 for(let i=0;i<100;i++){try{const response=await fetch(origin+'/health',{signal:AbortSignal.timeout(500)});assert.equal(await response.text(),'rufren-relay/4');break;}catch(error){if(i===99)throw error;await new Promise(resolve=>setTimeout(resolve,50));}}
 await state('bounded-real-http-upload-and-retry');let cursor=0;const batches=[];
 for(const event of events){let batch=batches.at(-1);if(!batch||batch.length===16||Buffer.byteLength(JSON.stringify([...batch,event]))>1000000){batch=[];batches.push(batch);}batch.push(event);}
 for(const batch of batches){const body={version:4,relay:relay||null,after:cursor,events:batch};assert(Buffer.byteLength(JSON.stringify(body))<1048576);const response=await request(body);assert.equal(response.status,200);relay=response.value.relay;cursor=response.value.cursor;assert.deepEqual(new Set(response.value.accepted),new Set(batch.map(event=>event.id)));uploadPages++;}
 assert.equal(cursor,events.length);const retry=await request({version:4,relay,after:cursor,events:batches[0]});assert.equal(retry.status,200);assert.equal(retry.value.cursor,cursor);assert.deepEqual(retry.value.events,[]);
 assert.equal((await request({version:4,relay,after:0,events:[]},'synthetic-wrong-key')).status,401);
 const legacy=await request({version:3,relay,after:cursor,events:[{version:3,id:randomUUID(),device:randomUUID(),recorded_at_ms:1,fact:{kind:'passage_revised',data:{passage:{id:randomUUID(),title:'Rejected synthetic legacy write',russian:'Тест.',translation:'Test.',source:'Synthetic protocol boundary.',coach_reviewed:false},parents:[]}}}]});assert.equal(legacy.status,409);assert(legacy.text.includes('Unsupported data version 3'));
 await state('independent-empty-client-pulls-exact-facts');let after=0,pulled=[];
 for(let page=0;page<100;page++){const response=await request({version:4,relay:page?relay:null,after,events:[]});assert.equal(response.status,200);assert.equal(response.value.relay,relay);assert(response.value.events.length<=128);pulled.push(...response.value.events);after=response.value.cursor;if(!response.value.has_more)break;}
 assert.equal(after,cursor);sameEvents(pulled);
 const reviews=events.filter(e=>e.fact.kind==='attempt_recorded'&&e.fact.data.policy==='teacher-delayed-revisit-v1');assert(reviews.length>0);
 for(const event of reviews){const context=events.find(e=>e.fact.kind==='attempt_context_recorded'&&e.fact.data.attempt===event.id);assert(context?.fact.data.reference_answer.includes('Rufren teacher revisit v1: '));assert(pulled.some(e=>e.id===context.id&&JSON.stringify(e)===JSON.stringify(context)));}
 await state('actual-empty-app-sync-and-queue-reconstruction');const {chromium}=require(process.env.RUFREN_PLAYWRIGHT_MODULE||'playwright');browser=await chromium.launch({headless:true,...(process.env.RUFREN_CHROMIUM_EXECUTABLE?{executablePath:process.env.RUFREN_CHROMIUM_EXECUTABLE}:{})});context=await browser.newContext({serviceWorkers:'block'});
 await context.addInitScript(asof=>{Date.now=()=>asof;},expected.as_of_ms);let inference=0,forbidden=0;const errors=[];
 await context.route('**/*',route=>{const url=new URL(route.request().url());if(url.origin!==new URL(preview).origin&&url.origin!==origin){forbidden++;return route.abort();}if(url.pathname.startsWith('/api/')&&route.request().method()==='POST'){inference++;return route.abort();}return route.continue();});
 const page=await context.newPage();page.on('pageerror',error=>errors.push(error.message));await page.goto(preview);const nav=name=>page.getByRole('navigation',{name:'Main navigation'}).getByRole('button',{name:new RegExp(name,'i')});await nav('Settings').click();await page.locator('#sync-url').fill(origin);await page.locator('#sync-key').fill(token);await page.getByRole('button',{name:'Sync now',exact:true}).click();await page.getByRole('button',{name:'Sync now',exact:true}).waitFor({timeout:60000});await page.waitForFunction(count=>window.rufrenLibrary.entries().filter(([key])=>key.startsWith('rufren.v1.event.')).length===count,events.length);await page.getByRole('button',{name:'Prepare export',exact:true}).click();const appExport=JSON.parse(await page.locator('#backup-json').inputValue());sameEvents(Object.values(appExport.library.events));
 await nav('Today').click();const more=page.getByRole('button',{name:'Show all planned returns',exact:true});if(await more.count())await more.click();await page.locator('.teacher-return-row').first().waitFor();const queue=await page.locator('.teacher-return-row').evaluateAll(rows=>rows.map(row=>({document_id:row.dataset.reviewDocument,practice_index:Number(row.dataset.reviewIndex),due_at_ms:Number(row.dataset.reviewDue)})).sort((a,b)=>a.practice_index-b.practice_index));assert.equal(queue.length,expected.entries.length);for(const row of queue){assert.equal(row.document_id,expected.document_id);assert.equal(row.due_at_ms,expected.entries.find(e=>e.practice_index===row.practice_index).due_at_ms);}
 const previewScripts=await page.locator('script[src]').evaluateAll(nodes=>nodes.map(node=>node.src));assert.equal(inference,0);assert.equal(forbidden,0);assert.deepEqual(errors,[]);await context.close();context=null;await browser.close();browser=null;
 await state('stopped-sqlite-byte-check-and-rust-reader');await stop();
 const python="import sqlite3,json,sys; source=json.load(open(sys.argv[1]))['events']; con=sqlite3.connect('file:'+sys.argv[2]+'?mode=ro',uri=True); rows=con.execute('select id,json from events').fetchall(); assert len(rows)==len(source); assert all(raw==json.dumps(source[i],ensure_ascii=False,separators=(',',':')) for i,raw in rows); print(json.dumps({'rows':len(rows),'schema':con.execute('pragma user_version').fetchone()[0]}))";
 const checked=JSON.parse((await exec('python3',['-c',python,source,database])).stdout);assert.equal(checked.schema,1);
 const reader=(await exec(binary,['--check-db',database],{cwd:run,env:cleanEnv})).stdout.trim();assert.equal(reader,`Verified ${events.length} immutable events and 0 recording metadata records`);
 const result={passed:true,protocol:4,scope:'HTTP facts and SQLite; original audio bytes were not uploaded or restored',source_export:source,preview_scripts:previewScripts,events:events.length,review_attempts:reviews.length,archive_documents:events.filter(e=>e.fact.kind==='teacher_document_recorded').length,recording_links:events.filter(e=>e.fact.kind==='recording_attached').length,upload_pages:uploadPages,compact_event_json_identical:true,empty_app_queue:queue,as_of_ms:expected.as_of_ms,queue_source:'Expected values from producer receipt; all four independently checked in empty app against real HTTP replica',typed_reader:reader,legacy_protocol_refused:true,retry_cursor_unchanged:true,inference_calls:0,forbidden_requests:forbidden,errors,owned_service_stopped:true};
 await writeFile(path.join(run,'result.json'),JSON.stringify(result,null,2),{mode:0o600,flag:'wx'});console.log(JSON.stringify(result));
}catch(error){await writeFile(path.join(run,'failure.json'),JSON.stringify({stage,error:redact(error.message)},null,2),{mode:0o600});console.error(redact(error.message));process.exitCode=1;}
finally{if(context)await context.close();if(browser)await browser.close();try{await stop();}catch(error){console.error(redact(error.message));process.exitCode=1;}}
