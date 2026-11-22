#![forbid(unsafe_code)]

//! ACID, platform-independent local persistence. No SQL engine or network daemon.
//! Key prefixes are namespaces, not untrusted file paths.

use std::{
    error::Error,
    fs::{self, OpenOptions},
    path::Path,
    time::Duration,
};

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Serialize, de::DeserializeOwned};

pub type StorageResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

const RECORDS: TableDefinition<&str, &[u8]> = TableDefinition::new("records_v1");

/// A file-backed store. Only the desktop/mobile clients open their own private store.
/// Server storage uses a different file; client and server never share a data file.
pub struct Storage {
    database: Database,
}

impl Storage {
    pub fn open(path: impl AsRef<Path>) -> StorageResult<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
        }
        drop(file);
        let mut retries = 0;
        let database = loop {
            match Database::create(path) {
                Ok(database) => break database,
                Err(redb::DatabaseError::DatabaseAlreadyOpen) if retries < 25 => {
                    retries += 1;
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(error) => return Err(error.into()),
            }
        };
        let txn = database.begin_write()?;
        {
            let _table = txn.open_table(RECORDS)?;
        }
        txn.commit()?;
        Ok(Self { database })
    }

    pub fn get<T: DeserializeOwned>(&self, key: &str) -> StorageResult<Option<T>> {
        let txn = self.database.begin_read()?;
        let table = txn.open_table(RECORDS)?;
        let value = table.get(key)?;
        match value {
            Some(bytes) => Ok(Some(serde_json::from_slice(bytes.value())?)),
            None => Ok(None),
        }
    }

    pub fn put<T: Serialize>(&self, key: &str, value: &T) -> StorageResult<()> {
        let serialized = serde_json::to_vec(value)?;
        let txn = self.database.begin_write()?;
        {
            let mut table = txn.open_table(RECORDS)?;
            table.insert(key, serialized.as_slice())?;
        }
        txn.commit()?;
        Ok(())
    }

    pub fn update<T: Serialize + DeserializeOwned, R>(
        &self,
        key: &str,
        edit: impl FnOnce(Option<T>) -> StorageResult<(T, R)>,
    ) -> StorageResult<R> {
        let txn = self.database.begin_write()?;
        let result = {
            let mut table = txn.open_table(RECORDS)?;
            let previous = table
                .get(key)?
                .map(|bytes| serde_json::from_slice(bytes.value()))
                .transpose()?;
            let (value, result) = edit(previous)?;
            let serialized = serde_json::to_vec(&value)?;
            table.insert(key, serialized.as_slice())?;
            result
        };
        txn.commit()?;
        Ok(result)
    }

    pub fn delete(&self, key: &str) -> StorageResult<bool> {
        let txn = self.database.begin_write()?;
        let removed = {
            let mut table = txn.open_table(RECORDS)?;
            table.remove(key)?.is_some()
        };
        txn.commit()?;
        Ok(removed)
    }

    pub fn list<T: DeserializeOwned>(&self, prefix: &str) -> StorageResult<Vec<(String, T)>> {
        let txn = self.database.begin_read()?;
        let table = txn.open_table(RECORDS)?;
        let mut result = Vec::new();
        for row in table.iter()? {
            let (key, bytes) = row?;
            if key.value().starts_with(prefix) {
                result.push((
                    key.value().to_owned(),
                    serde_json::from_slice(bytes.value())?,
                ));
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Fixture {
        label: String,
        count: usize,
    }

    #[test]
    fn persists_across_reopen_and_separates_namespaces() -> StorageResult<()> {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!(
            "v2rayq-storage-{}-{stamp}.redb",
            std::process::id()
        ));
        {
            let db = Storage::open(&path)?;
            db.put(
                "client/node/1",
                &Fixture {
                    label: "Node".into(),
                    count: 1,
                },
            )?;
            db.put(
                "server/node/1",
                &Fixture {
                    label: "Server".into(),
                    count: 2,
                },
            )?;
            assert_eq!(db.list::<Fixture>("client/")?.len(), 1);
        }
        {
            let db = Storage::open(&path)?;
            assert_eq!(
                db.get::<Fixture>("client/node/1")?.map(|v| v.label),
                Some("Node".into())
            );
            assert!(db.delete("client/node/1")?);
            assert!(!db.delete("client/node/1")?);
            assert_eq!(db.list::<Fixture>("server/")?.len(), 1);
        }
        fs::remove_file(&path)?;
        Ok(())
    }
}
