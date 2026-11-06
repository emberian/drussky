// Scoped rendered HTML text contrast regression; not a full accessibility audit.
// Text thresholds: https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html
// Uses disposable data, public synthetic audio, and no model/provider calls.
const {chromium}=require(process.env.RUFREN_PLAYWRIGHT_MODULE||'playwright');
const fs=require('node:fs/promises'),path=require('node:path'),assert=require('node:assert/strict');
const url=process.env.RUFREN_QA_URL||'http://127.0.0.1:8091';
const output=process.env.RUFREN_CONTRAST_OUTPUT||'.local/text-contrast-qa';
const preview=process.env.RUFREN_CONTRAST_PREVIEW==='1';
let browser,page;
const results={url,preview_source_css:preview,scenes:[],errors:[],limits:['HTML text/control paint colors, not antialiased edge pixels or a VoiceOver audit.','Solid background/alpha/ancestor opacity and brightness are composited. Non-solid backgrounds or unsupported filters need manual review.','Laid-out scrollable text is included; closed details, native-disabled controls, aria-hidden decoration and invisible text are excluded.','Native browser popup menus, text selection colors and sibling overlays are outside this ratio check.']};
const teacher={
 request:{mode:'answer_feedback',input:{request_id:'contrast-fixture',revision_id:'fixture-revision',attempt_id:null,context_id:null,exercise_id:null,skill:'writing',task:'Synthetic contrast test',prompt:'Say that you are reading.',reference:null,response:'Я читать.',cues:[],evidence:'written_text'}},
 provenance:{provider:'qa_synthetic',model:'fixture',review_status:'ai_generated_teaching',scope:'Synthetic text contrast fixture; no model call',prompt_version:'fixture-v1',response_id:'contrast-response-1'},
 usage:{accounting:'subscription_quota',input_tokens:0,output_tokens:0,estimated_micro_usd:0,reserved_micro_usd:0},
 content:{explanation:'**One useful contrast.**\n\nUse a finite verb in a complete sentence. The explanation can be rich while the next practice step stays small.',examples:[],practice:[],next_step:'Return to one short, meaningful sentence.',caveats:['**A small note.** This is a synthetic display fixture, not an assessment.'],feedback:{verdict:'partly_correct',suggested_response:'Я читаю книгу.',corrections:[{original:'Я читать.',replacement:'Я читаю.',reason:'Use the first-person form.'}]}}
};
// This function deliberately runs in the browser against computed, inherited styles.
function auditText(){
 const colorCache=new Map(),canvas=document.createElement('canvas');canvas.width=canvas.height=1;const ctx=canvas.getContext('2d',{willReadFrequently:true});
 const rgba=value=>{if(!colorCache.has(value)){const rgb=value.match(/^rgba?\(([^)]+)\)$/);if(rgb){const channels=rgb[1].split(/[,/\s]+/).filter(Boolean).map(Number);colorCache.set(value,[channels[0],channels[1],channels[2],channels.length>3?channels[3]:1]);return [...colorCache.get(value)];}ctx.clearRect(0,0,1,1);ctx.fillStyle=value;ctx.fillRect(0,0,1,1);const p=ctx.getImageData(0,0,1,1).data;colorCache.set(value,[p[0],p[1],p[2],p[3]/255]);}return [...colorCache.get(value)];};
 const over=(a,b)=>{const alpha=a[3]+b[3]*(1-a[3]);return alpha?[0,1,2].map(i=>(a[i]*a[3]+b[i]*b[3]*(1-a[3]))/alpha).concat(alpha):[0,0,0,0];};
 const luminance=rgb=>{const linear=rgb.slice(0,3).map(c=>{const x=c/255;return x<=.04045?x/12.92:((x+.055)/1.055)**2.4;});return linear[0]*.2126+linear[1]*.7152+linear[2]*.0722;};
 const ratio=(a,b)=>{const x=luminance(a),y=luminance(b);return (Math.max(x,y)+.05)/(Math.min(x,y)+.05);};
 const styles=new Map(),style=e=>{if(!styles.has(e))styles.set(e,getComputedStyle(e));return styles.get(e);};
 const selector=e=>e.id?'#'+e.id:e.tagName.toLowerCase()+(typeof e.className==='string'&&e.className.trim()?'.'+e.className.trim().split(/\s+/).slice(0,3).join('.'): '');
 const exclusions={decoration:0,disabled:0,not_laid_out:0,invisible:0};
 function eligible(element){
  if(element.closest('[aria-hidden="true"],script,style,canvas,template')){exclusions.decoration++;return false;}
  if(element.matches(':disabled')||element.closest('button:disabled,select:disabled,textarea:disabled,input:disabled')){exclusions.disabled++;return false;}
  const s=style(element);if(s.visibility!=='visible'||s.display==='none'||!element.getClientRects().length){exclusions.not_laid_out++;return false;}
  for(let e=element;e;e=e.parentElement)if(Number(style(e).opacity)===0){exclusions.invisible++;return false;}
  return true;
 }
 const rows=[],manual=[];
 function measure(element,text,kind='text',pseudo=null){
  if(!eligible(element))return;
  const s=pseudo?getComputedStyle(element,pseudo):style(element),raw=s.color;
  let foreground=rgba(raw),background=[0,0,0,0];if(pseudo)foreground[3]*=Number(s.opacity);
  for(let e=element;e;e=e.parentElement){const current=style(e),bg=rgba(current.backgroundColor);foreground=over(foreground,bg);background=over(background,bg);
   if(current.backgroundImage!=='none')manual.push({selector:selector(element),reason:'background image',text:text.slice(0,70)});
   if(current.filter!=='none'){
    const brightness=current.filter.match(/^brightness\(([\d.]+)\)$/);
    if(brightness){for(let c=0;c<3;c++){foreground[c]=Math.min(255,foreground[c]*Number(brightness[1]));background[c]=Math.min(255,background[c]*Number(brightness[1]));}}
    else manual.push({selector:selector(element),reason:'unsupported filter '+current.filter,text:text.slice(0,70)});
   }
   foreground[3]*=Number(current.opacity);background[3]*=Number(current.opacity);
  }
  foreground=over(foreground,[255,255,255,1]);background=over(background,[255,255,255,1]);
  const size=parseFloat(s.fontSize),weight=Number(s.fontWeight)||({bold:700,normal:400}[s.fontWeight]||400),large=size>=24||(size>=18.666666&&weight>=700),minimum=large?3:4.5,contrast=ratio(foreground,background);
  rows.push({selector:selector(element),kind,text:text.replace(/\s+/g,' ').trim().slice(0,100),color:raw,background:style(element).backgroundColor,foreground_rgb:foreground.slice(0,3).map(v=>Number(v.toFixed(2))),effective_background_rgb:background.slice(0,3).map(v=>Number(v.toFixed(2))),font_size:size,font_weight:weight,large,minimum,ratio:Number(contrast.toFixed(4)),pass:contrast+1e-8>=minimum});
 }
 const walker=document.createTreeWalker(document.body,NodeFilter.SHOW_TEXT);let node;
 while(node=walker.nextNode())if(node.textContent.trim()&&node.parentElement){const range=document.createRange();range.selectNodeContents(node);if(range.getClientRects().length)measure(node.parentElement,node.textContent);}
 for(const element of document.querySelectorAll('input,textarea,select')){
  if(element instanceof HTMLSelectElement){measure(element,[...element.selectedOptions].map(o=>o.text).join(', '),'select value');continue;}
  if(['hidden','file','checkbox','radio','range','color'].includes(element.type))continue;
  if(element.value)measure(element,element.type==='password'?'Password field value':element.value,'control value');
  else if(element.placeholder)measure(element,element.placeholder,'placeholder','::placeholder');
 }
 return {rows,violations:rows.filter(row=>!row.pass),manual_review:[...new Map(manual.map(row=>[JSON.stringify(row),row])).values()],exclusions};
}
async function nav(label){await page.getByRole('navigation').getByRole('button',{name:new RegExp(label+'$')}).click();}
async function scene(name,{screenshots=false}={}){
 for(const [device,width,height]of[['desktop',1380,1000],['tablet',1024,768],['phone',390,844]]){
  await page.setViewportSize({width,height});await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
  const report=await page.evaluate(auditText);results.scenes.push({name,device,viewport:{width,height},...report});
  if(screenshots&&device!=='desktop'){await page.screenshot({path:path.join(output,`${name}-${device}.png`),fullPage:true});if(name==='regions'){await page.locator('.audio-region-boundaries').first().scrollIntoViewIfNeeded();await page.screenshot({path:path.join(output,`${name}-controls-${device}.png`)});}}
 }
}
(async()=>{
 await fs.mkdir(output,{recursive:true});const launch={headless:true};if(process.env.RUFREN_CHROMIUM_EXECUTABLE)launch.executablePath=process.env.RUFREN_CHROMIUM_EXECUTABLE;
 browser=await chromium.launch(launch);const context=await browser.newContext({acceptDownloads:true});page=await context.newPage();page.setDefaultTimeout(45000);page.on('pageerror',error=>results.errors.push(error.message));page.on('request',request=>{if(/\.wasm(?:\?|$)/.test(request.url()))results.wasm=request.url();});
 const checker=await context.newPage();await checker.setContent('<body style="background:white"><p id="solid" style="color:black;background:white">Solid text</p><div style="opacity:.5;background:white"><p id="opacity" style="color:black">Faded text</p></div><div style="background:black"><span id="alpha" style="color:rgba(255,255,255,.5)">Alpha text</span></div><div style="background:black;opacity:.5"><span id="nested" style="color:white">Group compositing</span></div><button disabled style="color:#ddd;background:white">Disabled fixture</button><span aria-hidden="true" style="color:#ddd">Decoration</span></body>');const math=await checker.evaluate(auditText),row=id=>math.rows.find(r=>r.selector==='#'+id);assert.equal(row('solid').ratio,21);assert(Math.abs(row('opacity').ratio-3.9767)<.001);assert(Math.abs(row('alpha').ratio-5.2808)<.001);assert(Math.abs(row('nested').ratio-3.9767)<.001);assert(!math.rows.some(r=>r.text.includes('Disabled fixture')||r.text==='Decoration'));results.compositing_fixture_passed=true;await checker.close();
 await page.goto(url,{waitUntil:'networkidle'});await page.getByRole('navigation').waitFor();
 if(preview){const css=await fs.readFile('crates/rufren-app/src/style.css','utf8')+'\n'+await fs.readFile('crates/rufren-app/src/audio_studio.css','utf8')+'\n'+await fs.readFile('crates/rufren-app/src/tutor_view.css','utf8');await page.evaluate(css=>{const style=document.createElement('style');style.dataset.contrastPreview='true';style.textContent=css;document.body.append(style);},css);}
 await scene('today');
 await nav('Lessons');await page.locator('.lesson-link').first().click();const alphabet=page.locator('.alphabet');if(!await alphabet.evaluate(e=>e.open))await alphabet.locator('summary').click();await scene('lessons');
 await page.evaluate(value=>window.rufrenLibrary.write('rufren.v1.meta.teacher_response',JSON.stringify(value)),teacher);await nav('Teacher');await page.locator('.teacher-answer').waitFor();const nuance=page.getByText('Nuance and uncertainty',{exact:true});if(await nuance.count())await nuance.click();const recent=page.locator('.teacher-context-preview');if(await recent.count()&&!await recent.first().evaluate(e=>e.open))await recent.first().locator('summary').click();await page.getByText('Ask a different question',{exact:true}).click();await scene('teacher',{screenshots:true});
 const freeAnswer={task:'explain',provenance:{provider:'qa_synthetic',model:'fixture',review_status:'unreviewed_ai_suggestion',scope:'Synthetic contrast fixture',prompt_version:'fixture-v1',response_id:'contrast-tutor-response'},usage:{accounting:'api_token_estimate',input_tokens:0,output_tokens:0,estimated_micro_usd:0,reserved_micro_usd:0},content:{explanation:'An example of visible teacher explanation text.',examples:[{russian:'Я читаю книгу.',english:'I am reading a book.',note:'A short display example.'}],next_step:'Try another short sentence.',caveats:['A synthetic note, not a model assessment.']}};await page.evaluate(value=>window.rufrenLibrary.write('rufren.v1.meta.tutor_response',JSON.stringify(value)),freeAnswer);await nav('Today');await nav('Teacher');await page.getByText('Ask a different question',{exact:true}).click();await page.locator('.tutor-answer-footer').waitFor();const things=page.locator('.tutor-caveats');if(!await things.evaluate(e=>e.open))await things.locator('summary').click();await scene('teacher-answer');
 await nav('Settings');await scene('settings');
 await nav('Library');await page.locator('#passage-title').fill('Text contrast fixture');await page.locator('#passage-russian').fill('Привет!');await page.locator('#passage-meaning').fill('Hello!');await page.locator('#passage-source').fill('Public synthetic UI fixture, not pronunciation feedback.');await page.getByRole('button',{name:'Save passage',exact:true}).click();await scene('library');
 await page.getByRole('button',{name:'Read for meaning',exact:true}).click();await scene('practice');
 await nav('Library');await page.locator('.passage-card').filter({hasText:'Text contrast fixture'}).first().click();await page.getByRole('button',{name:'Hear & explore the voice',exact:true}).click();await page.locator('#audio-import-reference').setInputFiles(path.resolve('research/artifacts/silero-stress-v1/castle-word-replay.wav'));await page.waitForFunction(()=>document.getElementById('audio-name-reference').textContent.includes('castle-word'));
 await page.locator('#audio-import-reference').setInputFiles(path.resolve('research/artifacts/silero-stress-v1/lock-word-replay.wav'));await page.waitForFunction(()=>document.getElementById('audio-name-reference').textContent.includes('lock-word'));await scene('studio');
 const records=await page.evaluate(()=>new Promise((resolve,reject)=>{const r=indexedDB.open('rufren-audio-v1',1);r.onsuccess=()=>{const db=r.result,q=db.transaction('assets').objectStore('assets').getAll();q.onsuccess=()=>{resolve(q.result.map(({blob,...r})=>r));db.close();};q.onerror=()=>reject(q.error);};r.onerror=()=>reject(r.error);}));
 await page.locator('#audio-region-lab>summary').click();await page.locator('#audio-region-source-region_a').selectOption(records.find(r=>r.name==='castle-word-replay.wav').asset_id);await page.locator('#audio-region-source-region_b').selectOption(records.find(r=>r.name==='lock-word-replay.wav').asset_id);await page.waitForFunction(()=>[...document.querySelectorAll('.audio-region-metrics')].every(e=>e.textContent.includes('Selected duration')));
 for(const details of await page.locator('.audio-region-precise').all())if(!await details.evaluate(e=>e.open))await details.locator('summary').click();await page.locator('#audio-region-pitch').check();await scene('regions',{screenshots:true});
 // Hovered controls use brightness(), so check that effective text paint too.
 await page.setViewportSize({width:1380,height:1000});await page.locator('[data-region-play="region_a"]').hover();results.scenes.push({name:'regions-hover',device:'desktop',...await page.evaluate(auditText)});
 const failures=results.scenes.flatMap(({name,device,violations})=>violations.map(row=>({scene:name,device,...row})));results.violation_count=failures.length;results.manual_review_count=results.scenes.reduce((sum,scene)=>sum+scene.manual_review.length,0);results.checked_paints=results.scenes.reduce((sum,scene)=>sum+scene.rows.length,0);results.failures=failures;results.ok=failures.length===0&&results.errors.length===0;
 await fs.writeFile(path.join(output,'results.json'),JSON.stringify(results,null,2));
 const grouped=[...new Map(failures.map(row=>[[row.selector,row.color,row.effective_background_rgb.join(','),row.minimum].join('|'),row])).values()];console.log(JSON.stringify({ok:results.ok,checked_paints:results.checked_paints,violation_count:results.violation_count,manual_review_count:results.manual_review_count,unique_failures:grouped.map(({scene,device,selector,kind,text,color,effective_background_rgb,ratio,minimum})=>({scene,device,selector,kind,text,color,background:effective_background_rgb,ratio,minimum})),errors:results.errors},null,2));
 if(process.env.RUFREN_CONTRAST_ALLOW_FAILURE!=='1'){assert.equal(failures.length,0,'Visible text paint below its scoped contrast threshold');assert.deepEqual(results.errors,[]);}
 await browser.close();
})().catch(async error=>{console.error(error);if(page)await page.screenshot({path:path.join(output,'failure.png'),fullPage:true}).catch(()=>{});results.ok=false;results.error=error.stack;await fs.mkdir(output,{recursive:true});await fs.writeFile(path.join(output,'results.json'),JSON.stringify(results,null,2));await browser?.close();process.exitCode=1;});
