# Optional private HTTPS relay

The relay carries the full learner event history. This is an optional single-library deployment on a Linux machine you already control. No host, DNS record, subscription, provider account, or deployment is created by this repository. The bearer key grants full read/write access to that library. TLS protects transport; the relay database and backups contain plaintext personal text. Original audio remains in a separate IndexedDB database on each device. Settings can explicitly synchronize recordings as well as notebook facts. Original bytes are addressed by SHA-256; each take keeps its own identity, decoded metadata, and append-only timed cues even when two takes share identical bytes. The JSON library export still excludes binary audio; the speech studio offers a separate portable audio backup.

## Install on an existing host

Build for that host's architecture from this checkout: `cargo build --release -p rufren-relay`. Copy `target/release/rufren-relay` to `/opt/rufren/rufren-relay`. Keep binaries root-owned. Create a dedicated unprivileged `rufren` user/group and root-owned `/etc/rufren` directory. Install `rufren-relay.service` in `/etc/systemd/system/` and `relay.env.example` as `/etc/rufren/relay.env`, mode `0600`, root-owned.

Edit the environment file privately. Set `RUFREN_ALLOWED_ORIGINS` to the exact browser app HTTPS origin, with no trailing slash. Generate a 256-bit sync key directly into the environment file without printing it or putting it in shell history. For example, after installing the example file:

```sh
sudo python3 - <<'PY'
from pathlib import Path
import secrets
path = Path('/etc/rufren/relay.env')
text = path.read_text()
assert 'RUFREN_SYNC_TOKEN=\n' in text, 'Initialize from the example before generating a key'
text = text.replace('RUFREN_SYNC_TOKEN=\n', 'RUFREN_SYNC_TOKEN=' + secrets.token_urlsafe(32) + '\n', 1)
path.write_text(text)
path.chmod(0o600)
PY
sudo systemctl daemon-reload
sudo systemctl enable --now rufren-relay
```

Transfer the key into the app through your own private channel/password manager. Do not paste it into chat or commit it. The app holds the key in memory for the current session. The service reads it from its root-only environment file. `RUFREN_TUTOR_PROVIDER=disabled` is the default; syncing never invokes a model. Optional tutoring uses a separate server-side `ANTHROPIC_API_KEY` and explicit provider selection. Its budget is a conservative, process-local reservation limit, reset by restart; it is not an account billing cap.

