use std::sync::{Arc, Mutex, OnceLock};

use crate::error::{ErrorCode, VfsError, VfsResult};

static AT_TOKEN: OnceLock<Arc<Mutex<Option<String>>>> = OnceLock::new();

fn get_at_storage() -> &'static Arc<Mutex<Option<String>>> {
    AT_TOKEN.get_or_init(|| Arc::new(Mutex::new(None)))
}

pub fn set_at(at: &str) -> VfsResult<()> {
    let storage = get_at_storage();
    let mut guard = storage.lock().map_err(|_| {
        VfsError::new(ErrorCode::Unknown, "Failed to lock AT mutex")
    })?;
    *guard = Some(at.to_string());
    crate::hilog::log_info(&format!("AT token set, length={}", at.len()));
    Ok(())
}

pub fn get_at() -> VfsResult<String> {
    let storage = get_at_storage();
    let guard = storage.lock().map_err(|_| {
        VfsError::new(ErrorCode::Unknown, "Failed to lock AT mutex")
    })?;
    match guard.as_ref() {
        Some(at) => Ok(at.clone()),
        None => Err(VfsError::new(
            ErrorCode::WorkspaceNotSet,
            "AT token has not been set",
        )),
    }
}

pub fn is_at_set() -> bool {
    let storage = get_at_storage();
    match storage.lock() {
        Ok(guard) => guard.is_some(),
        Err(_) => false,
    }
}
