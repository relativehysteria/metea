//! Interface to the application's permanent filesystem storage.

use std::path::{Path, PathBuf};

/// Interface to the application's permanent storage.
pub struct Storage {
    /// Root path to the permanent storage.
    root: PathBuf,
}

impl Storage {
    /// Create a new interface to the storage.
    pub fn new(root: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&root)?;

        Ok(Self { root })
    }

    /// Write to `path` atomically.
    pub fn write_atomic(
        &self,
        path: &Path,
        data: &[u8],
    ) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let tmp = path.with_extension("tmp");

        std::fs::write(&tmp, data)?;
        std::fs::rename(&tmp, path)?;

        Ok(())
    }

    /// Get the path to the file that stores places retrieved
    /// from the geocoding API that are saved by the user.
    pub fn places(&self) -> PathBuf {
        self.root.join("places.json")
    }
}
