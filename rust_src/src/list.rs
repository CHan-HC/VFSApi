use crate::error::{ErrorCode, VfsError, VfsResult};
use crate::rcp::HttpClient;
use crate::workspace::{resolve_path, resolve_path_sync};
use crate::{vfs_log_debug, vfs_log_error, vfs_log_warn};
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub modified_time: u64,
    pub size: u64,
    pub source: u32,
    pub is_directory: bool,
}

#[derive(Debug, Clone)]
pub struct ListDirResult {
    pub files: Vec<FileInfo>,
    pub error_code: ErrorCode,
    pub error_message: Option<String>,
}

impl ListDirResult {
    pub fn success(files: Vec<FileInfo>) -> Self {
        Self {
            files,
            error_code: ErrorCode::Success,
            error_message: None,
        }
    }

    pub fn error(code: ErrorCode, message: String) -> Self {
        Self {
            files: vec![],
            error_code: code,
            error_message: Some(message),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ManifestEntry {
    #[serde(rename = "sha256")]
    pub sha256: String,
    pub size: u64,
    #[serde(rename = "mtime")]
    pub mtime: f64,
    #[serde(rename = "is_dir")]
    pub is_dir: bool,
}

/// List only local files at the given path (no cloud), returning a manifest
/// suitable for the WS-4 file_list_response. sha256 is left empty per spec.
#[allow(dead_code)]
pub(crate) async fn get_local_manifest(path_str: &str) -> VfsResult<std::collections::HashMap<String, ManifestEntry>> {
    vfs_log_debug!(">>> get_local_manifest START: path='{}'", path_str);

    let full_path = resolve_path(path_str).await?;
    vfs_log_debug!("Resolved local path: {:?}", full_path);

    let mut manifest = std::collections::HashMap::new();

    if !full_path.exists() || !full_path.is_dir() {
        vfs_log_debug!("Path does not exist or is not a directory, returning empty manifest");
        return Ok(manifest);
    }

    let entries = fs::read_dir(&full_path).map_err(|e| {
        VfsError::new(ErrorCode::IoError, format!("Failed to read directory: {}", e))
    })?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let name = entry.file_name().to_string_lossy().into_owned();

        if name.starts_with('.') {
            continue;
        }
        if name == ".sync_state" || name == ".offline_queue" {
            continue;
        }

        let is_dir = metadata.is_dir();
        let size = if is_dir { 0 } else { metadata.len() };
        let mtime = metadata.modified()
            .map(|t| t.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs_f64())
            .unwrap_or(0.0);

        vfs_log_debug!("[LOCAL_MANIFEST] name={}, is_dir={}, size={}, mtime={}", name, is_dir, size, mtime);

        manifest.insert(name, ManifestEntry {
            sha256: String::new(),
            size,
            mtime,
            is_dir,
        });
    }

    vfs_log_debug!("<<< get_local_manifest END: {} entries", manifest.len());
    Ok(manifest)
}

/// Synchronous version of get_local_manifest, for use in callbacks (e.g. WebSocket on_message).
pub(crate) fn get_local_manifest_sync(path_str: &str) -> VfsResult<std::collections::HashMap<String, ManifestEntry>> {
    vfs_log_debug!(">>> get_local_manifest_sync START: path='{}'", path_str);

    let full_path = resolve_path_sync(path_str)?;
    vfs_log_debug!("Resolved local path: {:?}", full_path);

    let mut manifest = std::collections::HashMap::new();

    if !full_path.exists() || !full_path.is_dir() {
        vfs_log_debug!("Path does not exist or is not a directory, returning empty manifest");
        return Ok(manifest);
    }

    let entries = fs::read_dir(&full_path).map_err(|e| {
        VfsError::new(ErrorCode::IoError, format!("Failed to read directory: {}", e))
    })?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let name = entry.file_name().to_string_lossy().into_owned();

        if name.starts_with('.') {
            continue;
        }
        if name == ".sync_state" || name == ".offline_queue" {
            continue;
        }

        let is_dir = metadata.is_dir();
        let size = if is_dir { 0 } else { metadata.len() };
        let mtime = metadata.modified()
            .map(|t| t.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs_f64())
            .unwrap_or(0.0);

        vfs_log_debug!("[LOCAL_MANIFEST_SYNC] name={}, is_dir={}, size={}, mtime={}", name, is_dir, size, mtime);

        manifest.insert(name, ManifestEntry {
            sha256: String::new(),
            size,
            mtime,
            is_dir,
        });
    }

    vfs_log_debug!("<<< get_local_manifest_sync END: {} entries", manifest.len());
    Ok(manifest)
}

#[derive(Deserialize)]
struct CloudFileList {
    files: Option<Vec<CloudFile>>,
}

#[derive(Deserialize)]
struct CloudFile {
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

pub async fn list_dir(path: &str) -> VfsResult<ListDirResult> {
    vfs_log_debug!(">>> list_dir START: path='{}'", path);

    let full_path = resolve_path(path).await?;
    vfs_log_debug!("Resolved local path: {:?}", full_path);

    let local_files = match list_local_files(&full_path).await {
        Ok(files) => {
            vfs_log_debug!("Local files result: {} files", files.len());
            files
        }
        Err(e) => {
            vfs_log_warn!("Local files failed: {}, will try cloud", e.message);
            vec![]
        }
    };

    vfs_log_debug!(">>> Calling list_cloud_files with path='{}'", path);
    let cloud_files = match list_cloud_files(path).await {
        Ok(files) => {
            vfs_log_debug!("Cloud files result: {} files", files.len());
            files
        }
        Err(e) => {
            vfs_log_warn!("Cloud files failed: {}", e.message);
            vec![]
        }
    };

    let merged_files = merge_files(local_files, cloud_files);
    vfs_log_debug!("<<< list_dir END: total {} files", merged_files.len());

    Ok(ListDirResult::success(merged_files))
}

fn get_at() -> String {
    crate::atmanager::get_at().unwrap_or_default()
}

async fn list_local_files(path: &Path) -> VfsResult<Vec<FileInfo>> {
    let mut files = Vec::new();
    vfs_log_debug!("list_local_files: path={:?}", path);

    if !path.exists() {
        vfs_log_debug!("list_local_files: path does not exist");
        return Ok(files);
    }

    if !path.is_dir() {
        vfs_log_error!("list_local_files: path is not a directory");
        return Err(VfsError::new(
            ErrorCode::PathNotFound,
            format!("Path is not a directory: {:?}", path),
        ));
    }

    let entries = fs::read_dir(path).map_err(|e| {
        VfsError::new(ErrorCode::IoError, format!("Failed to read directory: {}", e))
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            VfsError::new(ErrorCode::IoError, format!("Failed to read entry: {}", e))
        })?;

        let metadata = entry.metadata().map_err(|e| {
            VfsError::new(ErrorCode::IoError, format!("Failed to read metadata: {}", e))
        })?;

        let name = entry.file_name().to_string_lossy().into_owned();
        let is_directory = metadata.is_dir();
        let modified_time = metadata.modified().map_err(|e| {
            VfsError::new(ErrorCode::IoError, format!("Failed to read modified time: {}", e))
        })?;
        let modified_timestamp = system_time_to_timestamp(modified_time);
        let size = if is_directory { 0 } else { metadata.len() };

        vfs_log_debug!("[LOCAL_ITEM] name={}, is_dir={}, size={}", name, is_directory, size);

        files.push(FileInfo {
            name,
            modified_time: modified_timestamp,
            size,
            source: 1,
            is_directory,
        });
    }

