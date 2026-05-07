use crate::error::{ErrorCode, VfsError, VfsResult};
use crate::rcp::HttpClient;
use crate::workspace::resolve_path;
use crate::{vfs_log_debug, vfs_log_error, vfs_log_warn};
use serde::Deserialize;
use std::fs;

/// Result returned after syncing a local file/dir to cloud (WS-5).
#[derive(Debug, Clone)]
pub(crate) struct SyncResult {
    pub file_id: String,
    pub sha256: String,
}

/// Sync a local file or directory to the cloud.
/// - File → multipart upload (DK-1)
/// - Directory → create folder (DK-7)
/// Returns the cloud file/folder ID and SHA-256 (empty for directories).
pub(crate) async fn sync_file_to_cloud(path: &str) -> VfsResult<SyncResult> {
    vfs_log_debug!(">>> sync_file_to_cloud START: path='{}'", path);

    let full_path = match resolve_path(path).await {
        Ok(p) => p,
        Err(e) => {
            vfs_log_error!("sync_file_to_cloud: resolve_path failed: {}", e.message);
            return Err(e);
        }
    };
    vfs_log_debug!("Resolved local path: {:?}", full_path);

    if !full_path.exists() {
        return Err(VfsError::new(
            ErrorCode::PathNotFound,
            format!("Local path not found: {:?}", full_path),
        ));
    }

    if full_path.is_dir() {
        vfs_log_debug!("Path is a directory, creating cloud folder...");
        let client = HttpClient::new().await?;
        let folder_name = full_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let parent_id = get_parent_folder_id(&client, path).await?;
        let folder_id = create_folder(&client, folder_name, &parent_id).await?;
        vfs_log_debug!("<<< sync_file_to_cloud END: folder_id={}", folder_id);
        Ok(SyncResult { file_id: folder_id, sha256: String::new() })
    } else {
        vfs_log_debug!("Path is a file, uploading...");
        let content = fs::read(&full_path).map_err(|e| {
            VfsError::new(ErrorCode::IoError, format!("Failed to read file: {}", e))
        })?;
        vfs_log_debug!("Read {} bytes from local file", content.len());

        let file_name = full_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let client = HttpClient::new().await?;
        let parent_folder_id = get_parent_folder_id(&client, path).await?;
        let result = upload_to_cloud(&client, &file_name, &content, &parent_folder_id).await?;
        vfs_log_debug!("<<< sync_file_to_cloud END: file_id={}, sha256={}", result.file_id, result.sha256);
        Ok(result)
    }
}

#[allow(dead_code)]
pub async fn upload_file(path: &str) -> VfsResult<()> {
    vfs_log_debug!(">>> upload_file START: path='{}'", path);

    let full_path = match resolve_path(path).await {
        Ok(p) => p,
        Err(e) => {
            vfs_log_error!("upload_file: resolve_path failed: {}", e.message);
            return Err(e);
        }
    };
    vfs_log_debug!("Resolved local path: {:?}", full_path);

    if !full_path.exists() || !full_path.is_file() {
        return Err(VfsError::new(
            ErrorCode::PathNotFound,
            format!("File not found: {:?}", full_path),
        ));
    }

    let content = fs::read(&full_path).map_err(|e| {
        VfsError::new(ErrorCode::IoError, format!("Failed to read file: {}", e))
    })?;
    vfs_log_debug!("Read {} bytes from local file", content.len());

    let file_name = full_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let client = HttpClient::new().await?;

    let parent_folder_id = get_parent_folder_id(&client, path).await?;
    vfs_log_debug!("Parent folder ID: {}", parent_folder_id);

    upload_to_cloud(&client, &file_name, &content, &parent_folder_id).await?;

    vfs_log_debug!("<<< upload_file END: success");
    Ok(())
}

fn get_at() -> String {
    crate::atmanager::get_at().unwrap_or_default()
}

async fn get_parent_folder_id(client: &HttpClient, path: &str) -> VfsResult<String> {
    vfs_log_debug!(">>> get_parent_folder_id: path='{}'", path);

    let normalized_path = path.trim_start_matches('/');
    let parts: Vec<&str> = normalized_path.split('/').filter(|s| !s.is_empty()).collect();

    if parts.is_empty() {
        return Ok("applicationData".to_string());
    }

    let parent_parts = &parts[..parts.len() - 1];

    if parent_parts.is_empty() {
        return Ok("applicationData".to_string());
    }

    let mut current_parent_id = "applicationData".to_string();

    for part in parent_parts {
        match find_or_create_folder(client, part, &current_parent_id).await {
            Ok(id) => {
                vfs_log_debug!("Folder '{}': {}", part, id);
                current_parent_id = id;
            }
            Err(e) => {
                vfs_log_warn!("Failed to find/create folder '{}': {}", part, e.message);
                return Err(e);
            }
        }
    }

    Ok(current_parent_id)
}

