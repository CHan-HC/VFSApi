use crate::error::{ErrorCode, VfsError, VfsResult};
use crate::rcp::HttpClient;
use crate::runtime::RuntimeError;
use crate::workspace::{get_workspace_sync, get_base_path_sync, resolve_path, build_cloud_path};
use crate::{vfs_log_debug, vfs_log_error, vfs_log_warn};
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct ReadFileResult {
    pub content: Vec<u8>,
    pub error_code: ErrorCode,
    pub error_message: Option<String>,
}

impl ReadFileResult {
    pub fn success(content: Vec<u8>) -> Self {
        Self {
            content,
            error_code: ErrorCode::Success,
            error_message: None,
        }
    }

    pub fn error(code: ErrorCode, message: String) -> Self {
        Self {
            content: vec![],
            error_code: code,
            error_message: Some(message),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FileInfo {
    pub(crate) exists: bool,
    pub(crate) modified_time: u64,
    pub(crate) size: u64,
    #[allow(dead_code)]
    pub(crate) file_id: Option<String>,
}

pub async fn read_file(path: &str) -> VfsResult<ReadFileResult> {
    vfs_log_debug!(">>> read_file START: path='{}'", path);

    let full_path = resolve_path(path).await?;
    vfs_log_debug!("Resolved local path: {:?}", full_path);

    let local_info = get_local_file_info(&full_path);
    vfs_log_debug!("Local file info: exists={}, modified_time={}", local_info.exists, local_info.modified_time);

    let cloud_info = if crate::atmanager::is_at_set() {
        vfs_log_debug!("Getting cloud file info...");
        let client = HttpClient::new().await?;
        get_cloud_file_info(&client, path).await
    } else {
        FileInfo {
            exists: false,
            modified_time: 0,
            size: 0,
            file_id: None,
        }
    };
    vfs_log_debug!("Cloud file info: exists={}, modified_time={}, size={}", cloud_info.exists, cloud_info.modified_time, cloud_info.size);

    match (local_info.exists, cloud_info.exists) {
        (true, true) => {
            vfs_log_debug!("File exists both locally and in cloud, local_size={}, cloud_size={}", local_info.size, cloud_info.size);

            if local_info.size == cloud_info.size {
                vfs_log_debug!("Same size, reading from local (faster)");
                read_local_file(&full_path).await
            } else if local_info.modified_time >= cloud_info.modified_time {
                vfs_log_debug!("Local is newer, reading local + async cloud update");
                let result = read_local_file(&full_path).await?;
                let relative_owned = path.to_string();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    if let Err(e) = rt.block_on(crate::upload::upload_file(&relative_owned)) {
                        vfs_log_warn!("Background cloud sync failed: {}", e.message);
                    } else {
                        vfs_log_debug!("Background cloud sync succeeded");
                    }
                });
                Ok(result)
            } else {
                vfs_log_debug!("Cloud is newer, downloading from cloud");
                let client = HttpClient::new().await?;
                read_cloud_file_with_client(&client, path, &full_path).await
            }
        }
        (true, false) => {
            vfs_log_debug!("File exists locally only, reading from local");
            read_local_file(&full_path).await
        }
        (false, true) => {
            vfs_log_debug!("File exists in cloud only, downloading from cloud");
            let client = HttpClient::new().await?;
            read_cloud_file_with_client(&client, path, &full_path).await
        }
        (false, false) => {
            Err(VfsError::new(
                ErrorCode::PathNotFound,
                "File not found neither locally nor in cloud",
            ))
        }
    }
}

/// Read a file by its absolute path, with full fusion logic (local + cloud).
/// The path must already be resolved (workspace prefix included).
/// Returns the file content as raw bytes.
pub(crate) async fn read_file_by_absolute_path(path: &Path) -> Result<Vec<u8>, RuntimeError> {
    vfs_log_debug!(">>> read_file_by_absolute_path START: path={:?}", path);

    let base_path = get_base_path_sync().unwrap_or_default();
    let workspace = get_workspace_sync().map_err(RuntimeError::from)?;
    let full_prefix = base_path.join(&workspace);
    let relative_path = path.strip_prefix(&full_prefix).unwrap_or(path);
    let relative_str = relative_path.to_string_lossy();
    vfs_log_debug!("Derived relative path: '{}'", relative_str);

    let local_info = get_local_file_info(path);
    vfs_log_debug!("Local file info: exists={}, modified_time={}, size={}", local_info.exists, local_info.modified_time, local_info.size);

    let cloud_info = if crate::atmanager::is_at_set() {
        vfs_log_debug!("Getting cloud file info...");
        match HttpClient::new().await {
            Ok(client) => get_cloud_file_info(&client, &relative_str).await,
            Err(e) => {
                vfs_log_warn!("Cloud unavailable (HttpClient error), falling back to local: {}", e.message);
                FileInfo { exists: false, modified_time: 0, size: 0, file_id: None }
            }
        }
    } else {
        FileInfo { exists: false, modified_time: 0, size: 0, file_id: None }
    };
    vfs_log_debug!("Cloud file info: exists={}, modified_time={}, size={}", cloud_info.exists, cloud_info.modified_time, cloud_info.size);

    // If cloud query failed but local exists, fall back to local.
    if !cloud_info.exists && crate::atmanager::is_at_set() && local_info.exists {
        vfs_log_warn!("Cloud info unavailable, reading from local as fallback");
        return std::fs::read(path).map_err(RuntimeError::from);
    }

    match (local_info.exists, cloud_info.exists) {
        (true, true) => {
            vfs_log_debug!("Both exist: local_size={}, cloud_size={}, local_mtime={}, cloud_mtime={}",
                local_info.size, cloud_info.size, local_info.modified_time, cloud_info.modified_time);

            if local_info.size == cloud_info.size {
                vfs_log_debug!("Same size, reading from local (faster)");
                std::fs::read(path).map_err(RuntimeError::from)
            } else if local_info.modified_time >= cloud_info.modified_time {
                vfs_log_debug!("Size differs, local is newer → read local + async cloud update");
                let content = std::fs::read(path).map_err(RuntimeError::from)?;
                let relative_owned = relative_str.to_string();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    if let Err(e) = rt.block_on(crate::upload::upload_file(&relative_owned)) {
                        vfs_log_warn!("Background cloud sync failed: {}", e.message);
                    } else {
                        vfs_log_debug!("Background cloud sync succeeded for '{}'", relative_owned);
                    }
                });
                Ok(content)
            } else {
                vfs_log_debug!("Size differs, cloud is newer → download and overwrite local");
                let client = HttpClient::new().await.map_err(|e| RuntimeError::new(e.message))?;
                let result = read_cloud_file_with_client(&client, &relative_str, path)
                    .await
                    .map_err(RuntimeError::from)?;
                Ok(result.content)
            }
        }
        (true, false) => {
            vfs_log_debug!("File exists locally only, reading from local");
            std::fs::read(path).map_err(RuntimeError::from)
        }
        (false, true) => {
            vfs_log_debug!("File exists in cloud only, downloading from cloud");
            let client = HttpClient::new().await.map_err(|e| RuntimeError::new(e.message))?;
            let result = read_cloud_file_with_client(&client, &relative_str, path)
                .await
                .map_err(RuntimeError::from)?;
            Ok(result.content)
        }
        (false, false) => {
            Err(RuntimeError::new(format!("File not found: {:?}", path)))
        }
    }
}

