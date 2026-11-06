// `payload` comes from export_json. Text is never reparsed/pretty-printed here:
// whitespace, Unicode, and the resulting file bytes must remain exact.
try{
  const blob=new Blob([payload.text],{type:'application/json'});
  if(blob.size!==payload.byte_length)throw Error('The JSON export has an unexpected byte length. Your original is kept.');
  if(!crypto.subtle)throw Error('Use HTTPS, localhost, or the native app to export verified JSON.');
  const digest=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',await blob.arrayBuffer())),n=>n.toString(16).padStart(2,'0')).join('');
  if(digest!==payload.sha256)throw Error('The JSON export failed its integrity check. Your original is kept.');
  return await window.rufrenFileExports.save(blob,payload.filename,{native:payload.native,post:request=>{
    dioxus.send({kind:'export_file',export_request:request});return true;
  }});
}catch(error){return {status:'failed',message:error.message||String(error)};}
