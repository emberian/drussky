// Keep native PencilKit on the same paper as the lesson's HTML controls.
// Event-driven and deduplicated: no permanent animation-frame polling loop.
await new Promise(requestAnimationFrame);
const host = document.getElementById(hostId);
if (!host) throw Error('The writing paper could not open.');
window.drusskyNativePaperHosts ??= new Map();
window.drusskyNativePaperHosts.get(hostId)?.();
let frame = 0, disposed = false, previous = '';
let finish;
const closed = new Promise(resolve => { finish = resolve; });
const send = () => {
    frame = 0;
    if (disposed) return;
    const rect = host.getBoundingClientRect();
    const viewport = window.visualViewport;
    const value = {x:rect.x-(viewport?.offsetLeft||0),y:rect.y-(viewport?.offsetTop||0),width:host.isConnected?rect.width:0,
        height:host.isConnected?rect.height:0,
        viewport_width:viewport?.width||window.innerWidth,viewport_height:viewport?.height||window.innerHeight};
    const key = JSON.stringify(value);
    if (key !== previous) { previous = key; dioxus.send(value); }
};
const schedule = () => { if (!disposed && !frame) frame = requestAnimationFrame(send); };
const resize = new ResizeObserver(schedule);
resize.observe(host);
if (host.parentElement) resize.observe(host.parentElement);
const layoutChanges = new MutationObserver(schedule);
layoutChanges.observe(document.body,{childList:true,subtree:true,attributes:true,
    attributeFilter:['class','style','hidden','open']});
window.addEventListener('resize',schedule);
window.addEventListener('scroll',schedule,true);
window.visualViewport?.addEventListener('resize',schedule);
window.visualViewport?.addEventListener('scroll',schedule);
const dispose = () => {
    if (disposed) return;
    disposed = true;
    cancelAnimationFrame(frame);
    resize.disconnect();
    layoutChanges.disconnect();
    window.removeEventListener('resize',schedule);
    window.removeEventListener('scroll',schedule,true);
    window.visualViewport?.removeEventListener('resize',schedule);
    window.visualViewport?.removeEventListener('scroll',schedule);
    window.drusskyNativePaperHosts.delete(hostId);
    finish();
};
window.drusskyNativePaperHosts.set(hostId,dispose);
schedule();
await closed;
