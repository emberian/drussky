use rufren_core::*;
use serde::{Serialize, de::DeserializeOwned};
use std::collections::BTreeSet;
use uuid::Uuid;
pub type Result<T> = std::result::Result<T, String>;
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

pub struct Repository {
    #[cfg(not(target_arch = "wasm32"))]
    store: std::cell::RefCell<rufren_store::Store>,
    #[cfg(target_arch = "wasm32")]
    storage: web_sys::Storage,
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
            Ok(Self {
                store: std::cell::RefCell::new(
                    rufren_store::Store::open(path.join("library.sqlite")).map_err(err)?,
                ),
            })
        }
        #[cfg(target_arch = "wasm32")]
        {
            let storage = web_sys::window()
                .ok_or("No browser window")?
                .local_storage()
                .map_err(|_| "Browser storage unavailable")?
                .ok_or("Browser storage is disabled")?;
            Ok(Self { storage })
        }
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
    pub fn library(&self) -> Result<Library> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.store.borrow().library().map_err(err)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let mut events = Vec::new();
            for i in 0..self
                .storage
                .length()
                .map_err(|_| "Cannot read browser storage")?
            {
                if let Some(k) = self
                    .storage
                    .key(i)
                    .map_err(|_| "Cannot enumerate browser storage")?
                {
                    if k.starts_with("rufren.v1.event.") {
                        let json = self
                            .storage
                            .get_item(&k)
                            .map_err(|_| "Cannot read event")?
                            .ok_or("An event disappeared while reading")?;
                        events.push(serde_json::from_str(&json).map_err(err)?);
                    }
                }
            }
            let mut lib = Library::default();
            lib.merge(&events).map_err(err)?;
            Ok(lib)
        }
    }
    pub fn disconnect(&self) -> Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.store.borrow_mut().disconnect().map_err(err)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let mut keys = Vec::new();
            for i in 0..self
                .storage
                .length()
                .map_err(|_| "Cannot read sync state")?
            {
                if let Some(k) = self.storage.key(i).map_err(|_| "Cannot read sync key")? {
                    if k.starts_with("rufren.v1.ack.") {
                        keys.push(k);
                    }
                }
            }
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
            // Each key is immutable. Partial batches can safely be retried after quota/error.
            for event in events {
                self.storage.set_item(&format!("rufren.v1.event.{}",event.id),&serde_json::to_string(event).map_err(err)?)
                    .map_err(|_|"Could not save all events. Storage may be full; retry after exporting a backup.")?;
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
            let mut ids = BTreeSet::new();
            for i in 0..self
                .storage
                .length()
                .map_err(|_| "Cannot read sync state")?
            {
                if let Some(k) = self.storage.key(i).map_err(|_| "Cannot read sync key")? {
                    if let Some(id) = k.strip_prefix(&prefix) {
                        ids.insert(Uuid::parse_str(id).map_err(err)?);
                    }
                }
            }
            Ok(ids)
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
    let builder = reqwest::Client::builder();
    #[cfg(not(target_arch = "wasm32"))]
    let builder = builder.timeout(std::time::Duration::from_secs(30));
    let client = builder.build().map_err(err)?;
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
            version: SCHEMA_VERSION,
            relay: binding.as_ref().map(|b| b.relay),
            after: binding.as_ref().map_or(0, |b| b.cursor),
            events: pending,
        };
        let response = client
            .post(format!("{base}/v1/sync"))
            .bearer_auth(&token)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Could not reach your sync library: {e}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "Sync returned {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }
        let response: SyncResponse = response.json().await.map_err(err)?;
        if response.version != SCHEMA_VERSION {
            return Err(Error::Version(response.version).to_string());
        }
        if binding.as_ref().is_some_and(|b| b.relay != response.relay) {
            return Err(Error::RelayChanged.to_string());
        }
        if response.cursor < request.after
            || response.events.len() > SYNC_BATCH
            || (response.has_more && response.cursor == request.after)
        {
            return Err("Invalid sync cursor or page".into());
        }
        let sent: BTreeSet<_> = request.events.iter().map(|e| e.id).collect();
        if response.accepted.iter().copied().collect::<BTreeSet<_>>() != sent {
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
