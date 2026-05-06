use crate::error::{ErrorCode, VfsError, VfsResult};
use crate::rcp::HttpClient;
use crate::workspace::resolve_path;
use crate::{vfs_log_debug, vfs_log_error, vfs_log_warn};
use serde::Deserialize;
use std::fs;
use std::path::Path;

pub async fn mk_dir(at: &str, path: &str) -> VfsResult<bool> {
    vfs_log_debug!(">>> mk_dir START: path='{}', at_len={}", path, at.len());
    
    let full_path = resolve_path(path).await?;
    vfs_log_debug!("Resolved local path: {:?}", full_path);
    
    let local_created = if !full_path.exists() {
        create_local_dir(&full_path).await?
    } else {
        vfs_log_debug!("Local directory already exists");
        true
    };
    
    let cloud_created = if !at.is_empty() {
        vfs_log_debug!("Ensuring cloud directory exists...");
        match ensure_cloud_dir(at, path).await {
            Ok(created) => created,
            Err(e) => {
                vfs_log_warn!("Failed to ensure cloud directory: {}", e.message);
                false
            }
        }
    } else {
        false
    };
    
    let success = local_created && cloud_created;
    vfs_log_debug!("<<< mk_dir END: success={}, local={}, cloud={}", success, local_created, cloud_created);
    
    Ok(success)
}

async fn create_local_dir(path: &Path) -> VfsResult<bool> {
    vfs_log_debug!("create_local_dir: path={:?}", path);
    
    if path.exists() {
        return Ok(true);
    }
    
    fs::create_dir_all(path).map_err(|e| {
        VfsError::new(ErrorCode::IoError, format!("Failed to create directory: {}", e))
    })?;
    
    vfs_log_debug!("Created local directory: {:?}", path);
    Ok(true)
}

async fn ensure_cloud_dir(at: &str, path: &str) -> VfsResult<bool> {
    vfs_log_debug!(">>> ensure_cloud_dir: path='{}'", path);
    
    let client = HttpClient::new().await?;
    
    let normalized_path = path.trim_start_matches('/');
    let parts: Vec<&str> = normalized_path.split('/').filter(|s| !s.is_empty()).collect();
    
    if parts.is_empty() {
        vfs_log_debug!("Root directory, nothing to create");
        return Ok(true);
    }
    
    let mut current_parent_id = "applicationData".to_string();
    
    for part in &parts {
        match find_or_create_folder(&client, at, part, &current_parent_id).await {
            Ok(id) => {
                vfs_log_debug!("Folder '{}': {}", part, id);
                current_parent_id = id;
            }
            Err(e) => {
                vfs_log_error!("Failed to find/create folder '{}': {}", part, e.message);
                return Err(e);
            }
        }
    }
    
    vfs_log_debug!("<<< ensure_cloud_dir: success");
    Ok(true)
}

async fn find_or_create_folder(client: &HttpClient, at: &str, folder_name: &str, parent_id: &str) -> VfsResult<String> {
    vfs_log_debug!(">>> find_or_create_folder: name='{}', parent='{}'", folder_name, parent_id);
    
    match find_folder_in_parent(client, at, folder_name, parent_id).await {
        Ok(id) => {
            vfs_log_debug!("Folder exists: {}", id);
            return Ok(id);
        }
        Err(_) => {
            vfs_log_debug!("Folder not found, creating...");
        }
    }
    
    create_folder_in_cloud(client, at, folder_name, parent_id).await
}

async fn find_folder_in_parent(client: &HttpClient, at: &str, folder_name: &str, parent_id: &str) -> VfsResult<String> {
    let mut params = Vec::new();
    params.push("fields=*".to_string());
    params.push("form=json".to_string());
    params.push("containers=applicationData".to_string());
    params.push("pageSize=100".to_string());
    params.push(format!("queryParam=parentFolder='{}'", urlencoding::encode(parent_id)));
    
    let url = format!("https://driveapis.cloud.huawei.com.cn/drive/v1/files?{}", params.join("&"));
    
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

async fn create_folder_in_cloud(client: &HttpClient, at: &str, folder_name: &str, parent_id: &str) -> VfsResult<String> {
    vfs_log_debug!(">>> create_folder_in_cloud: name='{}', parent='{}'", folder_name, parent_id);
    
    let url = "https://driveapis.cloud.huawei.com.cn/drive/v1/files?fields=*";
    
    let body = serde_json::json!({
        "fileName": folder_name,
        "mimeType": "application/vnd.huawei-apps.folder",
        "parentFolder": [parent_id]
    });
    
    let mut headers = std::collections::HashMap::new();
    headers.insert("Authorization".to_string(), format!("Bearer {}", at));
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Accept".to_string(), "application/json".to_string());
    
    let body_bytes = body.to_string().into_bytes();
    let response = client.post_with_headers(url, Some(&body_bytes), Some("application/json"), headers).await?;
    
    if response.status_code != 200 {
        vfs_log_error!("Create folder failed: status={}", response.status_code);
        if let Some(body) = &response.body {
            let body_str = String::from_utf8_lossy(body);
            vfs_log_error!("Error body: {}", &body_str[..body_str.len().min(500)]);
        }
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
