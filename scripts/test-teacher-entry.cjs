// Isolated first-ever Teacher entry; all relay traffic is intercepted. No inference.
const { chromium } = require(process.env.RUFREN_PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const { navigate } = require('./qa-navigation.cjs');
const { notebookResponse } = require('./test-teacher-notebook.cjs');
const APP = process.env.RUFREN_QA_URL || 'http://127.0.0.1:8091';
const RELAY = 'http://127.0.0.1:18997';
const OUTPUT = path.resolve(process.env.RUFREN_ENTRY_QA_OUTPUT || '.local/teacher-entry-qa');
const status = { enabled:true, provider:'qa_synthetic', model:'Synthetic entry fixture', message:'No inference',
  data_sent:'Synthetic only', max_text_bytes:8192, max_context_bytes:8192, remaining_reservations:99,
  remaining_budget_micro_usd:0, accounting:'subscription_quota', lesson_flow:'guided_v1' };
async function main() {
  await fs.mkdir(OUTPUT,{recursive:true});
  const retained = process.env.RUFREN_ENTRY_LESSON ? JSON.parse(await fs.readFile(process.env.RUFREN_ENTRY_LESSON,'utf8')) : null;
  const browser = await chromium.launch({headless:true});
  const context = await browser.newContext({viewport:{width:820,height:1180}});
  const page = await context.newPage(), requests=[], errors=[], forbidden=[], checks=[];
  let mode='delayed', releaseStatus;
  const gate=new Promise(resolve=>releaseStatus=resolve);
  page.on('pageerror',error=>errors.push(error.message));
  const headers={'Access-Control-Allow-Origin':'*','Access-Control-Allow-Headers':'*','Access-Control-Allow-Methods':'GET,POST,OPTIONS'};
  await context.route('**/*',async route=>{
    const req=route.request(),url=new URL(req.url());
    if(url.origin===new URL(APP).origin&&!url.pathname.startsWith('/api/'))return route.continue();
    if(url.origin!==RELAY){forbidden.push(req.url());return route.abort('blockedbyclient');}
    if(req.method()==='OPTIONS')return route.fulfill({status:204,headers});
    const request=req.method()==='POST'?req.postDataJSON():null;
    requests.push({path:url.pathname,request,mode});
    if(url.pathname==='/api/teacher/status'){
      if(mode==='delayed')await gate;
      if(mode==='failed')return route.fulfill({status:503,headers,body:'Synthetic unavailable'});
      return route.fulfill({status:200,headers,contentType:'application/json',body:JSON.stringify({...status,lesson_flow:mode==='legacy'?null:'guided_v1'})});
    }
    if(url.pathname==='/api/teacher/teach'){
      if(request.mode==='teach_lesson')assert.equal(request.input.lesson_flow??null,mode==='legacy'?null:'guided_v1');
      const response=request.mode==='teach_lesson'&&retained&&mode!=='legacy'?{...structuredClone(retained),request}:notebookResponse(request,requests.length);
      return route.fulfill({status:200,headers,contentType:'application/json',body:JSON.stringify(response)});
    }
    forbidden.push(req.url());return route.abort('blockedbyclient');
  });
  const primary=()=>page.getByRole('button',{name:'Generate a new AI lesson',exact:false});
  const flow=page.locator('.teacher-lesson-flow');
  const openGeneration=async()=>{const panel=page.locator('details.teacher-start');if(!await panel.evaluate(e=>e.open))await panel.locator(':scope > summary').click();};
  const noInternalContext=async()=>{
    assert.equal(await page.locator('.teacher-context-text,.teacher-context-preview').count(),0);
    assert(!(await page.locator('body').innerText()).includes('Excerpts are quoted data'));
  };
  const teach=async()=>{
    const reply=page.waitForResponse(r=>r.url()===RELAY+'/api/teacher/teach'&&r.request().method()==='POST');
    await primary().click();assert((await reply).ok());
    await page.waitForFunction(()=>![...document.querySelectorAll('button')].some(b=>b.textContent.includes('Generating your AI lesson')||b.textContent.includes('Your teacher is working')));
  };
  try {
    await page.goto(APP);await page.locator('#today-primary').waitFor();
    await page.locator('#today-primary').click();
    assert.equal(requests.filter(r=>r.path==='/api/teacher/teach').length,0,'Starting prepared curriculum makes no lesson generation request');
    await navigate(page,'Teacher');
    assert.equal(await page.locator('details.teacher-start').evaluate(e=>e.open),false,'Generation is optional and collapsed');
    await openGeneration();await noInternalContext();
    assert(await page.getByRole('button',{name:'Connect your teacher',exact:false}).isVisible());
    assert.equal(await primary().count(),0,'Blank setup offers a useful action instead of a failing teach request');
    assert.equal(requests.length,0);
    await page.screenshot({path:path.join(OUTPUT,'blank-connection.png'),fullPage:true});
    await page.getByRole('button',{name:'Connect your teacher',exact:false}).click();
    await page.locator('#sync-key').waitFor();
    await page.locator('#sync-url').fill(RELAY);await page.locator('#sync-key').fill('synthetic-entry-no-secret');
    checks.push('Prepared Today start never generates; optional AI lesson disclosure offers setup and routes directly to its fields');
    await navigate(page,'Teacher');await openGeneration();
    const connecting=page.getByRole('button',{name:'Connecting to your teacher',exact:false});
    await connecting.waitFor();assert(await connecting.isDisabled());
    assert.equal(requests.filter(r=>r.path==='/api/teacher/teach').length,0);
    await page.screenshot({path:path.join(OUTPUT,'checking-capabilities.png'),fullPage:true});
    releaseStatus();await primary().waitFor();assert(await primary().isEnabled());
    await teach();await flow.waitFor();await noInternalContext();
    assert.equal(await flow.locator('.teacher-step-row').count(),0,'Optional step outline starts closed');
    await flow.getByRole('button',{name:/Lesson steps/}).click();
    assert(await flow.locator('.teacher-step-row').count()>1);
    await flow.getByRole('button',{name:/Lesson steps/}).click();
    checks.push('Delayed automatic capabilities cannot race a lesson request; ready request explicitly asks for guided_v1');
    const input=flow.locator('.teacher-exercise textarea').first();
    await input.waitFor();
    const response=retained?'Good, thanks. And you?':'Я читаю.';
    await input.fill(response);
    const source=await flow.getAttribute('data-lesson-document');
    await page.screenshot({path:path.join(OUTPUT,'first-response-draft.png'),fullPage:true});
    await navigate(page,'Today');assert.match(await page.locator('#today-primary').innerText(),/sounds & letters/);await page.getByRole('button',{name:'Return to this AI lesson',exact:true}).click();
    assert.equal(await input.inputValue(),response);
    await flow.getByRole('button',{name:'Discuss my answer',exact:false}).click();
    await page.locator('.teacher-feedback-task').waitFor();
    const submitted=requests.filter(r=>r.request?.mode==='answer_feedback').at(-1).request.input;
    assert.equal(submitted.response,response);
    assert((await page.locator('.teacher-feedback-task').innerText()).includes(response));
    await page.screenshot({path:path.join(OUTPUT,'first-feedback.png'),fullPage:true});
    checks.push('First guided response draft survives Today round trip; submit saves exact response and shows feedback beside it');
    await page.getByRole('button',{name:'Return to this lesson & exercise',exact:false}).click();
    await flow.waitFor();assert.equal(await flow.getAttribute('data-lesson-document'),source);
    mode='failed';await navigate(page,'Today');await page.getByRole('button',{name:'Return to this AI lesson',exact:true}).click();
    assert(await flow.isVisible(),'Saved lesson remains readable after connection failure');
    await openGeneration();
    await page.getByRole('button',{name:'Teacher connection',exact:false}).waitFor();
    assert(await primary().isEnabled(),'Transient failure permits deliberate retry');
    await teach();
    await page.waitForFunction(()=>document.querySelector('.teacher-lesson-flow'));
    assert.equal(requests.filter(r=>r.request?.mode==='teach_lesson').at(-1).request.input.lesson_flow,'guided_v1');
    checks.push('Failed status keeps saved lesson readable and explicit retry preserves guided request');
    mode='legacy';await navigate(page,'Today');await navigate(page,'Teacher');
    await openGeneration();
    await teach();await page.waitForFunction(()=>document.querySelector('.teacher-lesson-flow'));
    assert.equal(requests.filter(r=>r.request?.mode==='teach_lesson').at(-1).request.input.lesson_flow??null,null);
    checks.push('Successful explicit legacy capability remains compatible');
    assert.deepEqual(errors,[]);assert.deepEqual(forbidden,[]);
    await fs.writeFile(path.join(OUTPUT,'result.json'),JSON.stringify({ok:true,checks,requests,errors,forbidden,retained_lesson:!!retained},null,2));
    console.log('PASS first-ever Teacher entry, capability delay/error, first response, saved access and legacy compatibility');
  }catch(error){
    await page.screenshot({path:path.join(OUTPUT,'failure.png'),fullPage:true}).catch(()=>{});
    await fs.writeFile(path.join(OUTPUT,'failure.txt'),await page.locator('body').innerText()).catch(()=>{});
    throw error;
  }finally{releaseStatus();await context.close();await browser.close();}
}
if(require.main===module)main().catch(error=>{console.error(error);process.exitCode=1;});
