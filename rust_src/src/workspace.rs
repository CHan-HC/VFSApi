use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use crate::error::{ErrorCode, VfsError, VfsResult};

static WORKSPACE: OnceLock<Arc<Mutex<Option<PathBuf>>>> = OnceLock::new();

fn get_workspace_storage() -> &'static Arc<Mutex<Option<PathBuf>>> {
    WORKSPACE.get_or_init(|| Arc::new(Mutex::new(None)))
}

pub async fn set_workspace(path: &str) -> VfsResult<()> {
    let workspace_path = PathBuf::from(path);
    
    if !workspace_path.exists() {
        std::fs::create_dir_all(&workspace_path).map_err(|e| {
            VfsError::new(
                ErrorCode::PathNotFound,
                format!("Failed to create workspace directory: {}, error: {}", path, e),
            )
        })?;
        crate::hilog::log_info(&format!("Created workspace directory: {}", path));
    }
    
    if !workspace_path.is_dir() {
        return Err(VfsError::new(
            ErrorCode::InvalidParameter,
            format!("Workspace path is not a directory: {}", path),
        ));
    }
    
    let storage = get_workspace_storage();
    let mut guard = storage.lock().map_err(|_| {
        VfsError::new(ErrorCode::Unknown, "Failed to lock workspace mutex")
    })?;
    
    *guard = Some(workspace_path);
    
    crate::hilog::log_info(&format!("Workspace set to: {}", path));
    
    Ok(())
}

pub async fn get_workspace() -> VfsResult<PathBuf> {
    let storage = get_workspace_storage();
    let guard = storage.lock().map_err(|_| {
        VfsError::new(ErrorCode::Unknown, "Failed to lock workspace mutex")
    })?;
    
    match guard.as_ref() {
        Some(path) => Ok(path.clone()),
        None => Err(VfsError::new(
            ErrorCode::WorkspaceNotSet,
            "Workspace has not been set",
        )),
    }
}

pub async fn resolve_path(relative_path: &str) -> VfsResult<PathBuf> {
    let workspace = get_workspace().await?;
    let workspace_for_log = workspace.clone();
    
    let normalized_path = relative_path.trim_start_matches('/');
    
    let resolved = if normalized_path.is_empty() {
        workspace
    } else {
        workspace.join(normalized_path)
    };
    
    crate::hilog::log_info(&format!("resolve_path: workspace={:?}, input={}, resolved={:?}", 
        workspace_for_log, relative_path, resolved));
    
    Ok(resolved)
}

pub async fn clear_workspace() -> VfsResult<()> {
    let storage = get_workspace_storage();
    let mut guard = storage.lock().map_err(|_| {
        VfsError::new(ErrorCode::Unknown, "Failed to lock workspace mutex")
    })?;
    
    *guard = None;
    
    crate::hilog::log_info("Workspace cleared");
    
    Ok(())
}

pub async fn is_workspace_set() -> bool {
    let storage = get_workspace_storage();
    match storage.lock() {
        Ok(guard) => guard.is_some(),
        Err(_) => false,
    }
}
