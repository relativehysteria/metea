//! Interface to the application's permanent filesystem storage.

use std::path::{Path, PathBuf};

/// Interface to the application's permanent storage.
pub struct Storage {
    /// Root path to the permanent storage.
    root: PathBuf,
}

impl Storage {
    /// Create a new interface to the storage, using `root` as the root path to
    /// the application's permanent storage.
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
        }
    }

    /// Write to `file` atomically.
    pub fn write_atomic(
        &self,
        file: &Path,
        data: &[u8]
    ) -> std::io::Result<()> {
        let path = self.root.join(file);
        let tmp = path.with_added_extension("tmp");

        std::fs::write(&tmp, data)?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }

    /// Get the path to the file that stores places retrieved from the geocoding
    /// API that are saved by the user.
    pub fn places(&self) -> PathBuf {
        self.root.join("places.json")
    }
}
