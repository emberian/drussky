// `source` is the exact bundled original WAV data URL, bound by the Rust caller.
window.rufrenFoundationAudio?.cancel();
window.rufrenAudioStudio?.stopPlayback?.();
for (const api of window.rufrenAudioPlayers?.values() || []) api.stop?.();
const player = new Audio(source);
const key = 'rufren-foundation';
const registry = window.rufrenAudioPlayers ||= new Map();
let cancel;
let watchdog;
const finished = new Promise(resolve => {
    let settled = false;
    const finish = ok => {
        if (settled) return;
        settled = true;
        clearTimeout(watchdog);
        window.removeEventListener('pagehide', leave);
        document.removeEventListener('visibilitychange', visibility);
        player.onended = player.onerror = player.onabort = player.onpause = null;
        player.pause();
        player.removeAttribute('src');
        player.load();
        if (window.rufrenFoundationAudio?.player === player) window.rufrenFoundationAudio = null;
        if (registry.get(key)?.player === player) registry.delete(key);
        resolve(ok);
    };
    cancel = () => finish(false);
    const leave = () => finish(false);
    const visibility = () => { if (document.hidden) finish(false); };
    window.addEventListener('pagehide', leave);
    document.addEventListener('visibilitychange', visibility);
    player.onended = () => finish(true);
    player.onerror = player.onabort = () => finish(false);
    // HTML media queues a normal pause before ended. Only ended acknowledges;
    // the preceding natural-end pause must leave that event handler alive.
    player.onpause = () => { if (!player.ended) finish(false); };
    // Covers stalled loading, platform interruptions and a play promise that never settles.
    watchdog = setTimeout(() => finish(false), 15000);
});
const api = {player, cancel, stop: () => cancel()};
window.rufrenFoundationAudio = api;
registry.set(key, api);
try { Promise.resolve(player.play()).catch(cancel); } catch (_) { cancel(); }
return await finished;
