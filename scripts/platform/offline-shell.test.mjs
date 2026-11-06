import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import vm from "node:vm";
import { prepareWeb } from "./prepare-web.mjs";

async function fixture() {
  const directory = await mkdtemp(path.join(tmpdir(), "rufren-shell-"));
  await mkdir(path.join(directory, "assets"));
  await writeFile(path.join(directory, "index.html"), '<html><head></head><body><div id="main"></div><script src="./assets/arbitrary-hash-A.js"></script></body></html>');
  await writeFile(path.join(directory, "assets/arbitrary-hash-A.js"), "export const build = 1;");
  await writeFile(path.join(directory, "assets/unpredictable-9f3b.wasm"), "wasm-fixture");
  return directory;
}

function environment(source, failAsset = "") {
  const handlers = {};
  const stores = new Map();
  const fetches = [];
  let claims = 0;
  const caches = {
    async open(name) {
      if (!stores.has(name)) stores.set(name, new Map());
      const entries = stores.get(name);
      return { async put(request, response) { entries.set(request.url, response); }, async match(url) { return entries.get(typeof url === "string" ? url : url.url); } };
    },
    async delete(name) { return stores.delete(name); },
    async keys() { return [...stores.keys()]; },
  };
  const context = vm.createContext({
    URL, Set, Request, caches,
    self: { registration: { scope: "https://rufren.test/" }, clients: { async claim() { claims++; } }, addEventListener(name, fn) { handlers[name] = fn; } },
    async fetch(request) {
      fetches.push(request);
      if (request.url.includes(failAsset) && failAsset) throw new Error("Simulated interrupted update");
      return { ok: true, type: "basic", redirected: false, body: request.url };
    },
  });
  vm.runInContext(source, context);
  async function lifecycle(name) {
    let promise;
    handlers[name]({ waitUntil(value) { promise = value; } });
    await promise;
  }
  async function request(url, options = {}) {
    let promise;
    const req = new Request(url, options);
    handlers.fetch({ request: req, respondWith(value) { promise = value; } });
    return { intercepted: !!promise, response: await promise };
  }
  return { stores, fetches, lifecycle, request, claimed: () => claims };
}

test("exact emitted filenames are cached, preparation is deterministic, updates change the version", async () => {
  const dir = await fixture();
  try {
    const first = await prepareWeb(dir);
    assert.equal((await prepareWeb(dir)).version, first.version);
    const build = JSON.parse(await readFile(path.join(dir, "offline-build.json")));
    assert(build.files.includes("assets/unpredictable-9f3b.wasm"));
    assert(!build.files.includes("offline-build.json"));
    await writeFile(path.join(dir, "assets/arbitrary-hash-A.js"), "export const build = 2;");
    assert.notEqual((await prepareWeb(dir)).version, first.version);
  } finally { await rm(dir, { recursive: true }); }
});

test("offline shell excludes authenticated/API requests and interrupted updates preserve old cache", async () => {
  const dir = await fixture();
  try {
    await prepareWeb(dir);
    const source = await readFile(path.join(dir, "sw.js"), "utf8");
    const env = environment(source);
    await env.lifecycle("install");
    assert(env.fetches.every(request => request.credentials === "omit"));
    const cached = await env.request("https://rufren.test/assets/unpredictable-9f3b.wasm");
    assert(cached.intercepted);
    const count = env.fetches.length;
    assert.equal(cached.response.body, "https://rufren.test/assets/unpredictable-9f3b.wasm");
    assert.equal(env.fetches.length, count);
    for (const [url, options] of [
      ["https://rufren.test/v1/events", {}],
      ["https://relay.test/v1/events", {}],
      ["https://rufren.test/assets/arbitrary-hash-A.js", { headers: { Authorization: "Bearer test" } }],
      ["https://rufren.test/assets/arbitrary-hash-A.js?token=test", {}],
      ["https://rufren.test/assets/arbitrary-hash-A.js", { method: "POST" }],
    ]) assert.equal((await env.request(url, options)).intercepted, false);
    env.stores.set("rufren-shell:/:old-version", new Map());
    env.stores.set("unrelated-app-cache", new Map());
    await env.lifecycle("activate");
    assert.equal(env.stores.has("rufren-shell:/:old-version"), false);
    assert(env.stores.has("unrelated-app-cache"));
    assert.equal(env.claimed(), 1);

    const interrupted = environment(source, ".wasm");
    interrupted.stores.set("rufren-shell:/:old-version", new Map());
    await assert.rejects(interrupted.lifecycle("install"), /interrupted/);
    assert.deepEqual([...interrupted.stores.keys()], ["rufren-shell:/:old-version"]);
  } finally { await rm(dir, { recursive: true }); }
});
