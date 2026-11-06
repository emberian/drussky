#!/usr/bin/env node
// Run against serve-prod-preview.mjs, in an isolated browser context.
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import path from "node:path";
import { mkdir, writeFile } from "node:fs/promises";

const require = createRequire(import.meta.url);
let playwright;
try { playwright = require("playwright"); }
catch { playwright = require(process.env.RUFREN_PLAYWRIGHT || path.join(homedir(), "tools/playwright/node_modules/playwright")); }
const base = new URL(process.argv[2] || "http://127.0.0.1:8091/");
const artifactDir = path.resolve(process.env.RUFREN_QA_ARTIFACTS || ".local/platform-qa");
await mkdir(artifactDir, { recursive: true });
const browser = await playwright.chromium.launch({ headless: true });
const context = await browser.newContext({ viewport: { width: 1280, height: 900 }, serviceWorkers: "allow" });
const page = await context.newPage();
const errors = [];
page.on("pageerror", error => errors.push(error.message));
try {
  await page.goto(base.href);
  await page.getByRole("navigation", { name: "Main navigation" }).waitFor();
  assert.equal(await page.locator("html").getAttribute("lang"), "en", "The application document needs a default language");
  assert.equal(await page.getByRole("main").count(), 1, "The app should expose one main landmark");
  await page.waitForFunction(() => window.rufrenOffline?.state === "ready", null, { timeout: 60_000 });
  // First load can be uncontrolled until activation claims the page.
  await page.waitForFunction(() => navigator.serviceWorker.controller !== null);
  await page.keyboard.press("Tab");
  assert(await page.evaluate(() => ["A", "BUTTON", "INPUT", "TEXTAREA"].includes(document.activeElement.tagName)), "Tab must reach an actionable control");
  const nav = name => page.getByRole("navigation", { name: "Main navigation" }).getByRole("button", { name: new RegExp(name) });
  await nav("Library").click();
  await page.getByRole("button", { name: "New passage", exact: true }).click();
  await page.getByLabel("Title", { exact: true }).fill("Offline keyboard journey");
  const russian = "Здра́вствуй, мир. Ёжик здесь.";
  await page.getByLabel("Russian text", { exact: true }).focus();
  await page.keyboard.insertText(russian);
  assert.equal(await page.getByLabel("Russian text", { exact: true }).inputValue(), russian);
  await page.getByLabel("Meaning or translation", { exact: true }).fill("Hello, world. The hedgehog is here.");
  await page.locator("#passage-source").fill("Automated QA fixture; isolated browser only.");
  await page.getByRole("button", { name: "Save passage", exact: true }).click();
  await page.getByRole("button", { name: "Write from memory", exact: true }).click();
  await page.getByRole("button", { name: "I'm ready to try", exact: false }).click();
  const draft = "Мой ответ: ёжик и звёзды. е\u0301";
  await page.locator("#practice-response").focus();
  await page.keyboard.insertText(draft);
  await page.getByRole("button", { name: "Give me a hint", exact: true }).click();
  await context.setOffline(true);
  await page.reload({ waitUntil: "domcontentloaded" });
  await nav("Practice").click();
  await page.waitForFunction(expected => document.querySelector("#practice-response")?.value === expected, draft);
  assert.equal(await page.getByRole("button", { name: "Give me a hint", exact: true }).count(), 0, "Used hint must survive reload");
  // A fresh page must also start while the network is disabled, not merely keep
  // an already running WASM instance alive.
  const reopened = await context.newPage();
  await reopened.goto(base.href, { waitUntil: "domcontentloaded" });
  await reopened.getByRole("navigation", { name: "Main navigation" }).waitFor();
  await reopened.close();
  for (const url of [new URL("v1/events", base).href, new URL("pwa-register.js", base).href]) {
    const bypass = await page.evaluate(async url => {
      try { await fetch(url, { headers: { Authorization: "Bearer isolated-qa-test" } }); return false; }
      catch { return true; }
    }, url);
    assert(bypass, "Authenticated/API requests must fail offline instead of receiving cached responses");
  }
  const caches = await page.evaluate(async () => {
    const result = [];
    for (const name of await window.caches.keys()) {
      const cache = await window.caches.open(name);
      result.push({ name, urls: (await cache.keys()).map(request => request.url) });
    }
    return result;
  });
  assert(caches.length > 0);
  assert(caches.every(cache => cache.name.startsWith("rufren-shell:")));
  assert(caches.every(cache => cache.urls.every(url => !url.includes("/v1/") && !url.includes("?"))));
  await page.screenshot({ path: path.join(artifactDir, "offline-desktop.png"), fullPage: true });
  const labels = await page.evaluate(() => [...document.querySelectorAll("input,textarea,select")]
    .filter(element => element.getClientRects().length && element.type !== "hidden")
    .map(element => ({ id: element.id, named: !!(element.labels?.length || element.getAttribute("aria-label") || element.getAttribute("aria-labelledby")) })));
  assert(labels.every(control => control.named), `Visible form controls must have labels: ${JSON.stringify(labels)}`);
  assert.equal(await page.locator('nav [aria-current="page"]').count(), 1, "Navigation needs one announced current page");
  await writeFile(path.join(artifactDir, "practice-accessibility.txt"), await page.locator("body").ariaSnapshot());
  await page.setViewportSize({ width: 1024, height: 1366 });
  assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), "Tablet portrait layout must not overflow horizontally");
  await page.screenshot({ path: path.join(artifactDir, "offline-tablet.png"), fullPage: true });
  await page.setViewportSize({ width: 1366, height: 1024 });
  assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), "Tablet landscape layout must not overflow horizontally");
  await page.setViewportSize({ width: 390, height: 844 });
  assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), "Phone layout must not overflow horizontally");
  await page.screenshot({ path: path.join(artifactDir, "offline-phone.png"), fullPage: true });
  await page.getByRole("button", { name: "Compare my response", exact: false }).click();
  await page.getByRole("button", { name: "I recalled it", exact: true }).click();
  await page.getByRole("heading", { name: "A little more familiar.", exact: true }).waitFor();
  await page.reload({ waitUntil: "domcontentloaded" });
  await nav("Progress").click();
  assert.equal(await page.locator(".history-row").count(), 1, "Exactly one completed attempt should survive offline restart");
  assert.match(await page.locator(".history-row").innerText(), /With a hint/, "Self-rated recall must retain its assistance record");
  assert.deepEqual(errors, [], "No uncaught page errors during startup/resume");
  const result = { passed: true, checks: ["controlled shell install", "keyboard focus", "Cyrillic + combining accent", "offline reload", "fresh offline page", "draft + hint recovery", "completed attempt persistence", "assisted recall stays assisted", "authenticated/API bypass", "static cache boundary", "document language and main landmark", "form labels and current navigation", "tablet portrait/landscape overflow", "phone overflow"], caches: caches.map(cache => ({ name: cache.name, assets: cache.urls.length })) };
  await writeFile(path.join(artifactDir, "result.json"), JSON.stringify(result, null, 2) + "\n");
  console.log(JSON.stringify(result, null, 2));
} finally {
  await context.close();
  await browser.close();
}
