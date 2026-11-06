const element = document.getElementById(presentationId);
if (!element || !window.IntersectionObserver) return false;
window.rufrenPresentations ??= new Map();
window.rufrenPresentations.get(presentationId)?.();
return await new Promise(resolve => {
  let intersecting = false, done = false;
  const finish = shown => {
    if (done) return;
    done = true;
    observer.disconnect();
    removed.disconnect();
    document.removeEventListener('visibilitychange', visible);
    window.rufrenPresentations.delete(presentationId);
    resolve(shown);
  };
  const visible = () => {
    if (!element.isConnected) return finish(false);
    const style = getComputedStyle(element);
    if (intersecting && document.visibilityState === 'visible'
        && style.visibility === 'visible' && style.display !== 'none') finish(true);
  };
  const observer = new IntersectionObserver(entries => {
    intersecting = entries.some(entry => entry.target === element && entry.isIntersecting);
    visible();
  });
  const removed = new MutationObserver(() => {
    if (!element.isConnected) finish(false);
  });
  window.rufrenPresentations.set(presentationId, () => finish(false));
  document.addEventListener('visibilitychange', visible);
  removed.observe(document.documentElement, {childList:true, subtree:true});
  observer.observe(element);
});