async fn find_or_create_folder(client: &HttpClient, folder_name: &str, parent_id: &str) -> VfsResult<String> {
    vfs_log_debug!(">>> find_or_create_folder: name='{}', parent='{}'", folder_name, parent_id);

    match find_folder_in_parent(client, folder_name, parent_id).await {
        Ok(id) => {
            vfs_log_debug!("Folder exists: {}", id);
            return Ok(id);
        }
        Err(_) => {
            vfs_log_debug!("Folder not found, creating...");
        }
    }

    create_folder(client, folder_name, parent_id).await
}

async fn find_folder_in_parent(client: &HttpClient, folder_name: &str, parent_id: &str) -> VfsResult<String> {
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

async fn create_folder(client: &HttpClient, folder_name: &str, parent_id: &str) -> VfsResult<String> {
    vfs_log_debug!(">>> create_folder: name='{}', parent='{}'", folder_name, parent_id);

    let url = "https://driveapis.cloud.huawei.com.cn/drive/v1/files?fields=*";

    let body = serde_json::json!({
        "fileName": folder_name,
        "mimeType": "application/vnd.huawei-apps.folder",
        "parentFolder": [parent_id]
    });

    let at = get_at();
    let mut headers = std::collections::HashMap::new();
    headers.insert("Authorization".to_string(), format!("Bearer {}", at));
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Accept".to_string(), "application/json".to_string());

    let body_bytes = body.to_string().into_bytes();
    let response = client.post_with_headers(url, Some(&body_bytes), Some("application/json"), headers).await?;

    if response.status_code != 200 {
        return Err(VfsError::new(
            ErrorCode::NetworkError,
            format!("Failed to create folder, status: {}", response.status_code),
        ));
    }

    let response_body = response.body_as_string().unwrap_or_default();

    #[derive(Deserialize)]
    struct CreateFolderResponse {
        id: String,
    }

    let result: CreateFolderResponse = serde_json::from_str(&response_body).map_err(|e| {
        VfsError::new(ErrorCode::JsonError, format!("Failed to parse create folder response: {}", e))
    })?;

    vfs_log_debug!("Created folder: id={}", result.id);
    Ok(result.id)
}

async fn upload_to_cloud(client: &HttpClient, file_name: &str, content: &[u8], parent_folder_id: &str) -> VfsResult<SyncResult> {
    vfs_log_debug!(">>> upload_to_cloud: name='{}', size={}", file_name, content.len());

    let boundary = "----VFS_UPLOAD_BOUNDARY_20240430";

    let metadata = serde_json::json!({
        "fileName": file_name,
        "parentFolder": [parent_folder_id]
    });

    let mut multipart_body = Vec::new();

    multipart_body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    multipart_body.extend_from_slice(b"Content-Type: application/json\r\n\r\n");
    multipart_body.extend_from_slice(metadata.to_string().as_bytes());
    multipart_body.extend_from_slice(b"\r\n");

    multipart_body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    multipart_body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    multipart_body.extend_from_slice(content);
    multipart_body.extend_from_slice(b"\r\n");

    multipart_body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    let url = format!(
        "https://driveapis.cloud.huawei.com.cn/upload/drive/v1/files?uploadType=multipart&fields=*"
    );

    let at = get_at();
    let mut headers = std::collections::HashMap::new();
    headers.insert("Authorization".to_string(), format!("Bearer {}", at));
    headers.insert("Content-Type".to_string(), format!("multipart/related;boundary={}", boundary));

    vfs_log_debug!("Upload URL: {}", url);
    vfs_log_debug!("Upload body size: {} bytes", multipart_body.len());

    let response = client.post_with_headers(&url, Some(&multipart_body), None, headers).await?;
    vfs_log_debug!("Upload response status: {}", response.status_code);

    if response.status_code != 200 {
        vfs_log_error!("Upload failed: status={}", response.status_code);
        if let Some(body) = &response.body {
            let body_str = String::from_utf8_lossy(body);
            vfs_log_error!("Error body: {}", &body_str[..body_str.len().min(500)]);
        }
        return Err(VfsError::new(
            ErrorCode::NetworkError,
            format!("Upload failed with status: {}", response.status_code),
        ));
    }

    let response_body = response.body_as_string().unwrap_or_default();

    #[derive(Deserialize)]
    struct UploadResponse {
        id: String,
        #[serde(default)]
        sha256: String,
    }

    let result: UploadResponse = serde_json::from_str(&response_body).map_err(|e| {
        VfsError::new(ErrorCode::JsonError, format!("Failed to parse upload response: {}", e))
    })?;

    vfs_log_debug!("<<< upload_to_cloud: success, id={}, sha256={}", result.id, result.sha256);
    Ok(SyncResult { file_id: result.id, sha256: result.sha256 })
}
