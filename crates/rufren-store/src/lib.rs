//! Native SQLite event log. Used by the local app and the optional single-library relay.
use rufren_core::*;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Serialize, de::DeserializeOwned};
use std::{cell::RefCell, path::Path, time::Duration};
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
/// Physical SQLite layout; independent of event and sync protocol versions.
pub const DB_SCHEMA_VERSION: u32 = 1;
pub const SYNC_PAGE_BYTES: usize = 1_048_576;
pub struct Store {
    connection: Connection,
    projection: RefCell<(u64, Library)>,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        let version: u32 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > DB_SCHEMA_VERSION {
            return Err(Error::Version(version).into());
        }
        connection.execute_batch("BEGIN IMMEDIATE;
            CREATE TABLE IF NOT EXISTS events (seq INTEGER PRIMARY KEY AUTOINCREMENT, id TEXT UNIQUE NOT NULL, json TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS metadata (key TEXT PRIMARY KEY, json TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS acknowledgements (relay TEXT NOT NULL, event TEXT NOT NULL, PRIMARY KEY(relay,event));
            PRAGMA user_version=1; COMMIT;")?;
        let store = Self {
            connection,
            projection: RefCell::new((0, Library::default())),
        };
        store.library()?; // Reject incompatible/corrupt facts before opening a usable notebook.
        Ok(store)
    }
    /// Audio take registry shares the SQLite snapshot/backup boundary with event facts.
    pub fn merge_audio_record(
        &mut self,
        record: &AudioRecordMetadata,
    ) -> Result<AudioRecordMetadata> {
        record.validate()?;
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let key = format!("asset.record.{}", record.asset_id);
        let old: Option<String> = tx
            .query_row("SELECT json FROM metadata WHERE key=?1", [&key], |r| {
                r.get(0)
            })
            .optional()?;
        let merged = match old {
            Some(json) => serde_json::from_str::<AudioRecordMetadata>(&json)?.merge(record)?,
            None => record.merge(record)?,
        };
        tx.execute("INSERT INTO metadata(key,json) VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET json=excluded.json", params![key,serde_json::to_string(&merged)?])?;
        tx.commit()?;
        Ok(merged)
    }
    pub fn audio_records(&self, after: &str) -> Result<AssetPage> {
        if after.len() > 256 || after.chars().any(char::is_control) {
            return Err(Error::Invalid("invalid audio page cursor".into()).into());
        }
        let mut stmt = self.connection.prepare("SELECT key,json FROM metadata WHERE key>?1 AND key>= 'asset.record.' AND key<'asset.record/' ORDER BY key LIMIT ?2")?;
        let rows = stmt.query_map(
            params![format!("asset.record.{after}"), ASSET_PAGE_SIZE as u64 + 1],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )?;
        let mut records = Vec::new();
        let mut bytes = 0;
        let mut has_more = false;
        for row in rows {
            let (_, json) = row?;
            if records.len() == ASSET_PAGE_SIZE
                || (!records.is_empty() && bytes + json.len() > ASSET_PAGE_BYTES)
            {
                has_more = true;
                break;
            }
            let record: AudioRecordMetadata = serde_json::from_str(&json)?;
            record.validate()?;
            bytes += json.len();
            records.push(record);
        }
        let next_after = if has_more {
            records.last().map(|r| r.asset_id.clone())
        } else {
            None
        };
        Ok(AssetPage {
            version: ASSET_TRANSPORT_VERSION,
            records,
            next_after,
        })
    }
    pub fn identity(&self, key: &str) -> Result<Uuid> {
        // Insert-if-absent also works when two app windows open the same store.
        self.connection.execute(
            "INSERT OR IGNORE INTO metadata VALUES (?1,?2)",
            params![key, serde_json::to_string(&Uuid::new_v4())?],
        )?;
        Ok(self.get(key)?.ok_or("missing stored identity")?)
    }
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let value: Option<String> = self
            .connection
            .query_row("SELECT json FROM metadata WHERE key=?1", [key], |r| {
                r.get(0)
            })
            .optional()?;
        Ok(value
            .map(|v| serde_json::from_str::<Option<T>>(&v))
            .transpose()?
            .flatten())
    }
    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        self.connection.execute(
            "INSERT INTO metadata VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET json=excluded.json",
            params![key, serde_json::to_string(value)?],
        )?;
        Ok(())
    }
    /// Enumerate only matching local metadata, without unrelated drawing payloads.
    pub fn metadata_with_prefix(&self, prefix: &str) -> Result<Vec<(String, serde_json::Value)>> {
        self.metadata_selection(prefix, None)
    }
    /// Exact scope filtering happens in SQLite, before drawing JSON reaches the app.
    pub fn metadata_with_scope(
        &self,
        prefix: &str,
        scope: &str,
    ) -> Result<Vec<(String, serde_json::Value)>> {
        self.metadata_selection(prefix, Some(scope))
    }
    fn metadata_selection(
        &self,
        prefix: &str,
        scope: Option<&str>,
    ) -> Result<Vec<(String, serde_json::Value)>> {
        let mut stmt = self.connection.prepare(
            "SELECT key,json FROM metadata WHERE key>=?1 AND (?2 IS NULL OR key<?2)
             AND (?3 IS NULL OR (json_valid(json) AND json_extract(json,'$.scope_id')=?3)) ORDER BY key",
        )?;
        let rows = stmt.query_map(params![prefix, prefix_end(prefix), scope], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.map(|row| {
            let (key, raw) = row?;
            Ok((key, serde_json::from_str(&raw)?))
        })
        .collect()
    }
    pub fn library(&self) -> Result<Library> {
        read_library(&self.connection, &self.projection)
    }
    /// Forget delivery state, keeping all facts. Reconnect must be able to reseed a restored relay.
    pub fn disconnect(&mut self) -> Result<()> {
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute("DELETE FROM acknowledgements", [])?;
        tx.execute("DELETE FROM metadata WHERE key='sync'", [])?;
        tx.commit()?;
        Ok(())
    }
    pub fn append(&mut self, events: &[Event]) -> Result<()> {
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        append(&tx, &self.projection, events)?;
        tx.commit()?;
        Ok(())
    }
    pub fn acknowledged(&self, relay: Uuid) -> Result<std::collections::BTreeSet<Uuid>> {
        let mut stmt = self
            .connection
            .prepare("SELECT event FROM acknowledgements WHERE relay=?1")?;
        let rows = stmt.query_map([relay.to_string()], |r| r.get::<_, String>(0))?;
        rows.map(|r| Ok(Uuid::parse_str(&r?)?)).collect()
    }
    /// Records received facts, acknowledgement, and cursor in one transaction.
    pub fn accept(&mut self, binding: &SyncBinding, response: &SyncResponse) -> Result<()> {
        if response.version != SYNC_VERSION {
            return Err(Error::Version(response.version).into());
        }
        if binding.relay != response.relay {
            return Err(Error::RelayChanged.into());
        }
        if response.relay.is_nil()
            || binding.cursor != response.cursor
            || response.events.len() > SYNC_BATCH
            || response.accepted.len() > SYNC_BATCH
        {
            return Err(Error::Invalid("invalid sync page or binding".into()).into());
        }
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let previous: Option<SyncBinding> = tx
            .query_row("SELECT json FROM metadata WHERE key='sync'", [], |r| {
                r.get::<_, String>(0)
            })
            .optional()?
            .map(|s| serde_json::from_str(&s))
            .transpose()?
            .flatten();
        let after = previous.as_ref().map_or(0, |b| b.cursor);
        if previous
            .as_ref()
            .is_some_and(|b| b.relay != binding.relay || b.endpoint != binding.endpoint)
            || response.cursor < after
            || (response.events.is_empty() && response.cursor != after)
            || (response.has_more && response.events.is_empty())
            || (response.cursor > after && response.cursor - after < response.events.len() as u64)
        {
            return Err(Error::Invalid("invalid sync cursor or changed binding".into()).into());
        }
        let unique: std::collections::BTreeSet<_> = response.events.iter().map(|e| e.id).collect();
        let accepted: std::collections::BTreeSet<_> = response.accepted.iter().copied().collect();
        if unique.len() != response.events.len() || accepted.len() != response.accepted.len() {
            return Err(Error::Invalid("duplicate sync IDs".into()).into());
        }
        if response.cursor == after {
            for event in &response.events {
                let exists: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM events WHERE id=?1)",
                    [event.id.to_string()],
                    |r| r.get(0),
                )?;
                if !exists {
                    return Err(
                        Error::Invalid("nonadvancing page contains a new event".into()).into(),
                    );
                }
            }
        }
        append(&tx, &self.projection, &response.events)?;
        for id in &response.accepted {
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM events WHERE id=?1)",
                [id.to_string()],
                |r| r.get(0),
            )?;
            if !exists {
                return Err(
                    Error::Invalid("acknowledgement references an unsaved event".into()).into(),
                );
            }
        }
        for id in &response.accepted {
            tx.execute(
                "INSERT OR IGNORE INTO acknowledgements VALUES (?1,?2)",
                params![binding.relay.to_string(), id.to_string()],
            )?;
        }
        tx.execute("INSERT INTO metadata VALUES ('sync',?1) ON CONFLICT(key) DO UPDATE SET json=excluded.json",[serde_json::to_string(binding)?])?;
        tx.commit()?;
        Ok(())
    }
    pub fn exchange(&mut self, request: &SyncRequest) -> Result<SyncResponse> {
        if request.version != SYNC_VERSION {
            return Err(Error::Version(request.version).into());
        }
        if request.events.len() > SYNC_BATCH {
            return Err(Error::Invalid("sync batch too large".into()).into());
        }
        let relay = self.identity("relay")?;
        if request.relay.is_some_and(|id| id != relay) {
            return Err(Error::RelayChanged.into());
        }
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let before: u64 =
            tx.query_row("SELECT COALESCE(MAX(seq),0) FROM events", [], |r| r.get(0))?;
        if request.after > before || (request.relay.is_none() && request.after != 0) {
            return Err(Error::Invalid("sync cursor is ahead of this library".into()).into());
        }
        append(&tx, &self.projection, &request.events)?;
        let (events, cursor) = {
            let mut stmt =
                tx.prepare("SELECT seq,json FROM events WHERE seq>?1 ORDER BY seq LIMIT ?2")?;
            let rows = stmt.query_map(params![request.after, SYNC_BATCH as u64], |r| {
                Ok((r.get::<_, u64>(0)?, r.get::<_, String>(1)?))
            })?;
            let mut events = Vec::new();
            let mut cursor = request.after;
            let mut bytes = 0;
            for row in rows {
                let (seq, json) = row?;
                if !events.is_empty() && bytes + json.len() > SYNC_PAGE_BYTES {
                    break;
                }
                if json.len() > SYNC_PAGE_BYTES {
                    return Err(Error::Invalid("event exceeds sync page budget".into()).into());
                }
                bytes += json.len();
                cursor = seq;
                events.push(serde_json::from_str(&json)?);
            }
            (events, cursor)
        };
        let newest: u64 =
            tx.query_row("SELECT COALESCE(MAX(seq),0) FROM events", [], |r| r.get(0))?;
        tx.commit()?;
        Ok(SyncResponse {
            version: SYNC_VERSION,
            relay,
            accepted: request
                .events
                .iter()
                .map(|e| e.id)
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect(),
            events,
            cursor,
            has_more: cursor < newest,
        })
    }
}

