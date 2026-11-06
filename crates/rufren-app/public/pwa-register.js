/* Included only by prepare-web.mjs, never by the development preview. */
(() => {
  "use strict";
  if (!("serviceWorker" in navigator) || !window.isSecureContext) return;
  const scriptURL = new URL(document.currentScript.src);
  const scope = new URL("./", scriptURL);
  function status(state, detail = "") {
    window.rufrenOffline = Object.freeze({ state, detail });
    window.dispatchEvent(new CustomEvent("rufren-offline", { detail: window.rufrenOffline }));
  }
  function updateAvailable() {
    status("update-ready");
    if (document.getElementById("rufren-update")) return;
    const note = document.createElement("aside");
    note.id = "rufren-update";
    note.setAttribute("role", "status");
    note.style.cssText = "position:fixed;bottom:1rem;left:1rem;right:1rem;max-width:36rem;margin:auto;padding:1rem;border-radius:1rem;background:#203e3b;color:#faf5e9;box-shadow:0 4px 24px #0003;z-index:1000;font:1rem/1.5 system-ui";
    note.append("An update is ready. Close all Drussky windows and reopen when you’re ready.");
    const dismiss = document.createElement("button");
    dismiss.type = "button";
    dismiss.textContent = "Dismiss";
    dismiss.style.cssText = "margin-left:1rem;color:inherit;background:transparent;border:1px solid currentColor;border-radius:.5rem;padding:.25rem .5rem";
    dismiss.addEventListener("click", () => note.remove());
    note.append(dismiss);
    document.body.append(note);
  }
  status("installing");
  navigator.serviceWorker.register(new URL("sw.js", scope), { scope: scope.pathname, updateViaCache: "none" })
    .then(async registration => {
      if (registration.waiting) updateAvailable();
      registration.addEventListener("updatefound", () => {
        const worker = registration.installing;
        worker?.addEventListener("statechange", () => {
          if (worker.state === "installed" && navigator.serviceWorker.controller) updateAvailable();
          if (worker.state === "redundant") status("unavailable", "Offline installation did not complete.");
        });
      });
      await navigator.serviceWorker.ready;
      if (!registration.waiting) status("ready");
      // Recheck after a suspended/mobile app returns. Never reload a live lesson.
      document.addEventListener("visibilitychange", () => {
        if (document.visibilityState === "visible" && navigator.onLine) registration.update().catch(() => {});
      });
    })
    .catch(() => status("unavailable", "Offline startup is unavailable in this browser."));
})();
