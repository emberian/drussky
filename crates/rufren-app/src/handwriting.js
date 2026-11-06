/* Local hand-printed models and raw pointer ink. No recognition or pronunciation scoring. */
(function(global){
'use strict';
const VERSION=1, MODEL='hand-printed-six-v2', WIDTH=400, HEIGHT=300;
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
const clone=v=>JSON.parse(JSON.stringify(v));
const fitViewport=(r,width=WIDTH,height=HEIGHT)=>{const scale=Math.min(r.width/width,r.height/height);return {scale,x:(r.width-width*scale)/2,y:(r.height-height*scale)/2};};
const paper=p=>p||{width:WIDTH,height:HEIGHT,model_size:190,baseline:277,layout:'letter_box'};
const validMode=v=>['trace','copy','hidden'].includes(v);
const validInput=v=>['pen','finger','mouse'].includes(v);
function layout(target,supplied){const p=paper(supplied),size=p.model_size;let left,top;if(p.layout==='letter_box'){left=(p.width-size*[...target].length)/2;top=p.height-size;}else{const total=[...target].reduce((n,c)=>n+(c===' '?0.45:c==='.'?0.28:0.84)*size,0);left=(p.width-total)/2;top=p.baseline-88*size/100;}return {p,size,left,top};}
function guides(target,supplied){const l=layout(target,supplied);return [12,39,88].map(y=>l.top+y*l.size/100);}
function model(target,supplied){
 if(!target||[...target].some(c=>!Object.hasOwn(GLYPHS,c)))throw Error('This target needs an authored hand-printed model.');
 const l=layout(target,supplied);let x=l.left;const paths=[];for(const [index,glyph]of [...target].entries()){for(const [stroke,d]of GLYPHS[glyph].entries())paths.push({glyph,index,stroke,d,transform:`translate(${x} ${l.top}) scale(${l.size/100})`});x+=(l.p.layout==='letter_box'?1:glyph===' '?0.45:glyph==='.'?0.28:0.84)*l.size;}return paths;
}
function validate(d,config){
 if(!d||d.version!==VERSION||d.model_version!==MODEL||!Array.isArray(d.view_box)||d.view_box[0]!==paper(config.paper).width||d.view_box[1]!==paper(config.paper).height||d.scope_id!==config.scope_id||d.target!==config.target||d.task_id!==config.task_id||d.task_revision!==config.task_revision||!validMode(d.mode)||!validInput(d.input_mode)||!Array.isArray(d.strokes)||d.strokes.length>1000)throw Error('The saved ink belongs to a different task or drawing version. It is preserved.');
 for(const stroke of d.strokes){if(!['pen','touch','mouse'].includes(stroke.pointer_type)||!Array.isArray(stroke.points)||stroke.points.length>100000)throw Error('The saved stroke cannot be read. It is preserved.');for(const p of stroke.points){if(!['x','y','time_ms','pressure','tilt_x','tilt_y','twist','width','height'].every(k=>Number.isFinite(p[k]))||Math.abs(p.x)>10000||Math.abs(p.y)>10000)throw Error('The saved ink has invalid coordinates. It is preserved.');}}
 return d;
}
class InkSession {
 constructor(config,adapter,send=()=>{},uuid=()=>global.crypto.randomUUID(),now=()=>Date.now()){
  model(config.target,config.paper);this.config=config;this.adapter=adapter;this.queued=null;this.saving=false;this.tail=Promise.resolve(true);this.comparing=false;this.leaving=false;this.send=send;this.uuid=uuid;this.now=now;this.active=null;this.failed=false;let d=config.initial?validate(clone(config.initial),config):null;
  this.draft=d||{version:VERSION,model_version:MODEL,view_box:[paper(config.paper).width,paper(config.paper).height],scope_id:config.scope_id,task_id:config.task_id,task_revision:config.task_revision,target:config.target,audio_id:config.audio_id||null,audio_role:config.audio_role||'model',attempt_id:config.attempt_id||uuid(),created_at_ms:now(),mode:config.initial_mode||'trace',input_mode:config.input_mode||'pen',meaning_visible:config.meaning_visible!==false,sound_visible:config.sound_visible!==false,sealed:false,strokes:[],aids:[]};
  if(!validMode(this.draft.mode)||!validInput(this.draft.input_mode))throw Error('Choose a drawing and input mode.');
  if(!d){if(this.draft.mode!=='hidden')this.aid('visual_model',this.draft.mode);if(this.draft.meaning_visible)this.aid('meaning','shown');}
  // A partial stroke after process/navigation loss never resumes as a running pointer.
  for(const stroke of this.draft.strokes){if(stroke.active){stroke.active=false;stroke.interrupted=true;stroke.cancel_reason='restored_after_interruption';}}
  this.persist();
 }
 aid(kind,value){this.draft.aids.push({kind,value,at_ms:this.now()});}
 persist(){
  this.queued=clone(this.draft);this.send({kind:'status',dirty:true});if(this.saving)return this.tail;
  this.saving=true;this.tail=(async()=>{while(this.queued){const snapshot=this.queued;this.queued=null;try{await this.adapter.saveDraft(snapshot);this.failed=false;this.send({kind:'draft',draft:snapshot});}catch(e){this.failed=true;this.send({kind:'error',message:'Your ink could not be saved. Keep this screen open and retry before comparing or leaving.'});break;}}this.saving=false;this.send({kind:'status',dirty:this.failed||!!this.queued});return !this.failed;})();return this.tail;
 }
 async flush(){if(this.failed&&!this.saving)await this.persist();await this.tail;return !this.failed&&!this.queued;}

 accepts(type){return this.draft.input_mode==='pen'?type==='pen':this.draft.input_mode==='finger'?type==='touch':type==='mouse';}
 begin(id,type,point,geometry=null){if(this.leaving||this.draft.sealed||this.draft.pending_snapshot||this.comparing||this.active!==null||!this.accepts(type))return false;const stroke={pointer_id:id,pointer_type:type,capture_geometry:geometry?clone(geometry):null,active:true,interrupted:false,cancel_reason:null,points:[clone(point)]};this.draft.strokes.push(stroke);this.active=id;this.persist();return true;}
 move(id,points){if(this.active!==id||this.leaving||this.draft.sealed||this.draft.pending_snapshot)return false;const stroke=this.draft.strokes.at(-1);for(const point of points)stroke.points.push(clone(point));this.persist();return true;}
 end(id,reason=null){if(this.active!==id)return false;const stroke=this.draft.strokes.at(-1);stroke.active=false;stroke.interrupted=!!reason;stroke.cancel_reason=reason;this.active=null;this.persist();return true;}
 undo(){if(this.draft.sealed||this.draft.pending_snapshot||this.comparing)return false;if(this.active!==null)this.end(this.active,'undo');this.draft.strokes.pop();return this.persist();}
 clear(){if(this.draft.sealed||this.draft.pending_snapshot||this.comparing)return false;if(this.active!==null)this.end(this.active,'clear');this.draft.strokes=[];return this.persist();}
 mode(value){if(this.draft.pending_snapshot&&!this.draft.sealed)throw Error('Finish saving the original first.');if(!validMode(value))throw Error('Unknown visual cue.');if(this.active!==null)this.end(this.active,'mode_change');this.draft.mode=value;if(value!=='hidden')this.aid('visual_model',value);return this.persist();}
 input(value){if(this.draft.pending_snapshot&&!this.draft.sealed)throw Error('Finish saving the original first.');if(!validInput(value))throw Error('Unknown input mode.');if(this.active!==null)this.end(this.active,'input_mode_change');this.draft.input_mode=value;return this.persist();}
 cue(kind,visible){if(this.draft.pending_snapshot&&!this.draft.sealed)throw Error('Finish saving the original first.');if(!['meaning','sound'].includes(kind))throw Error('Unknown cue.');this.draft[`${kind}_visible`]=visible;if(visible)this.aid(kind,'shown');return this.persist();}
 async listen(){if(this.draft.pending_snapshot&&!this.draft.sealed)throw Error('Finish saving the original first.');this.aid(this.draft.audio_role==='task_input'?'sound_task_input_requested':'sound_model_requested','request_only');if(await this.persist())this.send({kind:'listen',attempt_id:this.draft.attempt_id});}
 audioResult(attempt_id,ok){if(attempt_id!==this.draft.attempt_id)return Promise.resolve(false);if(ok)this.aid(this.draft.audio_role==='task_input'?'sound_task_input_completed':'sound_model_completed',this.draft.audio_id||'');else this.aid('sound_unavailable',this.draft.audio_id||'');const saved=this.persist();if(ok)this.send({kind:'audio_completed',attempt_id:this.draft.attempt_id,audio_id:this.draft.audio_id});return saved;}
 replay(){if(this.draft.pending_snapshot&&!this.draft.sealed)throw Error('Finish saving the original first.');this.aid('stroke_model_replay','demonstration');return this.persist();}
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
function install(root,config,adapter,send){
 const {width:WIDTH,height:HEIGHT}=paper(config.paper);const viewport=r=>fitViewport(r,WIDTH,HEIGHT);
 config={...config,input_mode:config.input_mode||(navigator.maxTouchPoints===0?'mouse':'pen')};
 if(!root)throw Error('The handwriting pad did not mount.');
 const controller=new AbortController(),signal=controller.signal,canvas=root.querySelector('canvas'),ctx=canvas.getContext('2d');
 if(!ctx)throw Error('This device could not open the drawing canvas.');
 const session=new InkSession(config,adapter,send),modelBox=root.querySelector('[data-ink-model]'),comparison=root.querySelector('[data-ink-comparison]'),status=root.querySelector('[data-ink-status]');
 let animations=[],disposed=false;
 const paths=model(config.target,config.paper);const lineYs=guides(config.target,config.paper);
 const drawModel=(container)=>{container.replaceChildren();const label=document.createElement('span');label.className='ink-pane-label';label.textContent='Example model';container.append(label);const svg=document.createElementNS('http://www.w3.org/2000/svg','svg');svg.setAttribute('viewBox',`0 0 ${WIDTH} ${HEIGHT}`);svg.setAttribute('aria-label','One way to hand-print this target');svg.setAttribute('role','img');svg.setAttribute('preserveAspectRatio','xMidYMid meet');for(const [index,y] of lineYs.entries()){const line=document.createElementNS(svg.namespaceURI,'line');line.setAttribute('x1','20');line.setAttribute('x2',String(WIDTH-20));line.setAttribute('y1',y);line.setAttribute('y2',y);line.setAttribute('stroke',index===2?'#93a591':'#c2cbc0');line.setAttribute('stroke-width',index===2?'2':'1');if(index!==2)line.setAttribute('stroke-dasharray','8 7');svg.append(line)}for(const p of paths){const path=document.createElementNS(svg.namespaceURI,'path');path.setAttribute('d',p.d);path.setAttribute('transform',p.transform);path.setAttribute('fill','none');path.setAttribute('stroke','currentColor');path.setAttribute('stroke-width','4');path.setAttribute('stroke-linecap','round');path.setAttribute('stroke-linejoin','round');svg.append(path);}container.append(svg);};
 drawModel(modelBox);drawModel(comparison);
 const render=()=>{
  const box=canvas.getBoundingClientRect(),ratio=global.devicePixelRatio||1;canvas.width=Math.max(1,Math.round(box.width*ratio));canvas.height=Math.max(1,Math.round(box.height*ratio));ctx.setTransform(1,0,0,1,0,0);ctx.clearRect(0,0,canvas.width,canvas.height);const view=viewport(box);ctx.setTransform(view.scale*ratio,0,0,view.scale*ratio,view.x*ratio,view.y*ratio);
  for(const [index,y] of lineYs.entries()){ctx.strokeStyle=index===2?'#93a591':'#c2cbc0';ctx.lineWidth=index===2?2:1;ctx.setLineDash(index===2?[]:[8,7]);ctx.beginPath();ctx.moveTo(20,y);ctx.lineTo(WIDTH-20,y);ctx.stroke()}ctx.setLineDash([]);
  for(const s of session.draft.strokes){for(let i=0;i<s.points.length;i++){const p=s.points[i],prior=s.points[Math.max(0,i-1)];ctx.strokeStyle='#183c33';ctx.fillStyle='#183c33';ctx.lineWidth=2+Math.max(0,Math.min(1,p.pressure))*5;ctx.lineCap='round';ctx.lineJoin='round';ctx.beginPath();if(i===0){ctx.arc(p.x,p.y,ctx.lineWidth/2,0,Math.PI*2);ctx.fill()}else{ctx.moveTo(prior.x,prior.y);ctx.lineTo(p.x,p.y);ctx.stroke()}}}
  const d=session.draft;root.dataset.visual=d.mode;root.dataset.sealed=String(d.sealed);modelBox.hidden=d.mode==='hidden';comparison.hidden=!d.sealed;root.querySelector('[data-ink-guidance]').hidden=d.mode==='hidden'&&!d.sealed;root.querySelector('[data-ink-meaning]').hidden=!d.meaning_visible;root.querySelector('[data-ink-sound]').hidden=!d.sound_visible;
  for(const button of root.querySelectorAll('[data-ink-edit]'))button.disabled=d.sealed||!!d.pending_snapshot||session.leaving;
  root.querySelector('[data-ink-compare]').disabled=session.comparing||session.leaving;root.querySelector('[data-ink-new]').disabled=session.leaving;for(const button of root.querySelectorAll('[data-ink-control]'))button.disabled=session.leaving||session.comparing||!!d.pending_snapshot&&!d.sealed;root.querySelector('[data-ink-show]').hidden=d.mode!=='hidden'||d.sealed;root.querySelector('[data-ink-hide]').hidden=d.mode==='hidden'||d.sealed;root.querySelector('[data-ink-retry-save]').hidden=!session.failed;root.querySelector('[data-ink-device-options]').hidden=d.input_mode==='mouse';for(const input of root.querySelectorAll('[data-ink-mode]'))input.checked=input.dataset.inkMode===d.mode;for(const b of root.querySelectorAll('[data-ink-input]'))b.setAttribute('aria-pressed',String(b.dataset.inkInput===d.input_mode));root.querySelector('[data-ink-new]').hidden=!d.sealed||!!config.external_flow;root.querySelector('[data-ink-compare]').hidden=d.sealed;status.textContent=session.comparing?'Opening the example…':session.failed?'Your ink needs saving. Retry before comparing.':d.sealed?'Compare your shape with the example. You can try again when ready.':d.input_mode==='pen'?'Pencil only. Touches on this pad do not make ink.':'Drawing with '+(d.input_mode==='finger'?'a finger':'a mouse')+'.';
 };
 const sample=e=>{const r=canvas.getBoundingClientRect();return {x:(e.clientX-r.left-viewport(r).x)/viewport(r).scale,y:(e.clientY-r.top-viewport(r).y)/viewport(r).scale,time_ms:e.timeStamp,pressure:Number.isFinite(e.pressure)?e.pressure:0,tilt_x:e.tiltX||0,tilt_y:e.tiltY||0,twist:e.twist||0,width:e.width||0,height:e.height||0};};
 canvas.addEventListener('pointerdown',e=>{if(e.pointerType==='pen'&&session.draft.input_mode==='mouse')session.input('pen');if(session.begin(e.pointerId,e.pointerType,sample(e),(()=>{const r=canvas.getBoundingClientRect(),v=viewport(r);return {css_width:r.width,css_height:r.height,device_pixel_ratio:global.devicePixelRatio||1,viewport_scale:global.visualViewport?.scale||1,units_per_css_pixel:1/v.scale,offset_x:v.x,offset_y:v.y};})())){e.preventDefault();canvas.setPointerCapture(e.pointerId);render();}},{signal});
 canvas.addEventListener('pointermove',e=>{if(session.active!==e.pointerId)return;e.preventDefault();const coalesced=typeof e.getCoalescedEvents==='function'?e.getCoalescedEvents():[];session.move(e.pointerId,(coalesced.length?coalesced:[e]).map(sample));render();},{signal});
 const finish=(e,reason)=>{if(session.active!==e.pointerId)return;e.preventDefault();session.move(e.pointerId,[sample(e)]);session.end(e.pointerId,reason);if(canvas.hasPointerCapture(e.pointerId))canvas.releasePointerCapture(e.pointerId);render();};
 canvas.addEventListener('pointerup',e=>finish(e,null),{signal});canvas.addEventListener('pointercancel',e=>finish(e,'pointer_cancel'),{signal});canvas.addEventListener('lostpointercapture',e=>{if(session.active===e.pointerId){session.end(e.pointerId,'lost_pointer_capture');render();}},{signal});
 const action=(selector,f)=>root.querySelector(selector)?.addEventListener('click',async()=>{try{const result=f();render();await result;render()}catch(e){status.textContent=e.message;send({kind:'error',message:e.message})}},{signal});
 for(const input of root.querySelectorAll('[data-ink-mode]'))input.addEventListener('change',async()=>{if(input.checked){await session.mode(input.dataset.inkMode);render()}},{signal});
 for(const button of root.querySelectorAll('[data-ink-input]'))button.addEventListener('click',()=>{session.input(button.dataset.inkInput);render()},{signal});
 action('[data-ink-show]',()=>session.mode('copy'));action('[data-ink-hide]',()=>session.mode('hidden'));
 action('[data-ink-undo]',()=>session.undo());action('[data-ink-clear]',()=>session.clear());action('[data-ink-retry-save]',()=>session.persist());action('[data-ink-new]',()=>session.newTry());action('[data-ink-compare]',()=>session.compare());
 action('[data-ink-toggle-meaning]',()=>session.cue('meaning',!session.draft.meaning_visible));action('[data-ink-toggle-sound]',()=>session.cue('sound',!session.draft.sound_visible));action('[data-ink-listen]',()=>session.listen());
 action('[data-ink-replay]',async()=>{if(!await session.replay())return;animations.forEach(a=>a.cancel());animations=[];await session.mode('copy');modelBox.hidden=false;const nodes=[...modelBox.querySelectorAll('path')];nodes.forEach((path,index)=>{const length=path.getTotalLength();path.style.strokeDasharray=String(length);path.style.strokeDashoffset=String(length);const animation=path.animate([{strokeDashoffset:length},{strokeDashoffset:0}],{duration:500,delay:index*550,fill:'forwards'});animations.push(animation)});});
 const interrupt=reason=>{if(session.active!==null)session.end(session.active,reason)};
 const resize=new ResizeObserver(()=>{interrupt('canvas_resize');render()});resize.observe(canvas);
 global.addEventListener('pagehide',()=>interrupt('page_hidden'),{signal});document.addEventListener('visibilitychange',()=>{if(document.hidden)interrupt('document_hidden')},{signal});
 render();return {session,audioResult:(attempt_id,ok)=>session.audioResult(attempt_id,ok),flush:async()=>{session.leaving=true;interrupt('navigation_requested');render();const ok=await session.flush();if(!ok)session.leaving=false;render();return ok;},resumeDrawing(){session.leaving=false;render();},dispose(){if(disposed)return;disposed=true;interrupt('pad_unmounted');animations.forEach(a=>a.cancel());resize.disconnect();controller.abort();}};
}
const api={VERSION,MODEL,WIDTH,HEIGHT,GLYPHS,model,guides,validate,InkSession,install};global.RufrenHandwriting=api;if(typeof module!=='undefined')module.exports=api;
})(globalThis);
