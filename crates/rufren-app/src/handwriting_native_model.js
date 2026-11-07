// Authored hand-print paths shared with the portable paper; no independent glyphs.
const root = document.getElementById(modelId);
if (!root) throw Error('The writing example did not mount.');
window.rufrenNativeModels ??= new Map();
window.rufrenNativeModels.get(modelId)?.();
const controller = new AbortController();
let animations = [];
const cancel = () => { animations.forEach(animation => animation.cancel()); animations = []; };
const ns = 'http://www.w3.org/2000/svg';
const svg = document.createElementNS(ns, 'svg');
svg.setAttribute('viewBox', `0 0 ${config.paper.width} ${config.paper.height}`);
svg.setAttribute('role', 'img');
svg.setAttribute('aria-label', 'One way to hand-print this target');
svg.setAttribute('preserveAspectRatio', 'xMidYMid meet');
for (const [index, y] of window.RufrenHandwriting.guides(config.target, config.paper, config.model_version).entries()) {
    const line = document.createElementNS(ns, 'line');
    for (const [key, value] of Object.entries({x1: 20, x2: config.paper.width - 20, y1: y, y2: y, stroke: index === 2 ? '#93a591' : '#c2cbc0', 'stroke-width': index === 2 ? 2 : 1})) line.setAttribute(key, String(value));
    if (index !== 2) line.setAttribute('stroke-dasharray', '8 7');
    svg.append(line);
}
const paths = window.RufrenHandwriting.model(config.target, config.paper, config.model_version).map(stroke => {
    const path = document.createElementNS(ns, 'path');
    for (const [key, value] of Object.entries({d: stroke.d, transform: stroke.transform, fill: 'none', stroke: 'currentColor', 'stroke-width': 4, 'stroke-linecap': 'round', 'stroke-linejoin': 'round'})) path.setAttribute(key, value);
    svg.append(path);
    return path;
});
root.querySelector('[data-native-model-picture]').replaceChildren(svg);
root.querySelector('[data-native-model-replay]').addEventListener('click', () => {
    cancel();
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    // Every replay finishes within twelve seconds, including long messages.
    const step = Math.min(550, 12000 / Math.max(1, paths.length));
    paths.forEach((path, index) => {
        const length = path.getTotalLength();
        animations.push(path.animate([{strokeDasharray: String(length), strokeDashoffset: length}, {strokeDasharray: String(length), strokeDashoffset: 0}], {duration: step * .9, delay: index * step, fill: 'backwards'}));
    });
}, {signal: controller.signal});
const dispose = () => { cancel(); controller.abort(); root.querySelector('[data-native-model-picture]')?.replaceChildren(); window.rufrenNativeModels.delete(modelId); };
window.rufrenNativeModels.set(modelId, dispose);
