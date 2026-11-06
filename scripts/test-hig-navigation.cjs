// Real app navigation and layout, in an isolated profile with intercepted synthetic teaching.
const { chromium } = require(process.env.RUFREN_PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const { navigate, assertNavigation } = require('./qa-navigation.cjs');
const { notebookResponse, openLessonTools } = require('./test-teacher-notebook.cjs');
const APP = process.env.RUFREN_QA_URL || 'http://127.0.0.1:8091';
const RELAY = 'http://127.0.0.1:18996';
const OUTPUT = path.resolve(process.env.RUFREN_HIG_QA_OUTPUT || '.local/hig-navigation-qa');
const status = { enabled: true, provider: 'qa_synthetic', model: 'Synthetic navigation fixture',
  message: 'No inference', data_sent: 'Synthetic only', max_text_bytes: 8192, max_context_bytes: 8192,
  remaining_reservations: 99, remaining_budget_micro_usd: 0, accounting: 'subscription_quota',
  lesson_flow: 'guided_v1', dialogue: 'threaded_v1' };
async function main() {
  await fs.mkdir(OUTPUT, {recursive:true});
  const browser = await chromium.launch({headless:true});
  const context = await browser.newContext({viewport:{width:1200,height:900}});
  const page = await context.newPage();
  const errors=[], forbidden=[], requests=[], layouts=[], checks=[];
  page.on('pageerror', e=>errors.push(e.message));
  await context.route('**/*', async route=>{
    const req=route.request(), url=new URL(req.url());
    if(url.origin===new URL(APP).origin && !url.pathname.startsWith('/api/'))return route.continue();
    if(url.origin!==RELAY){forbidden.push(req.url());return route.abort('blockedbyclient');}
    const headers={'Access-Control-Allow-Origin':'*','Access-Control-Allow-Headers':'*','Access-Control-Allow-Methods':'GET,POST,OPTIONS'};
    if(req.method()==='OPTIONS')return route.fulfill({status:204,headers});
    requests.push({path:url.pathname,method:req.method()});
    let body;
    if(url.pathname==='/api/teacher/status'||url.pathname==='/api/tutor/status')body=status;
    else if(url.pathname==='/api/teacher/teach'){
      const input=req.postDataJSON();assert.equal(input.mode,'teach_lesson','Only one deliberate synthetic lesson generation is permitted');
      body=notebookResponse(input,1);
    }else{forbidden.push(req.url());return route.abort('blockedbyclient');}
    return route.fulfill({status:200,headers,contentType:'application/json',body:JSON.stringify(body)});
  });
  const nav=async name=>{await navigate(page,name);await page.waitForFunction(()=>document.activeElement===document.querySelector('main.main h1'));};
  const flow=page.locator('.teacher-lesson-flow');
  const input=page.locator('.teacher-lesson-current .teacher-exercise textarea');
  const teachingDraft='Навигация: мой незаконченный ответ';
  const talkDraft='I want to talk about the sound of Russian, later.';
  const active=async expected=>{
    const root=page.locator('.compact-navigation:visible, .primary-navigation:visible');
    assert.equal(await root.locator('[aria-current="page"]').innerText(),expected);
    assert.equal(await root.locator('[aria-current="page"]').count(),1);
    assert.equal(await root.locator('.destination.active').count(),1);
    assert.equal(await root.locator('.destination.active').innerText(),expected);
  };
  const capture=async(name,destination)=>{
    assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),`${name}/${destination} has no horizontal overflow`);
    await page.evaluate(async()=>{await new Promise(requestAnimationFrame);await new Promise(requestAnimationFrame);scrollTo(0,document.documentElement.scrollHeight);await new Promise(requestAnimationFrame);await new Promise(requestAnimationFrame);});
    await page.waitForFunction(()=>Math.abs(scrollY-Math.max(0,document.documentElement.scrollHeight-innerHeight))<2);
    const geometry=await page.evaluate(({name,destination})=>{
      const nav=document.querySelector('.compact-navigation');
      const tab=getComputedStyle(nav).display==='none'?null:nav.getBoundingClientRect();
      const controls=[...document.querySelectorAll('main.main button,main.main input,main.main textarea,main.main summary')].filter(n=>{if(!n.getClientRects().length)return false;for(let p=n.parentElement;p;p=p.parentElement)if(p.tagName==='DETAILS'&&!p.open&&!p.querySelector(':scope > summary')?.contains(n))return false;return true;});
      const last=controls.at(-1)?.getBoundingClientRect();
      return {name,destination,viewport:{width:innerWidth,height:innerHeight},tab:tab?.toJSON(),last:last?.toJSON(),paddingBottom:getComputedStyle(document.querySelector('main.main')).paddingBottom};
    },{name,destination});
    layouts.push(geometry);
    if(geometry.tab&&geometry.last)assert(geometry.last.bottom<=geometry.tab.top+1,`${name}/${destination} last control is clear of bottom navigation: ${JSON.stringify(geometry)}`);
    await page.evaluate(()=>scrollTo(0,0));
    await page.screenshot({path:path.join(OUTPUT,`${name}-${destination}.png`)});
  };
  try{
    await page.goto(APP);await page.locator('#tab-today').waitFor({state:'attached'});
    await nav('Today');
    assert.equal(await page.getByText('Browse your saved Teacher returns',{exact:true}).count(),0);
    assert.equal(await page.getByText('Browse your saved passage returns',{exact:true}).count(),0);
    assert.equal(await page.locator('#today-primary').innerText(),'Start learning');
    await nav('Settings');await page.locator('#sync-url').fill(RELAY);await page.locator('#sync-key').fill('synthetic-navigation-no-secret');
    await nav('Learn');await page.locator('.lesson-link').first().click();
    await page.getByRole('button',{name:'Learn with the AI teacher',exact:false}).click();
    await openLessonTools(page);
    const panel=page.locator('details.teacher-start');if(await panel.getAttribute('open')===null)await panel.locator(':scope > summary').click();
    await page.getByRole('button',{name:'Teach me this lesson',exact:false}).click();
    await flow.waitFor();await input.fill(teachingDraft);
    const document=await flow.getAttribute('data-lesson-document');
    await nav('Today');
    assert.equal(await page.locator('#today-primary').innerText(),'Continue this lesson');
    assert.equal(await page.locator('.teacher-resume').count(),1,'Resume is the hero, without a competing card');
    assert.equal(await page.getByRole('button',{name:'Continue this lesson',exact:true}).count(),1);
    assert.equal(await page.getByText('Browse your saved Teacher returns',{exact:true}).count(),0);
    assert.equal(await page.getByText('Browse your saved passage returns',{exact:true}).count(),0);
    await page.locator('#today-primary').click();await flow.waitFor();
    assert.equal(await flow.getAttribute('data-lesson-document'),document);
    assert.equal(await input.inputValue(),teachingDraft);
    checks.push('Today offers one accurate saved-lesson continuation; resumes exact document/draft; empty returns absent');
    await nav('Talk');await page.getByRole('button',{name:'New question',exact:true}).click();await page.locator('#dialogue-message').fill(talkDraft);
    for(const [name,width,height] of [['phone',390,844],['tablet',820,1180],['tablet-landscape',1024,768],['desktop',1440,1000]]){
      await page.setViewportSize({width,height});await assertNavigation(page,layouts);
      for(const [destination,parent] of [['Today','Today'],['Learn','Learn'],['Talk','Talk'],['Read','Read'],['Notebook','Notebook']]){
        await nav(destination);await active(parent);
        if(destination==='Today')assert.equal(await page.locator('#today-primary').innerText(),'Continue this lesson');
        await capture(name,destination.toLowerCase());
      }
      for(const [destination,parent] of [['Teacher','Learn'],['Practice','Learn'],['Studio','Learn'],['Progress','Notebook'],['Settings','Notebook']]){
        await nav(destination);await active(parent);await capture(name,destination.toLowerCase());
      }
      await nav('Talk');assert.equal(await page.locator('#dialogue-message').inputValue(),talkDraft);
      await nav('Learn');await nav('Teacher');
      assert.equal(await flow.getAttribute('data-lesson-document'),document);
      assert.equal(await input.inputValue(),teachingDraft);
      await active('Learn');
      checks.push(`${name}: all five primary and five secondary destinations, parent selection, keyboard focus, draft preservation, no overflow or bottom overlap`);
    }
    await nav('Notebook');
    await page.locator('#passage-title').fill('Synthetic navigation priority');
    await page.locator('#passage-russian').fill('Я читаю.');
    await page.locator('#passage-meaning').fill('I am reading.');
    await page.getByRole('button',{name:'Save passage',exact:true}).click();
    await page.getByRole('button',{name:'Write from memory',exact:true}).click();
    await page.locator('.practice-panel').waitFor();
    await nav('Today');
    assert.equal(await page.locator('#today-primary').innerText(),'Continue learning','An active ordinary session precedes an unfinished saved lesson');
    assert.equal(await page.locator('.teacher-resume').count(),0);
    await page.locator('#today-primary').click();await page.locator('.practice-panel').waitFor();await active('Learn');
    checks.push('Active passage session retains higher priority than unfinished teaching in Today hero');
    assert.deepEqual(errors,[]);assert.deepEqual(forbidden,[]);
    assert.equal(requests.filter(r=>r.path==='/api/teacher/teach').length,1);
    await fs.writeFile(path.join(OUTPUT,'result.json'),JSON.stringify({ok:true,checks,layouts,requests,errors,forbidden},null,2));
    console.log('PASS HIG navigation: phone/tablet/landscape/desktop routes, focus, drafts, touch targets and bottom clearance');
  }catch(error){
    await page.screenshot({path:path.join(OUTPUT,'failure.png'),fullPage:true}).catch(()=>{});
    await fs.writeFile(path.join(OUTPUT,'failure.txt'),await page.locator('body').innerText()).catch(()=>{});
    await fs.writeFile(path.join(OUTPUT,'failure-receipt.json'),JSON.stringify({error:String(error),checks,layouts,requests,errors,forbidden},null,2));
    throw error;
  }finally{await context.close();await browser.close();}
}
if(require.main===module)main().catch(error=>{console.error(error);process.exitCode=1;});