fn get_at() -> String {
    crate::atmanager::get_at().unwrap_or_default()
}

fn get_local_file_info(path: &Path) -> FileInfo {
    if path.exists() && path.is_file() {
        if let Ok(metadata) = fs::metadata(path) {
            if let Ok(modified) = metadata.modified() {
                let timestamp = modified.duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                return FileInfo {
                    exists: true,
                    modified_time: timestamp,
                    size: metadata.len(),
                    file_id: None,
                };
            }
        }
    }
    FileInfo {
        exists: false,
        modified_time: 0,
        size: 0,
        file_id: None,
    }
}

async fn read_local_file(path: &Path) -> VfsResult<ReadFileResult> {
    vfs_log_debug!("read_local_file: path={:?}", path);

    let content = fs::read(path).map_err(|e| {
        VfsError::new(ErrorCode::IoError, format!("Failed to read file: {}", e))
    })?;

    vfs_log_debug!("Read {} bytes from local file", content.len());
    Ok(ReadFileResult::success(content))
}

pub(crate) async fn get_cloud_file_info(client: &HttpClient, path: &str) -> FileInfo {
    vfs_log_debug!(">>> get_cloud_file_info: path='{}'", path);

    match find_file_info(client, path).await {
        Ok((file_id, modified_time, cloud_size)) => {
            vfs_log_debug!("Found cloud file: id={}, modified_time={}, size={}", file_id, modified_time, cloud_size);
            FileInfo {
                exists: true,
                modified_time,
                size: cloud_size,
                file_id: Some(file_id),
            }
        }
        Err(e) => {
            vfs_log_debug!("Cloud file not found: {}", e.message);
            FileInfo {
                exists: false,
                modified_time: 0,
                size: 0,
                file_id: None,
            }
        }
    }
}

