use crate::error::{ErrorCode, VfsError, VfsResult};
use crate::rcp::HttpClient;
use crate::runtime::RuntimeError;
use crate::workspace::{get_workspace_sync, get_base_path_sync, resolve_path, build_cloud_path};
use crate::{vfs_log_debug, vfs_log_warn};
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

const FOLDER_MIME_TYPE: &str = "application/vnd.huawei-apps.folder";

#[derive(Debug, Clone)]
pub struct StatFileResult {
    pub size: u64,
    pub is_file: bool,
    pub is_dir: bool,
    pub modified_time: u64,
    pub error_code: ErrorCode,
    pub error_message: Option<String>,
}

impl StatFileResult {
    pub fn success(size: u64, is_file: bool, is_dir: bool, modified_time: u64) -> Self {
        Self {
            size,
            is_file,
            is_dir,
            modified_time,
            error_code: ErrorCode::Success,
            error_message: None,
        }
    }

    pub fn error(code: ErrorCode, message: String) -> Self {
        Self {
            size: 0,
            is_file: false,
            is_dir: false,
            modified_time: 0,
            error_code: code,
            error_message: Some(message),
        }
    }
}

#[derive(Debug, Clone)]
struct FileMeta {
    exists: bool,
    modified_time: u64,
    size: u64,
    is_file: bool,
    is_dir: bool,
}

fn parse_rfc3339_time(time_str: &str) -> Option<u64> {
    let time_str = time_str.replace('Z', "+00:00");
    chrono::DateTime::parse_from_rfc3339(&time_str)
        .ok()
        .map(|dt| dt.timestamp() as u64)
}

pub async fn stat_file(path: &str) -> VfsResult<StatFileResult> {
    vfs_log_debug!(">>> stat_file START: path='{}'", path);

    let full_path = resolve_path(path).await?;
    vfs_log_debug!("Resolved local path: {:?}", full_path);

    let local_meta = get_local_meta(&full_path);
    vfs_log_debug!("Local file meta: exists={}, is_file={}, is_dir={}, size={}, modified_time={}",
        local_meta.exists, local_meta.is_file, local_meta.is_dir, local_meta.size, local_meta.modified_time);

    let cloud_meta = if crate::atmanager::is_at_set() {
        vfs_log_debug!("Getting cloud file meta...");
        let client = HttpClient::new().await?;
        get_cloud_meta(&client, path).await
    } else {
        FileMeta {
            exists: false,
            modified_time: 0,
            size: 0,
            is_file: false,
            is_dir: false,
        }
    };
    vfs_log_debug!("Cloud file meta: exists={}, is_file={}, is_dir={}, size={}, modified_time={}",
        cloud_meta.exists, cloud_meta.is_file, cloud_meta.is_dir, cloud_meta.size, cloud_meta.modified_time);

    match (local_meta.exists, cloud_meta.exists) {
        (true, true) => {
            vfs_log_debug!("File exists both locally and in cloud, comparing modification times");

            if local_meta.modified_time >= cloud_meta.modified_time {
                vfs_log_debug!("Local file is newer or equal, using local metadata");
                Ok(StatFileResult::success(
                    local_meta.size,
                    local_meta.is_file,
                    local_meta.is_dir,
                    local_meta.modified_time,
                ))
            } else {
                vfs_log_debug!("Cloud file is newer, using cloud metadata");
                Ok(StatFileResult::success(
                    cloud_meta.size,
                    cloud_meta.is_file,
                    cloud_meta.is_dir,
                    cloud_meta.modified_time,
                ))
            }
        }
        (true, false) => {
            vfs_log_debug!("File exists locally only");
            Ok(StatFileResult::success(
                local_meta.size,
                local_meta.is_file,
                local_meta.is_dir,
                local_meta.modified_time,
            ))
        }
        (false, true) => {
            vfs_log_debug!("File exists in cloud only");
            Ok(StatFileResult::success(
                cloud_meta.size,
                cloud_meta.is_file,
                cloud_meta.is_dir,
                cloud_meta.modified_time,
            ))
        }
        (false, false) => {
            Err(VfsError::new(
                ErrorCode::PathNotFound,
                "File not found neither locally nor in cloud",
            ))
        }
    }
}

