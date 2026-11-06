use rufren_core::*;
use serde::{Serialize, de::DeserializeOwned};
use std::collections::BTreeSet;
use uuid::Uuid;
pub type Result<T> = std::result::Result<T, String>;
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Surviving local response slots. Recovery never edits their originating workspace.
#[derive(serde::Deserialize)]
pub(crate) struct CachedTeacherMetadata {
    pub response: serde_json::Value,
    pub prepared_context: Option<serde_json::Value>,
}

pub struct Repository {
    #[cfg(not(target_arch = "wasm32"))]
    store: std::cell::RefCell<rufren_store::Store>,
    #[cfg(target_arch = "wasm32")]
    storage: BrowserStorage,
    #[cfg(target_arch = "wasm32")]
    projection: std::cell::RefCell<(u64, Library)>,
}
impl Repository {
    pub fn open() -> Result<Self> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = if let Ok(p) = std::env::var("RUFREN_DATA_DIR") {
                std::path::PathBuf::from(p)
            } else {
                directories::ProjectDirs::from("org", "ember", "rufren")
                    .ok_or("No application data directory")?
                    .data_dir()
                    .to_path_buf()
            };
            std::fs::create_dir_all(&path).map_err(err)?;
            Self::open_at(path.join("library.sqlite"))
        }
        #[cfg(target_arch = "wasm32")]
        {
            if js_sys::eval("!!window.rufrenLibrary")
                .map_err(|_| "Browser storage unavailable")?
                .as_bool()
                != Some(true)
            {
                return Err("Browser storage is still opening. Reopen Rufren.".into());
            }
            Ok(Self {
                storage: BrowserStorage,
                projection: std::cell::RefCell::new((0, Library::default())),
            })
        }
    }
    /// Open an explicit native SQLite path (including `:memory:` for isolated model tests).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_at(path: impl AsRef<std::path::Path>) -> Result<Self> {
        Ok(Self {
            store: std::cell::RefCell::new(rufren_store::Store::open(path).map_err(err)?),
        })
    }
    pub fn identity(&self) -> Result<Uuid> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.store.borrow().identity("device").map_err(err)
        }
        #[cfg(target_arch = "wasm32")]
        {
            // Different tabs get their own writer ID; facts are stored under immutable event keys.
            // The app keeps this ID for its lifetime; pending replication includes all unacked facts.
            Ok(Uuid::new_v4())
        }
    }
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.store.borrow().get(key).map_err(err)
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.storage
                .get_item(&format!("rufren.v1.meta.{key}"))
                .map_err(|_| "Cannot read browser storage".to_owned())?
                .map(|s| serde_json::from_str::<Option<T>>(&s).map_err(err))
                .transpose()
                .map(Option::flatten)
        }
    }
    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.store.borrow().set(key, value).map_err(err)
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.storage
                .set_item(
                    &format!("rufren.v1.meta.{key}"),
                    &serde_json::to_string(value).map_err(err)?,
                )
                .map_err(|_| {
                    "Browser storage is full or unavailable; your change was not saved".into()
                })
        }
    }
    pub(crate) fn cached_teacher_metadata(&self) -> Result<Vec<CachedTeacherMetadata>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let prepared_context = self
                .get::<serde_json::Value>("teacher_context_prepared")
                .ok()
                .flatten();
            let mut caches = Vec::new();
            for key in ["teacher_response", "teacher_lesson_response"] {
                if let Some(response) = self
                    .get::<serde_json::Value>(key)
                    .unwrap_or(Some(serde_json::Value::Null))
                {
                    caches.push(CachedTeacherMetadata {
                        response,
                        prepared_context: prepared_context.clone(),
                    });
                }
            }
            Ok(caches)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let json = js_sys::eval("JSON.stringify(window.rufrenLibrary.teacherCaches())")
                .map_err(
                    |_| "Cannot read saved teacher responses; their original caches were kept",
                )?
                .as_string()
                .ok_or("Invalid teacher response cache")?;
            serde_json::from_str(&json).map_err(err)
        }
    }

    pub fn library(&self) -> Result<Library> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.store.borrow().library().map_err(err)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let mut cached = self.projection.borrow_mut();
            let (revision, events) = self.storage.events_after(cached.0)?;
            if !events.is_empty() {
                cached.1.merge(&events).map_err(err)?;
            }
            cached.0 = revision;
            Ok(cached.1.clone())
        }
    }
    pub fn disconnect(&self) -> Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.store.borrow_mut().disconnect().map_err(err)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let keys = self
                .storage
                .entries()?
                .into_iter()
                .map(|(key, _)| key)
                .filter(|key| key.starts_with("rufren.v1.ack."));
            for k in keys {
                self.storage
                    .remove_item(&k)
                    .map_err(|_| "Could not forget sync acknowledgement")?;
            }
            self.storage
                .remove_item("rufren.v1.meta.sync")
                .map_err(|_| "Could not disconnect sync".into())
        }
    }
    pub fn append(&self, events: &[Event]) -> Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.store.borrow_mut().append(events).map_err(err)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let mut library = self.library()?;
            library.merge(events).map_err(err)?;
            let encoded = events
                .iter()
                .map(|event| {
                    let json = serde_json::to_string(event).map_err(err)?;
                    if json.len() > 1_048_576 {
                        return Err("Event exceeds the sync size budget".into());
                    }
                    Ok((event.id, json))
                })
                .collect::<Result<Vec<_>>>()?;
            // Each key is immutable. Partial batches can safely be retried after quota/error.
            for (id, json) in encoded {
                self.storage.set_item(&format!("rufren.v1.event.{id}"), &json)
                    .map_err(|_| "Could not save all events. Storage may be full; retry after exporting a backup.")?;
            }
            Ok(())
        }
    }
    pub fn acknowledged(&self, relay: Uuid) -> Result<BTreeSet<Uuid>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.store.borrow().acknowledged(relay).map_err(err)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let prefix = format!("rufren.v1.ack.{relay}.");
            self.storage
                .entries()?
                .into_iter()
                .filter_map(|(key, _)| {
                    key.strip_prefix(&prefix)
                        .map(|id| Uuid::parse_str(id).map_err(err))
                })
                .collect()
        }
    }
    pub fn accept(&self, binding: &SyncBinding, response: &SyncResponse) -> Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.store
                .borrow_mut()
                .accept(binding, response)
                .map_err(err)
        }
        #[cfg(target_arch = "wasm32")]
        {
            if response.version != SYNC_VERSION
                || binding.relay != response.relay
                || binding.cursor != response.cursor
            {
                return Err("Invalid sync page or binding".into());
            }
            // Durability order matters: events first, acks second, cursor last. A crash causes a
            // duplicate replay, never a cursor that skips events we haven't stored.
            self.append(&response.events)?;
            for id in &response.accepted {
                self.storage
                    .set_item(&format!("rufren.v1.ack.{}.{}", binding.relay, id), "1")
                    .map_err(|_| "Could not save sync acknowledgement")?;
            }
            self.set("sync", binding)
        }
    }
}

