const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const source = fs.readFileSync(require('node:path').join(__dirname, '../crates/rufren-app/src/handwriting_native_host.js'), 'utf8');

function target(extra = {}) {
    const listeners = new Map();
    return Object.assign(extra, {
        listeners,
        addEventListener(kind, fn, capture = false) {
            const key = `${kind}:${capture}`;
            if (!listeners.has(key)) listeners.set(key, new Set());
            listeners.get(key).add(fn);
        },
        removeEventListener(kind, fn, capture = false) {
            const key = `${kind}:${capture}`;
            listeners.get(key)?.delete(fn);
            if (!listeners.get(key)?.size) listeners.delete(key);
        },
        dispatch(kind) {
            for (const [key, callbacks] of listeners) {
                if (key.startsWith(`${kind}:`)) for (const fn of callbacks) fn();
            }
        },
    });
}
function fixture(visual = true) {
    let nextFrame = 1;
    const frames = new Map(), messages = [], observers = [];
    const host = {isConnected: true, parentElement: {}, rect: {x: 50, y: 120, width: 700, height: 300}, getBoundingClientRect() {return this.rect;}};
    const viewport = target({offsetLeft: 0, offsetTop: 0, width: 1024, height: 768});
    const window = target({innerWidth: 1024, innerHeight: 768, visualViewport: visual ? viewport : undefined});
    class Observer {
        constructor(callback) {this.callback = callback; this.targets = []; this.disconnected = false; observers.push(this);}
        observe(value, options) {this.targets.push({value, options});}
        disconnect() {this.disconnected = true; this.targets = [];}
        fire() {if (!this.disconnected) this.callback();}
    }
    const context = vm.createContext({window, hostId: 'paper', document: {body: {}, getElementById: () => host},
        ResizeObserver: Observer, MutationObserver: Observer,
        requestAnimationFrame(fn) {const id = nextFrame++; frames.set(id, fn); return id;},
        cancelAnimationFrame(id) {frames.delete(id);}, dioxus: {send(value) {messages.push(JSON.parse(JSON.stringify(value)));}},
    });
    const done = vm.runInContext(`(async () => {${source}\n})()`, context);
    let resolved = false;
    done.then(() => {resolved = true;});
    async function frame() {
        const pending = [...frames.values()]; frames.clear();
        for (const fn of pending) fn();
        await Promise.resolve(); await Promise.resolve();
    }
    async function start() {await frame(); await frame();}
    async function dispose() {window.drusskyNativePaperHosts.get('paper')?.(); await done;}
    return {host, window, viewport, frames, messages, observers, done, frame, start, dispose, resolved: () => resolved};
}

test('initial geometry, event batching and deduplication have no recurring animation loop', async () => {
    const f = fixture(); await f.start();
    assert.deepEqual(f.messages, [{x: 50, y: 120, width: 700, height: 300, viewport_width: 1024, viewport_height: 768}]);
    assert.equal(f.frames.size, 0);
    f.window.dispatch('scroll'); f.window.dispatch('resize'); f.observers[0].fire();
    assert.equal(f.frames.size, 1, 'independent events coalesce into one frame');
    await f.frame(); assert.equal(f.messages.length, 1, 'unchanged geometry is not resent');
    assert.equal(f.frames.size, 0);
    f.host.rect.y = 40; f.window.dispatch('scroll'); await f.frame();
    assert.equal(f.messages.at(-1).y, 40);
    f.host.rect.width = 600; f.observers[1].fire(); await f.frame();
    assert.equal(f.messages.at(-1).width, 600, 'DOM layout mutation updates geometry');
    await f.dispose();
});

test('visual viewport offsets and zoom are sampled with keyboard viewport height', async () => {
    const f = fixture(); await f.start();
    Object.assign(f.viewport, {offsetLeft: 30, offsetTop: 80, width: 512, height: 240});
    f.viewport.dispatch('scroll'); f.viewport.dispatch('resize'); await f.frame();
    assert.deepEqual(f.messages.at(-1), {x: 20, y: 40, width: 700, height: 300, viewport_width: 512, viewport_height: 240});
    assert.equal(f.frames.size, 0);
    await f.dispose();
    const fallback = fixture(false); await fallback.start();
    assert.equal(fallback.messages[0].viewport_width, fallback.window.innerWidth);
    assert.equal(fallback.messages[0].viewport_height, fallback.window.innerHeight);
    await fallback.dispose();
});

test('disconnected host becomes hidden and dispose releases every resource and bridge promise', async () => {
    const f = fixture(); await f.start();
    assert.equal(f.observers[0].targets.length, 2, 'observe host and parent size');
    assert.equal(f.observers[1].targets.length, 1, 'observe document layout changes');
    assert.equal(f.resolved(), false, 'bridge remains alive until disposal');
    f.host.isConnected = false; f.observers[1].fire(); await f.frame();
    assert.equal(f.messages.at(-1).width, 0); assert.equal(f.messages.at(-1).height, 0);
    f.window.dispatch('scroll'); assert.equal(f.frames.size, 1);
    const dispose = f.window.drusskyNativePaperHosts.get('paper');
    await f.dispose(); dispose();
    assert.equal(f.resolved(), true);
    assert.equal(f.frames.size, 0);
    assert.equal(f.window.listeners.size, 0);
    assert.equal(f.viewport.listeners.size, 0);
    assert.equal(f.window.drusskyNativePaperHosts.size, 0);
    assert.ok(f.observers.every(observer => observer.disconnected));
    const count = f.messages.length;
    f.window.dispatch('resize'); f.viewport.dispatch('scroll'); f.observers.forEach(observer => observer.fire());
    await f.frame(); assert.equal(f.messages.length, count); assert.equal(f.frames.size, 0);
});
