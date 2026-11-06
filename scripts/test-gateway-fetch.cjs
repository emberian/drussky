// Exercise the browser transport's actual streaming source without model calls.
const fs = require('node:fs');
const assert = require('node:assert/strict');
const AsyncFunction = Object.getPrototypeOf(async function() {}).constructor;
const source = fs.readFileSync('crates/rufren-app/src/gateway_fetch.js', 'utf8');
const run = new AsyncFunction('a', 'fetch', source);
const args = {url:'https://relay.test/api/speech',token:'synthetic',body:null,limit:64,error_limit:32,timeout:1000};
const enc = new TextEncoder();
async function response(status, parts, {fail = false, open = false} = {}) {
  let cancelled = false;
  const stream = new ReadableStream({
    start(controller) {
      for (const part of parts) controller.enqueue(enc.encode(part));
      if (fail) controller.error(new Error('private upstream diagnostic'));
      else if (!open) controller.close();
    },
    cancel() { cancelled = true; }
  });
  const result = await run(args, async (_url, options) => {
    assert.equal(options.redirect,'error');
    assert.equal(options.credentials,'omit');
    return new Response(stream,{status});
  });
  return {result,cancelled};
}
(async () => {
  assert.deepEqual((await response(400,['Ambiguous ','stress'])).result,{status:400,detail:'Ambiguous stress'});
  const large = await response(400,['x'.repeat(20),'y'.repeat(20)],{open:true});
  assert.deepEqual(large.result,{status:400});assert.equal(large.cancelled,true);
  assert.deepEqual((await response(415,['x'],{fail:true})).result,{status:415});
  assert.deepEqual((await response(200,['{"ok":','true}'])).result,{body:'{"ok":true}'});
  assert.deepEqual((await response(200,['x'.repeat(65)])).result,{error:'large'});
  assert.deepEqual(await run(args,async()=>{throw new Error('private diagnostic');}),{error:'connection'});
  assert.deepEqual(await run(args,async()=>{throw new DOMException('private timeout','AbortError');}),{error:'timeout'});
  console.log('PASS: 7 gateway streaming/error-boundary cases');
})().catch(error=>{console.error(error);process.exitCode=1;});