/// Get file metadata by absolute path, with fusion logic (local + cloud).
/// Returns `None` if the file exists in neither location.
pub(crate) async fn stat_file_by_absolute_path(path: &Path) -> Result<Option<crate::filesystem::FileStat>, RuntimeError> {
    vfs_log_debug!(">>> stat_file_by_absolute_path START: path={:?}", path);

    let base_path = get_base_path_sync().map_err(RuntimeError::from)?;
    let workspace = get_workspace_sync().map_err(RuntimeError::from)?;
    let full_prefix = base_path.join(&workspace);
    let relative_path = path.strip_prefix(&full_prefix).unwrap_or(path);
    let relative_str = relative_path.to_string_lossy();
    vfs_log_debug!("Derived relative path: '{}'", relative_str);

    let local_meta = get_local_meta(path);
    vfs_log_debug!("Local file meta: exists={}, is_file={}, is_dir={}, size={}",
        local_meta.exists, local_meta.is_file, local_meta.is_dir, local_meta.size);

    let cloud_meta = if crate::atmanager::is_at_set() {
        vfs_log_debug!("Getting cloud file meta...");
        let client = HttpClient::new().await
            .map_err(|e| RuntimeError::new(e.message))?;
        get_cloud_meta(&client, &relative_str).await
    } else {
        FileMeta { exists: false, modified_time: 0, size: 0, is_file: false, is_dir: false }
    };
    vfs_log_debug!("Cloud file meta: exists={}, is_file={}, is_dir={}, size={}",
        cloud_meta.exists, cloud_meta.is_file, cloud_meta.is_dir, cloud_meta.size);

    match (local_meta.exists, cloud_meta.exists) {
        (true, true) => {
            if local_meta.modified_time >= cloud_meta.modified_time {
                vfs_log_debug!("Local is newer, using local metadata");
                Ok(Some(crate::filesystem::FileStat {
                    size: local_meta.size,
                    is_file: local_meta.is_file,
                    is_dir: local_meta.is_dir,
                    is_symlink: false,
                }))
            } else {
                vfs_log_debug!("Cloud is newer, using cloud metadata");
                Ok(Some(crate::filesystem::FileStat {
                    size: cloud_meta.size,
                    is_file: cloud_meta.is_file,
                    is_dir: cloud_meta.is_dir,
                    is_symlink: false,
                }))
            }
        }
        (true, false) => {
            vfs_log_debug!("File exists locally only");
            Ok(Some(crate::filesystem::FileStat {
                size: local_meta.size,
                is_file: local_meta.is_file,
                is_dir: local_meta.is_dir,
                is_symlink: false,
            }))
        }
        (false, true) => {
            vfs_log_debug!("File exists in cloud only");
            Ok(Some(crate::filesystem::FileStat {
                size: cloud_meta.size,
                is_file: cloud_meta.is_file,
                is_dir: cloud_meta.is_dir,
                is_symlink: false,
            }))
        }
        (false, false) => {
            vfs_log_debug!("File not found neither locally nor in cloud");
            Ok(None)
        }
    }
}

fn get_at() -> String {
    crate::atmanager::get_at().unwrap_or_default()
}

fn get_local_meta(path: &Path) -> FileMeta {
    if !path.exists() {
        return FileMeta {
            exists: false,
            modified_time: 0,
            size: 0,
            is_file: false,
            is_dir: false,
        };
    }

    if let Ok(metadata) = fs::metadata(path) {
        let modified_time = metadata
            .modified()
            .ok()
            .map(|t| {
                t.duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            })
            .unwrap_or(0);

        FileMeta {
            exists: true,
            modified_time,
            size: if metadata.is_dir() { 0 } else { metadata.len() },
            is_file: metadata.is_file(),
            is_dir: metadata.is_dir(),
        }
    } else {
        FileMeta {
            exists: false,
            modified_time: 0,
            size: 0,
            is_file: false,
            is_dir: false,
        }
    }
}

async fn get_cloud_meta(client: &HttpClient, path: &str) -> FileMeta {
    vfs_log_debug!(">>> get_cloud_meta: path='{}'", path);

    match find_file_meta(client, path).await {
        Ok(meta) => {
            vfs_log_debug!("Found cloud file: size={}, is_file={}, is_dir={}, modified_time={}",
                meta.size, meta.is_file, meta.is_dir, meta.modified_time);
            meta
        }
        Err(e) => {
            vfs_log_debug!("Cloud file not found: {}", e.message);
            FileMeta {
                exists: false,
                modified_time: 0,
                size: 0,
                is_file: false,
                is_dir: false,
            }
        }
    }
}

