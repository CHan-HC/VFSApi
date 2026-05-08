use crate::error::{ErrorCode, VfsError, VfsResult};
use crate::rcp::HttpClient;
use crate::runtime::RuntimeError;
use crate::workspace::{get_workspace_sync, resolve_path};
use crate::{vfs_log_debug, vfs_log_error};
use std::fs;
use std::path::Path;

pub async fn write_file(path: &str, content: &[u8]) -> VfsResult<()> {
    vfs_log_debug!(">>> write_file START: path='{}', content_len={}", path, content.len());

    let full_path = match resolve_path(path).await {
        Ok(p) => p,
        Err(e) => {
            vfs_log_error!("write_file: resolve_path failed: {}", e.message);
            return Err(e);
        }
    };
    vfs_log_debug!("Resolved local path: {:?}", full_path);

    let local_exists = full_path.exists() && full_path.is_file();
    vfs_log_debug!("Local file exists: {}", local_exists);

    if !local_exists && crate::atmanager::is_at_set() {
        vfs_log_debug!("Local file not found, checking cloud...");
        let client = HttpClient::new().await
            .map_err(|e| VfsError::new(ErrorCode::NetworkError, format!("Failed to create HTTP client: {}", e.message)))?;
        let cloud_info = crate::read::get_cloud_file_info(&client, path).await;
        if cloud_info.exists {
            vfs_log_debug!("Cloud file exists (modified_time={}), downloading to local first...", cloud_info.modified_time);
            crate::read::read_cloud_file_with_client(&client, path, &full_path).await?;
            vfs_log_debug!("Downloaded cloud file to local: {:?}", full_path);
        } else {
            vfs_log_debug!("Cloud file also not found, will create new file");
        }
    }

    write_local_file(&full_path, content).await?;
    vfs_log_debug!("Write to local file success");

    if crate::atmanager::is_at_set() {
        vfs_log_debug!("Uploading to cloud...");
        match crate::upload::upload_file(path).await {
            Ok(_) => {
                vfs_log_debug!("Upload to cloud success");
            }
            Err(e) => {
                vfs_log_error!("Upload to cloud failed: {}", e.message);
                return Err(e);
            }
        }
    }

    vfs_log_debug!("<<< write_file END: success");
    Ok(())
}

/// Write content to a file by its absolute path, with full fusion logic.
/// If the file exists on cloud but not locally, download it first, then write and re-upload.
pub(crate) async fn write_file_by_absolute_path(path: &Path, content: &[u8]) -> Result<(), RuntimeError> {
    vfs_log_debug!(">>> write_file_by_absolute_path START: path={:?}, content_len={}", path, content.len());

    let workspace = get_workspace_sync().map_err(RuntimeError::from)?;
    let relative_path = path.strip_prefix(&workspace).unwrap_or(path);
    let relative_str = relative_path.to_string_lossy();
    vfs_log_debug!("Derived relative path: '{}'", relative_str);

    let local_exists = path.exists() && path.is_file();
    vfs_log_debug!("Local file exists: {}", local_exists);

    if !local_exists && crate::atmanager::is_at_set() {
        vfs_log_debug!("Local file not found, checking cloud...");
        let client = HttpClient::new().await
            .map_err(|e| RuntimeError::new(e.message))?;
        let cloud_info = crate::read::get_cloud_file_info(&client, &relative_str).await;
        if cloud_info.exists {
            vfs_log_debug!("Cloud file exists (modified_time={}), downloading to local first...", cloud_info.modified_time);
            crate::read::read_cloud_file_with_client(&client, &relative_str, path).await
                .map_err(RuntimeError::from)?;
            vfs_log_debug!("Downloaded cloud file to local: {:?}", path);
        } else {
            vfs_log_debug!("Cloud file also not found, will create new file");
        }
    }

    write_local_file_sync(path, content)?;
    vfs_log_debug!("Write to local file success");

    if crate::atmanager::is_at_set() {
        vfs_log_debug!("Uploading to cloud...");
        crate::upload::upload_file(&relative_str).await
            .map_err(|e| RuntimeError::new(e.message))?;
        vfs_log_debug!("Upload to cloud success");
    }

    vfs_log_debug!("<<< write_file_by_absolute_path END: success");
    Ok(())
}

fn write_local_file_sync(path: &Path, content: &[u8]) -> Result<(), RuntimeError> {
    vfs_log_debug!("write_local_file_sync: path={:?}", path);

    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(RuntimeError::from)?;
            vfs_log_debug!("Created parent directory: {:?}", parent);
        }
    }

    fs::write(path, content).map_err(RuntimeError::from)?;
    vfs_log_debug!("Wrote {} bytes to {:?}", content.len(), path);
    Ok(())
}

async fn write_local_file(path: &Path, content: &[u8]) -> VfsResult<()> {
    vfs_log_debug!("write_local_file: path={:?}", path);

    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| {
                VfsError::new(ErrorCode::IoError, format!("Failed to create parent directory: {}", e))
            })?;
            vfs_log_debug!("Created parent directory: {:?}", parent);
        }
    }

    fs::write(path, content).map_err(|e| {
        VfsError::new(ErrorCode::IoError, format!("Failed to write file: {}", e))
    })?;

    vfs_log_debug!("Wrote {} bytes to {:?}", content.len(), path);
    Ok(())
}