async fn find_file_info(client: &HttpClient, path: &str) -> VfsResult<(String, u64, u64)> {
    vfs_log_debug!(">>> find_file_info: path='{}'", path);

    let cloud_path = build_cloud_path(path)?;
    let parts: Vec<&str> = cloud_path.split('/').filter(|s| !s.is_empty()).collect();

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
    params.push(format!("queryParam=parentFolder='{}'", urlencoding::encode(&parent_folder_id)));

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
    struct FileItem {
        id: String,
        #[serde(rename = "fileName")]
        file_name: String,
        #[serde(rename = "editedTime")]
        edited_time: Option<String>,
        size: Option<u64>,
    }

    let result: SearchResult = serde_json::from_str(&body).map_err(|e| {
        VfsError::new(ErrorCode::JsonError, format!("Failed to parse search result: {}", e))
    })?;

    if let Some(files) = result.files {
        for file in &files {
            if file.file_name == file_name {
                let modified_time = file.edited_time.as_ref().and_then(|t| {
                    let time_str = t.replace('Z', "+00:00");
                    chrono::DateTime::parse_from_rfc3339(&time_str).ok().map(|dt| dt.timestamp() as u64)
                }).unwrap_or(0);

                let cloud_size = file.size.unwrap_or(0);

                vfs_log_debug!("<<< Found file: name='{}', id='{}', modified_time={}, size={}", file.file_name, file.id, modified_time, cloud_size);
                return Ok((file.id.clone(), modified_time, cloud_size));
            }
        }
    }

    Err(VfsError::new(
        ErrorCode::PathNotFound,
        format!("File '{}' not found in cloud", file_name),
    ))
}

pub(crate) async fn read_cloud_file_with_client(client: &HttpClient, path: &str, local_path: &Path) -> VfsResult<ReadFileResult> {
    vfs_log_debug!(">>> read_cloud_file_with_client START: path='{}'", path);

    let (file_id, _, _) = find_file_info(client, path).await?;
    vfs_log_debug!("Found file ID: {}", file_id);

    let url = format!(
        "https://driveapis.cloud.huawei.com.cn/drive/v1/files/{}?form=content",
        file_id
    );
    vfs_log_debug!("Download URL: {}", url);

    let at = get_at();
    let mut headers = std::collections::HashMap::new();
    headers.insert("Authorization".to_string(), format!("Bearer {}", at));

    vfs_log_debug!("Sending download request...");
    let response = client.get_with_headers(&url, headers).await?;
    vfs_log_debug!("Download response status: {}", response.status_code);

    if response.status_code != 200 {
        vfs_log_error!("Download failed: status={}", response.status_code);
        return Err(VfsError::new(
            ErrorCode::NetworkError,
            format!("Download failed with status: {}", response.status_code),
        ));
    }

    let content = response.body.unwrap_or_default();
    vfs_log_debug!("Downloaded {} bytes", content.len());

    if let Some(parent) = local_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| {
                VfsError::new(ErrorCode::IoError, format!("Failed to create parent directory: {}", e))
            })?;
            vfs_log_debug!("Created parent directory: {:?}", parent);
        }
    }

    fs::write(local_path, &content).map_err(|e| {
        VfsError::new(ErrorCode::IoError, format!("Failed to write file to local: {}", e))
    })?;
    vfs_log_debug!("Saved file to local: {:?}", local_path);

    Ok(ReadFileResult::success(content))
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

    let at = get_at();
    let mut headers = std::collections::HashMap::new();
    headers.insert("Authorization".to_string(), format!("Bearer {}", at));
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
        VfsError::new(ErrorCode::JsonError, format!("Failed to parse search result: {}", e))
    })?;

    if let Some(files) = result.files {
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

    Err(VfsError::new(
        ErrorCode::PathNotFound,
        format!("Folder '{}' not found", folder_name),
    ))
}
