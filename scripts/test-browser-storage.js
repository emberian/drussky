// Playwright tool function: run against a disposable browser context, never the active notebook.
async (page) => {
  const { readFile } = await import('node:fs/promises');
  const source = await readFile('crates/rufren-app/src/browser_store.js', 'utf8');
  const context = await page.context().browser().newContext();
  await context.route('https://rufren-storage.test/**', route => route.fulfill({contentType:'text/html', body:'<!doctype html><title>Rufren storage test</title>'}));
  const first = await context.newPage();
  const initialize = target => target.evaluate(async source => await new (Object.getPrototypeOf(async function(){}).constructor)(source)(), source);
  const waitForFlush = target => target.waitForFunction(() => !Object.keys(localStorage).some(key => key.startsWith('rufren.')));
  try {
    await first.goto('https://rufren-storage.test/');
    await first.evaluate(() => {
      localStorage.setItem('rufren.v1.event.legacy', '{"id":"legacy","version":1}');
      localStorage.setItem('rufren.v1.meta.session', '{"draft":"Привет"}');
    });
    await initialize(first);
    await waitForFlush(first);
    let result = await first.evaluate(() => window.rufrenLibrary.get('rufren.v1.meta.session'));
    if (JSON.parse(result).draft !== 'Привет') throw Error('Migration lost Cyrillic session draft');
    await first.reload();
    await initialize(first);
    result = await first.evaluate(() => window.rufrenLibrary.get('rufren.v1.event.legacy'));
    if (JSON.parse(result).version !== 1) throw Error('IndexedDB reopen lost legacy event');

    // Force the first durable transaction to abort, simulating interruption after journaling.
    await first.evaluate(() => {
      const original = IDBDatabase.prototype.transaction;
      let interrupt = true;
      IDBDatabase.prototype.transaction = function(...args) {
        const tx = original.apply(this, args);
        if (args[1] === 'readwrite' && interrupt) { interrupt = false; queueMicrotask(() => tx.abort()); }
        return tx;
      };
      window.rufrenLibrary.write('rufren.v1.event.interrupted', '{"id":"interrupted"}');
    });
    await first.waitForTimeout(50);
    if (!await first.evaluate(() => localStorage.getItem('rufren.journal.v1.rufren.v1.event.interrupted'))) throw Error('Aborted transaction lost recovery journal');
    await first.reload();
    await initialize(first);
    await waitForFlush(first);
    if (!await first.evaluate(() => window.rufrenLibrary.get('rufren.v1.event.interrupted'))) throw Error('Journal replay failed after reopen');

    const second = await context.newPage();
    await second.goto('https://rufren-storage.test/');
    await initialize(second);
    await Promise.all([
      first.evaluate(() => window.rufrenLibrary.write('rufren.v1.event.tab-a', '{"id":"tab-a"}')),
      second.evaluate(() => window.rufrenLibrary.write('rufren.v1.event.tab-b', '{"id":"tab-b"}')),
    ]);
    await waitForFlush(first);
    await first.waitForFunction(() => window.rufrenLibrary.get('rufren.v1.event.tab-b'));
    await second.waitForFunction(() => window.rufrenLibrary.get('rufren.v1.event.tab-a'));
    const delta = await first.evaluate(() => {
      const full = window.rufrenLibrary.eventsAfter(0);
      window.rufrenLibrary.write('rufren.v1.meta.test', '"metadata"');
      return {count:full.entries.length, next:window.rufrenLibrary.eventsAfter(full.revision)};
    });
    if (delta.count !== 4 || delta.next.entries.length !== 0) throw Error('Incremental event cache reparsed unchanged history');
    await second.evaluate(() => window.rufrenLibrary.write('rufren.v1.meta.session', null));
    await waitForFlush(second);
    await second.reload();
    await initialize(second);
    if (await second.evaluate(() => window.rufrenLibrary.get('rufren.v1.meta.session')) !== null) throw Error('Deleted metadata resurrected');
    return {passed:['legacy migration', 'Cyrillic metadata', 'IndexedDB reopen', 'aborted transaction retains journal', 'replay after interruption', 'concurrent tab events', 'metadata deletion', 'incremental event reads']};
  } finally { await context.close(); }
}
