// Playback lifecycle checks; these do not certify media decoding or Russian sounds.
const fs = require('node:fs'), assert = require('node:assert/strict');
const body = fs.readFileSync('crates/rufren-app/src/foundation_audio.js', 'utf8');
const AsyncFunction = Object.getPrototypeOf(async function() {}).constructor;
const instances = [], listeners = new Map();
let rejectPlay = false, studioStops = 0, otherStops = 0;
class Audio {
  constructor() { instances.push(this); }
  play() { return rejectPlay ? Promise.reject(Error('blocked')) : Promise.resolve(); }
  ended = false;
  pause() {}
  removeAttribute() {}
  load() {}
}
global.Audio = Audio;
global.window = {
  rufrenAudioStudio: {stopPlayback() { studioStops++; }},
  rufrenAudioPlayers: new Map([['other', {stop() { otherStops++; }}]]),
  addEventListener: (key, fn) => listeners.set(key, fn),
  removeEventListener: key => listeners.delete(key),
};
global.document = {
  hidden: false,
  addEventListener: (key, fn) => listeners.set(key, fn),
  removeEventListener: key => listeners.delete(key),
};
const run = () => new AsyncFunction('source', body)('fixture');
(async () => {
  let task = run();
  instances.at(-1).ended = true;
  instances.at(-1).onpause(); // Browser normal end queues pause first.
  let acknowledged = false; task.then(() => acknowledged = true);
  await Promise.resolve(); assert.equal(acknowledged, false);
  instances.at(-1).onended();
  assert.equal(await task, true);
  assert.equal(window.rufrenFoundationAudio, null);
  assert.equal(window.rufrenAudioPlayers.has('rufren-foundation'), false);
  assert.equal(listeners.size, 0);
  task = run(); window.rufrenFoundationAudio.cancel(); assert.equal(await task, false);
  task = run(); instances.at(-1).onpause(); assert.equal(await task, false);
  task = run(); const lateEnded = instances.at(-1).onended;
  const replacement = run(); assert.equal(await task, false);
  lateEnded(); // Replaced source must never gain a successful acknowledgement.
  instances.at(-1).onended(); assert.equal(await replacement, true);
  task = run(); document.hidden = true; listeners.get('visibilitychange')(); assert.equal(await task, false);
  document.hidden = false;
  task = run(); listeners.get('pagehide')(); assert.equal(await task, false);
  rejectPlay = true; assert.equal(await run(), false);
  assert.equal(window.rufrenFoundationAudio, null);
  assert.equal(listeners.size, 0);
  assert(studioStops >= 7 && otherStops >= 7);
  console.log('Foundation playback: ended-only acknowledgement, stop, pause, replacement, hidden/leave, rejection, shared-player cleanup passed.');
})().catch(error => { console.error(error); process.exitCode = 1; });
