// Playwright tool function; isolated contexts stand for two devices, no user data touched.
async (page) => {
  const {readFile}=await import('node:fs/promises');
  const {createHash}=await import('node:crypto');
  const source=await readFile('crates/rufren-app/src/asset_sync.js','utf8');
  const browser=page.context().browser();
  const contexts=await Promise.all([browser.newContext(),browser.newContext()]);
  const bytes=Buffer.from('original encoded audio transport fixture');
  const hash=createHash('sha256').update(bytes).digest('hex');
  const record={asset_id:'take-a',content_hash:hash,role:'attempt',name:'Synthetic fixture',passage_id:'',revision_id:'',duration_ms:100,sample_rate:48000,channels:1,frames:4800,byte_length:bytes.length,created_at_ms:1,mime_type:'audio/wav',interrupted:false,device:{channelCount:1,sampleRate:48000},annotations:[]};
  let evilRequests=0;
  try {
    for(const context of contexts)await context.route('**/*',async route=>{
      const url=new URL(route.request().url());
      if(url.hostname==='unexpected.test'){evilRequests++;return route.fulfill({body:'unexpected'});}
      if(url.pathname==='/redirect')return route.fulfill({status:307,headers:{Location:'https://unexpected.test/stolen'}});
      if(url.pathname==='/oversized')return route.fulfill({body:'x'.repeat(300001)});
      if(url.pathname==='/blob')return route.fulfill({contentType:'application/octet-stream',body:bytes});
      if(url.pathname==='/corrupt')return route.fulfill({contentType:'application/octet-stream',body:Buffer.from('wrong bytes')});
      return route.fulfill({contentType:'text/html',body:'<!doctype html><title>Audio sync test</title>'});
    });
    const [a,b]=await Promise.all(contexts.map(context=>context.newPage()));
    await Promise.all([a.goto('https://audio-sync.test/'),b.goto('https://audio-sync.test/')]);
    const call=(target,request)=>target.evaluate(async({source,request})=>await new (Object.getPrototypeOf(async function(){}).constructor)('request',source)(request),{source,request});
    const good=async(target,request)=>{const result=await call(target,request);if(result.error)throw Error(result.error);return result.value;};
    await good(a,{op:'begin',record});await good(a,{op:'chunk',id:record.asset_id,offset:0,base64:bytes.toString('base64')});await good(a,{op:'finish',id:record.asset_id});
    if(!await good(a,{op:'has',id:'take-a',hash}))throw Error('Native chunk bridge failed original hash verification');
    if(await good(b,{op:'get',id:'take-a'})!==null)throw Error('Contexts did not isolate devices');
    await good(b,{op:'download',url:'https://audio-sync.test/blob',method:'GET',token:'synthetic-test-key',relay:'synthetic-relay',record});
    if(!await good(b,{op:'has',id:'take-a',hash}))throw Error('Cross-device download failed');
    const cueA={start_seconds:.01,end_seconds:.04,note:'First device',created_at_ms:2},cueB={start_seconds:.04,end_seconds:.08,note:'Second device',created_at_ms:3};
    await good(b,{op:'merge',record:{...record,annotations:[cueA]}});
    const merged=await good(b,{op:'merge',record:{...record,device:{sampleRate:48000,channelCount:1},annotations:[cueB]}});
    if(merged.annotations.length!==2)throw Error('Concurrent timed cues were lost');
    const conflict=await call(b,{op:'merge',record:{...record,name:'Changed identity'}});if(!conflict.error)throw Error('Metadata identity collision accepted');
    const corrupted=await call(b,{op:'download',url:'https://audio-sync.test/corrupt',method:'GET',token:'synthetic-test-key',relay:'synthetic-relay',record});if(!corrupted.error)throw Error('Corrupted transfer accepted');
    if(!await good(b,{op:'has',id:'take-a',hash}))throw Error('Corrupted download replaced saved original');
    const oversized=await call(b,{op:'http',url:'https://audio-sync.test/oversized',method:'GET',token:'synthetic-test-key',relay:'synthetic-relay'});if(!oversized.error)throw Error('Oversized metadata response accepted');
    const redirect=await call(b,{op:'http',url:'https://audio-sync.test/redirect',method:'GET',token:'synthetic-test-key',relay:'synthetic-relay'});if(!redirect.error||evilRequests)throw Error('Redirect could expose recording credentials');
    const tooLarge=await call(b,{op:'begin',record:{...record,asset_id:'too-large',byte_length:32*1024*1024+1}});if(!tooLarge.error)throw Error('Unbounded incoming transfer accepted');
    const takeB={...record,asset_id:'take-b',sample_rate:44100,frames:4410};
    await good(b,{op:'download',url:'https://audio-sync.test/blob',method:'GET',token:'synthetic-test-key',relay:'synthetic-relay',record:takeB});
    const inventory=await good(b,{op:'inventory',after:''});if(inventory.records.length!==2||inventory.records[1].sample_rate!==44100)throw Error('Distinct take with identical original bytes was collapsed');
    const legacy={...record,asset_id:'take-legacy',original_mime_type:'audio/x-wav'};
    await good(b,{op:'begin',record:legacy});await good(b,{op:'chunk',id:legacy.asset_id,offset:0,base64:bytes.toString('base64')});await good(b,{op:'finish',id:legacy.asset_id});
    const normalized=await good(b,{op:'get',id:legacy.asset_id});if('original_mime_type' in normalized||normalized.device.source_mime_type!=='audio/x-wav')throw Error('Transitional MIME field leaked into strict transport DTO');
    const extended=await good(b,{op:'merge',record:{...normalized,device:{...normalized.device,source_note:'header-verified'}}});if(extended.device.source_mime_type!=='audio/x-wav')throw Error('Optional device provenance extension erased existing observation');
    const conflicting=await call(b,{op:'merge',record:{...normalized,device:{...normalized.device,source_mime_type:'audio/mp4'}}});if(!conflicting.error)throw Error('Device provenance conflict was silently overwritten');
    return {passed:['native chunk bridge', 'two isolated device downloads', 'SHA256 corruption rejection', 'existing original preserved', 'concurrent cue union', 'metadata order normalization', 'immutable take identity', 'response/transfer bounds', 'redirect rejection', 'distinct same-byte takes','transitional MIME normalization','append-only device provenance']};
  }finally{await Promise.all(contexts.map(context=>context.close()));}
}
