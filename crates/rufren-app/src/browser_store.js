// IndexedDB is durable; localStorage is a synchronous write-ahead journal.
// Editing metadata belongs to one Web-Lock-owned browser workspace, never another tab.
if (!window.rufrenLibrary) {
  const prefix='rufren.v1.',pendingPrefix='rufren.journal.v1.',workspacePrefix=prefix+'workspace.';
  const workspaceJournal='rufren.journal.workspace.v1.',workspaceToken='rufren.workspace.v1',lockPrefix='rufren-workspace-v1.';
  const cache=new Map(),eventVersions=new Map(),heldWorkspaces=new Set(),teacherRecoveryWorkspaces=new Set();let eventVersion=0,db,flushing=false,failure=null,workspace=null;
  const channel=typeof BroadcastChannel==='function'?new BroadcastChannel('rufren-library'):null;
  const scoped=name=>['session','session_plan','paused_practice_ids','active_passage_draft','passage_draft_ids','selected_lesson','tutor_response','teacher_response','teacher_lesson_response','teacher_request','teacher_context_prepared','teacher_notebook_selection','teacher_revisit_active','teacher_surface','teacher_dialogue_state','teacher_dialogue_new_scene','daily_plan_active','daily_plan_settings','practice_surface','sound_focus_active','sound_focus_paused_ids','reader_selection','reader_import_draft'].includes(name)||name.startsWith('reader_state:')||name.startsWith('sound_focus_session:')||name.startsWith('paused_practice.')||name.startsWith('passage_draft.')||name.startsWith('dialogue_transcript_draft:')||name.startsWith('dialogue_audio_draft:')||name.startsWith('teacher_revisit:')||name.startsWith('teacher_notebook_cursor:')||name.startsWith('teacher_practice:')||name.startsWith('transcript_draft:');
  const scopeKey=(id,name)=>workspacePrefix+id+'.meta.'+name;
  const infoKey=id=>workspacePrefix+id+'.info';
  const validId=id=>typeof id==='string'&&/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(id);
  const physical=key=>{
    const name=key.startsWith(prefix+'meta.')?key.slice((prefix+'meta.').length):null;
    if(name!==null&&scoped(name)){if(!workspace)throw Error('Saved window is still opening');return scopeKey(workspace,name);}
    return key;
  };
  const readPending=()=>{
    const entries=new Map(),sources=new Map(),keys=[];
    for(let i=0;i<localStorage.length;i++){const key=localStorage.key(i);if(key)keys.push(key);}
    // Legacy physical entries precede journals irrespective of Storage.key enumeration order.
    for(const key of keys)if(key.startsWith(prefix)){const raw=localStorage.getItem(key);if(raw!==null){entries.set(key,raw);sources.set(key,raw);}}
    for(const key of keys){const raw=localStorage.getItem(key);if(raw===null)continue;
      if(key.startsWith(pendingPrefix)){entries.set(key.slice(pendingPrefix.length),JSON.parse(raw));sources.set(key,raw);}
      if(key.startsWith(workspaceJournal)){for(const [target,value]of JSON.parse(raw))entries.set(target,value);sources.set(key,raw);}
    }
    return {entries,sources};
  };
  const overlay=entries=>{
    const batch=new Map(entries),stale=new Set();
    for(const[key,value]of batch)if(key.startsWith(workspacePrefix)&&key.endsWith('.info')&&value!==null){
      const incoming=JSON.parse(value),known=cache.has(key)?JSON.parse(cache.get(key)):null;
      if(known&&(incoming.revision||0)<(known.revision||0))stale.add(incoming.id);
    }
    for(const[key,value]of batch){
      if(key.startsWith(workspacePrefix)&&stale.has(key.slice(workspacePrefix.length).split('.')[0]))continue;
      if((cache.get(key)??null)===value)continue;
      value===null?cache.delete(key):cache.set(key,value);
      if(key.startsWith(prefix+'event.'))eventVersions.set(key,++eventVersion);
    }
  };
  const flush=async()=>{
    if(flushing||!db||failure)return;flushing=true;
    try{while(readPending().entries.size){let journal;
      await new Promise((resolve,reject)=>{
        const tx=db.transaction('entries','readwrite'),store=tx.objectStore('entries'),barrier=store.get('__rufren_transaction_barrier__');
        barrier.onsuccess=()=>{journal=readPending();for(const[key,value]of journal.entries){
          if(value===null)store.delete(key);
          else if(key.startsWith(prefix+'event.')){const read=store.get(key);read.onsuccess=()=>{if(read.result!==undefined&&read.result!==value){failure='Conflicting immutable event in browser storage; export your notebook before retrying.';tx.abort();}else store.put(value,key);};}
          else store.put(value,key);
        }};
        tx.oncomplete=resolve;tx.onabort=tx.onerror=()=>reject(Error(failure||tx.error?.message||'IndexedDB could not commit'));
      });
      // Delete only exactly the source bytes committed by this transaction.
      for(const[key,raw]of journal.sources)if(localStorage.getItem(key)===raw)localStorage.removeItem(key);
      channel?.postMessage([...journal.entries]);
    }}catch(error){console.warn('Rufren retained its storage journal:',error.message);}finally{flushing=false;}
  };
  const writeRaw=(key,value)=>{if(failure)throw Error(failure);localStorage.setItem(pendingPrefix+key,JSON.stringify(value));overlay([[key,value]]);void flush();};
  const info=()=>JSON.parse(cache.get(infoKey(workspace))||JSON.stringify({version:1,id:workspace,created_at_ms:Date.now(),last_used_ms:0}));
  const writeWorkspace=(entries)=>{
    if(failure)throw Error(failure);if(!workspace||!heldWorkspaces.has(workspace))throw Error('This saved window is no longer writable. Reopen Rufren.');
    const descriptor=info();descriptor.last_used_ms=Math.max(Date.now(),descriptor.last_used_ms+1);descriptor.revision=(descriptor.revision||0)+1;
    const batchKey=workspaceJournal+workspace,pending=new Map(JSON.parse(localStorage.getItem(batchKey)||'[]'));
    for(const[key,value]of entries)pending.set(key,value);pending.set(infoKey(workspace),JSON.stringify(descriptor));
    // One Storage.setItem commits a workspace change + its recovery descriptor together.
    localStorage.setItem(batchKey,JSON.stringify([...pending]));overlay(pending);void flush();
  };
  const databaseEntries=async starts=>new Promise((resolve,reject)=>{
    const entries=new Map(),tx=db.transaction('entries','readonly'),cursor=tx.objectStore('entries').openCursor();
    cursor.onsuccess=()=>{const row=cursor.result;if(row){if(!starts||row.key.startsWith(starts))entries.set(row.key,row.value);row.continue();}};
    tx.oncomplete=()=>resolve(entries);tx.onabort=tx.onerror=()=>reject(Error('Cannot read saved browser windows. Existing data was kept.'));
  });
  const claim=id=>new Promise((resolve,reject)=>{
    if(heldWorkspaces.has(id)){resolve(true);return;}
    navigator.locks.request(lockPrefix+id,{ifAvailable:true},async lock=>{
      if(!lock){resolve(false);return;}heldWorkspaces.add(id);resolve(true);
      // The browser releases this ownership when the document is destroyed, including crashes.
      await new Promise(()=>{});
    }).catch(reject);
  });
  const descriptors=entries=>{const records=[...entries].filter(([key])=>key.startsWith(workspacePrefix)&&key.endsWith('.info')).map(([,json])=>JSON.parse(json));for(const record of records)if(record.version!==1||!validId(record.id)||!Number.isSafeInteger(record.last_used_ms)||record.last_used_ms<0||!Number.isSafeInteger(record.revision||0))throw Error('A saved window has an unsupported format. Update Rufren before reopening it. Existing work was kept.');return records.sort((a,b)=>b.last_used_ms-a.last_used_ms||a.id.localeCompare(b.id));};
  const initializeWorkspace=async()=>{
    if(!navigator.locks?.request||!navigator.locks?.query)throw Error('This browser cannot protect drafts across open tabs. Use a current browser with Web Locks enabled. Your saved data was kept.');
    // Validate tab storage before selecting a durable workspace.
    const desired=sessionStorage.getItem(workspaceToken);sessionStorage.setItem(workspaceToken,desired||'');
    await navigator.locks.request('rufren-workspace-migration-v1',async()=>{
      // Refresh the entire bootstrap snapshot under the migration lock, including
      // deleted legacy pointers; a second upgrading tab must not copy a stale session.
      const snapshot=await databaseEntries();cache.clear();eventVersions.clear();eventVersion=0;
      overlay(snapshot);overlay(readPending().entries);
      const candidates=descriptors(cache),known=new Set(candidates.map(record=>record.id));
      if(validId(desired)&&known.has(desired)&&await claim(desired))workspace=desired;
      if(!workspace)for(const candidate of candidates)if(await claim(candidate.id)){workspace=candidate.id;break;}
      if(!workspace){workspace=crypto.randomUUID();if(!await claim(workspace))throw Error('Cannot claim a private saved window. Retry opening Rufren.');}
      // Re-read only after exclusive ownership; a recovered window cannot overwrite a stale snapshot.
      const fresh=await databaseEntries(workspacePrefix+workspace+'.');
      for(const key of [...cache.keys()])if(key.startsWith(workspacePrefix+workspace+'.'))cache.delete(key);
      overlay(fresh);overlay(readPending().entries);
      sessionStorage.setItem(workspaceToken,workspace);
      const migration=[];
      for(const[key,value]of cache){if(key.startsWith(prefix+'meta.')){const name=key.slice((prefix+'meta.').length);if(scoped(name)){const target=scopeKey(workspace,name);if(!cache.has(target))migration.push([target,value]);migration.push([key,null]);}}}
      writeWorkspace(migration);
      const locks=await navigator.locks.query(),busy=new Set(locks.held.map(lock=>lock.name).filter(name=>name.startsWith(lockPrefix)).map(name=>name.slice(lockPrefix.length)));
      for(const record of descriptors(cache))if(record.id===workspace||!busy.has(record.id))teacherRecoveryWorkspaces.add(record.id);
    });
  };
  const ready=new Promise((resolve,reject)=>{
    if(!window.indexedDB){reject(Error('IndexedDB is unavailable; your existing data was kept.'));return;}
    const open=indexedDB.open('rufren-library',1);open.onupgradeneeded=()=>open.result.createObjectStore('entries');
    open.onerror=()=>reject(Error('Cannot open durable browser storage; your existing data was kept.'));
    open.onblocked=()=>reject(Error('Close other Rufren tabs and reopen to finish updating storage.'));
    open.onsuccess=async()=>{try{
      db=open.result;db.onversionchange=()=>{db.close();failure='Storage was updated in another tab. Reopen Rufren.';};
      overlay(await databaseEntries());overlay(readPending().entries);await initializeWorkspace();await flush();if(failure)throw Error(failure);resolve(true);
    }catch(error){reject(error);}};
  });
  const savedWindows=async()=>{
    await ready;const entries=await databaseEntries(workspacePrefix);for(const[key,value]of readPending().entries){if(key.startsWith(workspacePrefix))value===null?entries.delete(key):entries.set(key,value);}
    const locks=await navigator.locks.query(),active=new Set(locks.held.map(lock=>lock.name).filter(name=>name.startsWith(lockPrefix)).map(name=>name.slice(lockPrefix.length)));
    const parse=(id,name)=>{const value=entries.get(scopeKey(id,name));return value===undefined?null:JSON.parse(value);};
    return descriptors(entries).map(record=>{
      const session=parse(record.id,'session'),draftId=parse(record.id,'active_passage_draft'),draft=draftId?parse(record.id,'passage_draft.'+draftId):null;
      return {id:record.id,last_used_ms:record.last_used_ms,last_used_label:new Date(record.last_used_ms).toLocaleString(),active:active.has(record.id),current:record.id===workspace,practice_response:session?.phase!=='complete'?(session?.draft||null):null,passage_title:draft?.editor?.title||null,paused_count:parse(record.id,'paused_practice_ids')?.length||0,passage_count:parse(record.id,'passage_draft_ids')?.length||0};
    });
  };
  window.rufrenLibrary={
    ready,
    entries:()=>{overlay(readPending().entries);return [...cache];},
    eventsAfter:after=>{overlay(readPending().entries);return {revision:eventVersion,entries:[...eventVersions].filter(([,version])=>version>after).map(([key])=>[key,cache.get(key)])};},
    get:key=>{overlay(readPending().entries);return cache.get(physical(key))??null;},
    write:(key,value)=>{const target=physical(key);if(target!==key)writeWorkspace([[target,value]]);else writeRaw(key,value);},
    // Read-only legacy recovery: current workspace plus inactive workspaces observed at
    // bootstrap. Another live window retains sole ownership of its mutable draft slots.
    teacherCaches:()=>{
      overlay(readPending().entries);const roots=[prefix+'meta.',...[...teacherRecoveryWorkspaces].map(id=>workspacePrefix+id+'.meta.')],result=[];
      const decode=raw=>{try{return JSON.parse(raw);}catch{return null;}};
      for(const root of roots)for(const name of ['teacher_response','teacher_lesson_response']){
        const raw=cache.get(root+name);if(raw===undefined||raw==='null')continue;
        result.push({response:decode(raw),prepared_context:decode(cache.get(root+'teacher_context_prepared')||'null')});
      }
      return result;
    },
    workspaces:savedWindows,
    switchWorkspace:async id=>{await ready;const known=await databaseEntries(workspacePrefix);for(const[key,value]of readPending().entries)if(key.startsWith(workspacePrefix))value===null?known.delete(key):known.set(key,value);if(!validId(id)||!descriptors(known).some(record=>record.id===id))throw Error('That saved window is unavailable. Refresh the list.');if(id!==workspace&&!await claim(id))throw Error('This saved window is open in another tab. Close it there before reopening it here.');sessionStorage.setItem(workspaceToken,id);return true;},
    flush,
  };
  channel&&(channel.onmessage=event=>{overlay(event.data);overlay(readPending().entries);});
  window.addEventListener('storage',()=>{overlay(readPending().entries);void flush();});window.addEventListener('online',()=>void flush());
  navigator.storage?.persist?.().catch(()=>{});
}
await window.rufrenLibrary.ready;
return true;
