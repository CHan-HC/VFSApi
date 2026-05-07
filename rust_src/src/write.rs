use crate::error::{ErrorCode, VfsError, VfsResult};
use crate::rcp::HttpClient;
use crate::workspace::resolve_path;
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
