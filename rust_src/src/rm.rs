use crate::error::{ErrorCode, VfsError, VfsResult};
use crate::rcp::HttpClient;
use crate::workspace::resolve_path;
use crate::{vfs_log_debug, vfs_log_error, vfs_log_warn};
use serde::Deserialize;
use std::fs;
use std::path::Path;

pub async fn rm_file(path: &str) -> VfsResult<bool> {
    vfs_log_debug!(">>> rm_file START: path='{}'", path);

    let full_path = resolve_path(path).await?;
    vfs_log_debug!("Resolved local path: {:?}", full_path);

    let local_deleted = if full_path.exists() {
        delete_local_file(&full_path).await?
    } else {
        vfs_log_debug!("Local file not found, skipping local delete");
        false
    };

    let at = crate::atmanager::get_at().unwrap_or_default();
    let cloud_deleted = if !at.is_empty() {
        vfs_log_debug!("Trying to delete from cloud...");
        match delete_cloud_file(path).await {
            Ok(deleted) => deleted,
            Err(e) => {
                vfs_log_warn!("Failed to delete from cloud: {}", e.message);
                false
            }
        }
    } else {
        false
    };

    let success = local_deleted || cloud_deleted;
    vfs_log_debug!("<<< rm_file END: success={}, local={}, cloud={}", success, local_deleted, cloud_deleted);

    Ok(success)
}

fn get_at() -> String {
    crate::atmanager::get_at().unwrap_or_default()
}

async fn delete_local_file(path: &Path) -> VfsResult<bool> {
    vfs_log_debug!("delete_local_file: path={:?}", path);

    if !path.exists() {
        return Ok(false);
    }

    fs::remove_file(path).map_err(|e| {
        VfsError::new(ErrorCode::IoError, format!("Failed to delete file: {}", e))
    })?;

    vfs_log_debug!("Deleted local file: {:?}", path);
    Ok(true)
}

async fn delete_cloud_file(path: &str) -> VfsResult<bool> {
    vfs_log_debug!(">>> delete_cloud_file: path='{}'", path);

    let client = HttpClient::new().await?;

    let file_id = match find_file_id(&client, path).await {
        Ok(id) => id,
        Err(e) => {
            vfs_log_debug!("File not found in cloud: {}", e.message);
            return Ok(false);
        }
    };

    vfs_log_debug!("Found file ID: {}, deleting...", file_id);

    let url = format!(
        "https://driveapis.cloud.huawei.com.cn/drive/v1/files/{}",
        file_id
    );

    let at = get_at();
    let mut headers = std::collections::HashMap::new();
    headers.insert("Authorization".to_string(), format!("Bearer {}", at));

    let response = client.delete_with_headers(&url, headers).await?;
    vfs_log_debug!("Delete response status: {}", response.status_code);

    if response.status_code != 204 && response.status_code != 200 {
        vfs_log_error!("Delete failed: status={}", response.status_code);
        return Err(VfsError::new(
            ErrorCode::NetworkError,
            format!("Delete failed with status: {}", response.status_code),
        ));
    }

    vfs_log_debug!("Deleted cloud file: {}", file_id);
    Ok(true)
}

async fn find_file_id(client: &HttpClient, path: &str) -> VfsResult<String> {
    vfs_log_debug!(">>> find_file_id: path='{}'", path);

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
    }

    let result: SearchResult = serde_json::from_str(&body).map_err(|e| {
        VfsError::new(ErrorCode::JsonError, format!("Failed to parse search result: {}", e))
    })?;

    if let Some(files) = result.files {
        for file in &files {
            if file.file_name == file_name {
                vfs_log_debug!("<<< Found file: name='{}', id='{}'", file.file_name, file.id);
                return Ok(file.id.clone());
            }
        }
    }

    Err(VfsError::new(
        ErrorCode::PathNotFound,
        format!("File '{}' not found in cloud", file_name),
    ))
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
