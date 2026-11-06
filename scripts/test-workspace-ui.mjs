#!/usr/bin/env node
// Actual Dioxus UI composition against a production preview; isolated browser data only.
import {createRequire} from 'node:module';
import {mkdir} from 'node:fs/promises';
const require=createRequire(import.meta.url),{chromium}=require(process.env.RUFREN_PLAYWRIGHT_MODULE||'playwright');
const browser=await chromium.launch({headless:true,...(process.env.RUFREN_CHROMIUM_EXECUTABLE?{executablePath:process.env.RUFREN_CHROMIUM_EXECUTABLE}:{})});
const context=await browser.newContext({viewport:{width:1280,height:950}}),unexpected=[];
context.on('request',request=>{if(request.method()==='POST'&&request.url().includes('/api/'))unexpected.push(request.url());});
const base=process.env.RUFREN_PREVIEW_URL||'http://127.0.0.1:8091';
const navigate=async(page,name)=>page.locator('nav').getByRole('button',{name:new RegExp(name)}).click();
const ready=async page=>page.locator('nav').waitFor({timeout:60000});
async function draftAndPractice(page,letter){
 await navigate(page,'Library');await page.getByRole('button',{name:'New passage',exact:true}).click();
 await page.locator('#passage-title').fill('Workspace practice '+letter);await page.locator('#passage-russian').fill(letter==='A'?'Это окно А.':'Это окно Б.');await page.locator('#passage-meaning').fill('This is window '+letter+'.');
 await page.getByRole('button',{name:'Save passage',exact:true}).click();await page.getByRole('button',{name:'Read for meaning',exact:true}).click();await page.getByRole('button',{name:"I'm ready to try →",exact:true}).click();
 await page.locator('#practice-response').fill('Independent response from '+letter);
 await navigate(page,'Library');await page.getByRole('button',{name:'New passage',exact:true}).click();await page.locator('#passage-title').fill('Unpublished window '+letter);await page.locator('#passage-russian').fill('Черновик '+letter+'.');
}
async function assertDraft(page,letter){
 await navigate(page,'Library');if(await page.locator('#passage-title').inputValue()!=='Unpublished window '+letter)throw Error('Window '+letter+' passage draft was replaced');
 if(await page.locator('#passage-russian').inputValue()!=='Черновик '+letter+'.')throw Error('Window '+letter+' Cyrillic draft was replaced');
}
async function assertPractice(page,letter){
 await navigate(page,'Practice');if(await page.locator('#practice-response').inputValue()!=='Independent response from '+letter)throw Error('Window '+letter+' response was replaced');
}
async function recover(page,letter){
 await navigate(page,'Settings');await page.getByRole('heading',{name:'Your saved windows',exact:true}).waitFor();await page.getByRole('button',{name:'Refresh saved windows',exact:true}).click();
 const panel=page.locator('section').filter({has:page.getByRole('heading',{name:'Your saved windows',exact:true})});
 const row=panel.locator('.history-row').filter({has:page.getByText('Unpublished window '+letter,{exact:true})});
 const button=row.getByRole('button',{name:'Reopen here',exact:true});await button.waitFor({timeout:15000});
 await Promise.all([page.waitForNavigation({waitUntil:'domcontentloaded'}),button.click()]);await ready(page);
}
try{
 const a=await context.newPage();await a.goto(base);await ready(a);await draftAndPractice(a,'A');
 const b=await context.newPage();await b.goto(base);await ready(b);await draftAndPractice(b,'B');
 await assertDraft(a,'A');await assertDraft(b,'B');await b.reload();await ready(b);await assertDraft(b,'B');await assertPractice(b,'B');await b.close();
 await navigate(a,'Settings');await a.getByRole('heading',{name:'Your saved windows',exact:true}).waitFor();await a.getByRole('button',{name:'Refresh saved windows',exact:true}).click();
 await mkdir('.local/ipad-delivery',{recursive:true});await a.screenshot({path:'.local/ipad-delivery/workspace-settings-desktop.png',fullPage:true});
 await a.setViewportSize({width:390,height:844});await a.screenshot({path:'.local/ipad-delivery/workspace-settings-phone.png',fullPage:true});
 if(await a.evaluate(()=>document.documentElement.scrollWidth>innerWidth))throw Error('SavedWindows Settings overflows phone width');
 await recover(a,'B');await assertDraft(a,'B');await assertPractice(a,'B');await recover(a,'A');await assertDraft(a,'A');await assertPractice(a,'A');
 if(unexpected.length)throw Error('Workspace QA unexpectedly sent a teacher/provider request');
 console.log(JSON.stringify({passed:['actual Dioxus independent tab edits','Cyrillic passage drafts retained','independent practice responses','tab reload recovery','Settings closed-window recovery','previous window remains recoverable','390px layout without overflow','no model requests']}));
}finally{await context.close();await browser.close();}
