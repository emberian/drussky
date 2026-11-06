// One bounded transfer at a time. This adapter only owns its pending chunks and IDB assets.
if (!window.rufrenAssetSync) {
  const LIMIT = 32 * 1024 * 1024, CHUNK = 256 * 1024;
  const pending = new Map();
  const database = new Promise((resolve,reject) => {
    const request=indexedDB.open('rufren-audio-v1',1);
    request.onupgradeneeded=()=>request.result.createObjectStore('assets',{keyPath:'asset_id'});
    request.onsuccess=()=>resolve(request.result);
    request.onerror=()=>reject(Error('Cannot open recording storage; saved recordings were kept.'));
    request.onblocked=()=>reject(Error('Close other Rufren recording tabs and retry.'));
  });
  const metadata = record => {
    const {blob,original_mime_type,...rest}=record;
    const device=rest.device==null?{}:{...rest.device};
    if(original_mime_type!==undefined){
      if(device.source_mime_type!==undefined&&device.source_mime_type!==original_mime_type)throw Error('Conflicting source audio MIME metadata. Existing recording was kept.');
      device.source_mime_type=original_mime_type;
    }
    return {...rest,frames:rest.frames||0,interrupted:!!rest.interrupted,device,annotations:rest.annotations||[]};
  };
  const stable = value => JSON.stringify(value, (_,v) => v && typeof v==='object' && !Array.isArray(v) ? Object.fromEntries(Object.entries(v).sort(([a],[b])=>a.localeCompare(b))) : v);
  const merge = (old,incoming) => {
    if (!old) return incoming;
    const a=metadata(old),b=metadata(incoming),cues=[...a.annotations,...b.annotations];
    a.annotations=[]; b.annotations=[];
    const device={...a.device};for(const [key,value]of Object.entries(b.device)){if(key in device&&stable(device[key])!==stable(value))throw Error('Conflicting recording device provenance. Existing metadata was kept.');device[key]=value;}a.device=device;b.device=device;
    if(!a.frames)a.frames=b.frames;if(!b.frames)b.frames=a.frames;
    if(stable(a)!==stable(b))throw Error('A recording ID has conflicting metadata. Existing recordings were kept.');
    const union=new Map(cues.map(cue=>[JSON.stringify([cue.start_seconds,cue.end_seconds,cue.note,cue.created_at_ms]),cue]));
    if(union.size>256)throw Error('This recording has too many timed cues to sync. Existing cues were kept.');
    return {...a,annotations:[...union.values()].sort((x,y)=>x.start_seconds-y.start_seconds||x.end_seconds-y.end_seconds||x.created_at_ms-y.created_at_ms||x.note.localeCompare(y.note))};
  };
  const get = async id => new Promise(async (resolve,reject) => {
    try {const tx=(await database).transaction('assets','readonly');const request=tx.objectStore('assets').get(id);request.onsuccess=()=>resolve(request.result);tx.onabort=tx.onerror=()=>reject(tx.error||Error('Cannot read saved recording'));}catch(error){reject(error);}
  });
  const hash = async blob => [...new Uint8Array(await crypto.subtle.digest('SHA-256',await blob.arrayBuffer()))].map(byte=>byte.toString(16).padStart(2,'0')).join('');
  const store = async (record,blob) => {
    if(blob && (blob.size!==record.byte_length || blob.size>LIMIT || await hash(blob)!==record.content_hash))throw Error('Downloaded recording failed its SHA-256 or length check. Existing recordings were kept.');
    return new Promise(async(resolve,reject)=>{
      try {
        const tx=(await database).transaction('assets','readwrite');const assets=tx.objectStore('assets');
        const request=assets.get(record.asset_id);let result;
        request.onsuccess=()=>{try{result=merge(request.result,record);const original=blob||request.result?.blob;if(!original)throw Error('Recording bytes are not available locally.');assets.put({...result,blob:original});}catch(error){reject(error);tx.abort();}};
        tx.oncomplete=()=>resolve(result);tx.onerror=tx.onabort=()=>reject(Error(tx.error?.name==='QuotaExceededError'?'Recording storage is full. Existing recordings were kept; export audio and retry.':'Could not commit downloaded recording. Existing recordings were kept.'));
      }catch(error){reject(error);}
    });
  };
  const fetchBounded = async (args,binary=false) => {
    const controller=new AbortController();const timeout=setTimeout(()=>controller.abort(),60000);
    try {
      let body=args.body==null?undefined:JSON.stringify(args.body);
      let type='application/json';
      if(args.upload_id){const record=await get(args.upload_id);if(!record?.blob||record.blob.size>LIMIT||await hash(record.blob)!==record.content_hash)throw Error('Local recording failed its integrity check. Restore an audio backup.');body=record.blob;type='application/octet-stream';}
      const response=await fetch(args.url,{method:args.method,redirect:'error',signal:controller.signal,headers:{Authorization:'Bearer '+args.token,'X-Rufren-Relay':args.relay,...(body?{'Content-Type':type}:{})},body});
      const limit=binary?Math.min(LIMIT,args.record.byte_length):300000;let chunks=[],size=0;
      if(binary&&response.ok&&response.headers.has('Content-Length')&&Number(response.headers.get('Content-Length'))!==args.record.byte_length)throw Error('Recording response length does not match its metadata');
      if(response.body){const reader=response.body.getReader();while(true){const chunk=await reader.read();if(chunk.done)break;size+=chunk.value.length;if(size>limit){await reader.cancel();throw Error('Recording sync response exceeds its size limit');}chunks.push(chunk.value);}}
      if(binary && response.ok)return {status:response.status,blob:new Blob(chunks,{type:args.record.mime_type})};
      const bytes=new Uint8Array(size);let offset=0;for(const chunk of chunks){bytes.set(chunk,offset);offset+=chunk.length;}
      return {status:response.status,body:new TextDecoder().decode(bytes)};
    }catch(error){if(error.name==='AbortError')throw Error('Recording sync timed out. Saved recordings were kept; retry sync.');throw error;}finally{clearTimeout(timeout);}
  };
  window.rufrenAssetSync={execute:async args=>{
    switch(args.op){
      case 'inventory':return new Promise(async(resolve,reject)=>{try{const tx=(await database).transaction('assets','readonly'),records=[];let bytes=0;const cursor=tx.objectStore('assets').openCursor(IDBKeyRange.lowerBound(args.after||'',true));let more=false;cursor.onsuccess=()=>{const row=cursor.result;if(!row)return;const record=metadata(row.value),size=JSON.stringify(record).length*2;if(records.length===64||(records.length&&bytes+size>256*1024)){more=true;return;}records.push(record);bytes+=size;row.continue();};tx.oncomplete=()=>resolve({version:1,records,next_after:more?records.at(-1).asset_id:null});tx.onerror=tx.onabort=()=>reject(Error('Cannot list saved recordings'));}catch(error){reject(error);}});
      case 'get':{const record=await get(args.id);return record?metadata(record):null;}
      case 'has':{const record=await get(args.id);return !!record?.blob&&record.content_hash===args.hash&&record.blob.size===record.byte_length&&await hash(record.blob)===args.hash;}
      case 'read':{const record=await get(args.id);if(!record?.blob||args.offset<0||args.length>CHUNK||args.length<1||record.blob.size>LIMIT)throw Error('Cannot read this saved recording');const bytes=new Uint8Array(await record.blob.slice(args.offset,args.offset+args.length).arrayBuffer());let binary='';for(let i=0;i<bytes.length;i+=8192)binary+=String.fromCharCode(...bytes.subarray(i,i+8192));return btoa(binary);}
      case 'begin':{if(!args.record||!Number.isSafeInteger(args.record.byte_length)||args.record.byte_length<1||args.record.byte_length>LIMIT)throw Error('Invalid recording byte length');pending.set(args.record.asset_id,{record:args.record,chunks:[],size:0});return true;}
      case 'chunk':{const transfer=pending.get(args.id);if(!transfer||args.offset!==transfer.size||args.base64.length>CHUNK*2)throw Error('Invalid recording transfer offset');const bytes=Uint8Array.from(atob(args.base64),c=>c.charCodeAt(0));if(bytes.length>CHUNK||transfer.size+bytes.length>Math.min(LIMIT,transfer.record.byte_length))throw Error('Recording transfer exceeds declared size');transfer.chunks.push(bytes);transfer.size+=bytes.length;return true;}
      case 'finish':{const transfer=pending.get(args.id);if(!transfer)throw Error('Recording transfer was interrupted; retry sync');try{return await store(transfer.record,new Blob(transfer.chunks,{type:transfer.record.mime_type}));}finally{pending.delete(args.id);}}
      case 'merge':return await store(args.record,null);
      case 'http':return await fetchBounded(args);
      case 'download':{const result=await fetchBounded(args,true);if(result.status<200||result.status>=300)throw Error('Recording download returned '+result.status+': '+(result.body||'').slice(0,1024));return await store(args.record,result.blob);}
      default:throw Error('Unknown recording storage operation');
    }
  }};
}
try { return {value:await window.rufrenAssetSync.execute(request)}; }
catch(error) { return {error:error.message||String(error)}; }
