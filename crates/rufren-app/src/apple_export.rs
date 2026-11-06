//! Native Apple export uses the existing WK download engine and an explicit
//! platform save/share interaction. No WebKit/Dioxus framework fork is needed.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(crate) const JAVASCRIPT: &str = include_str!("file_export.js");
pub(crate) const MAX_EXPORT_BYTES: u64 = 150 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExportRequest {
    pub request_id: Uuid,
    pub blob_url: String,
    pub name: String,
    pub mime_type: String,
    pub byte_length: u64,
    pub sha256: String,
}
impl ExportRequest {
    fn validate(&self) -> Result<(), String> {
        let leaf = self.blob_url.rsplit('/').next().unwrap_or_default();
        if self.request_id.is_nil()
            || !self.blob_url.starts_with("blob:")
            || self.blob_url.len() > 2048
            || self
                .blob_url
                .chars()
                .any(|c| c.is_control() || c.is_whitespace())
            || Uuid::parse_str(leaf).is_err()
            || self.name.is_empty()
            || self.name.chars().count() > 180
            || self.name.starts_with('.')
            || self
                .name
                .chars()
                .any(|c| c.is_control() || "/\\<>:\"|?*".contains(c))
            || !(1..=MAX_EXPORT_BYTES).contains(&self.byte_length)
            || self.sha256.len() != 64
            || !self
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || self.mime_type.len() > 128
            || !(self.mime_type == "application/json" || self.mime_type.starts_with("audio/"))
            || self.mime_type.chars().any(char::is_control)
        {
            return Err("This export needs a registered audio/backup Blob and a plain filename. Your originals are kept.".into());
        }
        Ok(())
    }
}

pub(crate) const fn supported() -> bool {
    cfg!(all(
        any(target_os = "macos", target_os = "ios"),
        any(feature = "desktop", feature = "mobile")
    ))
}

#[derive(Serialize)]
struct ExportResult {
    request_id: Uuid,
    status: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
}

#[cfg(all(
    any(target_os = "macos", target_os = "ios"),
    any(feature = "desktop", feature = "mobile")
))]
#[path = "apple_export_native.rs"]
mod native;

#[cfg(all(
    any(target_os = "macos", target_os = "ios"),
    any(feature = "desktop", feature = "mobile")
))]
pub(crate) use native::start_export;

#[cfg(not(all(
    any(target_os = "macos", target_os = "ios"),
    any(feature = "desktop", feature = "mobile")
)))]
pub(crate) fn start_export(request: ExportRequest) {
    let result = ExportResult {
        request_id: request.request_id,
        status: "failed",
        message: "Native save/share is unavailable in this app. Your original is kept.".into(),
        path: None,
    };
    let value = serde_json::to_string(&result).unwrap_or_default();
    let _ = dioxus::document::eval(&format!("window.rufrenFileExports?.complete({value});"));
}

#[cfg(not(target_arch = "wasm32"))]
fn verify_file(path: &std::path::Path, length: u64, expected_hash: &str) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("Could not read the completed export: {e}"))?;
    if file.metadata().map_err(|e| e.to_string())?.len() != length || length > MAX_EXPORT_BYTES {
        return Err("The exported file has an unexpected length. Your original is kept.".into());
    }
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut read = 0u64;
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        read += count as u64;
        if read > length {
            return Err("The export changed while it was checked.".into());
        }
        hash.update(&buffer[..count]);
    }
    if read != length || format!("{:x}", hash.finalize()) != expected_hash {
        return Err(
            "The completed export failed its integrity check. Your original is kept.".into(),
        );
    }
    Ok(())
}

/// The chooser authorizes `destination`; renderer requests never contain it.
/// A failed copy leaves an existing destination intact. Scratch space is owned
/// by the app, not an arbitrary sibling of a sandbox-authorized single file.
#[cfg(not(target_arch = "wasm32"))]
fn commit_file(
    source: &std::path::Path,
    destination: &std::path::Path,
    request: &ExportRequest,
) -> Result<(), String> {
    let scratch = source
        .parent()
        .ok_or("The verified export has no temporary folder.")?;
    commit_file_in(source, destination, request, scratch)
}