pub async fn synchronize(
    repo: std::rc::Rc<Repository>,
    endpoint: String,
    token: String,
) -> Result<usize> {
    let base = endpoint.trim().trim_end_matches('/').to_owned();
    let url = reqwest::Url::parse(&base).map_err(|_| "Enter a full sync URL")?;
    let loopback = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if url.scheme() != "https" && !(url.scheme() == "http" && loopback) {
        return Err("Use HTTPS for sync, or HTTP on localhost for development".into());
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("Use a sync URL without credentials, query, or fragment".into());
    }
    if token.is_empty() {
        return Err("Enter your sync key; it stays in memory for this session".into());
    }
    let mut binding: Option<SyncBinding> = repo.get("sync")?;
    if binding.as_ref().is_some_and(|b| b.endpoint != base) {
        return Err("Disconnect the existing sync library before changing its URL".into());
    }
    #[cfg(not(target_arch = "wasm32"))]
    let client = crate::native_http::client(&base, std::time::Duration::from_secs(30))?;
    #[cfg(target_arch = "wasm32")]
    let client = reqwest::Client::builder().build().map_err(err)?;
    let mut received = 0;
    for _ in 0..1000 {
        let lib = repo.library()?;
        let ack = if let Some(b) = &binding {
            repo.acknowledged(b.relay)?
        } else {
            BTreeSet::new()
        };
        // Include imported facts as well: a backup can seed a new relay without changing origin IDs.
        // Bound bytes as well as event count; heavily escaped text can be much larger in JSON.
        let mut pending = Vec::new();
        let mut bytes = 0;
        for event in lib
            .events
            .values()
            .filter(|e| !ack.contains(&e.id))
            .take(SYNC_BATCH)
        {
            let size = serde_json::to_vec(event).map_err(err)?.len();
            if !pending.is_empty() && bytes + size > 1_048_576 {
                break;
            }
            bytes += size;
            pending.push(event.clone());
        }
        let more_pending = lib.events.keys().filter(|id| !ack.contains(id)).count() > pending.len();
        let request = SyncRequest {
            version: SYNC_VERSION,
            relay: binding.as_ref().map(|b| b.relay),
            after: binding.as_ref().map_or(0, |b| b.cursor),
            events: pending,
        };
        let response = exchange_http(&client, &base, &token, &request).await?;
        if response.version != SYNC_VERSION {
            return Err(Error::Version(response.version).to_string());
        }
        if binding.as_ref().is_some_and(|b| b.relay != response.relay) {
            return Err(Error::RelayChanged.to_string());
        }
        if response.cursor < request.after
            || response.events.len() > SYNC_BATCH
            || response.relay.is_nil()
            || response.accepted.len() > SYNC_BATCH
            || (response.has_more && response.events.is_empty())
            || (response.events.is_empty() && response.cursor != request.after)
            || response.cursor - request.after < response.events.len() as u64
            || response
                .events
                .iter()
                .map(|e| e.id)
                .collect::<BTreeSet<_>>()
                .len()
                != response.events.len()
        {
            return Err("Invalid sync cursor or page".into());
        }
        let sent: BTreeSet<_> = request.events.iter().map(|e| e.id).collect();
        if response.accepted.len() != sent.len()
            || response.accepted.iter().copied().collect::<BTreeSet<_>>() != sent
        {
            return Err("Sync acknowledgement does not match the sent batch".into());
        }
        let next = SyncBinding {
            endpoint: base.clone(),
            relay: response.relay,
            cursor: response.cursor,
        };
        repo.accept(&next, &response)?;
        received += response.events.len();
        binding = Some(next);
        if !response.has_more && !more_pending {
            return Ok(received);
        }
    }
    Err("Sync paused after many pages. Run it again to continue.".into())
}

