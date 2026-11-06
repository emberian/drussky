#!/usr/bin/env node
// Real Dioxus + Chromium service-worker acceptance. No provider or relay calls.
// Start with an immutable prepared baseline. Publish {candidate: "/built/public"}
// to the control JSON after the next genuine Dioxus build is ready.
import assert from "node:assert/strict";
import {addSyntheticSpeechFacts as addSyntheticFacts} from "./fixtures/synthetic-speech-facts.mjs";
import { createHash, randomUUID } from "node:crypto";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import http from "node:http";
import { cp, mkdir, readFile, realpath, writeFile } from "node:fs/promises";

const require = createRequire(import.meta.url);
let playwright;
try { playwright = require("playwright"); }
catch { playwright = require(process.env.RUFREN_PLAYWRIGHT || path.join(homedir(), "tools/playwright/node_modules/playwright")); }
const [baselineArg, runArg, controlArg] = process.argv.slice(2);
if (!controlArg) throw Error("Usage: qa-shell-update.mjs <immutable baseline/public> <new run directory> <control.json>");
const baseline = await realpath(baselineArg), runDir = path.resolve(runArg), control = path.resolve(controlArg);
await mkdir(runDir); // Never overwrite a previous run or its browser profile.
const oldBuild = JSON.parse(await readFile(path.join(baseline, "offline-build.json")));
const hash = bytes => createHash("sha256").update(bytes).digest("hex");
const mime = { ".html": "text/html; charset=utf-8", ".js": "text/javascript", ".wasm": "application/wasm", ".css": "text/css", ".png": "image/png", ".svg": "image/svg+xml", ".webmanifest": "application/manifest+json", ".json": "application/json" };
let serving = baseline, failAsset = null, context, server, origin, currentStage = "starting";
const requests = [], lifecycle = [], versions = new Map(), pageErrors = [], forbidden = [];
const state = async (stage, details = {}) => {
  currentStage = stage;
  await writeFile(path.join(runDir, "status.json"), JSON.stringify({ stage, at: new Date().toISOString(), origin, baseline: oldBuild.version, ...details }, null, 2) + "\n");
  console.log(stage);
};
const waitFor = async (predicate, label, timeout = 30_000) => {
  const end = Date.now() + timeout;
  while (Date.now() < end) { if (await predicate()) return; await new Promise(resolve => setTimeout(resolve, 50)); }
  throw Error(`Timed out: ${label}`);
};
const nav = (page, name) => page.getByRole("navigation", { name: "Main navigation" }).getByRole("button", { name: new RegExp(name, "i") });
async function openApp(page, url) {
  page.on("pageerror", error => pageErrors.push(error.message));
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.getByRole("navigation", { name: "Main navigation" }).waitFor();
  await page.waitForFunction(() => navigator.serviceWorker.controller !== null, null, { timeout: 60_000 });
}
async function snapshot(page) {
  return page.evaluate(async () => {
    await window.rufrenLibrary.flush();
    const entries = window.rufrenLibrary.entries();
    const session = JSON.parse(window.rufrenLibrary.get("rufren.v1.meta.session"));
    return {
      workspace: sessionStorage.getItem("rufren.workspace.v1"),
      session: Object.fromEntries(["id", "revision", "skill", "phase", "draft", "cues"].map(key => [key, session[key]])),
      teacher: window.rufrenLibrary.get("rufren.v1.meta.teacher_response"),
      events: Object.fromEntries(entries.filter(([key]) => key.startsWith("rufren.v1.event."))),
    };
  });
}
async function verifyWork(page, expected) {
  await nav(page, "Practice").click();
  await page.waitForFunction(text => document.querySelector("#practice-response")?.value === text, expected.session.draft);
  const actual = await snapshot(page);
  assert.equal(actual.workspace, expected.workspace, "The latest saved window must recover after all old tabs close");
  assert.deepEqual(actual.session, expected.session, "Exact Cyrillic draft, support and attempt identity must survive");
  assert.equal(actual.teacher, expected.teacher, "The explicitly synthetic saved teacher response must survive");
  for (const [key, value] of Object.entries(expected.events)) assert.equal(actual.events[key], value, `Immutable fact changed/lost: ${key}`);
  assert.equal(await page.getByRole("button", { name: "Give me a hint", exact: true }).count(), 0);
}
async function wasmDigest(page, relative) {
  return page.evaluate(async relative => {
    const bytes = await (await fetch(new URL(relative, location.href))).arrayBuffer();
    return [...new Uint8Array(await crypto.subtle.digest("SHA-256", bytes))].map(n => n.toString(16).padStart(2, "0")).join("");
  }, relative);
}
async function cacheNames(page) { return page.evaluate(() => caches.keys()); }
async function seed(page) {
  await nav(page, "Library").click();
  await page.getByRole("button", { name: "New passage", exact: true }).click();
  await page.getByLabel("Title", { exact: true }).fill("Shell update story — synthetic QA");
  await page.getByLabel("Russian text", { exact: true }).fill("Ёжик смо́трит на звёзды.");
  await page.getByLabel("Meaning or translation", { exact: true }).fill("The hedgehog looks at the stars.");
  await page.locator("#passage-source").fill("Synthetic browser update QA; not a personal notebook.");
  await page.getByRole("button", { name: "Save passage", exact: true }).click();
  await page.getByRole("button", { name: "Write from memory", exact: true }).click();
  await page.getByRole("button", { name: "I'm ready to try", exact: false }).click();
  await page.locator("#practice-response").fill("Ёжик смотрит на звёзды.");
  await page.getByRole("button", { name: "Give me a hint", exact: true }).click();
  await page.getByRole("button", { name: "Compare my response", exact: false }).click();
  await page.getByRole("button", { name: "I recalled it", exact: true }).click();
  await page.getByRole("heading", { name: "A little more familiar.", exact: true }).waitFor();
  await nav(page, "Settings").click();
  await page.getByRole("button", { name: "Prepare export", exact: true }).click();
  const backup = JSON.parse(await page.locator("#backup-json").inputValue());
  const writing = Object.values(backup.library.events).find(event => event.fact.kind === "attempt_recorded");
  assert(writing && writing.fact.data.cues.length > 0, "Real UI completed an assisted attempt");
  const synthetic = addSyntheticFacts(backup, writing.fact.data.revision, writing);
  await writeFile(path.join(runDir, "synthetic-fact-backup.json"), JSON.stringify(backup, null, 2));
  await page.locator("#backup-json").fill(JSON.stringify(backup));
  await page.getByRole("button", { name: "Restore JSON", exact: true }).click();
  await page.getByText("Backup merged into your notebook.", { exact: true }).waitFor();
  const teacherPath = process.env.RUFREN_SHELL_TEACHER_FIXTURE || path.join(path.dirname(fileURLToPath(import.meta.url)), "fixtures/shell-update-teacher.json");
  const teacher = JSON.parse(await readFile(teacherPath));
  teacher.content.explanation = "SYNTHETIC SHELL UPDATE TEACHER FIXTURE — storage fixture; no new teacher request.\n\n" + teacher.content.explanation;
  teacher.provenance = { ...teacher.provenance, provider: "qa-shell-update-fixture", model: "synthetic-public-dto-copy", response_id: randomUUID() };
  await page.evaluate(async teacher => { window.rufrenLibrary.write("rufren.v1.meta.teacher_response", JSON.stringify(teacher)); await window.rufrenLibrary.flush(); }, teacher);
  await nav(page, "teacher").click();
  await page.getByText("SYNTHETIC SHELL UPDATE TEACHER FIXTURE", { exact: false }).first().waitFor();
  await nav(page, "Library").click();
  await page.getByRole("button", { name: /RUSSIAN Shell update story/ }).click();
  await page.getByRole("button", { name: "Write from memory", exact: true }).click();
  await page.getByRole("button", { name: "I'm ready to try", exact: false }).click();
  const draft = "Сохрани́ меня, ёжик — е\u0301. Это незаконченный ответ.";
  await page.locator("#practice-response").fill(draft);
  await page.getByRole("button", { name: "Give me a hint", exact: true }).click();
  const expected = await snapshot(page);
  assert.equal(expected.session.draft, draft); assert(expected.session.cues.length > 0);
  assert(expected.events[`rufren.v1.event.${synthetic.transcription}`]);
  await writeFile(path.join(runDir, "expected-durable-work.json"), JSON.stringify(expected, null, 2));
  return expected;
}