/// Exclusive binary-text upper bound for a literal prefix (not a LIKE pattern).
fn prefix_end(prefix: &str) -> Option<String> {
    for (offset, ch) in prefix.char_indices().rev() {
        let mut next = u32::from(ch) + 1;
        if next == 0xD800 {
            next = 0xE000;
        }
        if let Some(next) = char::from_u32(next) {
            let mut bound = prefix[..offset].to_owned();
            bound.push(next);
            return Some(bound);
        }
    }
    None
}

fn read_library(connection: &Connection, cache: &RefCell<(u64, Library)>) -> Result<Library> {
    let newest: u64 =
        connection.query_row("SELECT COALESCE(MAX(seq),0) FROM events", [], |r| r.get(0))?;
    let mut cached = cache.borrow_mut();
    if newest < cached.0 {
        *cached = (0, Library::default());
    }
    if newest != cached.0 {
        let mut stmt = connection.prepare("SELECT json FROM events WHERE seq>?1 ORDER BY seq")?;
        let rows = stmt.query_map([cached.0], |r| r.get::<_, String>(0))?;
        let events: Vec<Event> = rows
            .map(|row| Ok(serde_json::from_str(&row?)?))
            .collect::<Result<_>>()?;
        cached.1.merge(&events)?;
        cached.0 = newest;
    }
    Ok(cached.1.clone())
}
fn append(
    connection: &Connection,
    cache: &RefCell<(u64, Library)>,
    events: &[Event],
) -> Result<()> {
    let mut library = read_library(connection, cache)?;
    library.merge(events)?;
    for event in events {
        if serde_json::to_vec(event)?.len() > SYNC_PAGE_BYTES {
            return Err(Error::Invalid("event exceeds sync page budget".into()).into());
        }
        // Avoid INSERT OR IGNORE consuming AUTOINCREMENT sequence numbers on retries.
        connection.execute(
            "INSERT INTO events(id,json) SELECT ?1,?2 WHERE NOT EXISTS (SELECT 1 FROM events WHERE id=?1)",
            params![event.id.to_string(), serde_json::to_string(event)?],
        )?;
    }
    Ok(())
}
