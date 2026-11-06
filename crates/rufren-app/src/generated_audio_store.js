// Invoked only after Rust checks the exact request/revision/text and WAV shape.
try {
  const record=request.recording, binary=atob(request.wav_base64);
  if(binary.length!==record.byte_length||binary.length>4*1024*1024)throw Error('Generated audio exceeds its bounded length.');
  const bytes=Uint8Array.from(binary,c=>c.charCodeAt(0)),blob=new Blob([bytes],{type:'audio/wav'});
  if(!crypto.subtle)throw Error('Secure storage for generated audio needs HTTPS, localhost, or a supported native WebView.');
  const hash=[...new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))].map(b=>b.toString(16).padStart(2,'0')).join('');
  if(hash!==record.content_hash)throw Error('Generated audio failed its SHA-256 check. No audio was replaced.');
  const db=await new Promise((resolve,reject)=>{const r=indexedDB.open('rufren-audio-v1',1);r.onupgradeneeded=()=>r.result.createObjectStore('assets',{keyPath:'asset_id'});r.onsuccess=()=>resolve(r.result);r.onerror=()=>reject(r.error);r.onblocked=()=>reject(Error('Close another recording tab, then retry.'));});
  try{await new Promise((resolve,reject)=>{const tx=db.transaction('assets','readwrite');tx.objectStore('assets').add({...record,blob});tx.oncomplete=resolve;tx.onabort=tx.onerror=()=>reject(tx.error||Error('Generated audio could not be saved.'));});}
  finally{db.close();}
  window.dispatchEvent(new CustomEvent('rufren-audio-assets-changed',{detail:{asset_id:record.asset_id,revision_id:record.revision_id,passage_id:record.passage_id}}));
  return {recording:record};
}catch(error){return {error:error.message||String(error)};}
