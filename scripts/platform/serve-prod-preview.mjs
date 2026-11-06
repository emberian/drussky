#!/usr/bin/env node
// Standalone static preview: no devserver injection and no relay/API proxy.
import http from "node:http";
import path from "node:path";
import { readFile, realpath } from "node:fs/promises";

const root = await realpath(process.argv[2] || "target/dx/rufren-app/debug/web/public");
const port = Number(process.argv[3] || 8091);
const mime = { ".html": "text/html; charset=utf-8", ".js": "text/javascript; charset=utf-8", ".wasm": "application/wasm", ".css": "text/css; charset=utf-8", ".svg": "image/svg+xml", ".png": "image/png", ".json": "application/json", ".webmanifest": "application/manifest+json", ".woff2": "font/woff2" };
const server = http.createServer(async (request, response) => {
  if (!["GET", "HEAD"].includes(request.method)) { response.writeHead(405); response.end(); return; }
  try {
    const url = new URL(request.url, "http://localhost");
    let relative = decodeURIComponent(url.pathname).replace(/^\/+/, "");
    if (!relative) relative = "index.html";
    const file = await realpath(path.resolve(root, relative));
    if (!file.startsWith(root + path.sep)) throw new Error("Outside static root");
    const bytes = await readFile(file);
    response.writeHead(200, {
      "Content-Type": mime[path.extname(file)] || "application/octet-stream",
      "Cache-Control": "no-cache",
      "X-Content-Type-Options": "nosniff",
      "Referrer-Policy": "same-origin",
      "Permissions-Policy": "microphone=(self), camera=(), geolocation=()",
    });
    response.end(request.method === "HEAD" ? undefined : bytes);
  } catch {
    response.writeHead(404, { "Cache-Control": "no-store" });
    response.end("Not found");
  }
});
server.listen(port, "127.0.0.1", () => console.log(`Rufren production preview: http://127.0.0.1:${port} (pid ${process.pid})`));
