// Focused actual-Dioxus non-text/focus checks in a disposable browser context.
// No relay/provider calls. Preview mode is explicitly source-CSS evidence only.
// https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html
const {chromium}=require(process.env.RUFREN_PLAYWRIGHT_MODULE||'playwright');
const fs=require('node:fs/promises'),path=require('node:path'),assert=require('node:assert/strict');
const base=process.env.RUFREN_QA_URL||'http://127.0.0.1:8091';
const output=process.env.RUFREN_FOCUS_OUTPUT||'.local/control-focus-qa';
const preview=process.env.RUFREN_FOCUS_PREVIEW==='1';
const report={base,preview_source_css:preview,checks:[],errors:[],blocked_requests:[],limits:['Computed solid authored colors, not antialiased pixels or native popup/checkbox appearance.','Keyboard Tab/Shift+Tab reachability and visible authored focus are exercised; this is not a screen-reader or complete accessibility audit.','Only disposable synthetic passage/audio data; no microphone capture, recognition or teacher request.']};
let browser,page;
// Runs against actual inherited styles in the browser; rejects rather than rounds sub-3 ratios.
function inspectPaint({selector,kind,paintSelector}){
 const e=document.querySelector(selector),paint=paintSelector?document.querySelector(paintSelector):e;
 if(!e||!paint)throw Error('Missing control '+selector);
 const color=x=>{const m=x.match(/^rgba?\(([^)]+)\)$/);if(!m)throw Error('Unsupported color '+x);const n=m[1].split(/[,/\s]+/).filter(Boolean).map(Number);return [n[0],n[1],n[2],n[3]??1];};
 const over=(a,b)=>[...a.slice(0,3).map((v,i)=>v*a[3]+b[i]*(1-a[3])),1];
 function background(start){let result=[255,255,255,1];const chain=[];for(let n=start;n;n=n.parentElement)chain.push(n);for(const n of chain.reverse())result=over(color(getComputedStyle(n).backgroundColor),result);return result;}
 const luminance=c=>c.slice(0,3).map(v=>{const x=v/255;return x<=.04045?x/12.92:((x+.055)/1.055)**2.4;}).reduce((s,v,i)=>s+v*[.2126,.7152,.0722][i],0);
 const ratio=(a,b)=>{const x=luminance(a),y=luminance(b);return (Math.max(x,y)+.05)/(Math.min(x,y)+.05);};
 const s=getComputedStyle(paint),outside=background(paint.parentElement),inside=background(paint),surfaces=kind==='border'?[inside,outside]:[outside];
 const surface=paint.closest('.audio-region-surface');if(surface)surfaces.push(background(surface.querySelector('canvas')));
 const raw=kind==='border'?s.borderTopColor:kind==='selection'?(s.boxShadow.match(/rgba?\([^)]+\)/)?.[0]||'missing selection marker'):s.outlineColor;
 if(kind==='selection')surfaces.push(inside);
 const ratios=surfaces.map(bg=>ratio(over(color(raw),bg),bg));
 const rect=paint.getBoundingClientRect();
 return {selector,paintSelector:paintSelector||selector,kind,color:raw,backgrounds:surfaces.map(x=>x.slice(0,3)),ratios,minimum_ratio:Math.min(...ratios),border_width:s.borderTopWidth,outline_width:s.outlineWidth,outline_style:s.outlineStyle,outline_offset:s.outlineOffset,box_shadow:s.boxShadow,focused:document.activeElement===e,focus_visible:e.matches(':focus-visible'),rect:{x:rect.x,y:rect.y,width:rect.width,height:rect.height},opacity:s.opacity};
}
async function checkPaint(selector,kind,paintSelector){
 const row=await page.evaluate(inspectPaint,{selector,kind,paintSelector});
 report.checks.push({...row,viewport:page.viewportSize()});
 assert(row.minimum_ratio>=3,`${kind} ${selector}: ${row.minimum_ratio.toFixed(4)}:1 below3:1`);
 if(kind==='border')assert(parseFloat(row.border_width)>0,'Field has no visible authored boundary');
 if(kind==='focus'){assert(row.focused,'Target lost actual keyboard focus');assert(row.focus_visible,'Keyboard target lacks :focus-visible');assert(parseFloat(row.outline_width)>=2&&row.outline_style!=='none','No substantial visible outline');}
 return row;
}
async function focus(selector,paintSelector){
 const target=page.locator(selector);await target.scrollIntoViewIfNeeded();await target.focus();
 // Verify actual keyboard return to the same control, rather than only programmatic focus.
 await page.keyboard.press('Tab');await page.keyboard.press('Shift+Tab');
 assert(await target.evaluate(e=>e===document.activeElement),'Control unreachable by keyboard round-trip: '+selector);
 await page.mouse.move(0,0);return checkPaint(selector,'focus',paintSelector);
}
async function nav(name){await page.getByRole('navigation',{name:'Main navigation'}).getByRole('button',{name:new RegExp(name+'$')}).click();}
async function screenshot(name){await page.screenshot({path:path.join(output,name+'.png')});}
(async()=>{
 await fs.mkdir(output,{recursive:true});
 const launch={headless:true};if(process.env.RUFREN_CHROMIUM_EXECUTABLE)launch.executablePath=process.env.RUFREN_CHROMIUM_EXECUTABLE;
 browser=await chromium.launch(launch);const context=await browser.newContext({viewport:{width:1380,height:1000}});
 const allowed=new URL(base).origin;
 await context.route('**/*',route=>{const u=new URL(route.request().url());if((['http:','https:'].includes(u.protocol)&&u.origin!==allowed)||/^\/(api|v1)\//.test(u.pathname)){report.blocked_requests.push(u.origin+u.pathname);return route.abort();}return route.continue();});
 page=await context.newPage();page.setDefaultTimeout(30000);page.on('pageerror',e=>report.errors.push(e.message));
 page.on('request',r=>{if(/\.wasm(?:\?|$)/.test(r.url()))report.wasm=r.url();});
 await page.goto(base,{waitUntil:'networkidle'});await page.getByRole('navigation').waitFor();
 report.build=await page.evaluate(async()=>{const r=await fetch('/offline-build.json');return r.ok?(await r.json()).version:null;});
 if(preview){const css=(await Promise.all(['style.css','audio_studio.css','tutor_view.css'].map(n=>fs.readFile('crates/rufren-app/src/'+n,'utf8')))).join('\n');await page.evaluate(css=>{const style=document.createElement('style');style.dataset.focusPreview='true';style.textContent=css;document.body.append(style);},css);}
 await focus('.brand');await focus('.nav-item.active');await focus('.hero .primary');
 await nav('Library');
 for(const selector of ['#passage-title','#passage-russian','#passage-meaning']){await checkPaint(selector,'border');await focus(selector);}
 await screenshot('library-field-focus');
 await page.locator('#passage-title').fill('Focused control fixture');await page.locator('#passage-russian').fill('Ёжик смо́трит на звёзды.');await page.locator('#passage-meaning').fill('The hedgehog looks at the stars.');await page.getByRole('button',{name:'Save passage',exact:true}).click();
 await checkPaint('.passage-card.selected','selection');
 await nav('Lessons');await page.locator('.lesson-link').first().click();await focus('.lesson-link.selected');
 if(!preview){assert.equal(await page.locator('.lesson-link.selected').getAttribute('aria-pressed'),'true');assert((await page.locator('.lesson-link.selected').textContent()).includes('✓'));}
 const selected=await page.locator('.lesson-link.selected').evaluate(e=>getComputedStyle(e).boxShadow);assert.notEqual(selected,'none','Selection must include a structural marker');
 await nav('Teacher');await checkPaint('#teacher-lesson','border');await focus('#teacher-lesson');
 await focus('.teacher-context-preview>summary');await screenshot('teacher-summary-focus');
 await nav('Settings');const checkbox=page.locator('input[type=checkbox]').first();if(await checkbox.count())await focus('#'+await checkbox.getAttribute('id'));
 await nav('Library');await page.locator('.passage-card').filter({hasText:'Focused control fixture'}).click();await page.getByRole('button',{name:'Hear & explore the voice',exact:true}).click();
 await focus('#audio-import-reference','label[for="audio-import-reference"]');await checkPaint('#audio-library-reference','border');await focus('#audio-library-reference');
 // One existing public synthetic crop enables real region handles; no synthesis/inference.
 await page.locator('#audio-import-reference').setInputFiles(path.resolve('research/artifacts/silero-stress-v1/castle-word-replay.wav'));
 await page.waitForFunction(()=>document.getElementById('audio-name-reference').textContent.includes('castle-word'));
 const asset=await page.locator('#audio-library-reference').inputValue();
 await page.locator('#audio-region-lab>summary').click();await page.locator('#audio-region-source-region_a').selectOption(asset);
 await page.waitForFunction(()=>document.getElementById('audio-region-metrics-region_a').textContent.includes('Selected duration'));
 const handle='[data-region-slot="region_a"] [data-region-track="wave"] [data-region-edge="start"]';
 await focus(handle);await checkPaint('#audio-region-from-region_a','border');await focus('#audio-region-from-region_a');
 if(!preview){
  const endHandle=handle.replace('"start"','"end"');
  const before=await page.locator(handle).getAttribute('aria-valuenow');await page.locator(handle).click();assert(await page.locator(handle).evaluate(e=>document.activeElement===e),'Pointer boundary selection must focus visible slider');await page.keyboard.press('ArrowRight');
  assert.equal(Number(await page.locator(handle).getAttribute('aria-valuenow')),Number(before)+1);
  assert.equal(Number(await page.locator(endHandle).getAttribute('aria-valuemin')),Number(before)+2);
  assert.equal(Number(await page.locator(handle).getAttribute('aria-valuemax')),Number(await page.locator(endHandle).getAttribute('aria-valuenow'))-1);
  const help=await page.locator(handle).getAttribute('aria-describedby');assert(help&&await page.locator('#'+help).count(),'Slider must identify its keyboard help');
 }
 for(const [name,width,height]of[['desktop',1380,1000],['tablet',1024,768],['phone',390,844]]){
  await page.setViewportSize({width,height});await focus(handle);await screenshot('region-focus-'+name);
  await checkPaint('#audio-region-from-region_a','border');await focus('#audio-region-source-region_a');
 }
 assert.deepEqual(report.errors,[]);assert.deepEqual(report.blocked_requests,[],'No API/provider requests should be attempted');
 report.ok=true;report.checked_paints=report.checks.length;
 await fs.writeFile(path.join(output,'results.json'),JSON.stringify(report,null,2));console.log(JSON.stringify({ok:true,preview,build:report.build,checked_paints:report.checked_paints,min_ratio:Math.min(...report.checks.map(r=>r.minimum_ratio)),errors:report.errors}));await browser.close();
})().catch(async error=>{report.ok=false;report.error=error.stack;if(page)await page.screenshot({path:path.join(output,'failure.png')}).catch(()=>{});await fs.mkdir(output,{recursive:true});await fs.writeFile(path.join(output,'results.json'),JSON.stringify(report,null,2));console.error(error);await browser?.close();process.exitCode=1;});
