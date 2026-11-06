const test=require('node:test');const assert=require('node:assert/strict');const {InkSession,model}=require('../crates/rufren-app/src/handwriting.js');
const config={scope_id:'workspace-task-session',task_id:'hear-a-write-a',task_revision:'1',target:'А',initial_mode:'hidden',audio_id:'foundation-a',audio_role:'model'};
const point=(x=20,y=50)=>({x,y,time_ms:123.5,pressure:.65,tilt_x:12,tilt_y:-7,twist:30,width:2,height:2});
function repository(){const drafts=[];const originals=new Map();return {drafts,originals,async saveDraft(d){drafts.push(structuredClone(d))},async saveOriginal(o){const prior=originals.get(o.attempt_id);if(prior)assert.deepEqual(prior,o,'retry must use identical identity and content');originals.set(o.attempt_id,structuredClone(o))}}}
let count=0;function ink(adapter=repository(),extra={}){const events=[];const session=new InkSession({...config,...extra},adapter,e=>events.push(e),()=>`attempt-${++count}`,()=>123456+count);return {session,adapter,events}}
test('pen-only filtering, pressure/raw samples and interrupted resume',async()=>{const {session:s,adapter}=ink();assert.equal(s.begin(1,'touch',point()),false);assert.equal(s.begin(2,'pen',point()),true);s.move(2,[point(30,40),point(40,30)]);await s.flush();const restored=ink(adapter,{initial:structuredClone(s.draft)}).session;assert.equal(restored.active,null);assert.equal(restored.draft.strokes[0].interrupted,true);assert.deepEqual(restored.draft.strokes[0].points,[point(),point(30,40),point(40,30)]);});
test('immutable original is acknowledged before saved/reveal, distinct from draft edits',async()=>{const r=repository();let release;const original=r.saveOriginal;r.saveOriginal=async o=>{await new Promise(resolve=>release=resolve);await original(o)};const {session:s,events}=ink(r);s.begin(1,'pen',point());s.end(1);const saving=s.compare();await new Promise(resolve=>setImmediate(resolve));assert.equal(s.draft.sealed,false);assert.equal(events.some(e=>e.kind==='saved'),false);assert.equal(r.originals.size,0);assert.equal(s.begin(2,'pen',point()),false);release();const snapshot=await saving;assert.equal(snapshot.mode,'hidden');assert.equal(events.at(-1).kind,'saved');assert.equal(r.originals.size,1);await s.newTry();s.begin(3,'pen',point(80,80));s.end(3);await s.undo();assert.equal(s.draft.strokes.length,0);s.begin(4,'pen',point(90,90));s.end(4);await s.clear();assert.equal(s.draft.strokes.length,0);assert.deepEqual([...r.originals.values()][0],snapshot);assert.ok(s.draft.aids.some(a=>a.kind==='previous_model_this_activity'));});
test('partial original write/ack failure recovers frozen UUID and content',async()=>{const r=repository();const original=r.saveOriginal;let fail=true;r.saveOriginal=async o=>{await original(o);if(fail){fail=false;throw Error('simulated lost acknowledgment')}};const {session:s}=ink(r);s.begin(1,'pen',point());s.end(1);await assert.rejects(s.compare(),/acknowledgment/);const reserved=structuredClone(s.draft.pending_snapshot);assert.equal(s.draft.sealed,false);assert.equal(s.mode.bind(s,'copy') instanceof Function,true);assert.throws(()=>s.mode('copy'),/Finish saving/);const restored=ink(r,{initial:structuredClone(s.draft)}).session;const saved=await restored.compare();assert.deepEqual(saved,reserved);assert.equal(r.originals.size,1);});
test('failed draft save keeps model hidden and original uncommitted',async()=>{const r=repository();r.saveDraft=async()=>{throw Error('quota')};const {session:s,events}=ink(r);s.begin(1,'pen',point());s.end(1);await assert.rejects(s.compare(),/could not be saved/);assert.equal(s.draft.sealed,false);assert.equal(r.originals.size,0);assert.equal(events.some(e=>e.kind==='saved'),false);});
test('independent cues retain actual requested/completed audio and visual help history',async()=>{const {session:s}=ink(undefined,{audio_role:'task_input'});await s.cue('meaning',false);await s.cue('sound',false);await s.cue('sound',true);await s.listen();assert.ok(s.draft.aids.some(a=>a.kind==='sound_task_input_requested'));assert.ok(!s.draft.aids.some(a=>a.kind==='sound_task_input_completed'));await s.audioResult(s.draft.attempt_id,true);await s.mode('trace');await s.mode('hidden');assert.ok(s.draft.aids.some(a=>a.kind==='visual_model'&&a.value==='trace'));assert.ok(s.draft.aids.some(a=>a.kind==='sound_task_input_completed'));assert.equal(s.draft.mode,'hidden');});
test('authored printable shapes reject unknown letters instead of invented tracing',()=>{assert.ok(model('Аа Мм Оо Кк Тт Дд.').length>10);assert.throws(()=>model('Ж'),/authored/)});

