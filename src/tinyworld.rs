use std::path::Path;

use crate::storage_manager::{StorageManager, StorageManagerError};

pub struct TinyWorld {}

pub enum TWError {
    SMError(StorageManagerError),
}

impl TinyWorld {
    pub fn create() -> Self {
        Self {}
    }
    pub fn open(path: &str) -> Result<Self, TWError> {
        let (sm, header) = match StorageManager::open(Path::new(path)) {
            Ok(ok) => ok,
            Err(e) => return Err(TWError::SMError(e)),
        };

        Self {}
    }
}
