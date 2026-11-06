// The renderer adapter owns device APIs and binary persistence; Rust owns analysis.
// Bound to this mounted studio only. Explicit recording sync owns networking; no localStorage audio.
const root = typeof rufrenStudioRoot==='undefined'?document.getElementById('rufren-audio-studio'):rufrenStudioRoot;
if (!root) throw new Error('Speech studio did not mount.');
const instanceId=root.dataset.instance||crypto.randomUUID();root.dataset.instance=instanceId;
const previousExit = window.rufrenAudioStudio?.destroy();
const config = JSON.parse(root.dataset.config);
let compactAdvanced=!config.compact,choiceGeneration=0,pendingOperations=0;
let alive = true, db, context, stream, processor, source, sink, capture, starting = false, finish;
const lifetime = new Promise(resolve => finish = resolve);
const yieldQueue = [], yieldChannel = typeof MessageChannel === 'function' ? new MessageChannel() : null;
if(yieldChannel) yieldChannel.port1.onmessage = () => yieldQueue.shift()?.();
function yieldNow() {
  if(window.scheduler?.yield) return window.scheduler.yield();
  if(!yieldChannel) return new Promise(resolve=>setTimeout(resolve,0));
  return new Promise(resolve=>{yieldQueue.push(resolve);yieldChannel.port2.postMessage(0);});
}
const pendingWrites = new Set();
let exitPromise;
function keepWrite(promise){pendingWrites.add(promise);promise.then(()=>pendingWrites.delete(promise),()=>pendingWrites.delete(promise));return promise;}
let requestGeneration = 0, timer, workletUrl, workletLoaded = false, playing = [], playGeneration = 0;
const clips = new Map(), selected = {reference: null, attempt: null};
// One persistent media element retains Apple's user activation across an A/B pair.
const stretchPlayer=document.createElement('audio');stretchPlayer.preload='auto';stretchPlayer.hidden=true;stretchPlayer.setAttribute('data-studio-stretch','true');root.append(stretchPlayer);
const preservePitchKey=['preservesPitch','webkitPreservesPitch','mozPreservesPitch'].find(key=>key in stretchPlayer);
if(preservePitchKey)stretchPlayer[preservePitchKey]=true;
const $ = id => root.querySelector(`#${id}`);
const emit = value => { if (alive) dioxus.send(value); };
function status(message, error = false) {
  if (!alive) return;
  const node = $('audio-status'); node.textContent = message; node.dataset.error = String(error);
}
function controls() {
  root.querySelectorAll('[data-record]').forEach(b => { b.disabled = Boolean(capture || starting || pendingOperations); });
  $('audio-stop-recording').disabled = !(capture || starting);
  root.querySelectorAll('[data-needs]').forEach(b => { b.disabled = !selected[b.dataset.needs] || !!capture || starting || !!pendingOperations; });
  $('audio-compare').disabled = !selected.reference || !selected.attempt || !!capture || starting || !!pendingOperations;
  root.querySelectorAll('input[type=file], select[data-library]').forEach(n => { n.disabled = !!capture || starting || !!pendingOperations; });
  regionControls();
}
function beginOperation(){pendingOperations++;emit({kind:'recording_state',active:true});controls();}
function endOperation(){pendingOperations=Math.max(0,pendingOperations-1);emit({kind:'recording_state',active:!!(capture||starting||pendingOperations)});controls();}
function mic(active, message = '') {
  $('audio-mic-status').textContent = active ? `● Microphone active ${message}` : (starting ? 'Requesting microphone…' : 'Microphone off');
  $('audio-mic-status').dataset.active = String(active); controls();emit({kind:'recording_state',active:!!(capture||starting||pendingOperations)});
}
function failure(error) {
  const descriptions = {
    NotAllowedError: 'Microphone access was denied. Allow it in this site or app’s privacy settings, then try again.',
    NotFoundError: 'No microphone was found. Connect one or import an audio file.',
    NotReadableError: 'The microphone could not start. Check another app is not using it, then retry.',
    SecurityError: 'This WebView has not enabled microphone access. Import a recording, or use a secure browser.',
    QuotaExceededError: 'This device’s audio storage is full. Download your recording before leaving this page.'
  };
  status(descriptions[error?.name] || error?.message || String(error), true); controls();
}
async function audioContext(resume = true) {
  const Audio = window.AudioContext || window.webkitAudioContext;
  if (!Audio) throw Error('This browser or WebView does not support Web Audio. Open Rufren in a current browser.');
  if (!context || context.state === 'closed') {
    context = new Audio();
    context.addEventListener('statechange', () => {
      if (capture && context.state !== 'running') stopRecording('Recording interrupted by the device; the captured part was kept.').catch(failure);
    });
  }
  if(resume)await context.resume();
  if (resume && context.state !== 'running') throw Error('Audio is paused by the device. Tap the control again to resume.');
  return context;
}
function openDatabase() {
  return new Promise((resolve, reject) => {
    if (!window.indexedDB) return reject(Error('Durable audio storage is unavailable in this WebView. Import and playback remain available, but recordings cannot be saved here.'));
    const req = indexedDB.open('rufren-audio-v1', 1);
    req.onupgradeneeded = () => req.result.createObjectStore('assets', {keyPath:'asset_id'});
    req.onsuccess = () => { if (!alive) { req.result.close(); reject(Error('Audio studio was closed.')); return; } db = req.result; db.onversionchange = () => db.close(); resolve(db); };
    req.onerror = () => reject(req.error);
    req.onblocked = () => reject(Error('Close another Rufren audio tab to open audio storage.'));
  });
}
async function storage(operation, value) {
  if (!db) await openDatabase();
  return new Promise((resolve, reject) => {
    const tx = db.transaction('assets', operation === 'put' ? 'readwrite' : 'readonly');
    const store = tx.objectStore('assets');
    const req = operation === 'put' ? store.put(value) : operation === 'get' ? store.get(value) : store.getAll();
    let result; req.onsuccess = () => { result = req.result; };
    tx.oncomplete = () => resolve(result); tx.onerror = () => reject(tx.error); tx.onabort = () => reject(tx.error || Error('Audio storage was interrupted.'));
  });
}
async function digest(blob) {
  if (!crypto.subtle) throw Error('Audio integrity checking needs a secure browser context. Use HTTPS or localhost.');
  const hash = await crypto.subtle.digest('SHA-256', await blob.arrayBuffer());
  return Array.from(new Uint8Array(hash), x => x.toString(16).padStart(2,'0')).join('');
}
function metadata(record) { const {blob, ...rest} = record; return rest; }
function sourceLabel(record) {
  const p=record.device?.provenance;
  if(p?.kind==='ai_generated_speech')return `AI speech · ${p.provider} · ${p.voice}`;
  if(p?.kind==='system_generated_speech')return `System speech · ${p.voice}`;
  return record.role==='reference'?'Provided voice':'Your recorded audio';
}
async function refreshLibrary() {
  const records = await storage('all');
  for(const record of records)if(Object.hasOwn(record,'original_mime_type')){record.device={...record.device,source_mime_type:record.original_mime_type};delete record.original_mime_type;await storage('put',record);}
  if (!alive) return;
  for (const role of ['reference','attempt']) {
    const picker = $(`audio-library-${role}`); picker.replaceChildren(new Option('Choose a saved recording…', ''));
    records.filter(r => r.role === role && r.passage_id === config.passage_id && r.revision_id === config.revision_id)
      .sort((a,b) => b.created_at_ms - a.created_at_ms).forEach(r => {
        const label = `${sourceLabel(r)} · ${r.name} · ${(r.duration_ms/1000).toFixed(1)}s · ${new Date(r.created_at_ms).toLocaleString()}`;
        picker.add(new Option(label, r.asset_id));
      });
    picker.value = selected[role]?.asset_id || '';
  }
  regionSourceList(records);
}
// Float WAV keeps the actual captured mono PCM. Imported originals remain untouched.
function wav(samples, sampleRate) {
  const bytes = new ArrayBuffer(44 + samples.length * 4), view = new DataView(bytes);
  const text = (offset, s) => [...s].forEach((c,i) => view.setUint8(offset+i,c.charCodeAt(0)));
  text(0,'RIFF'); view.setUint32(4,36+samples.length*4,true); text(8,'WAVE'); text(12,'fmt ');
  view.setUint32(16,16,true); view.setUint16(20,3,true); view.setUint16(22,1,true);
  view.setUint32(24,sampleRate,true); view.setUint32(28,sampleRate*4,true); view.setUint16(32,4,true); view.setUint16(34,32,true);
  text(36,'data'); view.setUint32(40,samples.length*4,true);
  for (let i=0;i<samples.length;i++) view.setFloat32(44+i*4,samples[i],true);
  return new Blob([bytes],{type:'audio/wav'});
}
function analysisPcm(buffer) {
  // Send compact original decoded PCM, not pre-filtered samples: quality checks
  // must still see full-scale peaks and differences between original channels.
  const count=buffer.length*buffer.numberOfChannels;
  if(count>16000000) throw Error('This decoded file is too large for the instrument. The original is saved; choose a shorter phrase for analysis.');
  const bytes=new Uint8Array(count*4), view=new DataView(bytes.buffer);
  const channels=Array.from({length:buffer.numberOfChannels},(_,c)=>buffer.getChannelData(c));
  for(let f=0;f<buffer.length;f++) for(let c=0;c<channels.length;c++) view.setFloat32((f*channels.length+c)*4,channels[c][f],true);
  let binary=''; for(let i=0;i<bytes.length;i+=16384) binary+=String.fromCharCode(...bytes.subarray(i,i+16384));
  return btoa(binary);
}