/// Versioned portable text/fact backup. Audio Blobs are exported separately in the studio.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LibraryBackup {
    format: String,
    version: u32,
    event_version: u32,
    created_at_ms: i64,
    library: Library,
}
pub fn export_library(library: Library) -> Result<String> {
    serde_json::to_string_pretty(&LibraryBackup {
        format: "rufren-library".into(),
        version: 1,
        event_version: EVENT_VERSION,
        created_at_ms: now_ms(),
        library,
    })
    .map_err(err)
}
pub fn import_library(text: &str) -> Result<Library> {
    // Allow original unwrapped exports while rejecting newer backup envelopes explicitly.
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("Could not read backup: {e}"))?;
    let library = if value.get("format").is_some() {
        let backup: LibraryBackup = serde_json::from_value(value).map_err(err)?;
        if backup.format != "rufren-library"
            || backup.version != 1
            || backup.event_version > EVENT_VERSION
        {
            return Err(
                "Unsupported backup format or event version. Update Rufren before restoring."
                    .into(),
            );
        }
        backup.library
    } else {
        serde_json::from_value(value).map_err(err)?
    };
    if library.events.iter().any(|(id, e)| *id != e.id) {
        return Err("Backup event IDs do not match".into());
    }
    let mut validated = Library::default();
    validated
        .merge(&library.events.into_values().collect::<Vec<_>>())
        .map_err(err)?;
    Ok(validated)
}

