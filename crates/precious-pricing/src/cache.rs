use crate::types::PricingError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::debug;

#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry<T> {
    data: T,
    expires_at: i64,
}

pub struct FileCache {
    dir: PathBuf,
}

impl FileCache {
    pub fn new() -> Result<Self, PricingError> {
        let project_dirs = directories::ProjectDirs::from("io", "polymath", "precious")
            .ok_or_else(|| PricingError::Cache("failed to determine cache directory".into()))?;
        let dir = project_dirs.cache_dir().to_path_buf();
        std::fs::create_dir_all(&dir)
            .map_err(|e| PricingError::Cache(format!("failed to create cache dir: {e}")))?;
        Ok(Self { dir })
    }

    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        let path = self.dir.join(format!("{key}.json"));
        let data = std::fs::read_to_string(&path).ok()?;
        let entry: CacheEntry<T> = serde_json::from_str(&data).ok()?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs() as i64;

        if now > entry.expires_at {
            debug!("cache expired for key: {key}");
            let _ = std::fs::remove_file(&path);
            return None;
        }

        Some(entry.data)
    }

    pub fn set<T: Serialize>(
        &self,
        key: &str,
        data: &T,
        ttl_seconds: u64,
    ) -> Result<(), PricingError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| PricingError::Cache(e.to_string()))?
            .as_secs() as i64;

        let entry = CacheEntry {
            data,
            expires_at: now + ttl_seconds as i64,
        };

        let path = self.dir.join(format!("{key}.json"));
        let json = serde_json::to_string(&entry)
            .map_err(|e| PricingError::Cache(format!("serialization failed: {e}")))?;

        std::fs::write(&path, json)
            .map_err(|e| PricingError::Cache(format!("write failed: {e}")))?;

        Ok(())
    }
}
