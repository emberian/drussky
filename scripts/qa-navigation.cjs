// Shared actual-shell navigation for isolated browser acceptance. No injected CSS.
const assert = require('node:assert/strict');

async function navigate(page, name) {
  if (await page.locator('#tab-today').count()) {
    const primary = page.locator('.compact-navigation:visible, .primary-navigation:visible');
    const routes = {
      Today: ['Today'], Lessons: ['Learn'], Learn: ['Learn'],
      Teacher: ['Learn', 'teacher'], Talk: ['Talk'],
      Reader: ['Read'], Read: ['Read'], Library: ['Notebook'], Notebook: ['Notebook'],
      Practice: ['Learn', 'practice'], Studio: ['Learn', 'studio'],
      Progress: ['Notebook', 'progress'], Settings: ['Notebook', 'settings'],
    };
    const route = routes[name];
    assert(route, `Unknown destination: ${name}`);
    await primary.getByRole('button', { name: route[0], exact: true }).click();
    if (route[1]) await page.locator(`#context-${route[1]}:visible, #tools-${route[1]}:visible`).click();
    return;
  }
  const toggle = page.locator('#navigation-toggle');
  if (await toggle.isVisible() && await toggle.getAttribute('aria-expanded') !== 'true') await toggle.click();
  await page.getByRole('navigation', { name: 'Main navigation' })
    .getByRole('button', { name: new RegExp(name) }).click();
}

async function assertNavigation(page, audit) {
  if (await page.locator('#tab-today').count()) {
    const primary = page.locator('.compact-navigation:visible, .primary-navigation:visible');
    assert.deepEqual(await primary.getByRole('button').allTextContents(), ['Today', 'Learn', 'Talk', 'Read', 'Notebook']);
    assert.equal(await page.locator('#navigation-toggle').count(), 0, 'Primary sections stay visible');
    const talk = primary.getByRole('button', { name: 'Talk', exact: true });
    await talk.focus(); await page.keyboard.press('Enter');
    await page.waitForFunction(() => document.activeElement === document.querySelector('main.main h1'));
    assert.equal(await talk.getAttribute('aria-current'), 'page');
    const geometry = await page.evaluate(() => ({ width: innerWidth, height: innerHeight,
      heading: document.querySelector('main.main h1')?.getBoundingClientRect().toJSON(),
      overflow: document.documentElement.scrollWidth > innerWidth }));
    assert.equal(geometry.overflow, false);
    for (const button of await primary.getByRole('button').all()) {
      const bounds = await button.boundingBox();
      assert(bounds && bounds.width >= 44 && bounds.height >= 44, 'Navigation retains touch targets');
    }
    audit.push(geometry);
    return;
  }
  const toggle = page.locator('#navigation-toggle');
  const nav = page.locator('#main-navigation');
  const mobile = page.viewportSize().width <= 760;
  assert.equal(await toggle.isVisible(), mobile, 'Only the compact shell shows Menu');
  if (mobile) {
    assert.equal(await toggle.getAttribute('aria-controls'), 'main-navigation');
    assert.equal(await toggle.getAttribute('aria-expanded'), 'false');
    assert.equal(await nav.isVisible(), false, 'Collapsed destinations leave the reading viewport');
    await toggle.focus(); await page.keyboard.press('Enter');
    assert.equal(await toggle.getAttribute('aria-expanded'), 'true');
    await page.keyboard.press('Tab');
    assert(await nav.evaluate(node => node.contains(document.activeElement)), 'Menu destinations are keyboard reachable');
    await page.keyboard.press('Escape');
    assert.equal(await toggle.getAttribute('aria-expanded'), 'false');
    await page.waitForFunction(() => document.activeElement === document.getElementById('navigation-toggle'));
    await page.keyboard.press('Enter');
    await nav.getByRole('button', { name: /Teacher/ }).focus();
    await page.keyboard.press('Enter');
    await page.waitForFunction(() => document.activeElement === document.querySelector('main.main h1'));
    assert.equal(await toggle.getAttribute('aria-expanded'), 'false');
    assert.equal(await nav.isVisible(), false);
  } else {
    assert(await nav.isVisible(), 'Tablet/desktop retain the sidebar');
    const teacher = nav.getByRole('button', { name: /Teacher/ });
    await teacher.focus(); await page.keyboard.press('Enter');
    await page.waitForFunction(() => document.activeElement === document.querySelector('main.main h1'));
  }
  const geometry = await page.evaluate(() => ({ width: innerWidth, height: innerHeight,
    shellHeader: document.querySelector('.navigation-heading')?.getBoundingClientRect().toJSON(),
    heading: document.querySelector('main.main h1')?.getBoundingClientRect().toJSON(),
    overflow: document.documentElement.scrollWidth > innerWidth }));
  assert.equal(geometry.overflow, false);
  audit.push(geometry);
}

module.exports = { navigate, assertNavigation };