const MAX_SYNC_RESPONSE: usize = 1_100_000;
async fn exchange_http(
    client: &reqwest::Client,
    base: &str,
    token: &str,
    request: &SyncRequest,
) -> Result<SyncResponse> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut response = client
            .post(format!("{base}/v1/sync"))
            .bearer_auth(token)
            .json(request)
            .send()
            .await
            .map_err(|e| {
                format!(
                    "Could not reach your sync library: {e}. Your saved work is safe; retry sync."
                )
            })?;
        if response
            .content_length()
            .is_some_and(|n| n > MAX_SYNC_RESPONSE as u64)
        {
            return Err("Sync response exceeds the size limit".into());
        }
        let status = response.status();
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(err)? {
            if body.len() + chunk.len() > MAX_SYNC_RESPONSE {
                return Err("Sync response exceeds the size limit".into());
            }
            body.extend_from_slice(&chunk);
        }
        if !status.is_success() {
            return Err(format!(
                "Sync returned {status}: {}",
                String::from_utf8_lossy(&body[..body.len().min(1024)])
            ));
        }
        serde_json::from_slice(&body).map_err(err)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = client;
        // Fetch AbortController covers both headers and streaming body on web. `redirect:error`
        // prevents a redirect from sending this private key to another endpoint.
        let args = serde_json::json!({"url": format!("{base}/v1/sync"), "token":token, "body":serde_json::to_string(request).map_err(err)?, "limit":MAX_SYNC_RESPONSE});
        let script = format!(
            r#"const a = {args};
          const controller = new AbortController();
          const timer = setTimeout(() => controller.abort(), 30000);
          try {{
            const response = await fetch(a.url, {{method:'POST', redirect:'error', signal:controller.signal, headers:{{'Authorization':'Bearer '+a.token,'Content-Type':'application/json'}}, body:a.body}});
            const reader = response.body.getReader(); let size=0, chunks=[];
            while(true) {{ const part=await reader.read(); if(part.done) break; size+=part.value.byteLength; if(size>a.limit) {{ await reader.cancel(); throw Error('Sync response exceeds the size limit'); }} chunks.push(part.value); }}
            const bytes=new Uint8Array(size); let offset=0; for(const c of chunks){{bytes.set(c,offset);offset+=c.length;}}
            const body=new TextDecoder().decode(bytes);
            if(!response.ok) throw Error('Sync returned '+response.status+': '+body.slice(0,1024));
            return {{body}};
          }} catch(error) {{ return {{error:error.name==='AbortError'?'Sync timed out. Your work is saved; retry sync.':String(error.message)}}; }} finally {{ clearTimeout(timer); }}"#
        );
        let output: serde_json::Value = dioxus::document::eval(&script).await.map_err(err)?;
        if let Some(error) = output.get("error").and_then(|e| e.as_str()) {
            return Err(error.into());
        }
        serde_json::from_str(
            output
                .get("body")
                .and_then(|v| v.as_str())
                .ok_or("Empty sync response")?,
        )
        .map_err(err)
    }
}

