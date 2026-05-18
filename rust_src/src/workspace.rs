use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use crate::error::{ErrorCode, VfsError, VfsResult};

static WORKSPACE: OnceLock<Arc<Mutex<Option<PathBuf>>>> = OnceLock::new();
static BASE_PATH: OnceLock<Arc<Mutex<Option<PathBuf>>>> = OnceLock::new();

fn get_workspace_storage() -> &'static Arc<Mutex<Option<PathBuf>>> {
    WORKSPACE.get_or_init(|| Arc::new(Mutex::new(None)))
}

fn get_base_path_storage() -> &'static Arc<Mutex<Option<PathBuf>>> {
    BASE_PATH.get_or_init(|| Arc::new(Mutex::new(None)))
}

pub async fn set_workspace(path: &str) -> VfsResult<()> {
    let p = PathBuf::from(path);

    let storage = get_workspace_storage();
    let mut guard = storage.lock().map_err(|_| {
        VfsError::new(ErrorCode::Unknown, "Failed to lock workspace mutex")
    })?;

    *guard = Some(p);

    // Create the real directory under basePath
    let base = get_base_path_sync().unwrap_or_else(|_| PathBuf::new());
    let ws_rel = PathBuf::from(path.trim_start_matches('/'));
    let full = base.join(&ws_rel);
    if !full.exists() {
        std::fs::create_dir_all(&full).map_err(|e| {
            VfsError::new(
                ErrorCode::PathNotFound,
                format!("Failed to create workspace directory: {:?}, error: {}", full, e),
            )
        })?;
    }

    crate::hilog::log_info(&format!("Workspace set to: {}, full={:?}", path, full));

    Ok(())
}

pub async fn set_base_path(path: &str) -> VfsResult<()> {
    let base = PathBuf::from(path);

    if !base.exists() {
        std::fs::create_dir_all(&base).map_err(|e| {
            VfsError::new(
                ErrorCode::PathNotFound,
                format!("Failed to create base path directory: {}, error: {}", path, e),
            )
        })?;
    }

    let storage = get_base_path_storage();
    let mut guard = storage.lock().map_err(|_| {
        VfsError::new(ErrorCode::Unknown, "Failed to lock base path mutex")
    })?;

    *guard = Some(base);

    crate::hilog::log_info(&format!("Base path set to: {}", path));

    Ok(())
}

pub(crate) fn get_workspace_sync() -> VfsResult<PathBuf> {
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

pub(crate) fn get_base_path_sync() -> VfsResult<PathBuf> {
    let storage = get_base_path_storage();
    let guard = storage.lock().map_err(|_| {
        VfsError::new(ErrorCode::Unknown, "Failed to lock base path mutex")
    })?;

    match guard.as_ref() {
        Some(path) => Ok(path.clone()),
        None => Err(VfsError::new(
            ErrorCode::WorkspaceNotSet,
            "Base path has not been set",
        )),
    }
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

fn join_workspace(base: &Path, workspace: &Path, relative: &str) -> PathBuf {
    // Strip leading / from workspace to avoid replacing base in Path::join
    let ws_rel = PathBuf::from(workspace.to_string_lossy().trim_start_matches('/'));
    if relative.is_empty() {
        base.join(&ws_rel)
    } else {
        base.join(&ws_rel).join(relative)
    }
}

fn get_base_path_sync_or_empty() -> PathBuf {
    get_base_path_sync().unwrap_or_default()
}

pub(crate) fn resolve_path_sync(relative_path: &str) -> VfsResult<PathBuf> {
    let workspace = get_workspace_sync()?;
    let base_path = get_base_path_sync_or_empty();
    let normalized = relative_path.trim_start_matches('/');
    let resolved = join_workspace(&base_path, &workspace, normalized);
    crate::hilog::log_info(&format!(
        "resolve_path_sync: base={:?}, workspace={:?}, input={}, resolved={:?}",
        base_path, workspace, relative_path, resolved
    ));
    Ok(resolved)
}

pub async fn resolve_path(relative_path: &str) -> VfsResult<PathBuf> {
    let workspace = get_workspace().await?;
    let base_path = get_base_path_sync_or_empty();
    let normalized = relative_path.trim_start_matches('/');
    let resolved = join_workspace(&base_path, &workspace, normalized);
    crate::hilog::log_info(&format!(
        "resolve_path: base={:?}, workspace={:?}, input={}, resolved={:?}",
        base_path, workspace, relative_path, resolved
    ));
    Ok(resolved)
}

/// Build a cloud-relative path by prepending the workspace to the user-provided path.
///
/// Example: workspace="/qqq", path="/zzz/1.txt" → "qqq/zzz/1.txt"
/// If workspace is not set, returns the path as-is (stripped of leading slash).
pub(crate) fn build_cloud_path(relative_path: &str) -> VfsResult<String> {
    let workspace = get_workspace_sync()?;
    let ws = workspace.to_string_lossy();
    let ws = ws.trim_matches('/');
    let rp = relative_path.trim_matches('/');
    let full = match (ws.is_empty(), rp.is_empty()) {
        (true, true) => String::new(),
        (true, false) => rp.to_string(),
        (false, true) => ws.to_string(),
        (false, false) => format!("{}/{}", ws, rp),
    };
    crate::hilog::log_info(&format!(
        "build_cloud_path: workspace={}, input={}, result={}",
        ws, relative_path, full
    ));
    Ok(full)
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
