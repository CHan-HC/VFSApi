use crate::error::{ErrorCode, VfsError, VfsResult};
use crate::workspace::resolve_path;
use crate::{vfs_log_debug, vfs_log_error};
use std::fs;
use std::path::Path;

pub async fn write_file(at: &str, path: &str, content: &[u8]) -> VfsResult<()> {
    vfs_log_debug!(">>> write_file START: path='{}', content_len={}", path, content.len());
    
    let full_path = resolve_path(path).await?;
    vfs_log_debug!("Resolved local path: {:?}", full_path);
    
    write_local_file(&full_path, content).await?;
    vfs_log_debug!("Write to local file success");
    
    if !at.is_empty() {
        vfs_log_debug!("Uploading to cloud...");
        match crate::upload::upload_file(at, path).await {
            Ok(_) => {
                vfs_log_debug!("Upload to cloud success");
            }
            Err(e) => {
                vfs_log_error!("Upload to cloud failed: {}", e.message);
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