test('new dictation round restores hidden model and rejects stale audio completion',async()=>{const {session:s}=ink();const old=s.draft.attempt_id;await s.mode('copy');s.begin(1,'pen',point());s.end(1);await s.compare();await s.newTry();assert.equal(s.draft.mode,'hidden');assert.ok(s.draft.aids.some(a=>a.kind==='previous_model_this_activity'));assert.equal(await s.audioResult(old,true),false);assert.ok(!s.draft.aids.some(a=>a.kind==='sound_task_input_completed'));});

test('sound-to-writing comparison needs actual completed task input',async()=>{const {session:s,adapter}=ink(undefined,{audio_role:'task_input'});s.begin(1,'pen',point());s.end(1);await assert.rejects(s.compare(),/Listen to the sound first/);assert.equal(adapter.originals.size,0);await s.audioResult(s.draft.attempt_id,true);await s.compare();assert.equal(adapter.originals.size,1);});


test('alphabet renderer is explicit and preserves historical model paths',()=>{
 const api=require('../crates/rufren-app/src/handwriting.js');
 const old=model('Аа Мм.');
 assert.throws(()=>model('Я'),/authored/);
 const alphabet='АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯ';
 for(const glyph of alphabet+alphabet.toLowerCase()+'?—.') assert.ok(api.model(glyph,undefined,api.ALPHABET_MODEL).length,glyph);
 assert.deepEqual(model('Аа Мм.'),old);
 const {session:s}=ink(undefined,{target:'Я',model_version:api.ALPHABET_MODEL});
 assert.equal(s.draft.model_version,api.ALPHABET_MODEL);
 assert.throws(()=>api.validate(s.draft,{...config,target:'Я'}),/different task/);
});
test('successful draft retry resolves only persistence failure and retains exact ink',async()=>{
 const r=repository();const save=r.saveDraft;let fail=true;
 r.saveDraft=async d=>{if(fail)throw Error('journal full');await save(d)};
 const {session:s,events}=ink(r);s.begin(1,'pen',point());s.end(1);await s.tail;
 assert.equal(s.failed,true);assert.ok(events.some(e=>e.kind==='persistence_error'));
 const before=structuredClone(s.draft);fail=false;await s.persist();
 assert.equal(s.failed,false);assert.deepEqual(s.draft,before);
 assert.equal(events.at(-2).kind,'persistence_recovered');
 assert.equal(events.at(-1).dirty,false);assert.equal(r.originals.size,0);
});

test('authored alphabet paths and all continuation writing targets resolve under the new renderer',()=>{
 const api=require('../crates/rufren-app/src/handwriting.js');
 const source=require('../crates/rufren-content/content/extensions/alphabet-construction-glyphs-v1.json');
 for(const glyph of [...source.glyphs,...source.punctuation_additions]) assert.deepEqual(api.ALPHABET_GLYPHS[glyph.glyph],glyph.path_commands);
 const tasks=require('../crates/rufren-content/content/extensions/alphabet-construction-handwriting-v1.json');
 for(const task of tasks){
  const weight=[...task.target].reduce((n,c)=>n+(c===' '?0.45:c==='.'?0.28:0.84),0);
  const paper={width:Math.max(task.layout==='phrase_line'?640:400,Math.ceil(weight*125+80)),height:300,model_size:task.layout==='letter_box'?190:125,baseline:225,layout:task.layout};
  const paths=api.model(task.target,paper,api.ALPHABET_MODEL);assert.ok(paths.length,task.id);
  for(const path of paths){
   const [x,y,scale]=path.transform.match(/-?\d+(?:\.\d+)?/g).map(Number);
   assert.ok(x>=0&&y>=0,task.id);
   assert.ok(y+120*scale<=paper.height,task.id+' descender');
  }
 }
});
