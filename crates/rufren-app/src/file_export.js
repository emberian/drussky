// Reusable renderer-side export lifetime. Native receives a registered Blob URL,
// never a renderer-supplied filesystem destination or a giant base64 payload.
if(!window.rufrenFileExports){
  const pending=new Map(),limit=150*1024*1024;
  const validName=name=>typeof name==='string'&&name.length>0&&name.length<=180&&!name.startsWith('.')&&!/[\x00-\x1f\x7f/\\<>:"|?*]/.test(name);
  const terminal=new Set(['saved','shared','cancelled','failed']);
  function complete(result){
    const entry=pending.get(result?.request_id);if(!entry||!terminal.has(result.status))return false;
    pending.delete(result.request_id);URL.revokeObjectURL(entry.request.blob_url);entry.resolve(result);return true;
  }
  window.rufrenFileExports={
    owns:request=>{const entry=pending.get(request?.request_id);return !!entry&&Object.keys(entry.request).every(key=>entry.request[key]===request[key])&&Object.keys(request).length===Object.keys(entry.request).length;},
    complete,
    async save(blob,name,{native=false,post}={}){
      if(!(blob instanceof Blob)||!blob.size||blob.size>limit)throw Error('Choose an export under 150 MiB. Sync your originals or export individual recordings if the backup is larger.');
      if(!validName(name))throw Error('Choose a plain export filename without folders or special path characters.');
      if(!native){
        const url=URL.createObjectURL(blob),link=document.createElement('a');link.href=url;link.download=name;link.click();setTimeout(()=>URL.revokeObjectURL(url),30000);
        return {status:'requested',message:'Download requested. Check your browser’s downloads for '+name+'.'};
      }
      if(pending.size)throw Error('Finish or cancel the current export first.');
      if(!crypto.subtle||typeof post!=='function')throw Error('Native export is unavailable in this window. Your saved audio is kept.');
      const hash=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',await blob.arrayBuffer())),n=>n.toString(16).padStart(2,'0')).join('');
      if(pending.size)throw Error('Finish or cancel the current export first.');
      const request={request_id:crypto.randomUUID(),blob_url:URL.createObjectURL(blob),name,mime_type:blob.type||'application/octet-stream',byte_length:blob.size,sha256:hash};
      return await new Promise(resolve=>{
        pending.set(request.request_id,{request,resolve});
        try{if(post(request)!==true)complete({request_id:request.request_id,status:'cancelled',message:'Export cancelled because its window closed. Your original is kept.'});}
        catch(error){complete({request_id:request.request_id,status:'failed',message:error.message||'The native export could not start.'});}
      });
    }
  };
}
