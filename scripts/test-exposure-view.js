// Real IntersectionObserver contract, isolated from user app storage.
async page => {
  const {readFile} = await import('node:fs/promises');
  const source = await readFile('crates/rufren-app/src/exposure_view.js','utf8');
  const context = await page.context().browser().newContext({viewport:{width:700,height:500}});
  const target = await context.newPage();
  try {
    await target.setContent('<p id="visible">Привет.</p><details id="detail"><summary>Open</summary><p id="concealed">Здравствуйте.</p></details><div style="height:1500px"></div><p id="below">Я читаю книгу.</p><p id="removed">До свидания.</p>');
    await target.evaluate(source => {
      const AsyncFunction = Object.getPrototypeOf(async function(){}).constructor;
      window.observeText = new AsyncFunction('presentationId',source);
      window.observations = {};
    }, source);
    const start = (id,key=id) => target.evaluate(({id,key})=>{
      window.observeText(id).then(value=>window.observations[key]=value);
    },{id,key});
    const settled = (key,value) => target.waitForFunction(({key,value})=>window.observations[key]===value,{key,value});
    const pending = async key => {
      await target.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
      if(await target.evaluate(key=>key in window.observations,key))throw Error(key+' was observed before presentation');
    };
    await start('visible');await settled('visible',true);
    await start('concealed');await pending('concealed');
    await target.locator('#detail').evaluate(e=>e.open=true);await settled('concealed',true);
    await start('below','old');await pending('old');
    await start('below','new');await settled('old',false);await pending('new');
    await target.locator('#below').scrollIntoViewIfNeeded();await settled('new',true);
    await target.evaluate(()=>scrollTo(0,0));
    await start('removed');await pending('removed');
    await target.locator('#removed').evaluate(e=>e.remove());await settled('removed',false);
    await target.evaluate(()=>Object.defineProperty(document,'visibilityState',{value:'hidden',configurable:true}));
    await start('visible','background');await pending('background');
    await target.evaluate(()=>{Object.defineProperty(document,'visibilityState',{value:'visible',configurable:true});document.dispatchEvent(new Event('visibilitychange'));});
    await settled('background',true);
    // Rich prose observes child-bearing paragraph wrappers independently. Seeing
    // the first block must never acknowledge a later paragraph in the answer.
    await target.setContent('<article><div id="rich-first"><p><strong>Приве́т</strong> <em>мир</em>.</p></div><div style="height:1500px"></div><div id="rich-later"><p><a href="https://example.org">Другой абзац</a>.</p></div></article>');
    await start('rich-first');await settled('rich-first',true);
    await start('rich-later');await pending('rich-later');
    await target.locator('#rich-later').scrollIntoViewIfNeeded();await settled('rich-later',true);
    await start('missing');await settled('missing',false);
    if(await target.evaluate(()=>window.rufrenPresentations.size)!==0)throw Error('Observers remained after completion or removal');
    return {passed:['visible text acknowledged','collapsed answers withheld until opened','offscreen text withheld','replacement cancels old observer','scroll presentation acknowledged','removed text is not exposure','background document withheld','rich child wrappers observed independently','all observers released']};
  } finally {await context.close();}
}