async fn find_file_meta(client: &HttpClient, path: &str) -> VfsResult<FileMeta> {
    vfs_log_debug!(">>> find_file_meta: path='{}'", path);

    let cloud_path = build_cloud_path(path)?;
    let parts: Vec<&str> = cloud_path
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    if parts.is_empty() {
        return Err(VfsError::new(
            ErrorCode::InvalidParameter,
            "Path is empty",
        ));
    }

    let file_name = parts[parts.len() - 1];
    let parent_parts = &parts[..parts.len() - 1];

    let parent_folder_id = if parent_parts.is_empty() {
        "applicationData".to_string()
    } else {
        let mut current_parent_id = "applicationData".to_string();

        for part in parent_parts {
            match find_folder_in_parent(client, part, &current_parent_id).await {
                Ok(id) => {
                    vfs_log_debug!("Found folder '{}' with ID: {}", part, id);
                    current_parent_id = id;
                }
                Err(e) => {
                    vfs_log_warn!("Folder '{}' not found: {}", part, e.message);
                    return Err(e);
                }
            }
        }

        current_parent_id
    };

    vfs_log_debug!("Parent folder ID: {}", parent_folder_id);

    let mut params = Vec::new();
    params.push("fields=*".to_string());
    params.push("form=json".to_string());
    params.push("containers=applicationData".to_string());
    params.push("pageSize=100".to_string());
    params.push(format!(
        "queryParam=parentFolder='{}'",
        urlencoding::encode(&parent_folder_id)
    ));

    let url = format!(
        "https://driveapis.cloud.huawei.com.cn/drive/v1/files?{}",
        params.join("&")
    );
    vfs_log_debug!("Search URL: {}", url);

    let at = get_at();
    let mut headers = std::collections::HashMap::new();
    headers.insert(
        "Authorization".to_string(),
        format!("Bearer {}", at),
    );
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Accept".to_string(), "application/json".to_string());

    let response = client.get_with_headers(&url, headers).await?;
    vfs_log_debug!("Search response status: {}", response.status_code);

    if response.status_code != 200 {
        return Err(VfsError::new(
            ErrorCode::NetworkError,
            format!("Failed to search file, status: {}", response.status_code),
        ));
    }

    let body = response.body_as_string().unwrap_or_default();

    #[derive(Deserialize)]
    struct SearchResult {
        files: Option<Vec<FileItem>>,
    }

    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct FileItem {
        id: Option<String>,
        #[serde(rename = "fileName")]
        file_name: String,
        #[serde(rename = "parentFolder")]
        parent_folder: Option<Vec<String>>,
        #[serde(rename = "editedTime")]
        edited_time: Option<String>,
        size: Option<u64>,
        #[serde(rename = "mimeType")]
        mime_type: Option<String>,
    }

    let result: SearchResult = serde_json::from_str(&body).map_err(|e| {
        VfsError::new(
            ErrorCode::JsonError,
            format!("Failed to parse search result: {}", e),
        )
    })?;

    if let Some(files) = result.files {
        for file in &files {
            if file.file_name == file_name {
                let modified_time = file
                    .edited_time
                    .as_ref()
                    .and_then(|t| parse_rfc3339_time(t))
                    .unwrap_or(0);

                let is_dir = file
                    .mime_type
                    .as_ref()
                    .map(|mt| mt == FOLDER_MIME_TYPE)
                    .unwrap_or(false);

                vfs_log_debug!(
                    "<<< Found file: name='{}', size={}, is_file={}, is_dir={}, modified_time={}",
                    file.file_name,
                    file.size.unwrap_or(0),
                    !is_dir,
                    is_dir,
                    modified_time
                );
                return Ok(FileMeta {
                    exists: true,
                    modified_time,
                    size: file.size.unwrap_or(0),
                    is_file: !is_dir,
                    is_dir,
                });
            }
        }
    }

    Err(VfsError::new(
        ErrorCode::PathNotFound,
        format!("File '{}' not found in cloud", file_name),
    ))
}

async fn find_folder_in_parent(
    client: &HttpClient,
    folder_name: &str,
    parent_id: &str,
) -> VfsResult<String> {
    vfs_log_debug!(
        ">>> find_folder_in_parent: name='{}', parent='{}'",
        folder_name,
        parent_id
    );

    let mut params = Vec::new();
    params.push("fields=*".to_string());
    params.push("form=json".to_string());
    params.push("containers=applicationData".to_string());
    params.push("pageSize=100".to_string());
    params.push(format!(
        "queryParam=parentFolder='{}'",
        urlencoding::encode(parent_id)
    ));

    let url = format!(
        "https://driveapis.cloud.huawei.com.cn/drive/v1/files?{}",
        params.join("&")
    );

    let at = get_at();
    let mut headers = std::collections::HashMap::new();
    headers.insert(
        "Authorization".to_string(),
        format!("Bearer {}", at),
    );
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Accept".to_string(), "application/json".to_string());

    let response = client.get_with_headers(&url, headers).await?;

    if response.status_code != 200 {
        return Err(VfsError::new(
            ErrorCode::NetworkError,
            format!("Failed to search folder, status: {}", response.status_code),
        ));
    }

    let body = response.body_as_string().unwrap_or_default();

    #[derive(Deserialize)]
    struct SearchResult {
        files: Option<Vec<FolderItem>>,
    }

    #[derive(Deserialize)]
    struct FolderItem {
        id: String,
        #[serde(rename = "fileName")]
        file_name: String,
        #[serde(rename = "mimeType")]
        mime_type: Option<String>,
    }

    let result: SearchResult = serde_json::from_str(&body).map_err(|e| {
        VfsError::new(
            ErrorCode::JsonError,
            format!("Failed to parse search result: {}", e),
        )
    })?;

    if let Some(files) = result.files {
        for file in &files {
            if file.file_name == folder_name {
                if let Some(mime_type) = &file.mime_type {
                    if mime_type == FOLDER_MIME_TYPE {
                        vfs_log_debug!(
                            "<<< Found folder: name='{}', id='{}'",
                            file.file_name,
                            file.id
                        );
                        return Ok(file.id.clone());
                    }
                }
            }
        }
    }

    Err(VfsError::new(
        ErrorCode::PathNotFound,
        format!("Folder '{}' not found", folder_name),
    ))
}