function cueList(role, record) {
  const list = $(`audio-cues-${role}`); list.replaceChildren();
  for (const cue of record.annotations || []) {
    const item = document.createElement('li');
    item.textContent = `${cue.start_seconds.toFixed(2)}–${cue.end_seconds.toFixed(2)} s · ${cue.note}`; list.append(item);
  }
}
async function selectRecord(role, record, buffer) {
  if (!alive) return false;
  const token=nextAnalysis(role,record);
  if (!buffer) {
    if (await digest(record.blob) !== record.content_hash) throw Error('This audio file failed its integrity check. The saved original was kept; restore it from your audio backup.');
    buffer = await (await audioContext(false)).decodeAudioData(await record.blob.arrayBuffer());
  }
  if(!currentAnalysis(role,record.asset_id,token))return false;
  selected[role] = record; root.setAttribute(`data-selected-${role}`,record.asset_id); clips.set(record.asset_id, {...clips.get(record.asset_id),record,buffer});
  delete analyses[role]; for(const kind of ['wave','spectrum','pitch']) {const canvas=$(`audio-${kind}-${role}`);canvas.getContext('2d').clearRect(0,0,canvas.width,canvas.height);}
  $(`audio-from-${role}`).value = '0'; $(`audio-to-${role}`).value = buffer.duration.toFixed(3);
  $(`audio-name-${role}`).textContent = `${record.name} · ${buffer.duration.toFixed(2)} s · ${record.sample_rate.toLocaleString()} Hz`;
  $(`audio-library-${role}`).value = record.asset_id;
  const provenance=$(`audio-provenance-${role}`);if(provenance)provenance.textContent=sourceLabel(record);
  cueList(role,record); controls();
  if(!compactAdvanced){status('Take ready. Replay it, record again, or choose a saved take.');return true;}
  status('Original available. Drawing sound instruments…');
  await sendPcm(role,record,buffer,token);
  return currentAnalysis(role,record.asset_id,token);
}
async function audioMime(blob) {
  const bytes=new Uint8Array(await blob.slice(0,16).arrayBuffer());
  const text=(from,to)=>String.fromCharCode(...bytes.slice(from,to));
  if(text(0,4)==='RIFF'&&text(8,12)==='WAVE')return 'audio/wav';
  if(text(0,4)==='fLaC')return 'audio/flac';
  if(text(0,4)==='OggS')return 'audio/ogg';
  if(bytes[0]===255&&(bytes[1]&246)===240)return 'audio/aac';
  if(bytes[0]===255&&(bytes[1]&224)===224&&((bytes[1]>>1)&3)!==0)return 'audio/mpeg';
  if(text(4,8)==='ftyp')return 'audio/mp4';
  const declared=blob.type.split(';')[0].trim().toLowerCase();
  if(/^audio\/[a-z0-9!#$&^_.+-]+$/.test(declared))return declared;
  // Decoding proved audio is present, but its encoding has no identified subtype.
  return 'audio/x-rufren-encoded';
}
async function makeRecord(role,blob,info,name,interrupted=false,device={}) {
  const content_hash=await digest(blob);
  const asset_id=`${content_hash}-${crypto.randomUUID?crypto.randomUUID():Date.now().toString(36)}`;
  return {asset_id,content_hash,role,name,passage_id:config.passage_id,revision_id:config.revision_id,
    duration_ms:Math.round(info.duration*1000),sample_rate:info.sampleRate,channels:info.numberOfChannels,frames:info.length,
    byte_length:blob.size,created_at_ms:Date.now(),mime_type:await audioMime(blob),
    interrupted,device:{...device,source_mime_type:blob.type},annotations:[],blob};
}
async function saveAudio(role, blob, buffer, name, interrupted = false, device = {}) {
  const record=await makeRecord(role,blob,buffer,name,interrupted,device),asset_id=record.asset_id;
  // Keep a downloadable in-memory take even when persistence fails.
  clips.set(asset_id,{record,buffer}); selected[role] = record; controls();
  try { await storage('put',record); emit({kind:'saved',recording:metadata(record)}); await refreshLibrary(); }
  catch (error) { if(!alive)(window.rufrenAudioRecovery ||= []).push(record);await selectRecord(role,record,buffer);throw error; }
  await selectRecord(role,record,buffer);
  status(interrupted ? 'The interrupted take was saved. Listen for a cutoff before using it.' : 'Original audio saved on this device. Listen, choose a span, and try one useful change.');
}
function stopIndependentPlayers(){for(const api of window.rufrenAudioPlayers?.values()||[])api.stop?.();}
function stopPlayback() {
  playGeneration++;
  for (const node of playing) { try { node.stop(); } catch (_) {} }
  playing = [];
}
function selection(role) {
  const record = selected[role]; if (!record) throw Error(`Add a ${role === 'reference' ? 'reference voice' : 'learner attempt'} first.`);
  const buffer = clips.get(record.asset_id)?.buffer; if (!buffer) throw Error('Choose the recording again.');
  const from = Number($(`audio-from-${role}`).value), to = Number($(`audio-to-${role}`).value);
  if (!Number.isFinite(from) || !Number.isFinite(to) || from < 0 || to <= from || to > buffer.duration+0.005) throw Error('Choose a span with 0 ≤ start < end ≤ the recording duration.');
  return {record,buffer,from,to:Math.min(to,buffer.duration)};
}
function playStretched(role,span,rate,loop,generation) {
  if(!preservePitchKey)throw Error('Pitch-preserving slow playback is unavailable in this browser or WebView. Use natural speed.');
  const clip=clips.get(span.record.asset_id);clip.playback_url ||= URL.createObjectURL(span.record.blob);
  const media=stretchPlayer;
  return new Promise((resolve,reject)=>{
    let done=false,tick,started=false;
    const handle={stop:()=>finish(false)};playing.push(handle);
    function finish(ok,error) {
      if(done)return;done=true;clearTimeout(tick);media.pause();
      media.onloadedmetadata=media.onplaying=media.ontimeupdate=media.onended=media.onerror=null;
      playing=playing.filter(node=>node!==handle);
      if(error)reject(error);else resolve(ok&&generation===playGeneration);
    }
    function seek(){try{media.currentTime=span.from;media.playbackRate=rate;media[preservePitchKey]=true;}catch(error){finish(false,Error(`This device could not seek to the selected audio span: ${error.message}`));}}
    function boundary(){
      clearTimeout(tick);if(done)return;
      if(!alive||generation!==playGeneration){finish(false);return;}
      if(media.currentTime>=span.to-.002){
        if(!loop){finish(true);return;}
        media.currentTime=span.from;
        if(media.paused)media.play().catch(()=>finish(false,Error('Tap Play again to resume looping.')));
      }
      const remaining=Math.max(1,(span.to-media.currentTime)/rate*1000);
      tick=setTimeout(boundary,Math.min(50,remaining));
    }
    media.onloadedmetadata=seek;
    media.onplaying=()=>{if(!started){started=true;emitPlayback(role,span,rate);}boundary();};
    media.ontimeupdate=boundary;
    media.onended=()=>{if(loop){seek();media.play().catch(()=>finish(false,Error('Tap Play again to resume looping.')));}else finish(true);};
    media.onerror=()=>finish(false,Error('The device could not play this original with pitch preservation. Try natural speed or import a WAV/MP3 reference.'));
    if(media.src!==clip.playback_url){media.src=clip.playback_url;media.load();}
    media.playbackRate=rate;media[preservePitchKey]=true;
    if(media.readyState>=1)seek();
    // Request playback synchronously from the click, before waiting for metadata.
    media.play().catch(error=>finish(false,Error(`Playback could not start. Tap Play again, or check this app’s audio permission. ${error.message}`)));
    status(`${loop?'Looping':'Playing'} ${isRegion(role)?`region ${role==='region_a'?'A':'B'}`:role==='reference'?'reference voice':'your attempt'} at ${rate}×, keeping voice pitch.`);
  });
}
async function play(role, keep = false) {
  const span=selection(role);if(!alive)return false;stopIndependentPlayers();if(!keep)stopPlayback();
  const generation=playGeneration,rate=Number($('audio-rate').value),loop=$('audio-loop').checked&&!keep;
  if(rate!==1)return playStretched(role,span,rate,loop,generation);
  const ctx=await audioContext();if(!alive||generation!==playGeneration)return false;
  const node=ctx.createBufferSource();node.buffer=span.buffer;node.playbackRate.value=1;
  node.connect(ctx.destination);node.loop=loop;node.loopStart=span.from;node.loopEnd=span.to;playing.push(node);
  status(`${loop?'Looping':'Playing'} ${role==='reference'?'reference voice':'your attempt'} at natural speed.`);
  return new Promise(resolve=>{
    node.onended=()=>{playing=playing.filter(n=>n!==node);resolve(generation===playGeneration);};
    if(loop)node.start(0,span.from);else node.start(0,span.from,span.to-span.from);
    emitPlayback(role,span,rate);
  });
}

async function compare() {
  stopPlayback(); const generation = playGeneration;
  const ok = await play('reference',true); if (!ok || generation !== playGeneration) return;
  await new Promise(resolve => setTimeout(resolve,350)); if (generation !== playGeneration || !alive) return;
  await play('attempt',true);
}
async function startRecording(role) {
  if (capture || starting) return;
  if (!crypto.subtle) throw Error('Secure audio storage needs HTTPS, localhost, or a supported native WebView. Import or record in a secure browser.');
  if (!navigator.mediaDevices?.getUserMedia) throw Error('Microphone capture is unavailable here. Use HTTPS or localhost in a browser, or import a recording. Native apps also need microphone permission in their host configuration.');
  if(role==='attempt'){choiceGeneration++;clearAttempt();}
  starting = true; const generation = ++requestGeneration; mic(false); stopPlayback();stopIndependentPlayers();
  try {
    if (!db) await openDatabase();
    if (!alive || generation !== requestGeneration) return;
    const ctxPromise = audioContext();
    // This call is made by the DOM click itself, preserving the user gesture on Apple devices.
    const streamPromise = navigator.mediaDevices.getUserMedia({audio:{channelCount:1,echoCancellation:false,noiseSuppression:false,autoGainControl:false}});
    streamPromise.then(requested => { if (!alive || generation !== requestGeneration) requested.getTracks().forEach(t => t.stop()); else { stream=requested; mic(true,'starting audio…'); } },()=>{});
    const [ctx, requested] = await Promise.all([ctxPromise,streamPromise]);
    if (!alive || generation !== requestGeneration) { requested.getTracks().forEach(t => t.stop()); return; }
    stream = requested;
    const track = stream.getAudioTracks()[0], device = track.getSettings ? track.getSettings() : {};
    capture = {role,chunks:[],frames:0,sampleRate:ctx.sampleRate,started:performance.now(),device};
    if(track.readyState==='ended')throw Error('The microphone disconnected before capture started.');
    track.addEventListener('ended', () => { if (capture) stopRecording('Microphone disconnected. The captured part was saved.').catch(failure); });
    starting = false; mic(true,'starting capture…');
    source = ctx.createMediaStreamSource(stream); sink = ctx.createGain(); sink.gain.value = 0; sink.connect(ctx.destination);
    const collect = chunk => {
      if (!capture) return;
      capture.chunks.push(new Float32Array(chunk)); capture.frames += chunk.length;
      if (capture.frames >= capture.sampleRate * 30) stopRecording('The 30-second take was saved. Short phrases are easier to compare.').catch(failure);
    };
    if (ctx.audioWorklet) {
      try {
        if (!workletLoaded) {
        workletUrl = URL.createObjectURL(new Blob([`class RufrenCapture extends AudioWorkletProcessor { process(inputs) { if (inputs[0]?.[0]) this.port.postMessage(inputs[0][0]); return true; } } registerProcessor('rufren-capture', RufrenCapture);`], {type:'text/javascript'}));
        await ctx.audioWorklet.addModule(workletUrl); workletLoaded = true; }
        if (!capture || generation !== requestGeneration) return;
        processor = new AudioWorkletNode(ctx,'rufren-capture'); processor.port.onmessage = event => collect(event.data);
      } catch (_) { processor = null; }
      finally { if (workletUrl) { URL.revokeObjectURL(workletUrl); workletUrl = null; } }
    }
    if (!processor) {
      processor = ctx.createScriptProcessor(2048,1,1);
      processor.onaudioprocess = event => collect(event.inputBuffer.getChannelData(0));
    }
    source.connect(processor); processor.connect(sink);

    timer = setInterval(() => { if (capture) mic(true,`${((performance.now()-capture.started)/1000).toFixed(1)} s / 30 s`); },200);
    mic(true,'0.0 s / 30 s'); status('Recording. Say one short phrase, then press Stop & keep take.');
  } catch (error) { requestGeneration++; starting = false; capture = null; releaseCapture(); mic(false); throw error; }
}
function releaseCapture() {
  clearInterval(timer); stream?.getTracks().forEach(t => t.stop()); stream = null;
  try { source?.disconnect(); processor?.disconnect(); sink?.disconnect(); } catch (_) {}
  if (processor?.port) processor.port.onmessage = null;
  if (processor) processor.onaudioprocess = null;
  source = processor = sink = null;
}
function takeSamples(take) {
  const samples=new Float32Array(take.frames);let offset=0;
  for(const chunk of take.chunks){samples.set(chunk,offset);offset+=chunk.length;}
  return samples;
}
async function preserveExitTake(take) {
  if(!take?.frames)return;
  const samples=takeSamples(take),blob=wav(samples,take.sampleRate);
  const record=await makeRecord(take.role,blob,{duration:take.frames/take.sampleRate,sampleRate:take.sampleRate,numberOfChannels:1,length:take.frames},`${take.role==='reference'?'Reference':'My'} interrupted take`,true,take.device);
  try{await storage('put',record);}
  catch(error){(window.rufrenAudioRecovery ||= []).push(record);throw error;}
}
async function stopRecording(reason = '') {
  requestGeneration++; starting = false;
  const take = capture;if(take?.frames)beginOperation();capture = null; releaseCapture(); mic(false);
  if (!take) { status('Microphone stopped.'); return; }
  if (!take.frames) throw Error('No audio arrived from the microphone. Try again or import a recording.');
  const samples = takeSamples(take);
  const buffer = context.createBuffer(1,samples.length,take.sampleRate); buffer.copyToChannel(samples,0);
  try{await keepWrite(saveAudio(take.role,wav(samples,take.sampleRate),buffer,`${take.role === 'reference' ? 'Reference' : 'My'} take`,!!reason && !reason.startsWith('The 30-second'),take.device));}finally{endOperation();}
  if (reason) status(reason);
}
function preflightDuration(file) {
  // The media element asks the decoder for metadata before Web Audio allocates
  // a complete PCM buffer. Browser codecs remain a platform trust boundary.
  return new Promise((resolve,reject)=>{
    const url=URL.createObjectURL(file), media=document.createElement('audio');media.preload='metadata';
    const finish=(error,duration)=>{clearTimeout(timeout);media.onloadedmetadata=media.onerror=null;media.removeAttribute('src');media.load();URL.revokeObjectURL(url);error?reject(error):resolve(duration);};
    const timeout=setTimeout(()=>finish(Error('Audio metadata could not be read. Try a short WAV or MP3 file.')),10000);
    media.onloadedmetadata=()=>{const duration=media.duration;if(!Number.isFinite(duration)||duration<.05||duration>60)finish(Error('Choose a file with a readable duration between 0.05 and 60 seconds.'));else finish(null,duration);};
    media.onerror=()=>finish(Error('This device could not read the audio file. Try WAV, MP3, or M4A.'));
    media.src=url;
  });
}
async function importFile(role,file) {
  if (!file) return;
  if(role==='attempt'){choiceGeneration++;clearAttempt();}
  beginOperation();try{
  stopPlayback();
  if (file.size > 32*1024*1024) throw Error('Choose an audio file smaller than 32 MB and at most 60 seconds long.');
  const ctx = await audioContext(false); status('Checking audio duration…');
  await preflightDuration(file); if (!alive) return; status('Decoding the short clip…');
  let buffer;
  try { buffer = await ctx.decodeAudioData(await file.arrayBuffer()); }
  catch (_) { throw Error('This device could not decode that file. Try WAV, MP3, M4A, or another format supported by your browser.'); }
  if (!alive) return;
  if (buffer.length * buffer.numberOfChannels > 16000000) throw Error('The decoded clip is too large for the instrument. Choose a shorter phrase.');
  if (buffer.numberOfChannels > 8) throw Error('Choose a recording with at most eight audio channels.');
  if (buffer.duration > 60 || buffer.duration < 0.05) throw Error('Choose a phrase between 0.05 and 60 seconds long. The original file has not been changed.');
  await keepWrite(saveAudio(role,file,buffer,file.name));
  }finally{endOperation();}
}
async function saveCue(role) {
  const span = selection(role), note = $(`audio-note-${role}`).value.trim();
  if (!note) throw Error('Write a short listening observation or practice cue first.');
  if(new TextEncoder().encode(note).length>2048)throw Error('This cue is too long to save. Shorten it to one useful observation.');
  if((span.record.annotations||[]).length>=256)throw Error('This recording already has 256 cues. Keep the next observation with a new take.');
  const record = {...span.record,annotations:[...(span.record.annotations || []),{start_seconds:span.from,end_seconds:span.to,note,created_at_ms:Date.now()}]};
  if(new TextEncoder().encode(JSON.stringify(metadata(record))).length>96*1024)throw Error('This recording’s cue notebook is full. Keep the next observation with a new take.');
  await storage('put',record); selected[role] = record; clips.get(record.asset_id).record = record;
  cueList(role,record); $(`audio-note-${role}`).value = ''; emit({kind:'cue_saved',recording:metadata(record)}); status('Cue saved with this exact audio span.');
}
function download(blob,name) {
  const url = URL.createObjectURL(blob), link = document.createElement('a'); link.href = url; link.download = name; link.click();
  setTimeout(() => URL.revokeObjectURL(url),30000);
}
async function downloadClip(role) {
  const record = selected[role]; if (!record) return;
  download(record.blob,record.name.includes('.') ? record.name : `rufren-${role}-${record.created_at_ms}.wav`);
}
async function exportAudio() {
  const records = await storage('all');
  if (!records.length) throw Error('There are no saved recordings to export yet.');
  // A self-contained JSON backup is deliberately separate from passage/event sync.
  const assets = [];
  for (const record of records) {
    const bytes = new Uint8Array(await record.blob.arrayBuffer()); let binary = '';
    for (let i=0;i<bytes.length;i+=16384) binary += String.fromCharCode(...bytes.subarray(i,i+16384));
    assets.push({...metadata(record),data_base64:btoa(binary)});
  }
  download(new Blob([JSON.stringify({format:'rufren-audio-v1',assets})],{type:'application/json'}),`rufren-audio-${new Date().toISOString().slice(0,10)}.json`);
  status('Audio backup prepared. It includes originals and timed cues, separately from your notebook backup.');
}
function validBackupMetadata(asset) {
  const {data_base64,...fields}=asset;
  if(new TextEncoder().encode(JSON.stringify(fields)).length>96*1024)return false;
  const strings=['asset_id','name','passage_id','revision_id','mime_type'];
  if(strings.some(key=>typeof asset[key]!=='string'||asset[key].length>2000))return false;
  if(!Number.isSafeInteger(asset.created_at_ms)||!Number.isSafeInteger(asset.frames)||asset.frames<1)return false;
  if(!Number.isSafeInteger(asset.duration_ms)||asset.duration_ms<0||asset.duration_ms>60001)return false;
  if(!Number.isSafeInteger(asset.sample_rate)||asset.sample_rate<8000||asset.sample_rate>192000)return false;
  if(!Number.isInteger(asset.channels)||asset.channels<1||asset.channels>8||asset.frames*asset.channels>16000000)return false;
  if(!Number.isSafeInteger(asset.byte_length)||asset.byte_length<1||asset.byte_length>32*1024*1024)return false;
  if(!Array.isArray(asset.annotations)||asset.annotations.length>256)return false;
  return asset.annotations.every(c=>Number.isFinite(c.start_seconds)&&Number.isFinite(c.end_seconds)&&c.start_seconds>=0&&c.end_seconds>c.start_seconds&&c.end_seconds<=asset.duration_ms/1000+.01&&typeof c.note==='string'&&c.note.trim().length>0&&new TextEncoder().encode(c.note).length<=2048&&Number.isSafeInteger(c.created_at_ms));
}
async function restoreAudio(file) {
  if (!file) return;
  if (file.size > 150*1024*1024) throw Error('Choose an audio backup smaller than 150 MB.');
  const backup = JSON.parse(await file.text());
  if (backup.format !== 'rufren-audio-v1' || !Array.isArray(backup.assets)) throw Error('This is not a Rufren audio backup.');
  // Verify every asset before writing any; restore never clears existing recordings.
  const records = [];
  for (const asset of backup.assets) {
    if (!validBackupMetadata(asset) || !asset.asset_id || !['reference','attempt'].includes(asset.role) || typeof asset.data_base64 !== 'string' || asset.data_base64.length>48*1024*1024 || !/^[0-9a-f]{64}$/.test(asset.content_hash)) throw Error('The audio backup contains invalid metadata.');
    const {data_base64,...record} = asset;
    if(Object.hasOwn(record,'original_mime_type')){record.device={...record.device,source_mime_type:record.original_mime_type};delete record.original_mime_type;}
    record.blob = new Blob([Uint8Array.from(atob(data_base64), c => c.charCodeAt(0))],{type:record.mime_type});
    if (record.blob.size !== record.byte_length || await digest(record.blob) !== record.content_hash) throw Error('An audio backup integrity check failed. Existing recordings were kept.');
    const existing = await storage('get',record.asset_id);
    if (existing) {
      const keys=['content_hash','role','passage_id','revision_id','duration_ms','sample_rate','channels','byte_length','created_at_ms','mime_type'];
      if(keys.some(key=>existing[key]!==record[key])) throw Error('A restored audio ID conflicts with an existing recording. Existing recordings were kept.');
      const cues=new Map();for(const cue of [...(record.annotations||[]),...(existing.annotations||[])]) cues.set(JSON.stringify([cue.start_seconds,cue.end_seconds,cue.note,cue.created_at_ms]),cue);
      records.push({...existing,annotations:[...cues.values()]});
    } else records.push(record);
  }
  for (const record of records) await storage('put',record);
  await refreshLibrary(); status(`${records.length} audio assets restored. Choose a saved recording to listen.`);
}
function draw(role, analysis) {
  if (!alive) return;
  const wave = $(`audio-wave-${role}`), spectrum = $(`audio-spectrum-${role}`), pitch = $(`audio-pitch-${role}`);
  const width = 960, height = 180;
  for (const canvas of [wave,spectrum,pitch]) { canvas.width=width; canvas.height=height; }
  let ctx = wave.getContext('2d'); ctx.fillStyle='#f4f5ed'; ctx.fillRect(0,0,width,height); ctx.strokeStyle='#315e50'; ctx.beginPath();
  for (const bin of analysis.waveform) { const x=bin.start_seconds/analysis.duration_seconds*width; ctx.moveTo(x,height/2-bin.min*height*.47); ctx.lineTo(x,height/2-bin.max*height*.47); } ctx.stroke();
  const spec=analysis.spectrogram, frames=spec.frame_times_seconds.length, maxHz=Number($('audio-frequency').value), floor=Number($('audio-floor').value);
  ctx=spectrum.getContext('2d'); const image=ctx.createImageData(width,height);
  for(let x=0;x<width;x++) for(let y=0;y<height;y++) {
    const frame=Math.min(frames-1,Math.floor(x/width*frames));
    const bin=Math.min(spec.frequency_bins-1,Math.floor((height-1-y)/(height-1)*maxHz*spec.fft_size/spec.sample_rate));
    const value=Math.max(0,Math.min(1,(spec.power_db[frame*spec.frequency_bins+bin]-floor)/(-floor)));
    const i=(y*width+x)*4; image.data[i]=Math.round(16+239*value); image.data[i+1]=Math.round(37+184*Math.pow(value,1.6)); image.data[i+2]=Math.round(43+60*value); image.data[i+3]=255;
  } ctx.putImageData(image,0,0);
  ctx=pitch.getContext('2d'); ctx.fillStyle='#f4f5ed'; ctx.fillRect(0,0,width,height); ctx.fillStyle='#315e50';
  for (const p of analysis.pitch) if(p.hz && p.confidence>=0.65) {
    const y=height-(Math.log2(p.hz/50)/Math.log2(400/50))*height;
    ctx.globalAlpha=Math.max(.3,p.confidence); ctx.fillRect(p.time_seconds/analysis.duration_seconds*width,y,3,3);
  } ctx.globalAlpha=1;
  for (const c of [wave,spectrum,pitch]) c.dataset.duration = analysis.duration_seconds;
  const axis=$(`audio-time-${role}`);axis.replaceChildren(...[0,.25,.5,.75,1].map(f=>{const tick=document.createElement('span');tick.textContent=`${(analysis.duration_seconds*f).toFixed(2)} s`;return tick;}));
  $(`audio-spectrum-label-${role}`).textContent=`Spectrogram · 0–${maxHz.toLocaleString()} Hz · ${floor} to 0 dBFS · Hann ${spec.fft_size} / hop ${spec.hop_size}`;
}
const analyses = {};
function click(event) {
  const button=event.target.closest('[data-action]'); if (!button || !root.contains(button)) return;
  const role=button.dataset.role;
  const actions = {advanced:toggleAdvanced,record:()=>startRecording(role),stop:()=>stopRecording(),play:()=>play(role),compare,stopPlayback:()=>{stopPlayback();status('Playback stopped.');},cue:()=>saveCue(role),download:()=>downloadClip(role),backup:exportAudio};
  if (actions[button.dataset.action]) Promise.resolve().then(actions[button.dataset.action]).catch(failure);
}
function clearAttempt(){
  stopPlayback();selected.attempt=null;delete analyses.attempt;analysisTokens.attempt='';sourceGenerations.attempt='';root.setAttribute('data-selected-attempt','');
  for(const kind of ['wave','spectrum','pitch']){const canvas=$(`audio-${kind}-attempt`);canvas.getContext('2d').clearRect(0,0,canvas.width,canvas.height);}$('audio-from-attempt').value='0';$('audio-to-attempt').value='0';$('audio-cues-attempt').replaceChildren();
  $('audio-library-attempt').value='';$('audio-name-attempt').textContent='No recording selected';$('audio-provenance-attempt').textContent='No source selected';
  emit({kind:'source_clear',role:'attempt'});emit({kind:'selected',recording:null});controls();
}
function validAttempt(record){return record&&record.role==='attempt'&&record.passage_id===config.passage_id&&record.revision_id===config.revision_id;}
async function chooseExisting(role,id){
  const generation=++choiceGeneration;stopPlayback();if(role==='attempt')clearAttempt();if(!id)return;
  beginOperation();
  try{
    const record=await storage('get',id);if(!alive||generation!==choiceGeneration)return;
    if(!record||record.role!==role||record.passage_id!==config.passage_id||record.revision_id!==config.revision_id)throw Error('This recording does not belong to the current phrase. Choose its exact saved take.');
    const prepared=await selectRecord(role,record);
    if(prepared&&alive&&generation===choiceGeneration&&selected[role]?.asset_id===id&&role==='attempt'){if(record.interrupted)status('You selected a partial take. Replay it and check the cutoff before using it.');emit({kind:'selected',recording:metadata(record)});}
  }finally{endOperation();}
}
async function toggleAdvanced(){
  compactAdvanced=!compactAdvanced;root.dataset.advanced=String(compactAdvanced);
  const button=root.querySelector('[data-action="advanced"]');button?.setAttribute('aria-expanded',String(compactAdvanced));if(button)button.textContent=compactAdvanced?'Hide sound tools':'Show sound tools';
  if(compactAdvanced){for(const role of ['reference','attempt']){const record=selected[role];if(record)await selectRecord(role,record,clips.get(record.asset_id)?.buffer);}}
}
function change(event) {
  const node=event.target;
  if (node.dataset.import) { const file=node.files?.[0];importFile(node.dataset.import,file).catch(failure).finally(()=>{node.value='';}); }
  if (node.dataset.restore !== undefined) { restoreAudio(node.files?.[0]).catch(failure).finally(()=>{node.value='';}); }
  if (node.dataset.library) { chooseExisting(node.dataset.library,node.value).catch(failure); }
  if (['audio-floor','audio-frequency'].includes(node.id)){for (const [role,a] of Object.entries(analyses)) draw(role,a);drawRegions();}
}
// Independent A/B instrument state. No asset, note, or learning fact is mutated.
const REGION_ROLES=['region_a','region_b'];
const regionSlots=Object.fromEntries(REGION_ROLES.map(role=>[role,{record:null,source:null,analysis:null,span:null,metrics:null,token:'',request:'',revision:0,userVersion:0,valid:false,timer:null}]));
const analysisTokens={},sourceGenerations={};let sourceCounter=0,regionCounter=0,regionDrag=null,regionInit=false,focusDatabase,focusOpening;
const focusWrites=new Set(),focusTimers=new Map();
let regionWorkspace=config.workspace_id||'';
if(!regionWorkspace)try{regionWorkspace=sessionStorage.getItem('rufren.workspace.v1')||'';}catch(_){}
const regionLab=$('audio-region-lab');
function isRegion(role){return REGION_ROLES.includes(role);}
function nextAnalysis(role,record){
  const token=`${Date.now()}-${++sourceCounter}`;analysisTokens[role]=token;sourceGenerations[role]=token;selected[role]=record;
  root.setAttribute(`data-selected-${role}`,record.asset_id);root.setAttribute(`data-analysis-request-${role}`,token);
  return token;
}
function currentAnalysis(role,asset,token){return alive&&selected[role]?.asset_id===asset&&analysisTokens[role]===token;}
async function blobBase64(blob){const bytes=new Uint8Array(await blob.arrayBuffer());let binary='';for(let i=0;i<bytes.length;i+=16384)binary+=String.fromCharCode(...bytes.subarray(i,i+16384));return btoa(binary);}
async function sendPcm(role,record,buffer,token,decoded=false){
  if(!currentAnalysis(role,record.asset_id,token))return;
  const header=new Uint8Array(await record.blob.slice(0,12).arrayBuffer()),text=(a,b)=>String.fromCharCode(...header.slice(a,b));
  const wav=!decoded&&text(0,4)==='RIFF'&&text(8,12)==='WAVE';
  const message={kind:'pcm',role,recording:metadata(record),request_id:token,sample_rate:buffer.sampleRate,channels:buffer.numberOfChannels,samples:wav?'':analysisPcm(buffer),wav_base64:wav?await blobBase64(record.blob):''};
  if(currentAnalysis(role,record.asset_id,token))emit(message);
}
function regionSourceList(records){
  if(!regionLab)return;
  for(const role of REGION_ROLES){const picker=$(`audio-region-source-${role}`),value=regionSlots[role].record?.asset_id||selected[role]?.asset_id||'';picker.replaceChildren(new Option('Choose a saved recording…',''));
    for(const record of [...records].sort((a,b)=>b.created_at_ms-a.created_at_ms))picker.add(new Option(`${sourceLabel(record)} · ${record.name} · ${(record.duration_ms/1000).toFixed(2)} s`,record.asset_id));picker.value=value;
  }
}
async function loadRegion(role,id,preset){
  if(!regionLab||!isRegion(role)||!id)return;
  stopPlayback();const state=regionSlots[role];persistRegionFocus(role,true);clearTimeout(state.timer);
  // Invalidate the visible source at intent time, before even reading metadata.
  // An older IDB completion must never reopen its controls or overwrite a newer pick.
  const token=`${Date.now()}-${++sourceCounter}`;analysisTokens[role]=token;sourceGenerations[role]=token;selected[role]={asset_id:id};
  root.setAttribute(`data-selected-${role}`,id);root.setAttribute(`data-analysis-request-${role}`,token);root.setAttribute(`data-region-request-${role}`,'');
  Object.assign(state,{record:null,source:null,analysis:null,span:null,metrics:null,token,request:'',revision:0,valid:false,preset,renderKey:''});
  const range=$(`audio-region-range-${role}`);range.textContent='';for(const key of Object.keys(range.dataset))delete range.dataset[key];
  $(`audio-region-identity-${role}`).textContent='Opening the selected original…';$(`audio-region-metrics-${role}`).textContent='Preparing original-frame analysis…';
  for(const kind of ['wave','spectrum','pitch']){const c=$(`audio-${kind}-${role}`);c.getContext('2d').clearRect(0,0,c.width,c.height);}
  for(const e of root.querySelectorAll(`[data-region-highlight="${role}"]`))e.hidden=true;
  regionControls();updateRegionComparison();emit({kind:'source_clear',role});
  try{
    const record=await storage('get',id);if(!currentAnalysis(role,id,token))return;
    if(!record)throw Error('That recording is not available on this device. Sync or restore its original.');
    state.record=record;selected[role]=record;$(`audio-region-source-${role}`).value=id;
    $(`audio-region-identity-${role}`).textContent=`${record.name} · checking the original…`;
    if(await digest(record.blob)!==record.content_hash)throw Error('The selected original failed its integrity check. Existing files were kept.');
    const buffer=clips.get(id)?.buffer||await(await audioContext(false)).decodeAudioData(await record.blob.arrayBuffer());
    if(!currentAnalysis(role,id,token))return;
    clips.set(id,{...clips.get(id),record,buffer});await sendPcm(role,record,buffer,token);persistRegionPair();
  }catch(error){if(currentAnalysis(role,id,token)){$(`audio-region-metrics-${role}`).textContent=error.message;regionControls();throw error;}}
}

function focusKey(role,state){const m=state.source?.metadata;return JSON.stringify(['region-focus-v1',regionWorkspace,role,state.record?.asset_id,state.record?.content_hash,m?.content_hash,m?.sample_rate,m?.frames]);}
function pairKey(){return JSON.stringify(['region-pair-v1',regionWorkspace,config.passage_id,config.revision_id]);}
async function focusDb(){
  if(!regionWorkspace)return null;
  if(focusDatabase)return focusDatabase;if(focusOpening)return focusOpening;
  focusOpening=new Promise((resolve,reject)=>{const r=indexedDB.open('rufren-audio-focus-v1',1);r.onupgradeneeded=()=>r.result.createObjectStore('state',{keyPath:'key'});r.onsuccess=()=>{focusDatabase=r.result;focusDatabase.onversionchange=()=>focusDatabase.close();resolve(focusDatabase);};r.onerror=()=>reject(r.error);r.onblocked=()=>reject(Error('Recording focus storage is blocked by another window.'));});
  return focusOpening;
}
async function focusRead(key){const db=await focusDb();if(!db)return null;return new Promise((resolve,reject)=>{const tx=db.transaction('state'),r=tx.objectStore('state').get(key);r.onsuccess=()=>resolve(r.result||null);tx.onabort=tx.onerror=()=>reject(tx.error);});}
function focusWrite(value){
  if(!regionWorkspace)return Promise.resolve();
  const promise=(async()=>{const db=await focusDb();await new Promise((resolve,reject)=>{const tx=db.transaction('state','readwrite');tx.objectStore('state').put(value);tx.oncomplete=resolve;tx.onabort=tx.onerror=()=>reject(tx.error);});})();
  focusWrites.add(promise);promise.then(()=>focusWrites.delete(promise),()=>focusWrites.delete(promise));
  promise.catch(()=>{if(alive&&regionLab)$('audio-region-persistence').textContent='This focus could not be saved. Your original recordings are unchanged.';});return promise;
}
function persistRegionFocus(role,now=false){
  const state=regionSlots[role];if(!state?.source||!state.span||!state.valid)return Promise.resolve();clearTimeout(focusTimers.get(role));
  if(!now){focusTimers.set(role,setTimeout(()=>persistRegionFocus(role,true),180));return Promise.resolve();}
  return focusWrite({key:focusKey(role,state),version:1,workspace:regionWorkspace,asset_id:state.record.asset_id,content_hash:state.record.content_hash,pcm_hash:state.source.metadata.content_hash,sample_rate:state.source.metadata.sample_rate,frames:state.source.metadata.frames,span:{...state.span}});
}
function persistRegionPair(){if(!regionWorkspace)return;focusWrite({key:pairKey(),version:1,workspace:regionWorkspace,sources:Object.fromEntries(REGION_ROLES.map(role=>[role,regionSlots[role].record?{asset_id:regionSlots[role].record.asset_id,content_hash:regionSlots[role].record.content_hash}:null]))});}
async function initializeRegions(){
  if(!regionLab||regionInit)return;regionInit=true;const userVersions=Object.fromEntries(REGION_ROLES.map(role=>[role,regionSlots[role].userVersion]));regionSourceList(await storage('all'));
  $('audio-region-persistence').textContent=regionWorkspace?'Focus stays in this saved window and is checked against the exact original and PCM identity.':'Focus stays in this view. A saved-window identity is needed to restore it safely.';
  try{const saved=await focusRead(pairKey());if(saved?.version===1&&saved.workspace===regionWorkspace){for(const role of REGION_ROLES){const entry=saved.sources?.[role];if(!entry||regionSlots[role].userVersion!==userVersions[role])continue;const record=await storage('get',entry.asset_id);if(record?.content_hash===entry.content_hash&&regionSlots[role].userVersion===userVersions[role])await loadRegion(role,record.asset_id);}}}catch(_){$('audio-region-persistence').textContent='Previous focus could not be opened. Choose recordings; originals were kept.';}
  if(!regionSlots.region_a.record&&!sourceGenerations.region_a&&regionSlots.region_a.userVersion===userVersions.region_a&&selected.reference)await loadRegion('region_a',selected.reference.asset_id);
  if(!regionSlots.region_b.record&&!sourceGenerations.region_b&&regionSlots.region_b.userVersion===userVersions.region_b&&(selected.attempt||selected.reference))await loadRegion('region_b',(selected.attempt||selected.reference).asset_id);
}
function validSpan(source,span){return source&&span&&Number.isSafeInteger(span.start_frame)&&Number.isSafeInteger(span.end_frame)&&span.start_frame>=0&&span.end_frame>span.start_frame&&span.end_frame<=source.metadata.frames;}
function applyRegion(role,span,{save=true,measure=true}={}){
  const state=regionSlots[role];if(!validSpan(state.source,span)){state.valid=false;state.metrics=null;state.request=`${state.token}:invalid:${++regionCounter}`;root.setAttribute(`data-region-request-${role}`,state.request);clearTimeout(state.timer);$(`audio-region-metrics-${role}`).textContent='Boundaries were not applied. Keep 0 ≤ start < end ≤ the source frame count; the previous highlight is retained.';regionControls();updateRegionComparison();return false;}
  if(state.span&&(state.span.start_frame!==span.start_frame||state.span.end_frame!==span.end_frame))stopPlayback();
  state.span={...span};state.valid=true;state.metrics=null;state.revision++;state.request=`${state.token}:region:${++regionCounter}`;root.setAttribute(`data-region-request-${role}`,state.request);
  const rate=state.source.metadata.sample_rate;
  for(const [id,value]of [[`audio-region-first-${role}`,span.start_frame],[`audio-region-last-${role}`,span.end_frame],[`audio-region-from-${role}`,(span.start_frame/rate).toFixed(6)],[`audio-region-to-${role}`,(span.end_frame/rate).toFixed(6)]]){const input=$(id);if(input.value!==String(value))input.value=String(value);input.removeAttribute('aria-invalid');}
  const text=$(`audio-region-range-${role}`);text.textContent=`${(span.start_frame/rate).toFixed(6)}–${(span.end_frame/rate).toFixed(6)} s · frames [${span.start_frame}, ${span.end_frame}) at ${rate.toLocaleString()} Hz`;
  Object.assign(text.dataset,{startFrame:String(span.start_frame),endFrame:String(span.end_frame),assetId:state.record.asset_id,contentHash:state.record.content_hash,pcmHash:state.source.metadata.content_hash,sourceKind:state.source.kind});
  $(`audio-region-metrics-${role}`).textContent='Measuring the selected original frames…';regionControls();drawRegions();updateRegionComparison();
  if(save)persistRegionFocus(role);if(measure)scheduleRegionMeasurement(role);return true;
}
function scheduleRegionMeasurement(role,immediate=false){const state=regionSlots[role];clearTimeout(state.timer);const send=()=>{if(!alive||!state.valid||!state.source)return;emit({kind:'region',role,asset_id:state.record.asset_id,content_hash:state.record.content_hash,source_pcm_hash:state.source.metadata.content_hash,request_id:state.request,...state.span});};if(immediate)send();else state.timer=setTimeout(send,100);}
async function acceptRegionSource(message){
  const role=message.role,state=regionSlots[role];if(!state||!currentAnalysis(role,message.asset_id,message.request_id))return;
  state.source=message.source;state.analysis=message.analysis;
  const meta=state.source.metadata;
  $(`audio-region-identity-${role}`).textContent=`${state.record.name} · ${state.source.kind==='original_wav'?'Original WAV frames':'Device-decoded PCM frames'} · ${meta.sample_rate.toLocaleString()} Hz · ${meta.frames.toLocaleString()} frames · file ${state.record.content_hash.slice(0,12)}…`;
  for(const id of [`audio-region-first-${role}`,`audio-region-last-${role}`])$(id).max=String(meta.frames);
  for(const id of [`audio-region-from-${role}`,`audio-region-to-${role}`]){$(id).max=String(meta.frames/meta.sample_rate);$(id).step=String(1/meta.sample_rate);}
  const span=validSpan(state.source,state.preset)?state.preset:{start_frame:0,end_frame:meta.frames};applyRegion(role,span,{save:false});const revision=state.revision;
  try{const focus=await focusRead(focusKey(role,state));if(currentAnalysis(role,message.asset_id,message.request_id)&&state.revision===revision&&focus?.version===1&&focus.workspace===regionWorkspace&&focus.asset_id===state.record.asset_id&&focus.content_hash===state.record.content_hash&&focus.pcm_hash===meta.content_hash&&focus.sample_rate===meta.sample_rate&&focus.frames===meta.frames&&validSpan(state.source,focus.span))applyRegion(role,focus.span,{save:false});}catch(_){}
  persistRegionPair();
}
function regionControls(){
  if(!regionLab)return;
  for(const role of REGION_ROLES){const state=regionSlots[role],enabled=!!state?.source&&state.valid&&!capture&&!starting;
    root.querySelectorAll(`[data-region-play="${role}"],[data-region-export="${role}"]`).forEach(b=>b.disabled=!enabled);
    root.querySelectorAll(`[data-region-owner="${role}"]`).forEach(b=>b.disabled=!state?.source);
    root.querySelectorAll(`[data-region-seconds="${role}"],[data-region-frames="${role}"]`).forEach(input=>input.disabled=!state?.source);
    $(`audio-region-source-${role}`).disabled=!!capture||starting;
  }
  $('audio-region-compare').disabled=REGION_ROLES.some(role=>!regionSlots[role].source||!regionSlots[role].valid)||!!capture||starting;
}
function regionViewport(role){
  if(regionDrag?.views?.[role])return regionDrag.views[role];
  const state=regionSlots[role],ready=REGION_ROLES.map(r=>regionSlots[r]).filter(s=>s.source&&s.span);
  if(!$('audio-region-zoom').checked){return {start:0,duration:Math.max(.001,...ready.map(s=>s.source.metadata.duration_seconds))};}
  const widest=Math.max(.001,...ready.map(s=>(s.span.end_frame-s.span.start_frame)/s.source.metadata.sample_rate)),padding=Math.max(.005,widest*.12);
  return {start:Math.max(0,state.span.start_frame/state.source.metadata.sample_rate-padding),duration:widest+padding*2};
}
function drawRegions(){
  if(!regionLab)return;
  for(const role of REGION_ROLES){const state=regionSlots[role];if(!state.source||!state.analysis||!state.span)continue;const analysis=state.analysis,view=regionViewport(role),width=960,height=180;
    const start=state.span.start_frame/state.source.metadata.sample_rate,end=state.span.end_frame/state.source.metadata.sample_rate;
    const wave=$(`audio-wave-${role}`),spectrum=$(`audio-spectrum-${role}`),pitch=$(`audio-pitch-${role}`);
    const renderKey=JSON.stringify([state.source.metadata.content_hash,view.start,view.duration,$('audio-frequency').value,$('audio-floor').value]);
    if(state.renderKey!==renderKey){state.renderKey=renderKey;
    for(const c of [wave,spectrum,pitch]){if(c.width!==width)c.width=width;if(c.height!==height)c.height=height;c.dataset.viewStart=String(view.start);c.dataset.viewDuration=String(view.duration);}
    let ctx=wave.getContext('2d');ctx.fillStyle='#f1f3ed';ctx.fillRect(0,0,width,height);ctx.strokeStyle='#315e50';ctx.beginPath();
    for(const bin of analysis.waveform){const x=(bin.start_seconds-view.start)/view.duration*width;if(x<0||x>width)continue;ctx.moveTo(x,height/2-bin.min*height*.47);ctx.lineTo(x,height/2-bin.max*height*.47);}ctx.stroke();
    const spec=analysis.spectrogram,maxHz=Number($('audio-frequency').value),floor=Number($('audio-floor').value),frames=spec.frame_times_seconds.length;
    ctx=spectrum.getContext('2d');const pixels=state.spectrumPixels ||=ctx.createImageData(width,height);const first=spec.frame_times_seconds[0]||0,hop=spec.hop_size/spec.sample_rate;
    for(let x=0;x<width;x++){const time=view.start+x/width*view.duration,frame=Math.min(frames-1,Math.max(0,Math.round((time-first)/hop)));for(let y=0;y<height;y++){
      const index=(y*width+x)*4;if(time<0||time>=analysis.duration_seconds){pixels.data[index]=235;pixels.data[index+1]=237;pixels.data[index+2]=231;pixels.data[index+3]=255;continue;}
      const bin=Math.min(spec.frequency_bins-1,Math.floor((height-1-y)/(height-1)*maxHz*spec.fft_size/spec.sample_rate)),value=Math.max(0,Math.min(1,(spec.power_db[frame*spec.frequency_bins+bin]-floor)/(-floor)));
      pixels.data[index]=Math.round(16+239*value);pixels.data[index+1]=Math.round(37+184*value**1.6);pixels.data[index+2]=Math.round(43+60*value);pixels.data[index+3]=255;
    }}ctx.putImageData(pixels,0,0);
    ctx=pitch.getContext('2d');ctx.fillStyle='#f1f3ed';ctx.fillRect(0,0,width,height);ctx.fillStyle='#315e50';for(const p of analysis.pitch)if(p.hz&&p.confidence>=.65){const x=(p.time_seconds-view.start)/view.duration*width;if(x>=0&&x<width){ctx.globalAlpha=Math.max(.3,p.confidence);ctx.fillRect(x,height-Math.log2(p.hz/50)/Math.log2(400/50)*height,3,3);}}ctx.globalAlpha=1;
    const axis=$(`audio-time-${role}`);axis.replaceChildren(...[0,.25,.5,.75,1].map(f=>{const t=document.createElement('span');t.textContent=`${(view.start+f*view.duration).toFixed(view.duration<2?3:2)} s`;return t;}));
    $(`audio-region-scale-${role}`).textContent=`Shared view ${Math.round(view.duration*1000)} ms · 0–${maxHz.toLocaleString()} Hz · ${floor} to 0 dBFS. File ends at ${analysis.duration_seconds.toFixed(4)} s.`;
    }
    for(const surface of root.querySelectorAll(`[data-region-surface="${role}"]`)){
      const left=(start-view.start)/view.duration*100,right=(end-view.start)/view.duration*100,highlight=surface.querySelector('[data-region-highlight]');highlight.style.left=`${Math.max(0,left)}%`;highlight.style.width=`${Math.max(0,Math.min(100,right)-Math.max(0,left))}%`;highlight.hidden=false;
      for(const edge of ['start','end']){const handle=surface.querySelector(`[data-region-edge="${edge}"]`),frame=edge==='start'?state.span.start_frame:state.span.end_frame;handle.style.left=`${edge==='start'?left:right}%`;handle.setAttribute('aria-valuenow',String(frame));handle.setAttribute('aria-valuemin',String(edge==='start'?0:state.span.start_frame+1));handle.setAttribute('aria-valuemax',String(edge==='start'?state.span.end_frame-1:state.source.metadata.frames));handle.setAttribute('aria-valuetext',`${frame} frames, ${(frame/state.source.metadata.sample_rate).toFixed(6)} seconds`);}
    }
  }
  for(const plot of root.querySelectorAll('.audio-region-plot[data-region-track="pitch"]'))plot.hidden=!$('audio-region-pitch').checked;
}
function updateRegionComparison(){if(!regionLab)return;const a=regionSlots.region_a,b=regionSlots.region_b;$('audio-region-comparison').textContent=a.source&&b.source?(!a.valid||!b.valid?'Fix the invalid boundaries before comparing.':'Measuring the current selections…'):'Select both regions to compare.';
  $('audio-region-whole').textContent=a.analysis&&b.analysis?`Full recordings, for context only: A ${(a.analysis.duration_seconds*1000).toFixed(1)} ms at ${a.analysis.quality.rms_dbfs.toFixed(1)} dBFS; B ${(b.analysis.duration_seconds*1000).toFixed(1)} ms at ${b.analysis.quality.rms_dbfs.toFixed(1)} dBFS.`:'';
}
function regionMetrics(message){const state=regionSlots[message.role];if(!state||message.request_id!==state.request||message.asset_id!==state.record?.asset_id||message.content_hash!==state.record?.content_hash||message.metrics.source.content_hash!==state.source?.metadata.content_hash)return;
  state.metrics=message.metrics;const m=message.metrics,q=m.quality;
  $(`audio-region-metrics-${message.role}`).textContent=`Selected duration ${(m.duration_seconds*1000).toFixed(2)} ms · RMS ${q.rms_dbfs.toFixed(1)} dBFS · peak ${q.peak.toFixed(3)} · near full scale ${(q.clipping_fraction*100).toFixed(2)}%.`;
}
function regionComparison(message){const a=regionSlots.region_a,b=regionSlots.region_b;if(message.a_request_id!==a.request||message.b_request_id!==b.request||!a.metrics||!b.metrics)return;
  const c=message.comparison,delta=c.duration_delta_seconds*1000,level=c.warnings.some(w=>/floor/i.test(w))?'Level difference is unresolved at the display floor.':`B’s measured RMS is ${c.rms_delta_db>=0?'+':''}${c.rms_delta_db.toFixed(1)} dB relative to A.`;
  $('audio-region-comparison').textContent=`B lasts ${Math.abs(delta).toFixed(2)} ms ${delta<0?'less':'more'} than A (${c.duration_ratio.toFixed(3)}× A). ${level} ${c.warnings.join(' ')}`;
}
function regionTimeInput(input,frames){const role=input.dataset[frames?'regionFrames':'regionSeconds'],state=regionSlots[role];if(!state?.source)return;const rate=state.source.metadata.sample_rate;
  const start=Number($(frames?`audio-region-first-${role}`:`audio-region-from-${role}`).value),end=Number($(frames?`audio-region-last-${role}`:`audio-region-to-${role}`).value);
  const span={start_frame:frames?start:Math.round(start*rate),end_frame:frames?end:Math.round(end*rate)};
  if(!$(frames?`audio-region-first-${role}`:`audio-region-from-${role}`).value.trim()||!$(frames?`audio-region-last-${role}`:`audio-region-to-${role}`).value.trim()||!Number.isFinite(start)||!Number.isFinite(end)||(!frames&&(start<0||start>=end||end>state.source.metadata.duration_seconds))){applyRegion(role,{start_frame:NaN,end_frame:NaN});input.setAttribute('aria-invalid','true');return;}
  if(!applyRegion(role,span))input.setAttribute('aria-invalid','true');
}
function pointerFrame(surface,event){const role=surface.dataset.regionSurface,state=regionSlots[role],canvas=surface.querySelector('canvas'),rect=canvas.getBoundingClientRect(),view=regionViewport(role),x=Math.max(0,Math.min(1,(event.clientX-rect.left)/rect.width));return Math.max(0,Math.min(state.source.metadata.frames,Math.round((view.start+x*view.duration)*state.source.metadata.sample_rate)));}
function regionPointerDown(event){const surface=event.target.closest('[data-region-surface]');if(!surface||event.button!==0||regionDrag)return;const role=surface.dataset.regionSurface,state=regionSlots[role];if(!state.source)return;event.preventDefault();state.revision++;state.userVersion++;const edge=event.target.closest('[data-region-edge]')?.dataset.regionEdge;if(edge)root.querySelector(`[data-region-surface="${role}"][data-region-kind="wave"] [data-region-edge="${edge}"]`)?.focus({preventScroll:true});
  const views=Object.fromEntries(REGION_ROLES.filter(r=>regionSlots[r].source&&regionSlots[r].span).map(r=>[r,regionViewport(r)]));
  regionDrag={role,surface,views,pointer:event.pointerId,edge,anchor:pointerFrame(surface,event),old:{...state.span},startX:event.clientX,moved:false};surface.setPointerCapture(event.pointerId);
}
function regionPointerMove(event){const drag=regionDrag;if(!drag||drag.pointer!==event.pointerId)return;const state=regionSlots[drag.role],frame=pointerFrame(drag.surface,event);drag.moved ||=Math.abs(event.clientX-drag.startX)>2;
  if(!drag.moved&&!drag.edge)return;const span=drag.edge==='start'?{start_frame:Math.min(frame,state.span.end_frame-1),end_frame:state.span.end_frame}:drag.edge==='end'?{start_frame:state.span.start_frame,end_frame:Math.max(frame,state.span.start_frame+1)}:{start_frame:Math.min(drag.anchor,frame),end_frame:Math.max(drag.anchor,frame)};
  if(validSpan(state.source,span))applyRegion(drag.role,span);
}
function regionPointerUp(event){const drag=regionDrag;if(!drag||drag.pointer!==event.pointerId)return;regionDrag=null;try{drag.surface.releasePointerCapture(event.pointerId);}catch(_){}drawRegions();if(drag.moved||drag.edge){persistRegionFocus(drag.role,true);scheduleRegionMeasurement(drag.role,true);}}
function regionKey(event){const handle=event.target.closest('[data-region-edge]');if(!handle)return;const role=handle.dataset.regionOwner,state=regionSlots[role];if(!state?.source)return;const edge=handle.dataset.regionEdge,span={...state.span},amount=event.altKey?100:event.shiftKey?10:1,key=edge==='start'?'start_frame':'end_frame';let value=span[key];
  if(['ArrowLeft','ArrowDown'].includes(event.key))value-=amount;else if(['ArrowRight','ArrowUp'].includes(event.key))value+=amount;else if(event.key==='Home')value=edge==='start'?0:span.start_frame+1;else if(event.key==='End')value=edge==='start'?span.end_frame-1:state.source.metadata.frames;else return;
  event.preventDefault();span[key]=Math.max(edge==='start'?0:span.start_frame+1,Math.min(edge==='start'?span.end_frame-1:state.source.metadata.frames,value));applyRegion(role,span);scheduleRegionMeasurement(role,true);
}
function regionSelection(role){const state=regionSlots[role];if(!state?.source||!state.valid)throw Error('Choose a valid region first.');const buffer=clips.get(state.record.asset_id)?.buffer;if(!buffer)throw Error('Choose that recording again.');return {record:state.record,buffer,from:state.span.start_frame/state.source.metadata.sample_rate,to:state.span.end_frame/state.source.metadata.sample_rate,region:{source:state.source,span:{...state.span}}};}
function emitPlayback(role,span,rate){if(isRegion(role)){const state=span.region;emit({kind:'region_played',role,recording:metadata(span.record),asset_id:span.record.asset_id,content_hash:span.record.content_hash,source_pcm_hash:state.source.metadata.content_hash,...state.span,sample_rate:state.source.metadata.sample_rate,playback_rate:rate});}else if(role==='reference')emit({kind:'reference_played'});}
async function playRegion(role,keep=false){const span=regionSelection(role);if(!alive)return false;stopIndependentPlayers();if(!keep)stopPlayback();const generation=playGeneration,rate=Number($('audio-region-rate').value),loop=$('audio-region-loop').checked&&!keep;
  if(rate!==1)return playStretched(role,span,rate,loop,generation);const ctx=await audioContext();if(!alive||generation!==playGeneration)return false;
  const node=ctx.createBufferSource();node.buffer=span.buffer;node.connect(ctx.destination);node.loop=loop;node.loopStart=span.from;node.loopEnd=span.to;playing.push(node);
  return new Promise(resolve=>{let acknowledged=false,ackTimer;const beganAt=ctx.currentTime;
    const acknowledge=()=>{if(acknowledged||!alive||generation!==playGeneration)return;if(ctx.state==='running'&&ctx.currentTime>beganAt){acknowledged=true;emitPlayback(role,span,1);}else ackTimer=setTimeout(acknowledge,5);};
    node.onended=()=>{clearTimeout(ackTimer);acknowledge();playing=playing.filter(n=>n!==node);resolve(generation===playGeneration);};
    if(loop)node.start(0,span.from);else node.start(0,span.from,span.to-span.from);ackTimer=setTimeout(acknowledge,0);status(`Playing selected region ${role==='region_a'?'A':'B'} at natural speed.`);});
}
async function compareRegions(){stopPlayback();const generation=playGeneration;if(!await playRegion('region_a',true)||generation!==playGeneration)return;await new Promise(r=>setTimeout(r,300));if(alive&&generation===playGeneration)await playRegion('region_b',true);}
function exportRegion(role){const state=regionSlots[role];if(!state.source||!state.valid)return;download(new Blob([JSON.stringify({format:'rufren-region-selection-v1',asset_id:state.record.asset_id,original_byte_sha256:state.record.content_hash,source_pcm:state.source.metadata,source_kind:state.source.kind,span:state.span,start_seconds:state.span.start_frame/state.source.metadata.sample_rate,end_seconds:state.span.end_frame/state.source.metadata.sample_rate},null,2)],{type:'application/json'}),`rufren-region-${role==='region_a'?'A':'B'}.json`);}
function clearRegion(role){const state=regionSlots[role];if(!state)return;stopPlayback();persistRegionFocus(role,true);clearTimeout(state.timer);Object.assign(state,{record:null,source:null,analysis:null,span:null,metrics:null,valid:false,token:'',request:''});delete selected[role];analysisTokens[role]='';sourceGenerations[role]='';root.setAttribute(`data-selected-${role}`,'');root.setAttribute(`data-analysis-request-${role}`,'');root.setAttribute(`data-region-request-${role}`,'');$(`audio-region-identity-${role}`).textContent='No source selected';$(`audio-region-range-${role}`).textContent='';$(`audio-region-metrics-${role}`).textContent='Choose a recording to measure a region.';for(const kind of ['wave','spectrum','pitch']){const c=$(`audio-${kind}-${role}`);c.getContext('2d').clearRect(0,0,c.width,c.height);}for(const e of root.querySelectorAll(`[data-region-highlight="${role}"]`))e.hidden=true;emit({kind:'source_clear',role});regionControls();updateRegionComparison();persistRegionPair();}
function regionEditing(event){const role=event.target.dataset?.regionFrames||event.target.dataset?.regionSeconds||event.target.dataset?.regionOwner||event.target.dataset?.regionSource;const state=regionSlots[role];if(state){state.revision++;state.userVersion++;}}
async function regionAction(event){
  const picker=event.target.closest('[data-region-source]');if(picker){regionSlots[picker.dataset.regionSource].userVersion++;if(picker.value)await loadRegion(picker.dataset.regionSource,picker.value);else clearRegion(picker.dataset.regionSource);return;}
  if(event.target.dataset.regionSeconds){regionTimeInput(event.target,false);return;}if(event.target.dataset.regionFrames){regionTimeInput(event.target,true);return;}
  if(['audio-region-zoom','audio-region-pitch'].includes(event.target.id)){drawRegions();return;}
}
async function regionClick(event){const play=event.target.closest('[data-region-play]'),exporter=event.target.closest('[data-region-export]'),button=event.target.closest('[data-region-action]');
  if(play){await playRegion(play.dataset.regionPlay);return;}if(exporter){exportRegion(exporter.dataset.regionExport);return;}if(!button)return;
  const action=button.dataset.regionAction;if(action==='compare')await compareRegions();
  if(action==='copy-a'&&regionSlots.region_a.record){regionSlots.region_b.userVersion++;await loadRegion('region_b',regionSlots.region_a.record.asset_id);}
  if(action==='use-current'){for(const role of REGION_ROLES)regionSlots[role].userVersion++;if(selected.reference)await loadRegion('region_a',selected.reference.asset_id);if(selected.attempt||selected.reference)await loadRegion('region_b',(selected.attempt||selected.reference).asset_id);}
  if(action==='reset'){const role=button.dataset.regionRole,state=regionSlots[role];if(state.source)applyRegion(role,{start_frame:0,end_frame:state.source.metadata.frames});}
}
function regionToggle(){if(regionLab?.open)initializeRegions().catch(failure);else if(regionLab)stopPlayback();}
async function shutdownRegions(){
  if(!regionLab)return;
  for(const role of REGION_ROLES){clearTimeout(regionSlots[role].timer);clearTimeout(focusTimers.get(role));persistRegionFocus(role,true);}
  regionLab.removeEventListener('toggle',regionToggle);root.removeEventListener('pointerdown',regionPointerDown);root.removeEventListener('pointermove',regionPointerMove);root.removeEventListener('pointerup',regionPointerUp);root.removeEventListener('pointercancel',regionPointerUp);root.removeEventListener('keydown',regionKey);root.removeEventListener('focusin',regionEditing);root.removeEventListener('input',regionEditing);root.removeEventListener('change',regionActionListener);root.removeEventListener('click',regionClickListener);
  await Promise.allSettled([...focusWrites]);focusDatabase?.close();
}
function regionActionListener(event){regionAction(event).catch(failure);}function regionClickListener(event){regionClick(event).catch(failure);}
if(regionLab){regionLab.addEventListener('toggle',regionToggle);root.addEventListener('pointerdown',regionPointerDown);root.addEventListener('pointermove',regionPointerMove);root.addEventListener('pointerup',regionPointerUp);root.addEventListener('pointercancel',regionPointerUp);root.addEventListener('keydown',regionKey);root.addEventListener('focusin',regionEditing);root.addEventListener('input',regionEditing);root.addEventListener('change',regionActionListener);root.addEventListener('click',regionClickListener);}

async function assetsChanged(event) {
  const detail=event.detail;
  if(!alive||detail?.passage_id!==config.passage_id||detail?.revision_id!==config.revision_id)return;
  try{const record=await storage('get',detail.asset_id);if(!record||record.role!=='reference')return;await refreshLibrary();await selectRecord('reference',record);}
  catch(error){failure(error);}
}
async function importBytes(request) {
  try {
    if(!alive)throw Error('Reopen the speech studio before importing.');
    const limit=request.role==='restore'?150*1024*1024:32*1024*1024;
    if(typeof request.base64!=='string'||request.base64.length>Math.ceil(limit/3)*4)throw Error('The selected file is too large.');
    const bytes=Uint8Array.from(atob(request.base64),c=>c.charCodeAt(0));if(bytes.length>limit)throw Error('The selected file is too large.');
    const file=new File([bytes],request.name||'Imported audio',{type:request.mime_type||''});
    if(request.role==='restore')await restoreAudio(file);else await importFile(request.role,file);
    return {ok:true};
  }catch(error){failure(error);return {error:error.message||String(error)};}
}
function visibility() { if (document.hidden) { stopPlayback(); if(capture || starting) stopRecording('Recording stopped when Rufren left the foreground. The captured part was kept.').catch(failure); } }
function destroy() {
  if(!alive)return exitPromise;
  // Snapshot the buffers before teardown. Device tracks stop synchronously;
  // original persistence continues without touching this unmounted component.
  const take=capture;capture=null;starting=false;requestGeneration++;
  alive=false;finish();yieldQueue.splice(0).forEach(resolve=>resolve());yieldChannel?.port1.close();yieldChannel?.port2.close();
  releaseCapture();stopPlayback();
  const write=preserveExitTake(take),regionWrite=shutdownRegions();
  stretchPlayer.pause();stretchPlayer.removeAttribute('src');stretchPlayer.load();stretchPlayer.remove();for(const clip of clips.values())if(clip.playback_url)URL.revokeObjectURL(clip.playback_url);
  root.removeEventListener('click',click);root.removeEventListener('change',change);window.removeEventListener('rufren-audio-assets-changed',assetsChanged);document.removeEventListener('visibilitychange',visibility);
  window.removeEventListener('pagehide',destroy);observer.disconnect();context?.close().catch(()=>{});
  exitPromise=Promise.allSettled([...pendingWrites,write,regionWrite]).then(results=>{
    db?.close();
    const failed=results.find(r=>r.status==='rejected');
    if(failed)window.rufrenAudioExitError='An unfinished recording could not be saved. Reopen the speech studio to retry while this app remains open.';
  });
  return exitPromise;
}
const observer = new MutationObserver(()=>{ if(!root.isConnected) destroy(); }); observer.observe(document.body,{childList:true,subtree:true});
window.rufrenAudioStudio={instanceId,destroy:expected=>expected===undefined||expected===instanceId?destroy():Promise.resolve(),stopPlayback,yieldNow,importBytes};window.addEventListener('rufren-audio-assets-changed',assetsChanged); root.addEventListener('click',click); root.addEventListener('change',change);
document.addEventListener('visibilitychange',visibility); window.addEventListener('pagehide',destroy);
mic(false);
if(!preservePitchKey){
  for(const picker of [$('audio-rate'),$('audio-region-rate')].filter(Boolean))for(const option of picker.options)if(option.value!=='1')option.disabled=true;
  const support=$('audio-speed-support');if(support)support.textContent='Pitch-preserving slow playback is unavailable here. Natural-speed listening is available.';
}
try {
  await previousExit;await openDatabase();
  const recovery=window.rufrenAudioRecovery||[];
  for(const record of [...recovery]){await storage('put',record);recovery.splice(recovery.indexOf(record),1);}
  await refreshLibrary();
  if(config.selected_asset_id){const record=await storage('get',config.selected_asset_id);if(!validAttempt(record)){emit({kind:'selected',recording:null});throw Error('The exact response take is unavailable. Choose another saved take or record again.');}await selectRecord('attempt',record);}
  if(window.rufrenAudioExitError){status('The interrupted recording has been recovered on this device. Choose it from saved takes.');delete window.rufrenAudioExitError;}
  else if(config.compact)status(selected.attempt?'Your exact saved take is ready to replay.':'Record a short response, then stop to keep it. You can also choose an existing take.');
  else status('Record or import a reference voice, then make your own attempt. Originals stay on this device.'); }
catch(error) { failure(error); }
emit({kind:'ready'});
while (alive) {
  const message=await Promise.race([dioxus.recv(),lifetime.then(()=>null)]);
  if(!message)break;
  if(message.kind==='request_decoded'){const role=message.role,record=selected[role],buffer=record&&clips.get(record.asset_id)?.buffer;if(record&&buffer&&currentAnalysis(role,message.asset_id,message.request_id))await sendPcm(role,record,buffer,message.request_id,true);continue;}
  if(message.kind==='region_metrics'){regionMetrics(message);continue;}
  if(message.kind==='region_comparison'){regionComparison(message);continue;}
  if(message.kind==='region_error'){if(regionSlots[message.role]?.request===message.request_id)$(`audio-region-metrics-${message.role}`).textContent=message.error;continue;}
  if(message.kind==='analysis') {
    if(message.request_id&&!currentAnalysis(message.role,message.asset_id,message.request_id))continue;
    const spectrum=message.analysis.spectrogram;
    if(spectrum.power_db_base64) {
      const binary=atob(spectrum.power_db_base64), bytes=new Uint8Array(binary.length);
      for(let i=0;i<binary.length;i++)bytes[i]=binary.charCodeAt(i);
      const view=new DataView(bytes.buffer);spectrum.power_db=new Float32Array(bytes.length/4);
      for(let i=0;i<spectrum.power_db.length;i++)spectrum.power_db[i]=view.getFloat32(i*4,true);
      delete spectrum.power_db_base64;
    }
    if(isRegion(message.role))await acceptRegionSource(message);else{analyses[message.role]=message.analysis;draw(message.role,message.analysis);}
  }
}