Point a DNS name you control at the machine and install Caddy using its [official service instructions](https://caddyserver.com/docs/running). Adapt `Caddyfile.example` to your hostname, validate with `sudo caddy validate --config /etc/caddy/Caddyfile`, then reload Caddy. The example uses the documented [request body size limit](https://caddyserver.com/docs/caddyfile/directives/request_body). Expose HTTPS/443 (and port80 for certificate issuance/redirects, depending on the certificate setup). Keep 8787 bound to loopback and blocked from external access. `/health` returns a version string; it does not disclose notebook contents. All sync/tutor routes require the bearer key, including malformed requests. CORS permits only the app origin; it does not replace authentication for other clients.

Check the HTTPS health endpoint and a request without a key (expect401) before entering the relay URL in Rufren. No redirects are followed by the sync client. Deploying the static browser app is a separate step; use a trusted HTTPS origin and keep its URL stable, since browser data belongs to that origin.

## Backup and restore drill

Use Python3.11+ and `scripts/relay-backup.py`. It uses SQLite's [online backup API](https://www.sqlite.org/backup.html), including committed WAL pages, and checks database integrity/schema. Each backup has a versioned JSON manifest with the SQLite SHA-256, event count, cursor, relay identity, and every referenced original audio hash/length. A sibling `BACKUP.sqlite.assets/` directory holds those verified originals. Keep the database, manifest, and audio directory together. Files are created exclusively with private permissions; existing paths are never overwritten. A digest detects accidental corruption, not an adversary who can replace both files. Encrypt off-host copies with your existing backup system and protect the encryption key separately.

```sh
sudo -u rufren python3 /opt/rufren/relay-backup.py backup \
  /var/lib/rufren/relay.sqlite /var/lib/rufren/backups/relay-2026-11-05.sqlite \
  --assets /var/lib/rufren/assets
sudo -u rufren python3 /opt/rufren/relay-backup.py verify \
  /var/lib/rufren/backups/relay-2026-11-05.sqlite
sudo -u rufren python3 /opt/rufren/relay-backup.py restore \
  /var/lib/rufren/backups/relay-2026-11-05.sqlite /var/lib/rufren/drill/restored.sqlite
sudo -u rufren /opt/rufren/rufren-relay --check-db /var/lib/rufren/drill/restored.sqlite
```

The backup anchors one SQLite read transaction, then finalizes a standalone database file before hashing it. Do not copy a live SQLite file or mix WAL/SHM files into a restored snapshot. Original bytes are immutable and copied only when referenced by the frozen registry; recording/transcription facts are also checked against that registry. Facts can legitimately sync before audio: if a referenced take has not completed recording sync, the command reports **audio backup incomplete**, retains the fact snapshot for inspection, and publishes no usable manifest. Sync originals and their take metadata from the originating device, then retry into a fresh destination. This state does not mean the facts are corrupt.

Copy `scripts/relay-backup.py` to the shown host path first. Verify `--check-db` reports the manifest's event count and recording metadata count: it validates the complete immutable event graph and recording metadata with the actual Rust reader, including legacy v1/v2 and current v3 facts, without starting a server or contacting a model. A backup is useful only after this isolated restore succeeds. Repeat after format upgrades. Pick unique filenames for later backups. Schedule against the host's explicit timezone only after one successful manual run; no unattended schedule or retention deletion is installed here.

For a real restore, stop only `rufren-relay.service` through systemd, retain the old database plus its `-wal`/`-shm` sidecars together, restore to a **new database path**, and set `RUFREN_RELAY_DB` to that new path and `RUFREN_RELAY_ASSETS` to its matching restored `.sqlite.assets` directory. Do not overwrite a live SQLite database or mix old sidecars into the restored file. Start the service and verify health. The original relay identity is retained. Devices whose cursors are ahead of the restored snapshot will receive a conflict: disconnect and reconnect in the app to clear delivery state and resend every retained fact. Facts created on a device after the backup can reseed the relay; content absent from all devices and backups cannot be recovered.

Relay UUID, facts, and take metadata belong to the data snapshot. Sync/provider credentials, TLS private keys, process configuration, browser workspaces/drafts, and installed model weights do not. Restore configuration and credentials separately from secure storage, or rotate credentials and update clients. A changed authentication key does not change the restored library identity. Run `--check-db` only on the fresh restored working copy; it opens that copy with the real Rust reader. Verify health and client recovery before promoting it.

The current v3 recovery acceptance is recorded in [STORAGE_RECOVERY.md](../research/STORAGE_RECOVERY.md). It exercises actual app clients through a separately verified TLS boundary, ahead-cursor refusal followed by explicit reseeding, and rejection of a different library without losing a pending upload. It does not certify iPad/Safari certificate trust or protect against loss of the Mac itself.

To revoke a lost key, generate a new key privately in the environment file and restart only this systemd service, then re-enter it on authorized clients. The old key immediately loses access. This is one shared bearer key, with no per-device revocation or end-to-end encryption.

## Browser durability and portable export

Startup opens IndexedDB `rufren-library`, physical version1, and validates the event history before showing the notebook. Existing `rufren.v1.*` localStorage keys migrate without changing event IDs or versions. New writes first enter a synchronous localStorage journal, then move into IndexedDB in a transaction. The journal is removed only after commit, with comparison checks to avoid deleting another tab's later write. A crash or IndexedDB failure retains that journal for retry. Browser quota failure returns a visible save error; it never silently clears the notebook. Browser storage can still be evicted or explicitly cleared, so export regularly and sync when useful.

The JSON export is a versioned `rufren-library` envelope, with the event version and timestamp. Restore also accepts the original raw `Library` JSON and merges immutable facts transactionally on native; browser journal writes may partially succeed and can safely be retried. A newer unsupported envelope fails explicitly. Export excludes local drafts, relay keys, and binary audio. A browser recording export must be kept alongside the library backup when that audio matters. Explicit recording sync transfers local original audio and per-take metadata/cues to the same bound private relay, then downloads missing originals. A relay UUID header protects each transfer against a changed library. Uploads are bounded to 32 MiB, hash checked before an atomic create, and safe to retry. Metadata is advertised only after the original bytes commit. Native HTTP uses Rust rather than WebView fetch; browser fetch rejects redirects and aborts on timeout.

The relay defaults to a 512 MiB original-audio quota (`RUFREN_RELAY_ASSET_QUOTA_BYTES`). Reaching the quota returns 507 and keeps existing recordings; no automatic deletion is installed. A crash may leave an unreferenced partial file, which counts toward the quota and needs operator inspection. Corrupt stored originals return a conflict rather than being served or silently overwritten; restore a verified backup or use a new relay and reseed from devices. Backups copy the registry's referenced blobs after taking its consistent SQLite snapshot, so concurrent new uploads cannot make that snapshot refer to uncommitted bytes.

## Local continuity checks

`cargo test -p rufren-relay --test assets asset_` covers authentication/library binding, hash and size rejection, idempotent uploads, distinct takes sharing bytes, concurrent cue union, quota rejection, and disk corruption. `python3 scripts/test-relay-backup.py` checks live WAL snapshots, corruption, no-overwrite restores, and audio bundles.

With Playwright available to Node, `node scripts/test-audio-relay.mjs` launches its own disposable local relay and two isolated browser devices, transfers and decodes a valid synthetic WAV across actual authenticated CORS routes, merges concurrent timed cues, then backs up/restores the database and original audio and restarts the relay. It keeps its fixture directory for inspection, records its own PID/log under ignored `.local/`, and stops only that relay. Set `RUFREN_PLAYWRIGHT_MODULE` or `RUFREN_CHROMIUM_EXECUTABLE` when those tools live outside normal resolution. No real recordings, existing browser profiles, model providers, deployment hosts, or preview processes are used.
