#!/usr/bin/env node
// Actual authenticated relay + two isolated browser devices. Uses synthetic bytes only.
import {createRequire} from 'node:module';
import {randomBytes,createHash} from 'node:crypto';
import {spawn,execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {readFile,writeFile,mkdir,mkdtemp,unlink} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
const require=createRequire(import.meta.url),exec=promisify(execFile);
const {chromium}=require(process.env.RUFREN_PLAYWRIGHT_MODULE||'playwright');
const root=dirname(dirname(fileURLToPath(import.meta.url)));
const folder=await mkdtemp(join(tmpdir(),'rufren-audio-relay-'));
const logdir=join(root,'.local');await mkdir(logdir,{recursive:true});
const pidfile=join(logdir,'storage-audio-relay.pid'),logfile=join(logdir,'storage-audio-relay.log');
const binary=join(root,'target/debug/rufren-relay'),token=randomBytes(32).toString('base64url');
let child,base,logs='';
async function start(address='127.0.0.1:0',database=join(folder,'relay.sqlite')){
  child=spawn(binary,[],{cwd:root,env:{...process.env,RUFREN_RELAY_BIND:address,RUFREN_RELAY_DB:database,RUFREN_RELAY_ASSETS:database+'.assets',RUFREN_RELAY_ASSET_QUOTA_BYTES:'12000',RUFREN_SYNC_TOKEN:token,RUFREN_TUTOR_PROVIDER:'disabled',RUFREN_ALLOWED_ORIGINS:'http://127.0.0.1:8080'}});
  await writeFile(pidfile,String(child.pid));
  const url=await new Promise((resolve,reject)=>{
    const timer=setTimeout(()=>reject(Error('Disposable relay did not start')),10000);
    child.stdout.on('data',chunk=>{logs+=chunk;const match=String(chunk).match(/Rufren sync listening on ([^\s]+)/);if(match){clearTimeout(timer);resolve('http://'+match[1]);}});
    child.stderr.on('data',chunk=>logs+=chunk);
    child.once('exit',code=>{clearTimeout(timer);reject(Error('Disposable relay exited '+code));});
  });
  base=url;return url;
}
async function stop(){
  if(!child||child.exitCode!==null)return;
  const running=child;
  const {stdout}=await exec('ps',['-p',String(running.pid),'-o','command=']);
  if(!stdout.includes(binary))throw Error('Disposable relay PID changed; refusing to stop it');
  const exited=new Promise(resolve=>running.once('exit',resolve));running.kill('SIGINT');
  await Promise.race([exited,new Promise((_,reject)=>setTimeout(()=>reject(Error('Disposable relay did not stop gracefully')),5000))]);
  await writeFile(logfile,logs);await unlink(pidfile);child=null;
}
let browser;
try{
  await start();
  browser=await chromium.launch({headless:true,...(process.env.RUFREN_CHROMIUM_EXECUTABLE?{executablePath:process.env.RUFREN_CHROMIUM_EXECUTABLE}:{})});
  const contexts=await Promise.all([browser.newContext(),browser.newContext()]);
  for(const context of contexts){
    // Intercepted navigation has no network address; Chromium classifies it as public.
    // Grant loopback access only inside these disposable contexts, then test real CORS.
    await context.grantPermissions(['local-network-access'],{origin:'http://127.0.0.1:8080'});
    await context.route('http://127.0.0.1:8080/storage-sync-test',route=>route.fulfill({contentType:'text/html',body:'<!doctype html><title>Isolated recording sync device</title>'}));
  }
  const [a,b]=await Promise.all(contexts.map(context=>context.newPage()));
  if(process.env.RUFREN_TEST_DEBUG){for(const page of [a,b]){page.on('console',message=>{if(message.type()==='error')console.error('Browser:',message.text());});page.on('requestfailed',request=>console.error('Network:',request.url(),request.failure()));}}
  await Promise.all([a.goto('http://127.0.0.1:8080/storage-sync-test'),b.goto('http://127.0.0.1:8080/storage-sync-test')]);
  const source=await readFile(join(root,'crates/rufren-app/src/asset_sync.js'),'utf8');
  const call=async(page,request)=>{const result=await page.evaluate(async({source,request})=>await new(Object.getPrototypeOf(async function(){}).constructor)('request',source)(request),{source,request});if(result.error)throw Error(result.error);return result.value;};
  const request=async(method,path,body)=>{const response=await fetch(base+path,{method,headers:{Authorization:'Bearer '+token,'X-Rufren-Relay':relay,...(body?{'Content-Type':'application/json'}:{})},body:body?JSON.stringify(body):undefined});if(!response.ok)throw Error('Actual relay returned '+response.status+': '+await response.text());return response;};
  const first=await fetch(base+'/v1/sync',{method:'POST',headers:{Authorization:'Bearer '+token,'Content-Type':'application/json'},body:JSON.stringify({version:2,relay:null,after:0,events:[]})});const relay=(await first.json()).relay;if(!relay)throw Error('Fact sync did not establish relay identity');
  const bytes=Buffer.alloc(44+4800*2);bytes.write('RIFF',0);bytes.writeUInt32LE(bytes.length-8,4);bytes.write('WAVEfmt ',8);bytes.writeUInt32LE(16,16);bytes.writeUInt16LE(1,20);bytes.writeUInt16LE(1,22);bytes.writeUInt32LE(48000,24);bytes.writeUInt32LE(96000,28);bytes.writeUInt16LE(2,32);bytes.writeUInt16LE(16,34);bytes.write('data',36);bytes.writeUInt32LE(9600,40);for(let frame=0;frame<4800;frame++)bytes.writeInt16LE(Math.round(Math.sin(frame*2*Math.PI*220/48000)*2048),44+frame*2);
  const hash=createHash('sha256').update(bytes).digest('hex');
  const record={asset_id:'take-a',content_hash:hash,role:'attempt',name:'Synthetic cross-device fixture',passage_id:'',revision_id:'',duration_ms:100,sample_rate:48000,channels:1,frames:4800,byte_length:bytes.length,created_at_ms:1,mime_type:'audio/wav',interrupted:false,device:{},annotations:[]};
  await call(a,{op:'begin',record});await call(a,{op:'chunk',id:record.asset_id,offset:0,base64:bytes.toString('base64')});await call(a,{op:'finish',id:record.asset_id});
  const wire=async(page,method,path,extra={})=>call(page,{op:'http',url:base+path,method,token,relay,...extra});
  if((await wire(a,'HEAD','/v1/assets/blobs/'+hash)).status!==404)throw Error('Fresh relay unexpectedly had original');
  if((await wire(a,'PUT','/v1/assets/blobs/'+hash,{upload_id:'take-a'})).status!==204)throw Error('Original upload failed');
  if((await wire(a,'PUT','/v1/assets/records/take-a',{body:record})).status!==200)throw Error('Take metadata upload failed');
  const remote=(await request('GET','/v1/assets').then(r=>r.json())).records[0];
  await call(b,{op:'download',url:base+'/v1/assets/blobs/'+hash,method:'GET',token,relay,record:remote});
  if(!await call(b,{op:'has',id:'take-a',hash}))throw Error('Second device has no verified original');
  const encoded=await call(b,{op:'read',id:'take-a',offset:0,length:256*1024});
  const decoded=await b.evaluate(async encoded=>{const context=new AudioContext();try{const bytes=Uint8Array.from(atob(encoded),character=>character.charCodeAt(0));const audio=await context.decodeAudioData(bytes.buffer);return {duration:audio.duration,channels:audio.numberOfChannels};}finally{await context.close();}},encoded);
  if(Math.abs(decoded.duration-.1)>.001||decoded.channels!==1)throw Error('Downloaded original is not playable valid WAV');
  const cueA={start_seconds:.01,end_seconds:.04,note:'Device A',created_at_ms:2},cueB={start_seconds:.04,end_seconds:.08,note:'Device B',created_at_ms:3};
  await call(b,{op:'merge',record:{...record,annotations:[cueB]}});await wire(b,'PUT','/v1/assets/records/take-a',{body:{...record,annotations:[cueB]}});
  const merged=JSON.parse((await wire(a,'PUT','/v1/assets/records/take-a',{body:{...record,annotations:[cueA]}})).body);if(merged.annotations.length!==2)throw Error('Actual relay lost concurrent cues');await call(a,{op:'merge',record:merged});await call(b,{op:'merge',record:merged});
  const distinct={...record,asset_id:'take-b',sample_rate:44100,frames:4410};await request('PUT','/v1/assets/records/take-b',distinct);
  await call(b,{op:'download',url:base+'/v1/assets/blobs/'+hash,method:'GET',token,relay,record:distinct});
  if((await call(b,{op:'inventory',after:''})).records.length!==2)throw Error('Distinct takes collapsed');
  const more=Buffer.alloc(4000,7),moreHash=createHash('sha256').update(more).digest('hex');
  const quota=await fetch(base+'/v1/assets/blobs/'+moreHash,{method:'PUT',headers:{Authorization:'Bearer '+token,'X-Rufren-Relay':relay},body:more});if(quota.status!==507)throw Error('Actual relay quota did not reject excess original');
  const backup=join(folder,'backup.sqlite'),restored=join(folder,'restored.sqlite');
  await exec('python3',[join(root,'scripts/relay-backup.py'),'backup',join(folder,'relay.sqlite'),backup]);
  await exec('python3',[join(root,'scripts/relay-backup.py'),'restore',backup,restored]);
  await exec(binary,['--check-db',restored]);
  const previous=base;await stop();await start(new URL(previous).host,restored);
  const durable=(await request('GET','/v1/assets').then(r=>r.json())).records;if(durable.length!==2||durable[0].annotations.length!==2)throw Error('Restored relay lost take identities or cues');
  const original=Buffer.from(await request('GET','/v1/assets/blobs/'+hash).then(r=>r.arrayBuffer()));if(!original.equals(bytes))throw Error('Restored relay lost exact original bytes');
  await Promise.all([a.reload(),b.reload()]);if((await call(b,{op:'inventory',after:''})).records.length!==2)throw Error('Second-device audio did not survive reload');
  console.log(JSON.stringify({passed:['actual authenticated cross-origin upload/download','two isolated device stores','exact playable WAV original','concurrent cue union','distinct same-byte decoding observations','quota rejection preserves originals','online SQLite+audio backup','verified restore and relay restart','device reload recovery'],fixture_directory:folder}));
}finally{if(browser)await browser.close();await stop();}
