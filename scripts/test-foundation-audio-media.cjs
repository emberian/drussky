// Actual HTML media playback of every bundled WAV, with networking disabled.
const {chromium} = require(process.env.RUFREN_PLAYWRIGHT_MODULE || '/Users/ember/tools/playwright/node_modules/playwright');
const fs = require('node:fs/promises'), assert = require('node:assert/strict');
(async () => {
 const browser = await chromium.launch({headless:true,args:['--autoplay-policy=no-user-gesture-required']});
 try {
  const context = await browser.newContext({offline:true}); const page = await context.newPage();
  await page.setContent('<!doctype html><title>Offline foundation audio</title>');
  const bridge = await fs.readFile('crates/rufren-app/src/foundation_audio.js','utf8');
  const manifest = JSON.parse(await fs.readFile('crates/rufren-app/assets/foundations/manifest.json','utf8'));const results=[];
  for(const asset of manifest.assets) {
   const source='data:audio/wav;base64,'+(await fs.readFile('crates/rufren-app/assets/foundations/'+asset.file)).toString('base64');
   const result=await page.evaluate(async ({bridge,source})=>{
    const NativeAudio=window.Audio,events=[];
    window.Audio=function(src){const audio=new NativeAudio(src);for(const event of ['playing','pause','ended','error','abort'])audio.addEventListener(event,()=>events.push({event,ended:audio.ended,currentTime:audio.currentTime,error:audio.error?.code}));return audio;};
    try {const run=new(Object.getPrototypeOf(async function(){}).constructor)('source',bridge);return {ok:await run(source),events};}finally{window.Audio=NativeAudio;}
   },{bridge,source});
   results.push({id:asset.id,...result}); console.log(JSON.stringify(results.at(-1)));
   if(process.env.RUFREN_EXPECT_BASELINE_FAILURE!=='1') {assert.equal(result.ok,true,asset.id);assert(result.events.some(event=>event.event==='ended'),asset.id);}
  }
  const interruptionSource='data:audio/wav;base64,'+(await fs.readFile('crates/rufren-app/assets/foundations/foundation-a.wav')).toString('base64');
  const interrupted=await page.evaluate(async ({bridge,source})=>{
   const run=new(Object.getPrototypeOf(async function(){}).constructor)('source',bridge);
   const task=run(source); const player=window.rufrenFoundationAudio.player;
   player.addEventListener('playing',()=>setTimeout(()=>player.pause(),30),{once:true});
   return await task;
  },{bridge,source:interruptionSource});
  assert.equal(interrupted,false,'real interrupted pause must reject acknowledgement');
  console.log('Actual interrupted playback returns false.');
  if(process.env.RUFREN_EXPECT_BASELINE_FAILURE==='1')assert(results.some(result=>!result.ok),'baseline should demonstrate failure');
  if(process.env.RUFREN_QA_OUTPUT) {await fs.mkdir(process.env.RUFREN_QA_OUTPUT,{recursive:true});await fs.writeFile(process.env.RUFREN_QA_OUTPUT+'/media-results.json',JSON.stringify({offline:true,actual_media:true,interrupted_pause_acknowledged:interrupted,results},null,2));}
  await context.close();
 } finally {await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
