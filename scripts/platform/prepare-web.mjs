#!/usr/bin/env node
// Run AFTER dx build. Cache URLs come from the resulting files, including hashed
// assets and wasm-bindgen snippets, rather than from assumptions about dx names.
import { createHash } from "node:crypto";
import { cp, mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const project = path.resolve(here, "../..");
const publicSource = path.join(project, "crates/rufren-app/public");
const staticExtension = /\.(?:js|wasm|css|svg|png|jpg|jpeg|webp|ico|woff2?|ttf|otf)$/i;

export async function prepareWeb(output) {
  output = path.resolve(output);
  let html = await readFile(path.join(output, "index.html"), "utf8");
  if (/__dx-toast|_dioxus|https:\/\/fonts\.googleapis\.com/.test(html)) {
    throw new Error("Development HTML detected. Run dx build with the checked-in index.html before packaging offline startup.");
  }
  if (!/<script\b[^>]*src=/.test(html)) throw new Error("No Dioxus loader found in output index.html.");
  await cp(publicSource, output, { recursive: true });
  const head = `<!-- rufren-pwa:start -->
    <meta name="theme-color" content="#203e3b">
    <meta name="apple-mobile-web-app-capable" content="yes">
    <meta name="apple-mobile-web-app-title" content="Rufren">
    <link rel="manifest" href="./manifest.webmanifest">
    <link rel="icon" type="image/svg+xml" href="./icons/icon.svg">
    <link rel="apple-touch-icon" href="./icons/apple-touch-icon.png">
    <script defer src="./pwa-register.js"></script>
    <!-- rufren-pwa:end -->`;
  html = html.replace(/<!-- rufren-pwa:start -->[\s\S]*?<!-- rufren-pwa:end -->\s*/g, "");
  html = html.replace(/<\/head>/i, `${head}\n  </head>`);
  await writeFile(path.join(output, "index.html"), html);
  const files = ["index.html", "manifest.webmanifest", "pwa-register.js"];
  async function visit(relative) {
    for (const entry of (await readdir(path.join(output, relative), { withFileTypes: true })).sort((a, b) => a.name.localeCompare(b.name))) {
      const name = path.posix.join(relative, entry.name);
      if (entry.isSymbolicLink()) throw new Error(`Refusing symbolic link in web output: ${name}`);
      if (entry.isDirectory()) await visit(name);
      else if (entry.isFile() && staticExtension.test(name)) files.push(name);
    }
  }
  // Only build-owned static directories. User data, relay URLs, arbitrary JSON,
  // recordings and API responses are never discovered or cached here.
  for (const directory of ["assets", "wasm", "icons"]) {
    try { await visit(directory); }
    catch (error) { if (error.code !== "ENOENT") throw error; }
  }
  files.sort();
  if (!files.some(name => name.endsWith(".wasm"))) throw new Error("No WebAssembly artifact found; pass dx's web/public output directory.");
  for (const required of ["icons/icon-192.png", "icons/icon-512.png", "icons/apple-touch-icon.png"]) {
    if (!files.includes(required)) throw new Error(`Missing ${required}; run scripts/platform/generate-icons.sh.`);
  }
  const template = await readFile(path.join(here, "service-worker.js"), "utf8");
  const hash = createHash("sha256").update(template);
  for (const file of files) hash.update(file).update("\0").update(await readFile(path.join(output, file)));
  const version = hash.digest("hex").slice(0, 20);
  const worker = template.replace("__RUFREN_VERSION__", JSON.stringify(version)).replace("__RUFREN_FILES__", JSON.stringify(files));
  await writeFile(path.join(output, "sw.js"), worker);
  await writeFile(path.join(output, "offline-build.json"), JSON.stringify({ version, files }, null, 2) + "\n");
  return { output, version, assets: files.length };
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  if (!process.argv[2]) throw new Error("Usage: node scripts/platform/prepare-web.mjs <dx web/public directory>");
  console.log(JSON.stringify(await prepareWeb(process.argv[2]), null, 2));
}
