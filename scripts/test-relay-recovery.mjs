#!/usr/bin/env node
// Private TLS + actual Dioxus recovery acceptance. All data/keys/processes are disposable.
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {createHash,randomBytes,randomUUID} from 'node:crypto';
import {spawn,execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {mkdir,readFile,writeFile,copyFile,cp,chmod,unlink,open} from 'node:fs/promises';
import {existsSync} from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import net from 'node:net';
import https from 'node:https';
import {addSyntheticSpeechFacts} from './platform/fixtures/synthetic-speech-facts.mjs';

const root=path.dirname(path.dirname(fileURLToPath(import.meta.url))),exec=promisify(execFile),require=createRequire(import.meta.url);
const runDir=path.resolve(process.argv[2]||path.join(root,'.local/relay-recovery-qa',new Date().toISOString().replaceAll(':','-')));
await mkdir(path.dirname(runDir),{recursive:true,mode:0o700});await mkdir(runDir,{mode:0o700});
const token=randomBytes(32).toString('base64url'),restoreToken=randomBytes(32).toString('base64url');
const redact=text=>String(text).replaceAll(token,'[redacted]').replaceAll(restoreToken,'[redacted]');
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const processes=new Map(),receipts=[],sent=new Map();let browser,context,stage='initializing',relayId,base,ca;
const preview=process.env.RUFREN_PREVIEW_URL||'http://127.0.0.1:8091';
const sourceDb=path.join(runDir,'source.sqlite'),sourceAssets=sourceDb+'.assets',backup=path.join(runDir,'backup.sqlite'),restored=path.join(runDir,'restored.sqlite');
const binary=path.join(runDir,'rufren-relay'),backupTool=path.join(root,'scripts/relay-backup.py');
const privateWrite=async(file,data)=>writeFile(file,data,{flag:'wx',mode:0o600});
const state=async name=>{stage=name;await writeFile(path.join(runDir,'status.json'),JSON.stringify({stage,at:new Date().toISOString()},null,2));console.log(name);};
const freePort=()=>new Promise((resolve,reject)=>{const server=net.createServer();server.once('error',reject);server.listen(0,'127.0.0.1',()=>{const port=server.address().port;server.close(()=>resolve(port));});});
const waitFor=async(predicate,label)=>{const end=Date.now()+15000;while(Date.now()<end){if(await predicate())return;await new Promise(resolve=>setTimeout(resolve,50));}throw Error('Timed out: '+label);};
async function start(label,command,args,environment){
 assert(!processes.has(label));
 const log=await open(path.join(runDir,label+'.log'),'a',0o600);
 const child=spawn(command,args,{cwd:runDir,env:environment,stdio:['ignore',log.fd,log.fd],detached:true});await log.close();
 await privateWrite(path.join(runDir,label+'.pid'),String(child.pid));processes.set(label,{child,command});
}
async function stop(label){
 const running=processes.get(label);if(!running)return;
 if(running.child.exitCode===null){
  const {stdout}=await exec('ps',['-p',String(running.child.pid),'-o','command=']);assert(stdout.includes(running.command),'Owned test PID changed');
  const exited=new Promise(resolve=>running.child.once('exit',resolve));running.child.kill('SIGINT');
  let timer;try{await Promise.race([exited,new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Owned '+label+' did not stop')),10000);})]);}finally{clearTimeout(timer);}
 }
 await unlink(path.join(runDir,label+'.pid'));processes.delete(label);
}
const cleanEnv=Object.fromEntries(Object.entries(process.env).filter(([key])=>!/^(RUFREN_|ANTHROPIC_|OPENAI_|CLAUDE_)/.test(key)));
async function startRelay(database,key,port){
 await start('fixture-relay',binary,[],{...cleanEnv,RUFREN_RELAY_BIND:'127.0.0.1:'+port,RUFREN_RELAY_DB:database,RUFREN_RELAY_ASSETS:database+'.assets',RUFREN_SYNC_TOKEN:key,RUFREN_TUTOR_PROVIDER:'disabled',RUFREN_SPEECH_PROVIDER:'disabled',RUFREN_ASR_PROVIDER:'disabled',RUFREN_ALLOWED_ORIGINS:new URL(preview).origin});
}
async function tlsRequest(method,relative,body=undefined,key=token,identity=relayId){
 const payload=body===undefined?undefined:Buffer.isBuffer(body)?body:Buffer.from(JSON.stringify(body));
 const url=new URL(relative,base);assert.equal(url.origin,new URL(base).origin);
 return new Promise((resolve,reject)=>{
  const request=https.request(url,{method,ca,rejectUnauthorized:true,headers:{Authorization:'Bearer '+key,...(identity?{'X-Rufren-Relay':identity}:{}),...(payload?{'Content-Type':Buffer.isBuffer(body)?'application/octet-stream':'application/json','Content-Length':payload.length}:{})}},response=>{
   const chunks=[];let size=0;response.on('data',chunk=>{size+=chunk.length;if(size>36*1024*1024){request.destroy(Error('Oversized fixture response'));return;}chunks.push(chunk);});response.on('end',()=>resolve({status:response.statusCode,headers:response.headers,body:Buffer.concat(chunks)}));
  });request.setTimeout(15000,()=>request.destroy(Error('Fixture request timed out')));request.on('error',reject);request.end(payload);
 });
}
async function pull(key=token,identity=relayId){
 let after=0,events=[],library=identity;for(let page=0;page<100;page++){
  const response=await tlsRequest('POST','/v1/sync',{version:3,relay:library||null,after,events:[]},key,undefined);assert.equal(response.status,200);const value=JSON.parse(response.body);library=value.relay;after=value.cursor;events.push(...value.events);if(!value.has_more)return{relay:library,cursor:after,events};
 }throw Error('Fixture pagination did not finish');
}
async function push(events,after=0){
 const response=await tlsRequest('POST','/v1/sync',{version:3,relay:relayId||null,after,events});assert.equal(response.status,200,redact(response.body.toString()));
 for(const event of events)sent.set(event.id,event);return JSON.parse(response.body);
}
const nav=(page,name)=>page.getByRole('navigation',{name:'Main navigation'}).getByRole('button',{name:new RegExp(name,'i')});
const ready=page=>page.getByRole('navigation',{name:'Main navigation'}).waitFor({timeout:60000});
async function secretInput(page,key){await page.locator('#sync-key').waitFor();await page.evaluate(key=>{const element=document.querySelector('#sync-key');Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(element,key);element.dispatchEvent(new Event('input',{bubbles:true}));},key);}
async function settings(page,key=token){await nav(page,'Settings').click();await page.locator('#sync-url').fill(base);await secretInput(page,key);}
async function sync(page){await page.getByRole('button',{name:'Sync now',exact:true}).click();await page.getByRole('button',{name:'Sync now',exact:true}).waitFor({timeout:60000});await page.waitForTimeout(50);}
async function exportFacts(page){await nav(page,'Settings').click();await page.getByRole('button',{name:'Prepare export',exact:true}).click();return JSON.parse(await page.locator('#backup-json').inputValue()).library.events;}
const binding=page=>page.evaluate(()=>JSON.parse(window.rufrenLibrary.get('rufren.v1.meta.sync')));
async function assetsInBrowser(page){return page.evaluate(()=>new Promise((resolve,reject)=>{const request=indexedDB.open('rufren-audio-v1',1);request.onsuccess=()=>{const db=request.result,tx=db.transaction('assets','readonly'),read=tx.objectStore('assets').getAll();read.onsuccess=async()=>{try{const result=[];for(const record of read.result){const{blob,...metadata}=record;const bytes=new Uint8Array(await blob.arrayBuffer());const sha=[...new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))].map(b=>b.toString(16).padStart(2,'0')).join('');result.push({metadata,sha,bytes:Array.from(bytes)});}db.close();resolve(result);}catch(error){reject(error);}};read.onerror=()=>reject(read.error);};request.onerror=()=>reject(request.error);}));}
async function tool(...args){return exec('python3',[backupTool,...args],{cwd:root});}
async function mustRefuse(source,target,label){let refused=false;try{await tool('restore',source,target);}catch{refused=true;}assert(refused,label);assert(!existsSync(target),'Rejected restore created a usable destination');receipts.push(label);}

try{
 await state('preparing-isolated-tls-and-frozen-binary');await copyFile(path.join(root,'target/debug/rufren-relay'),binary);await chmod(binary,0o700);
 const relayPort=await freePort(),tlsPort=await freePort();base='https://127.0.0.1:'+tlsPort;
 const caKey=path.join(runDir,'fixture-ca-key.pem'),caPath=path.join(runDir,'fixture-ca.pem'),leafKey=path.join(runDir,'fixture-leaf-key.pem'),leaf=path.join(runDir,'fixture-leaf.pem'),csr=path.join(runDir,'fixture-leaf.csr'),extensions=path.join(runDir,'fixture-leaf.cnf');
 await exec('openssl',['ecparam','-name','prime256v1','-genkey','-noout','-out',caKey]);await chmod(caKey,0o600);
 await exec('openssl',['req','-new','-x509','-sha256','-days','1','-key',caKey,'-out',caPath,'-subj','/CN=Disposable Rufren recovery fixture CA','-addext','basicConstraints=critical,CA:TRUE','-addext','keyUsage=critical,keyCertSign,cRLSign']);
 await exec('openssl',['ecparam','-name','prime256v1','-genkey','-noout','-out',leafKey]);await chmod(leafKey,0o600);
 await exec('openssl',['req','-new','-key',leafKey,'-out',csr,'-subj','/CN=localhost']);await privateWrite(extensions,'basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=serverAuth\nsubjectKeyIdentifier=hash\nauthorityKeyIdentifier=keyid,issuer\nsubjectAltName=DNS:localhost,IP:127.0.0.1\n');
 await exec('openssl',['x509','-req','-sha256','-days','1','-in',csr,'-CA',caPath,'-CAkey',caKey,'-set_serial','0x'+randomBytes(16).toString('hex'),'-out',leaf,'-extfile',extensions]);await exec('openssl',['verify','-x509_strict','-CAfile',caPath,leaf]);ca=await readFile(caPath);
 const caddy=(await exec('which',['caddy'])).stdout.trim(),config=path.join(runDir,'Caddyfile');
 await privateWrite(config,`{\n admin off\n auto_https off\n persist_config off\n storage file_system {\n  root ${path.join(runDir,'caddy-storage')}\n }\n}\n${base} {\n bind 127.0.0.1\n tls ${leaf} ${leafKey}\n reverse_proxy 127.0.0.1:${relayPort}\n}\n`);
 await exec(caddy,['validate','--config',config,'--adapter','caddyfile']);
 await startRelay(sourceDb,token,relayPort);await start('fixture-tls',caddy,['run','--config',config,'--adapter','caddyfile'],{...cleanEnv,XDG_DATA_HOME:path.join(runDir,'caddy-data'),XDG_CONFIG_HOME:path.join(runDir,'caddy-config')});
 await waitFor(async()=>{try{return(await tlsRequest('GET','/health')).body.toString()==='rufren-relay/3';}catch{return false;}},'fixture TLS health');
 relayId=(await pull(token,null)).relay;
 const{chromium}=require(process.env.RUFREN_PLAYWRIGHT_MODULE||'playwright');browser=await chromium.launch({headless:true,...(process.env.RUFREN_CHROMIUM_EXECUTABLE?{executablePath:process.env.RUFREN_CHROMIUM_EXECUTABLE}:{})});
 context=await browser.newContext({serviceWorkers:'block'});let interceptedTeacher=0,forbidden=0;
 // Native-style trust boundary: the real app's fetches use a Node TLS client with
 // this fixture CA and hostname verification, never an ignore-certificate flag.
 const forwardRoute=async route=>{const request=route.request(),url=new URL(request.url());
  if(url.origin===new URL(preview).origin&&!/^\/(api|v1)\//.test(url.pathname))return route.continue();
  if(url.origin!==new URL(base).origin){forbidden++;return route.abort();}
  if(url.pathname.startsWith('/api/')&&request.method()==='POST'){interceptedTeacher++;return route.fulfill({status:503,contentType:'text/plain',body:'Synthetic recovery fixture: no model is executed'});}
  try{const result=await new Promise((resolve,reject)=>{const body=request.postDataBuffer();const outgoing=https.request(url,{method:request.method(),ca,rejectUnauthorized:true,headers:request.headers()},response=>{const chunks=[];response.on('data',chunk=>chunks.push(chunk));response.on('end',()=>resolve({status:response.statusCode,headers:response.headers,body:Buffer.concat(chunks)}));});outgoing.on('error',reject);outgoing.end(body||undefined);});const headers=Object.fromEntries(Object.entries(result.headers).filter(([key])=>!['transfer-encoding','connection','content-length'].includes(key)).map(([key,value])=>[key,Array.isArray(value)?value.join(', '):String(value)]));await route.fulfill({status:result.status,headers,body:result.body});}catch{await route.abort();}
 };
 await context.route('**/*',forwardRoute);

 const page=await context.newPage();await page.goto(preview);await ready(page);await state('creating-real-ui-and-synthetic-v3-evidence');
 await nav(page,'Library').click();await page.getByRole('button',{name:'New passage',exact:true}).click();await page.locator('#passage-title').fill('Recovery fixture — no personal notebook');await page.locator('#passage-russian').fill('Ёжик читает книгу.');await page.locator('#passage-meaning').fill('The hedgehog reads a book.');await page.locator('#passage-source').fill('Synthetic recovery QA only.');await page.getByRole('button',{name:'Save passage',exact:true}).click();await page.getByRole('button',{name:'Write from memory',exact:true}).click();await page.getByRole('button',{name:"I'm ready to try →",exact:true}).click();await page.locator('#practice-response').fill('Ёжик читает книгу.');await page.getByRole('button',{name:'Give me a hint',exact:true}).click();await page.getByRole('button',{name:'Compare my response →',exact:true}).click();await page.getByRole('button',{name:'I recalled it',exact:true}).click();
 let events=await exportFacts(page),writing=Object.values(events).find(e=>e.fact.kind==='attempt_recorded'),envelope={format:'rufren-library',version:1,event_version:3,created_at_ms:Date.now(),library:{events}};
 const synthetic=addSyntheticSpeechFacts(envelope,writing.fact.data.revision,writing);
 const pcm=Buffer.alloc(16000*4),wav=Buffer.alloc(44+pcm.length);wav.write('RIFF',0);wav.writeUInt32LE(wav.length-8,4);wav.write('WAVEfmt ',8);wav.writeUInt32LE(16,16);wav.writeUInt16LE(3,20);wav.writeUInt16LE(1,22);wav.writeUInt32LE(16000,24);wav.writeUInt32LE(64000,28);wav.writeUInt16LE(4,32);wav.writeUInt16LE(32,34);wav.write('data',36);wav.writeUInt32LE(pcm.length,40);pcm.copy(wav,44);const audioHash=hash(wav);
 const transcription=events[synthetic.transcription].fact.data,attachment=events[transcription.recording].fact.data;attachment.asset.sha256=audioHash;attachment.note='Synthetic silence WAV; no learner speech/model inference.';transcription.audio_sha256=audioHash;const audit=JSON.parse(transcription.audit_json);audit.audio_sha256=audioHash;audit.source_pcm_sha256=hash(pcm);audit.derivative.pcm_sha256=hash(pcm);transcription.audit_json=JSON.stringify(audit);
 await page.locator('#backup-json').fill(JSON.stringify(envelope));await page.getByRole('button',{name:'Restore JSON',exact:true}).click();await page.getByText('Backup merged into your notebook.',{exact:true}).waitFor();await settings(page);await sync(page);assert((await page.locator('.notice').innerText()).includes('up to date'));
 const pending=path.join(runDir,'pending.sqlite');let pendingError='';try{await tool('backup',sourceDb,pending);}catch(error){pendingError=error.stderr;}assert(pendingError.includes('have not completed recording sync'));assert(!existsSync(pending+'.json'));receipts.push('fact-first sync is reported as pending audio, not accepted as complete backup');
 const metadata={asset_id:attachment.asset_id,content_hash:audioHash,role:'attempt',name:'Synthetic silence original',passage_id:events[writing.fact.data.revision].fact.data.passage.id,revision_id:writing.fact.data.revision,duration_ms:1000,sample_rate:16000,channels:1,frames:16000,byte_length:wav.length,created_at_ms:1,mime_type:'audio/wav',interrupted:false,device:{fixture:true},annotations:[{start_seconds:0.01,end_seconds:0.05,note:'Synthetic timed cue',created_at_ms:2}]};
 const bridge=await readFile(path.join(root,'crates/rufren-app/src/asset_sync.js'),'utf8');const storage=async request=>{const result=await page.evaluate(async({bridge,request})=>new(Object.getPrototypeOf(async function(){}).constructor)('request',bridge)(request),{bridge,request});if(result.error)throw Error(result.error);return result.value;};
 const records=[metadata,{...metadata,asset_id:'recovery-reference',role:'reference',annotations:[]},{...metadata,asset_id:'recovery-other-decode',role:'reference',sample_rate:48000,frames:48000,annotations:[]}];
 for(const record of records){await storage({op:'begin',record});await storage({op:'chunk',id:record.asset_id,offset:0,base64:wav.toString('base64')});await storage({op:'finish',id:record.asset_id});if(record!==metadata){const e={version:3,id:randomUUID(),device:writing.device,recorded_at_ms:Date.now(),fact:{kind:'recording_attached',data:{asset_id:record.asset_id,asset:{sha256:audioHash,media_type:'audio/wav',sample_rate_hz:record.sample_rate,channels:1,frames:record.frames},target:{target:'revision',id:record.revision_id},role:'reference',note:'Distinct synthetic take identity shares bytes only'}}};events[e.id]=e;}}
 await page.locator('#backup-json').fill(JSON.stringify(envelope));await page.getByRole('button',{name:'Restore JSON',exact:true}).click();await page.locator('#sync-recordings').check();await sync(page);assert((await page.locator('.notice').innerText()).includes('Notebook synced'));
 // Persist a real generated-question context through the app, without a model call.
 const teacher=JSON.parse(await readFile(path.join(root,'scripts/platform/fixtures/shell-update-teacher.json')));teacher.provenance={...teacher.provenance,provider:'recovery-fixture',model:'synthetic-no-model',response_id:randomUUID()};teacher.content.explanation='SYNTHETIC RECOVERY TEACHER FIXTURE';teacher.content.practice=[{prompt:'Write that you read a book. Synthetic recovery question.',expected_response:'Я читаю книгу.',teaching_note:'Storage fixture, not a generated lesson.',reference_russian:'Я читаю книгу.',reference_english:'I read a book.',skill:'writing'}];
 await page.evaluate(teacher=>window.rufrenLibrary.write('rufren.v1.meta.teacher_response',JSON.stringify(teacher)),teacher);await nav(page,'Teacher').click();await page.locator('textarea[id^="teacher-practice-"]').fill('Я читаю книгу.');await page.getByRole('button',{name:'Discuss my answer →',exact:true}).click();await waitFor(async()=>interceptedTeacher===1,'local-only intercepted teacher request');await settings(page);await sync(page);
 const seeded=await pull();for(const e of seeded.events)sent.set(e.id,e);assert(seeded.events.some(e=>e.fact.kind==='attempt_context_recorded'&&String(e.fact.data.exercise_id).startsWith('teacher:v2:')));await privateWrite(path.join(runDir,'fixture-facts.json'),JSON.stringify(seeded.events,null,2));
 const orphan=Buffer.from('synthetic excluded orphan');await privateWrite(path.join(sourceAssets,hash(orphan)+'.blob'),orphan);
 await state('online-backup-while-protocol-writes-continue');let writes=0,writerCursor=seeded.cursor;const writer=(async()=>{for(let i=0;i<50;i++){const e={version:3,id:randomUUID(),device:writing.device,recorded_at_ms:Date.now(),fact:{kind:'passage_revised',data:{passage:{id:randomUUID(),title:'Concurrent recovery fixture '+i,russian:'Тест.',translation:'Test.',source:'Synthetic concurrent writer.',coach_reviewed:false},parents:[]}}};writerCursor=(await push([e],writerCursor)).cursor;writes++;await new Promise(r=>setTimeout(r,5));}})();await waitFor(()=>writes>=2,'concurrent writer started');const before=writes;await tool('backup',sourceDb,backup);const during=writes-before;await writer;assert(during>0,'No protocol write overlapped the backup operation');
 const manifest=JSON.parse(await readFile(backup+'.json'));assert.equal(Object.keys(manifest.assets).length,1);assert.equal(manifest.assets[audioHash],wav.length);await tool('verify',backup);assert(!existsSync(path.join(backup+'.assets',hash(orphan)+'.blob')));
 const tail={version:3,id:randomUUID(),device:writing.device,recorded_at_ms:Date.now(),fact:{kind:'passage_revised',data:{passage:{id:randomUUID(),title:'Known after-snapshot fact',russian:'После снимка.',translation:'After the snapshot.',source:'Synthetic recovery boundary.',coach_reviewed:false},parents:[]}}};await push([tail],writerCursor);await sync(page);const sourceBinding=await binding(page),beforeRestore=await exportFacts(page);assert(sourceBinding.cursor>manifest.cursor);
 await state('rejecting-incomplete-and-mismatched-bundles');
 async function fault(name,mutate){const prefix=path.join(runDir,'fault-'+name+'.sqlite');await copyFile(backup,prefix);await copyFile(backup+'.json',prefix+'.json');await cp(backup+'.assets',prefix+'.assets',{recursive:true});await mutate(prefix);await mustRefuse(prefix,path.join(runDir,'refused-'+name+'.sqlite'),name);}
 await fault('missing-manifest',p=>unlink(p+'.json'));await fault('wrong-identity-claim',async p=>{const m=JSON.parse(await readFile(p+'.json'));m.relay=randomUUID();await writeFile(p+'.json',JSON.stringify(m));});await fault('truncated-manifest',p=>writeFile(p+'.json','{"format":'));
 await fault('altered-original',async p=>{const changed=Buffer.from(wav);changed[changed.length-1]^=1;await writeFile(path.join(p+'.assets',audioHash+'.blob'),changed);});await fault('mismatched-sqlite',async p=>{const bytes=await readFile(p);await writeFile(p,Buffer.concat([bytes,Buffer.from('corruption')]));});
 await tool('restore',backup,restored);const reader=await exec(binary,['--check-db',restored],{cwd:runDir,env:cleanEnv});await privateWrite(path.join(runDir,'typed-reader.txt'),reader.stdout);assert(reader.stdout.includes('Verified '+manifest.event_count+' immutable events'));
 await state('cutover-disposable-restore-and-recover-ahead-client');await stop('fixture-relay');await startRelay(restored,restoreToken,relayPort);await waitFor(async()=>{try{return(await tlsRequest('GET','/health',undefined,restoreToken)).status===200;}catch{return false;}},'restored relay');
 const snapshot=await pull(restoreToken,relayId);assert.equal(snapshot.relay,relayId);assert.equal(snapshot.cursor,manifest.cursor);assert.equal(snapshot.events.length,manifest.event_count);for(const e of snapshot.events)assert.deepEqual(e,sent.get(e.id));assert(!snapshot.events.some(e=>e.id===tail.id));
 const remoteRecords=JSON.parse((await tlsRequest('GET','/v1/assets',undefined,restoreToken)).body).records;assert.equal(remoteRecords.length,records.length);for(const record of remoteRecords){assert.deepEqual(record,records.find(r=>r.asset_id===record.asset_id));const original=await tlsRequest('GET','/v1/assets/blobs/'+record.content_hash,undefined,restoreToken);assert.equal(original.status,200);assert.equal(hash(original.body),audioHash);assert(original.body.equals(wav));}
 await sync(page);assert((await page.locator('.notice').innerText()).includes('401'));assert.deepEqual(await binding(page),sourceBinding);await secretInput(page,restoreToken);await sync(page);assert((await page.locator('.notice').innerText()).includes('cursor is ahead'));assert.deepEqual(await binding(page),sourceBinding);assert.deepEqual(await exportFacts(page),beforeRestore);
 await page.getByRole('button',{name:'Disconnect',exact:true}).click();await secretInput(page,restoreToken);await sync(page);assert((await page.locator('.notice').innerText()).includes('Notebook synced'));const reseeded=await pull(restoreToken,relayId);assert.equal(reseeded.relay,relayId);assert.deepEqual(new Map(reseeded.events.map(e=>[e.id,e])),new Map(Object.values(beforeRestore).map(e=>[e.id,e])));
 await state('fresh-client-restores-v3-graph-originals-and-associations');
 const fresh=await context.newPage();await fresh.goto(preview);await ready(fresh); // Same library, independent workspace; use a separate IDB profile below for actual empty-client acceptance.
 await fresh.close();const second=await browser.newContext({serviceWorkers:'block'});await second.route('**/*',forwardRoute);
 const empty=await second.newPage();await empty.goto(preview);await ready(empty);await settings(empty,restoreToken);await empty.locator('#sync-recordings').check();await sync(empty);const after=await exportFacts(empty);assert.deepEqual(after,beforeRestore);const localAssets=await assetsInBrowser(empty);assert.equal(localAssets.length,records.length);for(const saved of localAssets){assert.equal(saved.sha,audioHash);assert(Buffer.from(saved.bytes).equals(wav));assert.deepEqual(saved.metadata,records.find(r=>r.asset_id===saved.metadata.asset_id));}
 const decoded=await empty.evaluate(async()=>{const db=await new Promise((resolve,reject)=>{const r=indexedDB.open('rufren-audio-v1',1);r.onsuccess=()=>resolve(r.result);r.onerror=()=>reject(r.error);});const record=await new Promise(resolve=>{const r=db.transaction('assets','readonly').objectStore('assets').getAll();r.onsuccess=()=>resolve(r.result[0]);});db.close();const audio=new AudioContext();try{const result=await audio.decodeAudioData(await record.blob.arrayBuffer());return{seconds:result.duration,channels:result.numberOfChannels};}finally{await audio.close();}});assert.equal(decoded.seconds,1);assert.equal(decoded.channels,1);
 await second.close();await state('refusing-different-library-without-losing-pending-work');
 await nav(page,'Library').click();await page.getByRole('button',{name:'New passage',exact:true}).click();await page.locator('#passage-title').fill('Offline pending recovery guard');await page.locator('#passage-russian').fill('Новая запись.');await page.locator('#passage-meaning').fill('New entry.');await page.getByRole('button',{name:'Save passage',exact:true}).click();const withPending=await exportFacts(page),oldBinding=await binding(page);
 await stop('fixture-relay');await startRelay(path.join(runDir,'wrong-library.sqlite'),restoreToken,relayPort);await waitFor(async()=>{try{return(await tlsRequest('GET','/health',undefined,restoreToken)).status===200;}catch{return false;}},'different fixture library');await sync(page);assert((await page.locator('.notice').innerText()).includes('different sync library'));assert.deepEqual(await binding(page),oldBinding);assert.deepEqual(await exportFacts(page),withPending);const wrong=await pull(restoreToken,null);assert.notEqual(wrong.relay,relayId);assert.equal(wrong.events.length,0);assert.equal(forbidden,0);
 const result={passed:true,protocol:3,checks:['actual Dioxus v3 Teacher question/context and presentations','synthetic ASR/confirmation/feedback bindings preserved','strict private TLS client boundary','online snapshot during acknowledged protocol writes','only snapshot-referenced originals copied','distinct take IDs/decoded observations/timed cues preserved','incomplete/mismatched restore refusals','actual Rust graph reader','same library identity and rolled-back cursor','authentication rotation separate from data snapshot','ahead client keeps facts/cursor then explicitly reseeds','fresh app client downloads exact playable originals','wrong library refuses pending upload'],snapshot_events:manifest.event_count,restored_and_reseeded_events:reseeded.events.length,original_hashes:Object.keys(manifest.assets).length,take_records:records.length,acknowledged_writes_during_backup:during,typed_reader:reader.stdout.trim(),intercepted_synthetic_teacher_requests:interceptedTeacher,inference_calls:0,fixture_directory:runDir};await privateWrite(path.join(runDir,'result.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result));
}catch(error){await writeFile(path.join(runDir,'failure.json'),JSON.stringify({stage,error:redact(error.message)},null,2),{mode:0o600});console.error(redact(stage+': '+error.message));process.exitCode=1;}
finally{if(context)await context.close();if(browser)await browser.close();for(const label of ['fixture-relay','fixture-tls'])try{await stop(label);}catch(error){console.error(redact(error.message));process.exitCode=1;}}