/// Hydrate the durable IndexedDB cache before constructing application state.
pub async fn initialize_browser() -> Result<()> {
    #[cfg(target_arch = "wasm32")]
    {
        let value = dioxus::document::eval(include_str!("browser_store.js"))
            .await
            .map_err(err)?;
        if value.as_bool() != Some(true) {
            return Err("Could not initialize durable browser storage".into());
        }
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
struct BrowserStorage;
#[cfg(target_arch = "wasm32")]
impl BrowserStorage {
    fn entries(&self) -> Result<Vec<(String, String)>> {
        let json = js_sys::eval("JSON.stringify(window.rufrenLibrary.entries())")
            .map_err(|_| "Cannot read browser notebook")?
            .as_string()
            .ok_or("Invalid browser cache")?;
        serde_json::from_str(&json).map_err(err)
    }
    fn events_after(&self, revision: u64) -> Result<(u64, Vec<Event>)> {
        #[derive(serde::Deserialize)]
        struct Delta {
            revision: u64,
            entries: Vec<(String, String)>,
        }
        let json = js_sys::eval(&format!(
            "JSON.stringify(window.rufrenLibrary.eventsAfter({revision}))"
        ))
        .map_err(|_| "Cannot read browser event cache")?
        .as_string()
        .ok_or("Invalid browser cache")?;
        let delta: Delta = serde_json::from_str(&json).map_err(err)?;
        let events = delta
            .entries
            .into_iter()
            .map(|(key, json)| {
                let event: Event = serde_json::from_str(&json).map_err(err)?;
                if key != format!("rufren.v1.event.{}", event.id) {
                    return Err("Stored event ID does not match its key".into());
                }
                Ok(event)
            })
            .collect::<Result<_>>()?;
        Ok((delta.revision, events))
    }
    fn get_item(&self, key: &str) -> Result<Option<String>> {
        let expression = format!(
            "window.rufrenLibrary.get({})",
            serde_json::to_string(key).map_err(err)?
        );
        Ok(js_sys::eval(&expression)
            .map_err(|_| "Cannot read browser storage")?
            .as_string())
    }
    fn set_item(&self, key: &str, value: &str) -> Result<()> {
        self.write(key, Some(value))
    }
    fn remove_item(&self, key: &str) -> Result<()> {
        self.write(key, None)
    }
    fn write(&self, key: &str, value: Option<&str>) -> Result<()> {
        let expression = format!(
            "window.rufrenLibrary.write({}, {})",
            serde_json::to_string(key).map_err(err)?,
            serde_json::to_string(&value).map_err(err)?
        );
        js_sys::eval(&expression).map(|_| ()).map_err(|_| {
            "Cannot save browser notebook; export a backup and check available storage".into()
        })
    }
}

pub fn now_ms() -> i64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as i64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, serde::Deserialize, PartialEq)]
pub struct BrowserWorkspace {
    pub id: String,
    pub last_used_ms: i64,
    pub last_used_label: String,
    pub active: bool,
    pub current: bool,
    pub practice_response: Option<String>,
    pub passage_title: Option<String>,
    pub paused_count: usize,
    pub passage_count: usize,
}
/// Inventory only; switching obtains ownership again rather than trusting this snapshot.
#[cfg(target_arch = "wasm32")]
pub async fn browser_workspaces() -> Result<Vec<BrowserWorkspace>> {
    #[cfg(target_arch = "wasm32")]
    {
        let result=dioxus::document::eval("try { return {windows:await window.rufrenLibrary.workspaces()}; } catch(error) {return {error:error.message};}").await.map_err(err)?;
        if let Some(error) = result.get("error").and_then(serde_json::Value::as_str) {
            return Err(error.into());
        }
        serde_json::from_value(
            result
                .get("windows")
                .cloned()
                .ok_or("Saved window list is unavailable")?,
        )
        .map_err(err)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(Vec::new())
    }
}
#[cfg(target_arch = "wasm32")]
pub async fn switch_browser_workspace(id: &str) -> Result<()> {
    #[cfg(target_arch = "wasm32")]
    {
        let id = serde_json::to_string(id).map_err(err)?;
        let script = format!(
            "try {{ return {{switched:await window.rufrenLibrary.switchWorkspace({id})}}; }} catch(error) {{return {{error:error.message}};}}"
        );
        let result = dioxus::document::eval(&script).await.map_err(err)?;
        if let Some(error) = result.get("error").and_then(serde_json::Value::as_str) {
            return Err(error.into());
        }
        if result.get("switched").and_then(serde_json::Value::as_bool) != Some(true) {
            return Err("Saved window could not be reopened".into());
        }
        web_sys::window()
            .ok_or("Browser window is unavailable")?
            .location()
            .reload()
            .map_err(|_| "Reload Rufren to finish reopening the saved window".into())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = id;
        Err("Saved browser windows are available in the browser app".into())
    }
}