    Ok(files)
}

fn system_time_to_timestamp(time: SystemTime) -> u64 {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

async fn list_cloud_files(path: &str) -> VfsResult<Vec<FileInfo>> {
    vfs_log_debug!(">>> list_cloud_files START: path='{}'", path);

    let at = get_at();
    if at.is_empty() {
        vfs_log_debug!("No access token, skipping cloud files");
        return Ok(vec![]);
    }

    let client = HttpClient::new().await?;

    let normalized_path = path.trim_start_matches('/');
    vfs_log_debug!("Normalized path: '{}'", normalized_path);

    let parent_folder_id = if normalized_path.is_empty() {
        vfs_log_debug!("Root path -> using 'applicationData'");
        "applicationData".to_string()
    } else {
        let path_parts: Vec<&str> = normalized_path.split('/').filter(|s| !s.is_empty()).collect();
        vfs_log_debug!("Path parts: {:?}", path_parts);

        let mut current_parent_id = "applicationData".to_string();

        for (i, part) in path_parts.iter().enumerate() {
            vfs_log_debug!("Finding part[{}] '{}' under parent '{}'", i, part, current_parent_id);

            match find_folder_in_parent(&client, part, &current_parent_id).await {
                Ok(id) => {
                    vfs_log_debug!("Found '{}' with ID: {}", part, id);
                    current_parent_id = id;
                }
                Err(e) => {
                    vfs_log_warn!("Folder '{}' not found under {}: {}", part, current_parent_id, e.message);
                    return Ok(vec![]);
                }
            }
        }

        vfs_log_debug!("Final parent_folder_id: {}", current_parent_id);
        current_parent_id
    };

    let mut params = Vec::new();
    params.push("fields=*".to_string());
    params.push("form=json".to_string());
    params.push("containers=applicationData".to_string());
    params.push("pageSize=100".to_string());
    params.push(format!("queryParam=parentFolder='{}'", urlencoding::encode(&parent_folder_id)));

    let url = format!("https://driveapis.cloud.huawei.com.cn/drive/v1/files?{}", params.join("&"));
    vfs_log_debug!("Query URL: {}", url);

    let mut headers = std::collections::HashMap::new();
    headers.insert("Authorization".to_string(), format!("Bearer {}", at));
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Accept".to_string(), "application/json".to_string());

    vfs_log_debug!("Sending HTTP request...");
    let response = client.get_with_headers(&url, headers).await?;
    vfs_log_debug!("Response status: {}", response.status_code);

    if response.status_code != 200 {
        vfs_log_error!("API error: status={}", response.status_code);
        if let Some(body) = &response.body {
            let body_str = String::from_utf8_lossy(body);
            vfs_log_error!("Error body: {}", &body_str[..body_str.len().min(500)]);
        }
        return Err(VfsError::new(
            ErrorCode::NetworkError,
            format!("Cloud API returned status: {}", response.status_code),
        ));
    }

    let body = response.body_as_string().unwrap_or_default();
    vfs_log_debug!("Response body length: {}", body.len());

    let cloud_list: CloudFileList = serde_json::from_str(&body).map_err(|e| {
        vfs_log_error!("JSON parse failed: {}", e);
        VfsError::new(ErrorCode::InvalidParameter, format!("Failed to parse cloud response: {}", e))
    })?;

    let cloud_files = cloud_list.files.unwrap_or_default();
    vfs_log_debug!("Parsed {} cloud files", cloud_files.len());

    for (i, f) in cloud_files.iter().enumerate() {
        let id_str = f.id.as_deref().unwrap_or("N/A");
        let parent_str = f.parent_folder.as_ref()
            .map(|v| v.join(","))
            .unwrap_or_else(|| "N/A".to_string());
        vfs_log_debug!("[CLOUD_FILE] {}. name={}, id={}, parent={}", i + 1, f.file_name, id_str, parent_str);
    }

    let files: Vec<FileInfo> = cloud_files
        .into_iter()
        .filter_map(|f| {
            let modified_time = f.edited_time.as_ref().and_then(|t| {
                parse_rfc3339_time(t)
            }).unwrap_or(0);

            let is_directory = f.mime_type.as_ref()
                .map(|mt| mt.contains("folder"))
                .unwrap_or(false);

            Some(FileInfo {
                name: f.file_name,
                modified_time,
                size: f.size.unwrap_or(0),
                source: 2,
                is_directory,
            })
        })
        .collect();

    vfs_log_debug!("<<< list_cloud_files END: {} files", files.len());
    Ok(files)
}

async fn find_folder_in_parent(client: &HttpClient, folder_name: &str, parent_id: &str) -> VfsResult<String> {
    vfs_log_debug!(">>> find_folder_in_parent: name='{}', parent='{}'", folder_name, parent_id);

    let mut params = Vec::new();
    params.push("fields=*".to_string());
    params.push("form=json".to_string());
    params.push("containers=applicationData".to_string());
    params.push("pageSize=100".to_string());
    params.push(format!("queryParam=parentFolder='{}'", urlencoding::encode(parent_id)));

    let url = format!("https://driveapis.cloud.huawei.com.cn/drive/v1/files?{}", params.join("&"));
    vfs_log_debug!("Search URL: {}", url);

    let at = get_at();
    let mut headers = std::collections::HashMap::new();
    headers.insert("Authorization".to_string(), format!("Bearer {}", at));
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Accept".to_string(), "application/json".to_string());

    let response = client.get_with_headers(&url, headers).await?;
    vfs_log_debug!("Search response status: {}", response.status_code);

    if response.status_code != 200 {
        vfs_log_error!("Search failed: status={}", response.status_code);
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
        vfs_log_error!("Parse search result failed: {}", e);
        VfsError::new(ErrorCode::InvalidParameter, format!("Failed to parse search result: {}", e))
    })?;

    if let Some(files) = result.files {
        vfs_log_debug!("Found {} items under parent", files.len());

        for file in &files {
            if file.file_name == folder_name {
                if let Some(mime_type) = &file.mime_type {
                    if mime_type.contains("folder") {
                        vfs_log_debug!("<<< Found folder: name='{}', id='{}'", file.file_name, file.id);
                        return Ok(file.id.clone());
                    }
                }
            }
        }
    }

    vfs_log_warn!("Folder '{}' not found under parent '{}'", folder_name, parent_id);
    Err(VfsError::new(
        ErrorCode::PathNotFound,
        format!("Folder '{}' not found under parent", folder_name),
    ))
}

fn parse_rfc3339_time(time_str: &str) -> Option<u64> {
    let time_str = time_str.replace('Z', "+00:00");

    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&time_str) {
        return Some(dt.timestamp() as u64);
    }

    None
}

fn merge_files(local_files: Vec<FileInfo>, cloud_files: Vec<FileInfo>) -> Vec<FileInfo> {
    let mut merged: std::collections::HashMap<String, FileInfo> = std::collections::HashMap::new();

    for file in local_files {
        merged.insert(file.name.clone(), file);
    }

    for file in cloud_files {
        if let Some(existing) = merged.get_mut(&file.name) {
            if file.modified_time > existing.modified_time {
                *existing = file;
            }
        } else {
            merged.insert(file.name.clone(), file);
        }
    }

    let mut result: Vec<FileInfo> = merged.into_values().collect();
    result.sort_by(|a, b| a.name.cmp(&b.name));

    result
}
