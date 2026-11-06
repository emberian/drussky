/* Generated with an exact build asset list. Do not serve this template directly. */
"use strict";
const VERSION = __RUFREN_VERSION__;
const FILES = __RUFREN_FILES__;
const SCOPE = new URL(self.registration.scope);
const PREFIX = `rufren-shell:${SCOPE.pathname}:`;
const CACHE = PREFIX + VERSION;
const ALLOWED = new Set(FILES.map(file => new URL(file, SCOPE).href));
const SHELL = new URL("index.html", SCOPE).href;

self.addEventListener("install", event => {
  event.waitUntil((async () => {
    const cache = await caches.open(CACHE);
    try {
      // A failed/partial new version must never replace the working old one.
      for (const url of ALLOWED) {
        const request = new Request(url, { credentials: "omit", cache: "reload" });
        const response = await fetch(request);
        if (!response.ok || response.type === "opaque" || response.redirected) throw new Error("Shell asset unavailable");
        await cache.put(request, response);
      }
    } catch (error) {
      await caches.delete(CACHE);
      throw error;
    }
    // Intentionally no skipWaiting: close all windows before changing versions.
  })());
});

self.addEventListener("activate", event => {
  event.waitUntil((async () => {
    for (const name of await caches.keys()) {
      if (name.startsWith(PREFIX) && name !== CACHE) await caches.delete(name);
    }
    await self.clients.claim();
  })());
});

self.addEventListener("fetch", event => {
  const request = event.request;
  const url = new URL(request.url);
  // Fail closed: do not intercept any authenticated, cross-origin, query-bearing
  // or non-GET request. There is no runtime data cache and no API fallback.
  if (request.method !== "GET" || request.headers.has("Authorization") || url.origin !== SCOPE.origin || url.search) return;
  const shellNavigation = request.mode === "navigate" && (url.href === SCOPE.href || url.href === SHELL);
  if (!shellNavigation && !ALLOWED.has(url.href)) return;
  event.respondWith((async () => {
    const cache = await caches.open(CACHE);
    const cached = await cache.match(shellNavigation ? SHELL : url.href);
    // Pin a page and all modules to one shell version. Missing cache entries
    // may be retrieved, but responses are never added to the cache at runtime.
    return cached || fetch(request);
  })());
});
