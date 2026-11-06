//! Immutable, bounded, SHA256-verified blob storage. Metadata stays in the SQLite snapshot.
use rufren_core::{MAX_AUDIO_BYTES, valid_audio_hash};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

#[derive(Debug)]
pub enum AssetError {
    Invalid,
    TooLarge,
    Quota,
    Missing,
    Corrupt,
    Io(std::io::Error),
}
impl From<std::io::Error> for AssetError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

pub struct AssetStore {
    root: PathBuf,
    quota: u64,
}
impl AssetStore {
    pub fn open(root: impl AsRef<Path>, quota: u64) -> Result<Self, AssetError> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Self { root, quota })
    }
    fn path(&self, hash: &str) -> Result<PathBuf, AssetError> {
        if !valid_audio_hash(hash) {
            return Err(AssetError::Invalid);
        }
        Ok(self.root.join(format!("{hash}.blob")))
    }
    fn verified_file(&self, hash: &str) -> Result<(File, u64), AssetError> {
        let path = self.path(hash)?;
        let metadata = match fs::symlink_metadata(&path) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(AssetError::Missing);
            }
            Err(error) => return Err(error.into()),
        };
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(AssetError::Corrupt);
        }
        if metadata.len() == 0 || metadata.len() > MAX_AUDIO_BYTES as u64 {
            return Err(AssetError::TooLarge);
        }
        Ok((File::open(path)?, metadata.len()))
    }
    pub fn verify(&self, hash: &str) -> Result<u64, AssetError> {
        let (mut file, size) = self.verified_file(hash)?;
        let mut digest = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            digest.update(&buffer[..count]);
        }
        if format!("{:x}", digest.finalize()) != hash {
            return Err(AssetError::Corrupt);
        }
        Ok(size)
    }
    pub fn read(&self, hash: &str) -> Result<Vec<u8>, AssetError> {
        let (file, size) = self.verified_file(hash)?;
        let mut bytes = Vec::with_capacity(size as usize);
        file.take(MAX_AUDIO_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() != size as usize || format!("{:x}", Sha256::digest(&bytes)) != hash {
            return Err(AssetError::Corrupt);
        }
        Ok(bytes)
    }
    pub fn put(&self, hash: &str, bytes: &[u8]) -> Result<(), AssetError> {
        let destination = self.path(hash)?;
        if bytes.is_empty() || bytes.len() > MAX_AUDIO_BYTES {
            return Err(AssetError::TooLarge);
        }
        if format!("{:x}", Sha256::digest(bytes)) != hash {
            return Err(AssetError::Corrupt);
        }
        match self.read(hash) {
            Ok(existing) => {
                return if existing == bytes {
                    Ok(())
                } else {
                    Err(AssetError::Corrupt)
                };
            }
            Err(AssetError::Missing) => {}
            Err(error) => return Err(error),
        }
        let mut used = 0u64;
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_file() {
                used = used.saturating_add(metadata.len());
            }
        }
        if used.saturating_add(bytes.len() as u64) > self.quota {
            return Err(AssetError::Quota);
        }
        let temporary = self.root.join(format!("partial-{}", Uuid::new_v4()));
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let result = (|| {
            let mut file = options.open(&temporary)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            // Atomic create without replacing another writer's content-addressed file.
            match fs::hard_link(&temporary, &destination) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if self.read(hash)? != bytes {
                        return Err(AssetError::Corrupt);
                    }
                }
                Err(error) => return Err(error.into()),
            }
            File::open(&self.root)?.sync_all()?;
            Ok(())
        })();
        // Only our own exclusive temporary file is ever removed.
        let _ = fs::remove_file(temporary);
        result
    }
}