try {
  server = http.createServer(async (request, response) => {
    const url = new URL(request.url, "http://fixture.local");
    const relative = decodeURIComponent(url.pathname).replace(/^\/+/, "") || "index.html";
    requests.push({ at: Date.now(), path: url.pathname, method: request.method, failed: relative === failAsset });
    if (relative === failAsset) { response.writeHead(503, { "Cache-Control": "no-store" }); response.end("Deliberately interrupted new shell asset"); return; }
    try {
      const file = await realpath(path.resolve(serving, relative));
      if (!file.startsWith(serving + path.sep)) throw Error("Outside fixture root");
      const bytes = await readFile(file);
      response.writeHead(200, { "Content-Type": mime[path.extname(file)] || "application/octet-stream", "Cache-Control": "no-cache", "X-Content-Type-Options": "nosniff" }); response.end(bytes);
    } catch { response.writeHead(404, { "Cache-Control": "no-store" }); response.end("No API or fallback on this fixture origin"); }
  });
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  origin = `http://127.0.0.1:${server.address().port}`;
  await writeFile(path.join(runDir, "runtime.json"), JSON.stringify({ pid: process.pid, origin, baseline, control }, null, 2));
  context = await playwright.chromium.launchPersistentContext(path.join(runDir, "browser-profile"), { headless: true, viewport: { width: 1280, height: 900 }, serviceWorkers: "allow" });
  await context.route("**/*", route => {
    const url = new URL(route.request().url());
    if (!["http:", "https:"].includes(url.protocol) || url.origin === origin) return route.continue();
    forbidden.push(url.origin + url.pathname); return route.abort();
  });
  const observer = context.pages()[0] || await context.newPage();
  await observer.goto("about:blank");
  const cdp = await context.newCDPSession(observer);
  cdp.on("ServiceWorker.workerVersionUpdated", event => { for (const version of event.versions) { versions.set(version.versionId, version); lifecycle.push({ at: Date.now(), ...version }); } });
  await cdp.send("ServiceWorker.enable");
  const oldPage = await context.newPage(); await openApp(oldPage, origin + "/");
  const keeper = await context.newPage(); await openApp(keeper, origin + "/");
  const expected = await seed(oldPage);
  const oldVersion = [...versions.values()].find(version => version.scriptURL === origin + "/sw.js" && version.status === "activated");
  assert(oldVersion, "Real browser reports the baseline worker activated");
  const oldWasm = oldBuild.files.find(file => file.endsWith(".wasm"));
  const oldWasmHash = hash(await readFile(path.join(baseline, oldWasm)));
  assert.equal(await wasmDigest(oldPage, oldWasm), oldWasmHash);
  await oldPage.screenshot({ path: path.join(runDir, "baseline-draft.png"), fullPage: true });
  await state("baseline-real-work-ready-awaiting-new-artifact", { workspace: expected.workspace, facts: Object.keys(expected.events).length });
  let requested;
  while (!requested) {
    try { const value = JSON.parse(await readFile(control)); if (value.stop) throw Error("Stopped by fixture control"); requested = value.candidate; }
    catch (error) { if (error.message === "Stopped by fixture control") throw error; if (error.code !== "ENOENT" && !(error instanceof SyntaxError)) throw error; }
    if (!requested) await new Promise(resolve => setTimeout(resolve, 1000));
  }
  const originalCandidate = await realpath(requested), nextBuild = JSON.parse(await readFile(path.join(originalCandidate, "offline-build.json")));
  assert.notEqual(nextBuild.version, oldBuild.version, "A distinct genuine prepared Dioxus build is required");
  const candidate = path.join(runDir, `candidate-${nextBuild.version}`); await cp(originalCandidate, candidate, { recursive: true });
  serving = await realpath(candidate);
  const newWasm = nextBuild.files.find(file => file.endsWith(".wasm")), newWasmHash = hash(await readFile(path.join(candidate, newWasm)));
  assert.notEqual(newWasmHash, oldWasmHash, "The real new Dioxus WASM must differ from baseline");
  failAsset = newWasm;
  await state("testing-interrupted-new-install", { candidate: nextBuild.version });
  await oldPage.evaluate(() => navigator.serviceWorker.getRegistration().then(registration => registration.update()));
  await waitFor(() => [...versions.values()].some(v => v.registrationId === oldVersion.registrationId && v.versionId !== oldVersion.versionId && v.status === "redundant"), "new failed install becomes redundant");
  assert(requests.some(request => request.failed), "Real worker installation fetched the deliberately unavailable WASM");
  assert.deepEqual(await cacheNames(oldPage), [`rufren-shell:/:${oldBuild.version}`], "Incomplete candidate cache was removed, baseline preserved");
  await context.setOffline(true); await oldPage.reload({ waitUntil: "domcontentloaded" }); await verifyWork(oldPage, expected);
  assert.equal(await wasmDigest(oldPage, oldWasm), oldWasmHash, "Old app still boots fully offline after interrupted install");
  for (const relative of ["api/teacher/status", "v1/events", "pwa-register.js"]) {
    assert(await oldPage.evaluate(async relative => { try { await fetch(new URL(relative, location.href), { headers: { Authorization: "Bearer synthetic-offline-probe" } }); return false; } catch { return true; } }, relative), "Authenticated/private requests must not receive a cached shell");
  }
  await context.setOffline(false); failAsset = null;
  const documentToken = randomUUID(); await oldPage.evaluate(value => { window.__shellUpdateDocument = value; }, documentToken);
  await state("testing-waiting-new-version", { candidate: nextBuild.version });
  await oldPage.evaluate(() => navigator.serviceWorker.getRegistration().then(registration => registration.update()));
  await oldPage.waitForFunction(async () => (await navigator.serviceWorker.getRegistration()).waiting?.state === "installed", null, { timeout: 60_000 });
  await oldPage.locator("#rufren-update").waitFor();
  assert.equal(await oldPage.evaluate(() => window.__shellUpdateDocument), documentToken, "Update must not automatically reload a live draft");
  await verifyWork(oldPage, expected);
  assert.equal(await wasmDigest(oldPage, oldWasm), oldWasmHash, "Active old worker pins old WASM while new worker waits");
  const waitingVersion = [...versions.values()].find(v => v.registrationId === oldVersion.registrationId && v.versionId !== oldVersion.versionId && v.status === "installed"); assert(waitingVersion);
  await oldPage.screenshot({ path: path.join(runDir, "update-waiting-draft.png"), fullPage: true });
  await oldPage.reload({ waitUntil: "domcontentloaded" }); await verifyWork(oldPage, expected);
  assert.equal(await wasmDigest(oldPage, oldWasm), oldWasmHash, "Reload with another old tab still uses the old coherent shell");
  await oldPage.close();
  assert(await keeper.evaluate(async () => !!(await navigator.serviceWorker.getRegistration()).waiting), "Closing only one old tab must not activate over the remaining old client");
  await keeper.close();
  await waitFor(() => versions.get(waitingVersion.versionId)?.status === "activated", "new worker activates after all controlled old tabs close");
  await state("new-worker-activated-after-all-old-tabs-closed", { candidate: nextBuild.version });
  await context.setOffline(true);
  const updated = await context.newPage(); await openApp(updated, origin + "/"); await verifyWork(updated, expected);
  assert.equal(await wasmDigest(updated, newWasm), newWasmHash, "New browser client is controlled by the actual new shell");
  assert.deepEqual(await cacheNames(updated), [`rufren-shell:/:${nextBuild.version}`], "Activation cleaned only the previous shell version");
  const cachedUrls = await updated.evaluate(async () => { const result = []; for (const key of await caches.keys()) for (const request of await (await caches.open(key)).keys()) result.push(request.url); return result; });
  assert(cachedUrls.every(url => !new URL(url).search && !/\/(api|v1)\//.test(new URL(url).pathname)), "No private API or query URL entered Cache Storage");
  await nav(updated, "teacher").click(); await updated.getByText("SYNTHETIC SHELL UPDATE TEACHER FIXTURE", { exact: false }).first().waitFor();
  await updated.screenshot({ path: path.join(runDir, "new-version-saved-teacher.png"), fullPage: true });
  assert.deepEqual(pageErrors, [], "No uncaught Dioxus/browser errors"); assert.deepEqual(forbidden, [], "No external/provider/relay call was attempted");
  const result = { passed: true, baseline: oldBuild.version, candidate: nextBuild.version, oldWasmHash, newWasmHash, checks: ["actual UI assisted completed attempt", "exact Cyrillic unfinished draft and support", "clearly synthetic imported transcript/confirmation/assessment facts", "clearly synthetic saved public teacher DTO", "interrupted real installation preserves bootable offline old shell", "new worker waits without reloading draft", "old tab reload remains pinned", "one remaining old tab prevents activation", "all old tabs close before activation", "fresh offline new-version startup preserves durable work", "private/authenticated requests and API cache boundary"], factCount: Object.keys(expected.events).length, apiCalls: requests.filter(r => /^\/(api|v1)\//.test(r.path)).length };
  assert.equal(result.apiCalls, 0, "Fixture must make zero API calls");
  await writeFile(path.join(runDir, "result.json"), JSON.stringify(result, null, 2) + "\n");
  await state("passed", result);
} catch (error) {
  await writeFile(path.join(runDir, "failure.json"), JSON.stringify({ stage: currentStage, message: error.message, stack: error.stack }, null, 2) + "\n");
  console.error(error); process.exitCode = 1;
} finally {
  await writeFile(path.join(runDir, "worker-lifecycle.json"), JSON.stringify(lifecycle, null, 2));
  await writeFile(path.join(runDir, "http-requests.json"), JSON.stringify(requests, null, 2));
  await context?.close(); if (server) await new Promise(resolve => server.close(resolve));
}
