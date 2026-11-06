//! Native SQLite event log. Used by the local app and the optional single-library relay.
use rufren_core::*;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Serialize, de::DeserializeOwned};
use std::{path::Path, time::Duration};
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub struct Store {
    connection: Connection,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        let version: u32 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > SCHEMA_VERSION {
            return Err(Error::Version(version).into());
        }
        connection.execute_batch("BEGIN IMMEDIATE;
            CREATE TABLE IF NOT EXISTS events (seq INTEGER PRIMARY KEY AUTOINCREMENT, id TEXT UNIQUE NOT NULL, json TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS metadata (key TEXT PRIMARY KEY, json TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS acknowledgements (relay TEXT NOT NULL, event TEXT NOT NULL, PRIMARY KEY(relay,event));
            PRAGMA user_version=1; COMMIT;")?;
        Ok(Self { connection })
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
    pub fn library(&self) -> Result<Library> {
        read_library(&self.connection)
    }
    /// Forget delivery state, keeping all facts. Reconnect must be able to reseed a restored relay.
    pub fn disconnect(&mut self) -> Result<()> {
        let tx = self.connection.transaction()?;
        tx.execute("DELETE FROM acknowledgements", [])?;
        tx.execute("DELETE FROM metadata WHERE key='sync'", [])?;
        tx.commit()?;
        Ok(())
    }
    pub fn append(&mut self, events: &[Event]) -> Result<()> {
        let tx = self.connection.transaction()?;
        append(&tx, events)?;
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
        if response.version != SCHEMA_VERSION {
            return Err(Error::Version(response.version).into());
        }
        if binding.relay != response.relay {
            return Err(Error::RelayChanged.into());
        }
        let tx = self.connection.transaction()?;
        append(&tx, &response.events)?;
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
        if request.version != SCHEMA_VERSION {
            return Err(Error::Version(request.version).into());
        }
        if request.events.len() > SYNC_BATCH {
            return Err(Error::Invalid("sync batch too large".into()).into());
        }
        let relay = self.identity("relay")?;
        if request.relay.is_some_and(|id| id != relay) {
            return Err(Error::RelayChanged.into());
        }
        let tx = self.connection.transaction()?;
        let before: u64 =
            tx.query_row("SELECT COALESCE(MAX(seq),0) FROM events", [], |r| r.get(0))?;
        if request.after > before || (request.relay.is_none() && request.after != 0) {
            return Err(Error::Invalid("sync cursor is ahead of this library".into()).into());
        }
        append(&tx, &request.events)?;
        let (events, cursor) = {
            let mut stmt =
                tx.prepare("SELECT seq,json FROM events WHERE seq>?1 ORDER BY seq LIMIT ?2")?;
            let rows = stmt.query_map(params![request.after, SYNC_BATCH as u64], |r| {
                Ok((r.get::<_, u64>(0)?, r.get::<_, String>(1)?))
            })?;
            let mut events = Vec::new();
            let mut cursor = request.after;
            for row in rows {
                let (seq, json) = row?;
                cursor = seq;
                events.push(serde_json::from_str(&json)?);
            }
            (events, cursor)
        };
        let newest: u64 =
            tx.query_row("SELECT COALESCE(MAX(seq),0) FROM events", [], |r| r.get(0))?;
        tx.commit()?;
        Ok(SyncResponse {
            version: SCHEMA_VERSION,
            relay,
            accepted: request.events.iter().map(|e| e.id).collect(),
            events,
            cursor,
            has_more: cursor < newest,
        })
    }
}

fn read_library(connection: &Connection) -> Result<Library> {
    let mut stmt = connection.prepare("SELECT json FROM events ORDER BY seq")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    let events: Vec<Event> = rows
        .map(|row| Ok(serde_json::from_str(&row?)?))
        .collect::<Result<_>>()?;
    let mut library = Library::default();
    library.merge(&events)?;
    Ok(library)
}
fn append(connection: &Connection, events: &[Event]) -> Result<()> {
    let mut library = read_library(connection)?;
    library.merge(events)?;
    for event in events {
        connection.execute(
            "INSERT OR IGNORE INTO events(id,json) VALUES (?1,?2)",
            params![event.id.to_string(), serde_json::to_string(event)?],
        )?;
    }
    Ok(())
}
