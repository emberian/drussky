// Shared actual-shell navigation for isolated browser acceptance. No injected CSS.
const assert = require('node:assert/strict');

async function navigate(page, name) {
  const toggle = page.locator('#navigation-toggle');
  if (await toggle.isVisible() && await toggle.getAttribute('aria-expanded') !== 'true') await toggle.click();
  await page.getByRole('navigation', { name: 'Main navigation' })
    .getByRole('button', { name: new RegExp(name) }).click();
}

async function assertNavigation(page, audit) {
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
