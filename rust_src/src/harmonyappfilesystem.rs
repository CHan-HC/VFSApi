use std::path::Path;

use crate::filesystem::{FileDirEntry, FileStat, FileSystemAdapter};
use super::runtime::RuntimeError;

/// HarmonyOS app file system — delegates to the local+cloud fusion layer.
pub struct HarmonyAppFilesystem;

#[async_trait::async_trait(?Send)]
impl FileSystemAdapter for HarmonyAppFilesystem {
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>, RuntimeError> {
        crate::hilog::log_info(&format!("HarmonyAppFilesystem::read_file path={:?}", path));
        crate::read::read_file_by_absolute_path(path).await
    }

    async fn write_file(&self, path: &Path, content: &[u8]) -> Result<(), RuntimeError> {
        crate::hilog::log_info(&format!("HarmonyAppFilesystem::write_file path={:?}, len={}", path, content.len()));
        crate::write::write_file_by_absolute_path(path, content).await
    }

    async fn stat(&self, path: &Path) -> Result<Option<FileStat>, RuntimeError> {
        crate::hilog::log_info(&format!("HarmonyAppFilesystem::stat path={:?}", path));
        crate::stat::stat_file_by_absolute_path(path).await
    }

    async fn list_dir(&self, path: &Path) -> Result<Vec<FileDirEntry>, RuntimeError> {
        crate::hilog::log_info(&format!("HarmonyAppFilesystem::list_dir path={:?}", path));
        crate::list::list_dir_by_absolute_path(path).await
    }

    async fn create_dir_all(&self, path: &Path) -> Result<(), RuntimeError> {
        crate::hilog::log_info(&format!("HarmonyAppFilesystem::create_dir_all path={:?}", path));
        crate::mkdir::create_dir_all_by_absolute_path(path).await
    }
}
