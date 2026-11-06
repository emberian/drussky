// Bounded installation of packaged originals. No network, decoding, generation,
// or learning facts: the caller registers the returned exact source metadata.
// `request.models` contains immutable metadata and original encoded WAV bytes.
let db;
try {
  if(!Array.isArray(request.models)||request.models.length<2||request.models.length>4)throw Error('Invalid Sound Focus model collection.');
  if(!crypto.subtle||!indexedDB)throw Error('Saved sound examples need secure, durable audio storage.');
  const models=[];let total=0;
  const hash=async blob=>Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',await blob.arrayBuffer())),n=>n.toString(16).padStart(2,'0')).join('');
  const immutable=['asset_id','content_hash','role','name','passage_id','revision_id','duration_ms','sample_rate','channels','frames','byte_length','created_at_ms','mime_type','interrupted'];
  // Older installs may lack verified source-envelope fields. Merge new keys
  // only; an existing value can never be replaced by a different value.
  const object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
  const compatible=(actual,expected)=>actual===undefined||expected===undefined?true:object(actual)&&object(expected)?Object.entries(expected).every(([k,v])=>compatible(actual[k],v)):JSON.stringify(actual)===JSON.stringify(expected);
  const merge=(actual,expected)=>object(actual)&&object(expected)?Object.fromEntries([...new Set([...Object.keys(actual),...Object.keys(expected)])].map(k=>[k,merge(actual[k],expected[k])])):actual===undefined?expected:actual;
  const matches=(actual,expected)=>immutable.every(k=>actual[k]===expected[k])&&compatible(actual.device,expected.device);
  for(const item of request.models){
    const r=item.recording;
    if(!r||r.role!=='reference'||r.interrupted||!r.asset_id||!r.passage_id||!r.revision_id||!/^[0-9a-f]{64}$/.test(r.content_hash)||r.mime_type!=='audio/wav'||!Number.isInteger(r.byte_length)||r.byte_length<44||r.byte_length>512*1024||typeof item.wav_base64!=='string'||item.wav_base64.length>Math.ceil(r.byte_length/3)*4)throw Error('A packaged sound example has invalid metadata.');
    total+=r.byte_length;if(total>2*1024*1024)throw Error('The sound example collection exceeds its storage bound.');
    const binary=atob(item.wav_base64);if(binary.length!==r.byte_length)throw Error('A packaged example has a different original length.');
    const bytes=Uint8Array.from(binary,c=>c.charCodeAt(0)),blob=new Blob([bytes],{type:'audio/wav'});
    if(await hash(blob)!==r.content_hash)throw Error('A packaged sound example failed its integrity check.');
    if(models.some(m=>m.asset_id===r.asset_id))throw Error('The collection repeats an example identity.');
    models.push({...r,blob});
  }
  db=await new Promise((resolve,reject)=>{const r=indexedDB.open('rufren-audio-v1',1);r.onupgradeneeded=()=>r.result.createObjectStore('assets',{keyPath:'asset_id'});r.onsuccess=()=>resolve(r.result);r.onerror=()=>reject(r.error);r.onblocked=()=>reject(Error('Close another audio tab, then retry.'));});
  // Verify originals already installed without overwriting notes or provenance.
  for(const model of models){const old=await new Promise((resolve,reject)=>{const r=db.transaction('assets').objectStore('assets').get(model.asset_id);r.onsuccess=()=>resolve(r.result);r.onerror=()=>reject(r.error);});
    if(old&&(!matches(old,model)||!(old.blob instanceof Blob)||old.blob.size!==model.byte_length||await hash(old.blob)!==model.content_hash))throw Error('An existing model has conflicting metadata or damaged original bytes. Its saved data was kept.');
  }
  await new Promise((resolve,reject)=>{const tx=db.transaction('assets','readwrite'),store=tx.objectStore('assets');let problem;
    for(const model of models){const r=store.get(model.asset_id);r.onsuccess=()=>{if(r.result){if(!matches(r.result,model)){problem=Error('A concurrent sound model has different metadata.');tx.abort();}else store.put({...r.result,device:merge(r.result.device,model.device)});}else store.add(model);};}
    tx.oncomplete=resolve;tx.onerror=()=>reject(problem||tx.error);tx.onabort=()=>reject(problem||tx.error||Error('The examples could not be saved.'));
  });
  for(const {blob,...recording}of models)window.dispatchEvent(new CustomEvent('rufren-audio-assets-changed',{detail:recording}));
  return {records:models.map(({blob,...recording})=>recording)};
}catch(error){return {error:error.message||String(error)};}
finally{db?.close();}
