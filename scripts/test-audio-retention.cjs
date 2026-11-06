// Small Node-only lifecycle regression; no WebView, browser, microphone or model.
// Run: node --max-old-space-size=64 scripts/test-audio-retention.cjs
const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const source=fs.readFileSync('crates/rufren-app/src/audio_bridge.js','utf8');
const between=(start,end)=>source.slice(source.indexOf(start),source.indexOf(end,source.indexOf(start)));
const helpers=between('function pruneClips(){','// One persistent media element');
const save=between('async function saveAudio(','function stopIndependentPlayers');
const stop=between('function stopPlayback() {','function selection(');
const destroy=between('function destroy() {','const observer =');
const receiver=source.slice(source.indexOf('async function receiveMessages(){'));
const steps=[];
(async()=>{
 let releaseWrite;const writeGate=new Promise(resolve=>releaseWrite=resolve);
 const sandbox={assert,Promise,Map,Set,URL:{revokeObjectURL:url=>sandbox.revoked.push(url)},revoked:[],window:{removeEventListener(){}},document:{removeEventListener(){}},record:{asset_id:'late',blob:'original-bytes'},writes:[],writeGate};
 vm.createContext(sandbox);vm.runInContext(`
 let alive=true,playing=[],playGeneration=0,requestGeneration=0,starting=false,capture=null,exitPromise,regionDrag={};
 const instanceId='old',clips=new Map(),selected={reference:null,attempt:null},analyses={},regionSlots={region_a:{},region_b:{}},focusTimers=new Map(),pendingWrites=new Set(),yieldQueue=[];let yieldChannel;
 const stretchPlayer={src:'',pause(){},removeAttribute(){this.src=''},load(){},remove(){}};
 let finish=()=>{},db={close(){}},context={close:()=>Promise.resolve()},root={removeEventListener(){}},observer={disconnect(){}};
 let stopped=false,focusSnapshot;
 function releaseCapture(){stopped=true;}
 async function preserveExitTake(take){if(!take)return;await writeGate;writes.push({asset_id:'partial',frames:take.frames,interrupted:true});}
 async function shutdownRegions(){focusSnapshot=Object.keys(selected);}
 async function makeRecord(){await writeGate;return record;}
 async function storage(op,value){if(op==='put')writes.push(value);}
 const click=()=>{},change=()=>{},assetsChanged=()=>{},visibility=()=>{};
 const emit=()=>{},metadata=r=>r,refreshLibrary=async()=>{},selectRecord=async()=>{},status=()=>{},controls=()=>{};
 ${helpers}\n${save}\n${stop}\n${destroy}
 globalThis.api={clips,selected,analyses,regionSlots,focusTimers,stretchPlayer,pruneClips,releaseStudioMemory,saveAudio,destroy,
 setPlaying:value=>playing=value,setAlive:value=>alive=value,setCapture:value=>capture=value,stopped:()=>stopped,focusSnapshot:()=>focusSnapshot};
 `,sandbox);
 const a=sandbox.api;
 for(let i=0;i<2000;i++){const id=`take-${i}`;a.selected.attempt={asset_id:id};a.clips.set(id,{buffer:new Float32Array(256),playback_url:`blob:${id}`});a.pruneClips();assert.equal(a.clips.size,1);}
 assert.equal(sandbox.revoked.length,1999);
 a.setPlaying([{assetId:'take-1999',stop(){}}]);a.selected.attempt={asset_id:'next'};a.clips.set('next',{buffer:new Float32Array(256)});a.pruneClips();assert.equal(a.clips.size,2,'sounding old take remains pinned');
 a.setPlaying([]);a.pruneClips();assert.equal(a.clips.size,1);assert.equal(sandbox.revoked.at(-1),'blob:take-1999');
 for(const role of ['reference','region_a','region_b']){a.selected[role]={asset_id:role};a.clips.set(role,{buffer:new Float32Array(256)});}a.pruneClips();assert.equal(a.clips.size,4);
 steps.push('2000 source switches retain only selected decoded buffers; active playback is pinned until stopped and evicted Blob URLs are revoked');
 a.analyses.attempt={large:'analysis'};a.regionSlots.region_a.record={blob:'bytes'};a.regionSlots.region_a.analysis={large:'region-analysis'};a.focusTimers.set('a',1);sandbox.window.rufrenAudioStudio={instanceId:'old'};a.setCapture({frames:512});
 const saveLate=a.saveAudio('attempt','original-bytes',{},'take');const exited=a.destroy();assert(a.stopped(),'tracks stop before asynchronous persistence');assert.equal(a.clips.size,0);assert.equal(Object.keys(a.selected).length,0);assert.equal(Object.keys(a.analyses).length,0);assert.equal(a.regionSlots.region_a.record,null);assert.equal(a.regionSlots.region_a.analysis,null);assert.equal(a.focusTimers.size,0);assert.equal(sandbox.window.rufrenAudioStudio,undefined);assert.equal(sandbox.window.rufrenAudioExit,exited,'rapid remount can await unfinished writes without retaining the adapter API');assert(a.focusSnapshot().includes('region_a'),'focus snapshots precede memory release');
 const newer={instanceId:'new'};sandbox.window.rufrenAudioStudio=newer;a.releaseStudioMemory();assert.equal(sandbox.window.rufrenAudioStudio,newer,'old cleanup cannot drop new adapter');
 releaseWrite();await Promise.all([exited,saveLate]);assert.equal(sandbox.window.rufrenAudioExit,undefined);assert.equal(a.clips.size,0,'late save cannot repopulate destroyed cache');assert(sandbox.writes.some(v=>v.asset_id==='partial'&&v.frames===512&&v.interrupted));assert(sandbox.writes.some(v=>v.asset_id==='late'&&v.blob==='original-bytes'));
 steps.push('Destroy drops global owner/cache/analysis after focus snapshots; tracks stop immediately while interrupted and already-pending originals still finish persistence; late saves cannot rehydrate cache');
 let subscriptions=0,end,late,waiting,receives=0;const pending=new Promise(resolve=>waiting=resolve);
 const context={alive:true,dioxus:{recv(){receives++;if(receives<=10000)return Promise.resolve({kind:'noop'});waiting();return new Promise(resolve=>late=resolve);}},lifetime:{then(resolve){subscriptions++;end=resolve;}},currentAnalysis(){throw Error('late message reached destroyed view');}};vm.createContext(context);
 const completed=vm.runInContext(`(async()=>{${receiver}})()`,context);await pending;assert.equal(subscriptions,1);assert.equal(receives,10001);context.alive=false;end();await completed;late({kind:'analysis',request_id:'late'});await Promise.resolve();await Promise.resolve();assert.equal(subscriptions,1);assert.equal(receives,10001);
 steps.push('10000 bridge messages attach exactly one lifetime subscription; destroy resolves an idle receiver and a late message cannot touch the closed view');
 const memory=process.resourceUsage().maxRSS*1024;assert(memory<128*1024*1024,`Node test exceeded 128MiB peak RSS: ${memory}`);
 const result={ok:true,node_only:true,peak_rss_bytes:memory,steps};fs.mkdirSync('.local/audio-retention-qa',{recursive:true});fs.writeFileSync('.local/audio-retention-qa/results.json',JSON.stringify(result,null,2));console.log(JSON.stringify(result,null,2));
})().catch(error=>{console.error(error);process.exitCode=1});
