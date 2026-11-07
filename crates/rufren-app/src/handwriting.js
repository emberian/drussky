/* Local hand-printed models and raw pointer ink. No recognition or pronunciation scoring. */
(function(global){
'use strict';
const VERSION=1, MODEL='hand-printed-six-v2', WIDTH=400, HEIGHT=300;
const MAX_STROKES=1000, MAX_POINTS=200000, MAX_STROKE_POINTS=100000, MAX_AIDS=4096;
// One possible unjoined printed form. Each array item is a demonstrated pen stroke.
const GLYPHS={
 'А':['M12 88 L42 12 L72 88','M24 58 L60 58'],
 'а':['M59 45 C43 29 20 37 20 61 C20 85 44 92 59 70','M60 38 L60 88'],
 'М':['M10 88 L10 14 L42 58 L74 14 L74 88'],
 'м':['M12 88 L12 39 L39 69 L66 39 L66 88'],
 'О':['M42 12 C2 12 2 88 42 88 C82 88 82 12 42 12'],
 'о':['M40 37 C9 37 9 89 40 89 C71 89 71 37 40 37'],
 'К':['M15 12 L15 88','M69 12 L15 55 L72 88'],
 'к':['M17 39 L17 88','M65 39 L17 65 L68 88'],
 'Т':['M5 14 L77 14','M41 14 L41 88'],
 'т':['M8 40 L72 40','M40 40 L40 88'],
 'Д':['M9 82 L24 82 L33 14 L62 14 L62 82','M9 98 L9 82 L76 82 L76 98'],
 'д':['M11 82 L24 82 L34 39 L59 39 L59 82','M11 98 L11 82 L73 82 L73 98'],
 '.':['M38 88 L39 88'],
 'Н':['M15 12 L15 88','M69 12 L69 88','M15 50 L69 50'],
 'н':['M17 39 L17 88','M65 39 L65 88','M17 64 L65 64'],
 'И':['M15 12 L15 88 L69 12 L69 88'],
 'и':['M17 39 L17 88 L65 39 L65 88'],
 ' ':[]
};
// Immutable second renderer: the original GLYPHS and layout remain available for historical ink.
const ALPHABET_MODEL='hand-printed-alphabet-v1';
const ALPHABET_GLYPHS=Object.freeze({...GLYPHS,...{"Я":["M68 88 L68 12 L42 12 C8 12 8 49 42 49 L68 49","M42 49 L12 88"],"я":["M65 88 L65 39 L40 39 C13 39 13 65 40 65 L65 65","M40 65 L16 88"],"Ы":["M12 12 L12 88 L34 88 C64 88 64 51 34 51 L12 51","M72 12 L72 88"],"ы":["M12 39 L12 88 L31 88 C55 88 55 62 31 62 L12 62","M70 39 L70 88"],"Е":["M68 12 L15 12 L15 88 L68 88","M15 50 L59 50"],"е":["M16 63 L67 63 C67 28 16 28 16 63 C16 89 49 98 67 80"],"Э":["M16 25 C54 -3 76 22 76 50 C76 78 54 103 16 75","M38 50 L76 50"],"э":["M16 47 C45 26 70 43 70 64 C70 87 46 101 16 81","M38 64 L70 64"],"С":["M69 24 C43 -4 12 17 12 50 C12 83 43 104 69 76"],"с":["M64 47 C43 27 15 41 15 64 C15 89 43 102 64 80"],"Л":["M9 88 L33 14 L68 14 L68 88"],"л":["M12 88 L32 39 L65 39 L65 88"],"Р":["M15 88 L15 12 L43 12 C81 12 81 52 43 52 L15 52"],"р":["M17 108 L17 39","M17 48 C45 24 69 42 69 64 C69 88 45 104 17 80"],"У":["M12 12 L42 58 L70 12","M42 58 L23 88"],"у":["M13 39 L40 87","M68 39 L35 108 C30 119 23 119 16 114"],"В":["M15 88 L15 12 L42 12 C77 12 77 48 42 48 L15 48","M42 48 C82 48 82 88 42 88 L15 88"],"в":["M17 88 L17 39 L40 39 C68 39 68 62 40 62 L17 62","M40 62 C72 62 72 88 40 88 L17 88"],"Б":["M68 12 L15 12 L15 88 L42 88 C82 88 82 49 42 49 L15 49"],"б":["M65 15 C49 26 23 10 18 52","M18 55 C28 29 68 36 68 64 C68 95 15 101 16 64 C16 61 17 58 18 55"],"П":["M15 88 L15 12 L69 12 L69 88"],"п":["M17 88 L17 39 L65 39 L65 88"],"З":["M15 24 C37 -1 72 14 69 34 C68 47 54 50 39 50","M39 50 C79 50 82 85 48 90 C32 93 20 84 12 76"],"з":["M17 47 C34 29 63 37 63 51 C63 60 53 64 39 64","M39 64 C73 64 74 87 46 91 C32 93 23 86 16 80"],"Г":["M15 88 L15 12 L69 12"],"г":["M17 88 L17 39 L65 39"],"Х":["M12 12 L70 88","M70 12 L12 88"],"х":["M15 39 L65 88","M65 39 L15 88"],"Ж":["M41 12 L41 88","M8 12 L41 50 L74 12","M8 88 L41 50 L74 88"],"ж":["M40 39 L40 88","M10 39 L40 64 L70 39","M10 88 L40 64 L70 88"],"Ш":["M12 12 L12 88 L42 88 L42 12","M42 88 L72 88 L72 12"],"ш":["M13 39 L13 88 L40 88 L40 39","M40 88 L67 88 L67 39"],"Ё":["M68 18 L15 18 L15 88 L68 88","M15 53 L59 53","M28 5 L30 5","M53 5 L55 5"],"ё":["M16 63 L67 63 C67 28 16 28 16 63 C16 89 49 98 67 80","M29 23 L31 23","M52 23 L54 23"],"Ю":["M12 12 L12 88","M12 50 L35 50","M57 12 C27 12 27 88 57 88 C87 88 87 12 57 12"],"ю":["M13 39 L13 88","M13 64 L34 64","M55 39 C27 39 27 89 55 89 C83 89 83 39 55 39"],"Й":["M15 20 L15 88 L69 20 L69 88","M28 4 C33 14 51 14 56 4"],"й":["M17 39 L17 88 L65 39 L65 88","M27 22 C32 32 50 32 55 22"],"Ь":["M17 12 L17 88 L42 88 C82 88 82 49 42 49 L17 49"],"ь":["M17 39 L17 88 L40 88 C72 88 72 62 40 62 L17 62"],"Ц":["M15 12 L15 88 L69 88 L69 12","M69 88 L76 88 L76 106"],"ц":["M17 39 L17 88 L65 88 L65 39","M65 88 L73 88 L73 103"],"Ч":["M15 12 L15 43 C15 61 45 61 69 45","M69 12 L69 88"],"ч":["M17 39 L17 59 C17 72 43 72 65 59","M65 39 L65 88"],"Щ":["M12 12 L12 88 L42 88 L42 12","M42 88 L72 88 L72 12","M72 88 L79 88 L79 106"],"щ":["M13 39 L13 88 L40 88 L40 39","M40 88 L67 88 L67 39","M67 88 L75 88 L75 103"],"Ф":["M42 23 C0 23 0 78 42 78 C84 78 84 23 42 23","M42 6 L42 96"],"ф":["M41 41 C4 19 4 105 41 84 C78 105 78 19 41 41","M41 20 L41 111"],"Ъ":["M8 12 L28 12 L28 88 L47 88 C83 88 83 49 47 49 L28 49"],"ъ":["M10 39 L28 39 L28 88 L46 88 C75 88 75 62 46 62 L28 62"],"?":["M20 25 C20 2 66 2 66 29 C66 46 43 47 43 65","M42 86 L44 86"],"—":["M8 62 L75 62"]}});
const modelVersion=config=>config.model_version||MODEL;
const advance=(glyph,version)=>glyph===' '?0.45:version===ALPHABET_MODEL&&['.','?',','].includes(glyph)?0.4:glyph==='.'?0.28:0.84;
const glyphSet=version=>{if(version===MODEL)return GLYPHS;if(version===ALPHABET_MODEL)return ALPHABET_GLYPHS;throw Error('Unknown printed model version. Original ink is preserved.');};
const clone=v=>JSON.parse(JSON.stringify(v));
// View transforms include old out-of-paper ink without rewriting its coordinates.
const fitViewport=(r,width=WIDTH,height=HEIGHT,bounds=null)=>{const b=bounds||{x:0,y:0,width,height};const scale=Math.min(r.width/b.width,r.height/b.height);return {scale,x:(r.width-b.width*scale)/2-b.x*scale,y:(r.height-b.height*scale)/2-b.y*scale};};
function drawingBounds(draft,supplied){
 const {width,height}=paper(supplied);let left=0,top=0,right=width,bottom=height;
 // Four canonical units include the maximum pressure stroke radius and its
 // antialiasing edge. Guides and the model retain their original paper anchors.
 for(const stroke of draft.strokes)for(const point of stroke.points){left=Math.min(left,point.x-4);top=Math.min(top,point.y-4);right=Math.max(right,point.x+4);bottom=Math.max(bottom,point.y+4);}
 return {x:left,y:top,width:right-left,height:bottom-top};
}
const paper=p=>p||{width:WIDTH,height:HEIGHT,model_size:190,baseline:277,layout:'letter_box'};
const validMode=v=>['trace','copy','hidden'].includes(v);
const validInput=v=>['pen','finger','mouse'].includes(v);
function layout(target,supplied,version=MODEL){const p=paper(supplied);let size=p.model_size;if(version===ALPHABET_MODEL){const weight=[...target].reduce((n,c)=>n+advance(c,version),0);size=Math.min(size,(p.width-40)/Math.max(1,weight),(p.height-40)/120*100);}let left,top;if(p.layout==='letter_box'){left=(p.width-size*[...target].length)/2;top=p.height-size;}else{const total=[...target].reduce((n,c)=>n+advance(c,version)*size,0);left=(p.width-total)/2;top=p.baseline-88*size/100;}if(version===ALPHABET_MODEL)top=Math.max(20,Math.min(top,p.height-20-120*size/100));return {p,size,left,top};}
function guides(target,supplied,version=MODEL){const l=layout(target,supplied,version);return [12,39,88].map(y=>l.top+y*l.size/100);}
function model(target,supplied,version=MODEL){
 const glyphs=glyphSet(version);
 if(!target||[...target].some(c=>!Object.hasOwn(glyphs,c)))throw Error('This target needs an authored hand-printed model.');
 const l=layout(target,supplied,version);let x=l.left;const paths=[];for(const [index,glyph]of [...target].entries()){for(const [stroke,d]of glyphs[glyph].entries())paths.push({glyph,index,stroke,d,transform:`translate(${x} ${l.top}) scale(${l.size/100})`});x+=(l.p.layout==='letter_box'?1:advance(glyph,version))*l.size;}return paths;
}
function validate(d,config){
 if(!d||d.version!==VERSION||d.model_version!==modelVersion(config)||!Array.isArray(d.view_box)||d.view_box[0]!==paper(config.paper).width||d.view_box[1]!==paper(config.paper).height||d.scope_id!==config.scope_id||d.target!==config.target||d.task_id!==config.task_id||d.task_revision!==config.task_revision||!validMode(d.mode)||!validInput(d.input_mode)||!Array.isArray(d.strokes)||d.strokes.length>MAX_STROKES||!Array.isArray(d.aids)||d.aids.length>MAX_AIDS)throw Error('The saved ink belongs to a different task or drawing version. It is preserved.');
 let total=0;for(const stroke of d.strokes){if(!['pen','touch','mouse'].includes(stroke.pointer_type)||!Array.isArray(stroke.points)||!stroke.points.length||stroke.points.length>MAX_STROKE_POINTS||(total+=stroke.points.length)>MAX_POINTS)throw Error('The saved stroke cannot be read. It is preserved.');for(const p of stroke.points){if(!['x','y','time_ms','pressure','tilt_x','tilt_y','twist','width','height'].every(k=>Number.isFinite(p[k]))||Math.abs(p.x)>10000||Math.abs(p.y)>10000||p.pressure<0||p.pressure>1)throw Error('The saved ink has invalid coordinates. It is preserved.');}}
 return d;
}
class InkSession {
 constructor(config,adapter,send=()=>{},uuid=()=>global.crypto.randomUUID(),now=()=>Date.now()){
  model(config.target,config.paper,modelVersion(config));this.config=config;this.adapter=adapter;this.queued=null;this.saving=false;this.tail=Promise.resolve(true);this.comparing=false;this.leaving=false;this.send=send;this.uuid=uuid;this.now=now;this.active=null;this.failed=false;let d=config.initial?clone(validate(config.initial,config)):null;
  this.draft=d||{version:VERSION,model_version:modelVersion(config),view_box:[paper(config.paper).width,paper(config.paper).height],scope_id:config.scope_id,task_id:config.task_id,task_revision:config.task_revision,target:config.target,audio_id:config.audio_id||null,audio_role:config.audio_role||'model',attempt_id:config.attempt_id||uuid(),created_at_ms:now(),mode:config.initial_mode||'trace',input_mode:config.input_mode||'pen',meaning_visible:config.meaning_visible!==false,sound_visible:config.sound_visible!==false,sealed:false,strokes:[],aids:[]};
  if(!validMode(this.draft.mode)||!validInput(this.draft.input_mode))throw Error('Choose a drawing and input mode.');
  if(!d){if(this.draft.mode!=='hidden')this.aid('visual_model',this.draft.mode);if(this.draft.meaning_visible)this.aid('meaning','shown');}
  // A partial stroke after process/navigation loss never resumes as a running pointer.
  for(const stroke of this.draft.strokes){if(stroke.active){stroke.active=false;stroke.interrupted=true;stroke.cancel_reason='restored_after_interruption';}}
  this.persist();
 }
 aid(kind,value){if(this.draft.aids.length>=MAX_AIDS)throw Error('This attempt has reached its help-history limit. Compare the drawing before starting another attempt.');this.draft.aids.push({kind,value,at_ms:this.now()});}
 limit(){if(this.active!==null){const stroke=this.draft.strokes.at(-1);stroke.active=false;stroke.interrupted=true;stroke.cancel_reason='capture_limit';this.active=null;}this.persist();this.send({kind:'error',message:'This sheet has reached its drawing limit. The ink already captured is kept. Compare it, or undo a stroke before continuing.'});return false;}
 persist(){
  this.queued=true;this.send({kind:'status',dirty:true});if(this.saving)return this.tail;
  this.saving=true;this.tail=(async()=>{while(this.queued){this.queued=false;const snapshot=clone(this.draft);try{await this.adapter.saveDraft(snapshot);this.failed=false;this.send({kind:'persistence_recovered'});}catch(e){this.failed=true;this.send({kind:'persistence_error',message:'Your ink could not be saved. Keep this screen open and retry before comparing or leaving.'});break;}}this.saving=false;this.send({kind:'status',dirty:this.failed||!!this.queued});return !this.failed;})();return this.tail;
 }
 async flush(){if(this.failed&&!this.saving)await this.persist();await this.tail;return !this.failed&&!this.queued;}

 accepts(type){return this.draft.input_mode==='pen'?type==='pen':this.draft.input_mode==='finger'?type==='touch':type==='mouse';}
 begin(id,type,point,geometry=null){if(this.leaving||this.draft.sealed||this.draft.pending_snapshot||this.comparing||this.active!==null||!this.accepts(type))return false;if(this.draft.strokes.length>=MAX_STROKES||this.draft.strokes.reduce((n,s)=>n+s.points.length,0)>=MAX_POINTS)return this.limit();const stroke={pointer_id:id,pointer_type:type,capture_geometry:geometry?clone(geometry):null,active:true,interrupted:false,cancel_reason:null,points:[clone(point)]};this.draft.strokes.push(stroke);this.active=id;this.persist();return true;}
 move(id,points){if(this.active!==id||this.leaving||this.draft.sealed||this.draft.pending_snapshot)return false;const stroke=this.draft.strokes.at(-1);const available=Math.min(MAX_STROKE_POINTS-stroke.points.length,MAX_POINTS-this.draft.strokes.reduce((n,s)=>n+s.points.length,0));for(const point of points.slice(0,available))stroke.points.push(clone(point));if(points.length>available)return this.limit();this.persist();return true;}
 end(id,reason=null){if(this.active!==id)return false;const stroke=this.draft.strokes.at(-1);stroke.active=false;stroke.interrupted=!!reason;stroke.cancel_reason=reason;this.active=null;this.persist();return true;}
 undo(){if(this.draft.sealed||this.draft.pending_snapshot||this.comparing)return false;if(this.active!==null)this.end(this.active,'undo');this.draft.strokes.pop();return this.persist();}
 clear(){if(this.draft.sealed||this.draft.pending_snapshot||this.comparing)return false;if(this.active!==null)this.end(this.active,'clear');this.draft.strokes=[];return this.persist();}
 mode(value){if(this.draft.pending_snapshot&&!this.draft.sealed)throw Error('Finish saving the original first.');if(!validMode(value))throw Error('Unknown visual cue.');if(this.active!==null)this.end(this.active,'mode_change');const request=this.modeRequest=(this.modeRequest||0)+1;if(value==='hidden'){this.draft.mode=value;return this.persist();}this.aid('visual_model',value);return(async()=>{if(!await this.persist()||this.modeRequest!==request||this.leaving)return false;this.draft.mode=value;return this.persist();})();}
 input(value){if(this.draft.pending_snapshot&&!this.draft.sealed)throw Error('Finish saving the original first.');if(!validInput(value))throw Error('Unknown input mode.');if(this.active!==null)this.end(this.active,'input_mode_change');this.draft.input_mode=value;return this.persist();}
 cue(kind,visible){if(this.draft.pending_snapshot&&!this.draft.sealed)throw Error('Finish saving the original first.');if(!['meaning','sound'].includes(kind))throw Error('Unknown cue.');this.cueRequests??={};const request=this.cueRequests[kind]=(this.cueRequests[kind]||0)+1;if(!visible){this.draft[`${kind}_visible`]=false;return this.persist();}this.aid(kind,'shown');return(async()=>{if(!await this.persist()||this.cueRequests[kind]!==request||this.leaving)return false;this.draft[`${kind}_visible`]=true;return this.persist();})();}
 async listen(){if(this.draft.pending_snapshot&&!this.draft.sealed)throw Error('Finish saving the original first.');this.aid(this.draft.audio_role==='task_input'?'sound_task_input_requested':'sound_model_requested','request_only');if(await this.persist())this.send({kind:'listen',attempt_id:this.draft.attempt_id});}
 audioResult(attempt_id,ok){if(attempt_id!==this.draft.attempt_id)return Promise.resolve(false);if(ok)this.aid(this.draft.audio_role==='task_input'?'sound_task_input_completed':'sound_model_completed',this.draft.audio_id||'');else this.aid('sound_unavailable',this.draft.audio_id||'');const saved=this.persist();if(ok)this.send({kind:'audio_completed',attempt_id:this.draft.attempt_id,audio_id:this.draft.audio_id});return saved;}
 replay(){if(this.draft.pending_snapshot&&!this.draft.sealed)throw Error('Finish saving the original first.');if(this.draft.sealed)return Promise.resolve(true);if(this.draft.mode==='hidden')return Promise.resolve(false);this.aid('stroke_model_replay','demonstration');return this.persist();}
 async compare(){
  if(this.active!==null)this.end(this.active,'comparison_requested');
  if(this.draft.sealed)return clone(this.draft.pending_snapshot||this.draft);
  if(this.comparing)throw Error('Your original is being saved.');
  if(!this.draft.strokes.some(s=>s.points.length))throw Error('Make a stroke first, or skip this activity.');
  if(this.draft.audio_role==='task_input'&&this.draft.audio_id&&!this.draft.aids.some(a=>a.kind==='sound_task_input_completed'&&a.value===this.draft.audio_id))throw Error('Listen to the sound first, then compare your drawing.');
  this.comparing=true;
  try{
   if(!await this.flush())throw Error('Your ink could not be saved. Retry before comparing.');
   if(!this.draft.pending_snapshot){this.draft.pending_snapshot=clone({...this.draft,pending_snapshot:undefined,sealed:true,saved_at_ms:this.now()});if(!await this.persist())throw Error('Your original could not be reserved. Retry; the model stays hidden.');}
   const snapshot=clone(this.draft.pending_snapshot);
   await this.adapter.saveOriginal(snapshot);
   this.draft.sealed=true;this.draft.saved_at_ms=snapshot.saved_at_ms;if(!await this.persist()){this.draft.sealed=false;throw Error('Your original is saved, but the current place could not be saved. Retry before continuing.');}
   this.send({kind:'saved',submission:snapshot});return snapshot;
  }catch(e){this.failed=true;throw Error(e.message||'Your original could not be saved. Retry; the model stays hidden.');}finally{this.comparing=false;}
 }

 newTry(){this.send({kind:'stop_audio'});const previous=this.draft.attempt_id;if(this.comparing)throw Error('Finish saving first.');if(this.active!==null)this.end(this.active,'new_try');if(this.draft.pending_snapshot&&!this.draft.sealed)throw Error('Finish saving the original first.');if(!this.draft.sealed&&this.draft.strokes.length)throw Error('Compare or clear this drawing before starting a new try.');this.draft={...this.draft,attempt_id:this.uuid(),created_at_ms:this.now(),strokes:[],sealed:false,aids:[],mode:this.config.initial_mode||'trace',meaning_visible:this.config.meaning_visible!==false,sound_visible:this.config.audio_role==='task_input'||this.config.sound_visible!==false,saved_at_ms:undefined,pending_snapshot:undefined,prior_attempt_id:previous};this.aid('previous_model_this_activity','practice_after_comparison');if(this.draft.mode!=='hidden')this.aid('visual_model',this.draft.mode);if(this.draft.meaning_visible)this.aid('meaning','shown');return this.persist();}
}
function drawInk(ctx,draft){
 for(const s of draft.strokes){for(let i=0;i<s.points.length;i++){const p=s.points[i],prior=s.points[Math.max(0,i-1)];ctx.strokeStyle='#183c33';ctx.fillStyle='#183c33';ctx.lineWidth=2+Math.max(0,Math.min(1,p.pressure))*5;ctx.lineCap='round';ctx.lineJoin='round';ctx.beginPath();if(i===0){ctx.arc(p.x,p.y,ctx.lineWidth/2,0,Math.PI*2);ctx.fill()}else{ctx.moveTo(prior.x,prior.y);ctx.lineTo(p.x,p.y);ctx.stroke()}}}
}
function preview(canvas,draft,config){
 validate(draft,config);const {width,height}=paper(config.paper);
 if(![width,height].every(n=>Number.isFinite(n)&&n>0&&n<=16384))throw Error('Invalid drawing dimensions.');
 const bounds=drawingBounds(draft,config.paper),scale=Math.min(1,1024/bounds.width,1024/bounds.height);canvas.width=Math.max(1,Math.round(bounds.width*scale));canvas.height=Math.max(1,Math.round(bounds.height*scale));
 const ctx=canvas.getContext('2d');if(!ctx)throw Error('The saved drawing could not be displayed.');
 ctx.setTransform(1,0,0,1,0,0);ctx.clearRect(0,0,canvas.width,canvas.height);const view=fitViewport({width:canvas.width,height:canvas.height},width,height,bounds);ctx.setTransform(view.scale,0,0,view.scale,view.x,view.y);
 for(const [index,y]of guides(config.target,config.paper,modelVersion(config)).entries()){ctx.strokeStyle=index===2?'#93a591':'#c2cbc0';ctx.lineWidth=index===2?2:1;ctx.setLineDash(index===2?[]:[8,7]);ctx.beginPath();ctx.moveTo(20,y);ctx.lineTo(width-20,y);ctx.stroke();}ctx.setLineDash([]);
 drawInk(ctx,draft);return true;
}
function install(root,config,adapter,send){
 const {width:WIDTH,height:HEIGHT}=paper(config.paper);
 config={...config,input_mode:config.input_mode||(navigator.maxTouchPoints===0?'mouse':'pen')};
 if(!root)throw Error('The handwriting pad did not mount.');
 const controller=new AbortController(),signal=controller.signal,canvas=root.querySelector('canvas'),ctx=canvas.getContext('2d');
 if(!ctx)throw Error('This device could not open the drawing canvas.');
 const session=new InkSession(config,adapter,send),modelBox=root.querySelector('[data-ink-model]'),comparison=root.querySelector('[data-ink-comparison]'),status=root.querySelector('[data-ink-status]');
 let animations=[],disposed=false,viewportBounds=drawingBounds(session.draft,config.paper),boundsKey='',previousSealed=session.draft.sealed,previousMode=session.draft.mode;
 const viewport=r=>fitViewport(r,WIDTH,HEIGHT,viewportBounds);
 const cancelAnimations=()=>{animations.forEach(a=>a.cancel());animations=[];};
 const paths=model(config.target,config.paper,modelVersion(config));const lineYs=guides(config.target,config.paper,modelVersion(config));
 const drawModel=(container)=>{container.replaceChildren();const label=document.createElement('span');label.className='ink-pane-label';label.textContent='Example model';container.append(label);const svg=document.createElementNS('http://www.w3.org/2000/svg','svg');svg.setAttribute('viewBox',`0 0 ${WIDTH} ${HEIGHT}`);svg.setAttribute('aria-label','One way to hand-print this target');svg.setAttribute('role','img');svg.setAttribute('preserveAspectRatio','xMidYMid meet');for(const [index,y] of lineYs.entries()){const line=document.createElementNS(svg.namespaceURI,'line');line.setAttribute('x1','20');line.setAttribute('x2',String(WIDTH-20));line.setAttribute('y1',y);line.setAttribute('y2',y);line.setAttribute('stroke',index===2?'#93a591':'#c2cbc0');line.setAttribute('stroke-width',index===2?'2':'1');if(index!==2)line.setAttribute('stroke-dasharray','8 7');svg.append(line)}for(const p of paths){const path=document.createElementNS(svg.namespaceURI,'path');path.setAttribute('d',p.d);path.setAttribute('transform',p.transform);path.setAttribute('fill','none');path.setAttribute('stroke','currentColor');path.setAttribute('stroke-width','4');path.setAttribute('stroke-linecap','round');path.setAttribute('stroke-linejoin','round');svg.append(path);}container.append(svg);};
 drawModel(modelBox);drawModel(comparison);
 const refreshBounds=()=>{
  // Never move the sheet beneath an active Pencil. Pen-up, restore and sealed
  // comparison may fit the full original, including legacy letterbox margins.
  if(session.active!==null)return;
  const bounds=drawingBounds(session.draft,config.paper),key=JSON.stringify(bounds);if(key===boundsKey)return;
  boundsKey=key;viewportBounds=bounds;
  root.style.setProperty('--ink-paper-ratio',`${bounds.width} / ${bounds.height}`);
  root.style.setProperty('--ink-paper-max-width',`${320*bounds.width/bounds.height}px`);
  for(const box of [modelBox,comparison])box.querySelector('svg').setAttribute('viewBox',`${bounds.x} ${bounds.y} ${bounds.width} ${bounds.height}`);
 };
 const render=()=>{
  if(disposed)return;if(previousSealed!==session.draft.sealed||previousMode!==session.draft.mode){cancelAnimations();previousSealed=session.draft.sealed;previousMode=session.draft.mode;}
  refreshBounds();const box=canvas.getBoundingClientRect();if(!box.width||!box.height)return;const ratio=Math.min(global.devicePixelRatio||1,4096/Math.max(box.width,box.height),Math.sqrt(4194304/(box.width*box.height)));const pixelWidth=Math.max(1,Math.round(box.width*ratio)),pixelHeight=Math.max(1,Math.round(box.height*ratio));if(canvas.width!==pixelWidth)canvas.width=pixelWidth;if(canvas.height!==pixelHeight)canvas.height=pixelHeight;ctx.setTransform(1,0,0,1,0,0);ctx.clearRect(0,0,canvas.width,canvas.height);const view=viewport(box);ctx.setTransform(view.scale*ratio,0,0,view.scale*ratio,view.x*ratio,view.y*ratio);
  for(const [index,y] of lineYs.entries()){ctx.strokeStyle=index===2?'#93a591':'#c2cbc0';ctx.lineWidth=index===2?2:1;ctx.setLineDash(index===2?[]:[8,7]);ctx.beginPath();ctx.moveTo(20,y);ctx.lineTo(WIDTH-20,y);ctx.stroke()}ctx.setLineDash([]);
  drawInk(ctx,session.draft);
  const d=session.draft;root.dataset.visual=d.mode;root.dataset.sealed=String(d.sealed);modelBox.hidden=d.mode==='hidden';comparison.hidden=!d.sealed;root.querySelector('[data-ink-guidance]').hidden=d.mode==='hidden'&&!d.sealed;root.querySelector('[data-ink-replay]').hidden=d.mode==='hidden'&&!d.sealed;root.querySelector('[data-ink-meaning]').hidden=!d.meaning_visible;root.querySelector('[data-ink-sound]').hidden=!d.sound_visible;
  for(const button of root.querySelectorAll('[data-ink-edit]'))button.disabled=d.sealed||!!d.pending_snapshot||session.leaving;
  root.querySelector('[data-ink-compare]').disabled=session.comparing||session.leaving||(d.audio_role==='task_input'&&!!d.audio_id&&!d.aids.some(a=>a.kind==='sound_task_input_completed'&&a.value===d.audio_id));root.querySelector('[data-ink-new]').disabled=session.leaving;for(const button of root.querySelectorAll('[data-ink-control]'))button.disabled=session.leaving||session.comparing||!!d.pending_snapshot&&!d.sealed;root.querySelector('[data-ink-show]').hidden=d.mode!=='hidden'||d.sealed;root.querySelector('[data-ink-hide]').hidden=d.mode==='hidden'||d.sealed;root.querySelector('[data-ink-retry-save]').hidden=!session.failed;root.querySelector('[data-ink-device-options]').hidden=d.input_mode==='mouse';for(const input of root.querySelectorAll('[data-ink-mode]'))input.checked=input.dataset.inkMode===d.mode;for(const b of root.querySelectorAll('[data-ink-input]'))b.setAttribute('aria-pressed',String(b.dataset.inkInput===d.input_mode));root.querySelector('[data-ink-new]').hidden=!d.sealed||!!config.external_flow;root.querySelector('[data-ink-compare]').hidden=d.sealed;status.textContent=session.comparing?'Opening the example…':session.failed?'Your ink needs saving. Retry before comparing.':d.sealed?'Compare your shape with the example. You can try again when ready.':d.input_mode==='pen'?'Pencil only. Touches on this pad do not make ink.':'Drawing with '+(d.input_mode==='finger'?'a finger':'a mouse')+'.';
 };
 const sample=e=>{const r=canvas.getBoundingClientRect();return {x:(e.clientX-r.left-viewport(r).x)/viewport(r).scale,y:(e.clientY-r.top-viewport(r).y)/viewport(r).scale,time_ms:e.timeStamp,pressure:Number.isFinite(e.pressure)?e.pressure:0,tilt_x:e.tiltX||0,tilt_y:e.tiltY||0,twist:e.twist||0,width:e.width||0,height:e.height||0};};
 canvas.addEventListener('pointerdown',e=>{if(e.pointerType==='pen'&&session.draft.input_mode==='mouse')session.input('pen');if(session.begin(e.pointerId,e.pointerType,sample(e),(()=>{const r=canvas.getBoundingClientRect(),v=viewport(r);return {css_width:r.width,css_height:r.height,device_pixel_ratio:global.devicePixelRatio||1,viewport_scale:global.visualViewport?.scale||1,units_per_css_pixel:1/v.scale,offset_x:v.x,offset_y:v.y};})())){e.preventDefault();canvas.setPointerCapture(e.pointerId);render();}},{signal});
 canvas.addEventListener('pointermove',e=>{if(session.active!==e.pointerId)return;e.preventDefault();const coalesced=typeof e.getCoalescedEvents==='function'?e.getCoalescedEvents():[];session.move(e.pointerId,(coalesced.length?coalesced:[e]).map(sample));render();},{signal});
 const finish=(e,reason)=>{if(session.active!==e.pointerId)return;e.preventDefault();session.move(e.pointerId,[sample(e)]);session.end(e.pointerId,reason);if(canvas.hasPointerCapture(e.pointerId))canvas.releasePointerCapture(e.pointerId);render();};
 canvas.addEventListener('pointerup',e=>finish(e,null),{signal});canvas.addEventListener('pointercancel',e=>finish(e,'pointer_cancel'),{signal});canvas.addEventListener('lostpointercapture',e=>{if(session.active===e.pointerId){session.end(e.pointerId,'lost_pointer_capture');render();}},{signal});
 const action=(selector,f)=>root.querySelector(selector)?.addEventListener('click',async()=>{try{const result=f();render();await result;render()}catch(e){status.textContent=e.message;send({kind:'error',message:e.message})}},{signal});
 for(const input of root.querySelectorAll('[data-ink-mode]'))input.addEventListener('change',async()=>{if(input.checked){try{await session.mode(input.dataset.inkMode);render()}catch(e){status.textContent=e.message;send({kind:'error',message:e.message})}}},{signal});
 for(const button of root.querySelectorAll('[data-ink-input]'))button.addEventListener('click',()=>{session.input(button.dataset.inkInput);render()},{signal});
 action('[data-ink-show]',()=>session.mode('copy'));action('[data-ink-hide]',()=>{cancelAnimations();return session.mode('hidden')});
 action('[data-ink-undo]',()=>session.undo());action('[data-ink-clear]',()=>session.clear());action('[data-ink-retry-save]',()=>session.persist());action('[data-ink-new]',()=>session.newTry());action('[data-ink-compare]',()=>session.compare());
 action('[data-ink-toggle-meaning]',()=>session.cue('meaning',!session.draft.meaning_visible));action('[data-ink-toggle-sound]',()=>session.cue('sound',!session.draft.sound_visible));action('[data-ink-listen]',()=>session.listen());
 action('[data-ink-replay]',async()=>{if(!await session.replay()||disposed||(!session.draft.sealed&&session.draft.mode==='hidden'))return;cancelAnimations();if(global.matchMedia?.('(prefers-reduced-motion: reduce)').matches)return;const visible=session.draft.sealed?comparison:modelBox;const nodes=[...visible.querySelectorAll('path')],step=Math.min(550,12000/Math.max(1,nodes.length));nodes.forEach((path,index)=>{const length=path.getTotalLength();animations.push(path.animate([{strokeDasharray:String(length),strokeDashoffset:length},{strokeDasharray:String(length),strokeDashoffset:0}],{duration:step*.9,delay:index*step,fill:'backwards'}))});});
 const interrupt=reason=>{if(session.active!==null)session.end(session.active,reason)};
 const resize=new ResizeObserver(()=>{interrupt('canvas_resize');render()});resize.observe(canvas);
 global.addEventListener('pagehide',()=>interrupt('page_hidden'),{signal});document.addEventListener('visibilitychange',()=>{if(document.hidden)interrupt('document_hidden')},{signal});
 render();return {session,viewport(){const box=canvas.getBoundingClientRect();return {...viewport(box),bounds:{...viewportBounds},css_width:box.width,css_height:box.height};},audioResult:async(attempt_id,ok)=>{const saved=await session.audioResult(attempt_id,ok);if(!disposed)render();return saved;},flush:async()=>{session.leaving=true;interrupt('navigation_requested');render();const ok=await session.flush();if(!ok)session.leaving=false;render();return ok;},resumeDrawing(){session.leaving=false;render();},dispose(){if(disposed)return;disposed=true;interrupt('pad_unmounted');cancelAnimations();resize.disconnect();controller.abort();}};
}
const api={VERSION,MODEL,ALPHABET_MODEL,WIDTH,HEIGHT,GLYPHS,ALPHABET_GLYPHS,model,guides,validate,drawingBounds,fitViewport,InkSession,install,preview};global.RufrenHandwriting=api;if(typeof module!=='undefined')module.exports=api;
})(globalThis);
