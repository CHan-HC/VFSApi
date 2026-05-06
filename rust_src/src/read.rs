use crate::error::{ErrorCode, VfsError, VfsResult};
use crate::rcp::HttpClient;
use crate::workspace::resolve_path;
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
struct FileInfo {
    exists: bool,
    modified_time: u64,
    #[allow(dead_code)]
    file_id: Option<String>,
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
            file_id: None,
        }
    };
    vfs_log_debug!("Cloud file info: exists={}, modified_time={}", cloud_info.exists, cloud_info.modified_time);

    match (local_info.exists, cloud_info.exists) {
        (true, true) => {
            vfs_log_debug!("File exists both locally and in cloud, comparing modification times");

            if local_info.modified_time >= cloud_info.modified_time {
                vfs_log_debug!("Local file is newer or equal, reading from local");
                read_local_file(&full_path).await
            } else {
                vfs_log_debug!("Cloud file is newer, downloading from cloud");
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
                    file_id: None,
                };
            }
        }
    }
    FileInfo {
        exists: false,
        modified_time: 0,
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

async fn get_cloud_file_info(client: &HttpClient, path: &str) -> FileInfo {
    vfs_log_debug!(">>> get_cloud_file_info: path='{}'", path);

    match find_file_info(client, path).await {
        Ok((file_id, modified_time)) => {
            vfs_log_debug!("Found cloud file: id={}, modified_time={}", file_id, modified_time);
            FileInfo {
                exists: true,
                modified_time,
                file_id: Some(file_id),
            }
        }
        Err(e) => {
            vfs_log_debug!("Cloud file not found: {}", e.message);
            FileInfo {
                exists: false,
                modified_time: 0,
                file_id: None,
            }
        }
    }
}

async fn find_file_info(client: &HttpClient, path: &str) -> VfsResult<(String, u64)> {
    vfs_log_debug!(">>> find_file_info: path='{}'", path);

    let normalized_path = path.trim_start_matches('/');
    let parts: Vec<&str> = normalized_path.split('/').filter(|s| !s.is_empty()).collect();

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

                vfs_log_debug!("<<< Found file: name='{}', id='{}', modified_time={}", file.file_name, file.id, modified_time);
                return Ok((file.id.clone(), modified_time));
            }
        }
    }

    Err(VfsError::new(
        ErrorCode::PathNotFound,
        format!("File '{}' not found in cloud", file_name),
    ))
}

async fn read_cloud_file_with_client(client: &HttpClient, path: &str, local_path: &Path) -> VfsResult<ReadFileResult> {
    vfs_log_debug!(">>> read_cloud_file_with_client START: path='{}'", path);

    let (file_id, _) = find_file_info(client, path).await?;
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