#[cfg(not(target_arch = "wasm32"))]
fn commit_file_in(
    source: &std::path::Path,
    destination: &std::path::Path,
    request: &ExportRequest,
    scratch: &std::path::Path,
) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    use std::io::{Read, Write};
    if !destination.is_absolute() {
        return Err("Choose an absolute export destination.".into());
    }
    let temporary = scratch.join(format!(".rufren-export-{}.tmp", request.request_id));
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| format!("Could not create the selected export: {e}"))?;
    let result = (|| {
        let mut input = std::fs::File::open(source).map_err(|e| e.to_string())?;
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        let mut size = 0u64;
        loop {
            let count = input.read(&mut buffer).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            size += count as u64;
            if size > request.byte_length || size > MAX_EXPORT_BYTES {
                return Err("The completed export exceeds its declared size.".into());
            }
            hash.update(&buffer[..count]);
            output
                .write_all(&buffer[..count])
                .map_err(|e| e.to_string())?;
        }
        if size != request.byte_length || format!("{:x}", hash.finalize()) != request.sha256 {
            return Err("The completed export failed its integrity check. The selected file was not replaced.".into());
        }
        output.sync_all().map_err(|e| e.to_string())?;
        drop(output);
        std::fs::rename(&temporary, destination)
            .map_err(|e| format!("Could not save the selected export: {e}"))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> ExportRequest {
        ExportRequest {
            request_id: Uuid::new_v4(),
            blob_url: format!("blob:dioxus://index.html/{}", Uuid::new_v4()),
            name: "rufren-audio-2026-11-06.json".into(),
            mime_type: "application/json".into(),
            byte_length: 100,
            sha256: "a".repeat(64),
        }
    }
    #[test]
    fn exports_reject_paths_remote_requests_bad_hashes_and_oversize() {
        let valid = request();
        valid.validate().unwrap();
        for name in [
            "../backup.json",
            "/tmp/backup.json",
            "C:\\backup.json",
            ".hidden",
            "bad\nname.json",
        ] {
            let mut value = valid.clone();
            value.name = name.into();
            assert!(value.validate().is_err());
        }
        for url in [
            "https://example.com/backup",
            "file:///tmp/backup",
            "blob:dioxus://index.html/not-a-token",
        ] {
            let mut value = valid.clone();
            value.blob_url = url.into();
            assert!(value.validate().is_err());
        }
        let mut value = valid.clone();
        value.byte_length = MAX_EXPORT_BYTES + 1;
        assert!(value.validate().is_err());
        let mut value = valid;
        value.sha256 = "a".repeat(63);
        assert!(value.validate().is_err());
    }
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn failed_integrity_preserves_destination_and_success_commits_exact_bytes() {
        use sha2::{Digest, Sha256};
        let directory = std::env::temp_dir().join(format!("rufren-export-test-{}", Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let source = directory.join("source");
        let destination = directory.join("chosen.json");
        std::fs::write(&source, b"exact original bytes").unwrap();
        std::fs::write(&destination, b"existing destination").unwrap();
        let mut value = request();
        value.byte_length = 20;
        assert!(commit_file(&source, &destination, &value).is_err());
        assert_eq!(
            std::fs::read(&destination).unwrap(),
            b"existing destination"
        );
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 2);
        value.sha256 = format!("{:x}", Sha256::digest(b"exact original bytes"));
        verify_file(&source, value.byte_length, &value.sha256).unwrap();
        commit_file(&source, &destination, &value).unwrap();
        assert_eq!(
            std::fs::read(&destination).unwrap(),
            b"exact original bytes"
        );
        std::fs::remove_file(source).unwrap();
        std::fs::remove_file(destination).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
