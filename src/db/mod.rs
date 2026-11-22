//! redb storage backend. Kept as a stable facade for future services.
#![forbid(unsafe_code)]

pub use v2rayq_storage::{Storage, StorageResult};

pub fn open(path: &str) -> StorageResult<Storage> {
    Storage::open(path)
}
