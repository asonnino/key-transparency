pub mod akd_storage;

/// Convenient name for rocksdb's error.
pub type StoreError = rocksdb::Error;
type StoreResult<T> = Result<T, StoreError>;

/// Wrapper around rocksdb.
pub struct Storage(rocksdb::DB);

impl Storage {
    /// Create a new persistent storage.
    pub fn new(path: &str) -> StoreResult<Self> {
        let db = rocksdb::DB::open_default(path)?;
        Ok(Self(db))
    }

    /// Read a value from storage.
    pub fn read(&self, key: &[u8]) -> StoreResult<Option<Vec<u8>>> {
        self.0.get(&key)
    }

    /// Write a value to storage.
    pub fn write(&self, key: &[u8], value: &[u8]) -> StoreResult<()> {
        self.0.put(key, value)
    }
}
