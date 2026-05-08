//! Filesystem abstaction for platform-agnostic file I/O.

use std::path::Path;

use super::runtime::RuntimeError;

/// Filesystem metadata for a single entry.
#[derive(Debug, Clone)]
pub struct FileStat {
    pub size: u64,
    pub is_file: bool,
    pub is_dir: bool,
    #[allow(dead_code)]
    pub is_symlink: bool,
}

/// A single directory entry.
#[derive(Debug, Clone)]
pub struct FileDirEntry {
    pub name: String,
    pub stat: FileStat,
}

/// Abstractions filesystem I/O so tools are not coupled to `tokio::fs`.
///
/// Implementations must be `Send + Sync` (shared via `Arc`).
/// All paths passed to methods are absolute (tools resolve relative paths before calling).
#[async_trait::async_trait(?Send)]
pub trait FileSystemAdapter: Send + Sync {
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>, RuntimeError>;
    async fn write_file(&self, path: &Path, content: &[u8]) -> Result<(), RuntimeError>;
    async fn stat(&self, path: &Path) -> Result<Option<FileStat>, RuntimeError>;
    async fn list_dir(&self, path: &Path) -> Result<Vec<FileDirEntry>, RuntimeError>;
    async fn create_dir_all(&self, path: &Path) -> Result<(), RuntimeError>;
}


