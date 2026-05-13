use crate::error::{ErrorCode, VfsError, VfsResult};
use crate::rcp::HttpClient;
use crate::{vfs_log_debug, vfs_log_error};
use serde::Deserialize;

fn get_at() -> String {
    crate::atmanager::get_at().unwrap_or_default()
}

/// List cloud files at a raw path directly under applicationData, bypassing workspace.
/// path: e.g. "/" or "/usr10086/session100000"
/// Returns a human-readable string for display.
pub async fn list_cloud_raw(path: &str) -> VfsResult<String> {
    vfs_log_error!(">>> list_cloud_raw START: path='{}'", path);

    let at = get_at();
    if at.is_empty() {
        return Ok("No access token, please click 'get at' first".to_string());
    }

    let client = HttpClient::new().await?;

    let normalized = path.trim_start_matches('/');
    vfs_log_error!("list_cloud_raw: normalized='{}'", normalized);

    let parent_folder_id = if normalized.is_empty() {
        vfs_log_error!("list_cloud_raw: root -> applicationData");
        "applicationData".to_string()
    } else {
        let path_parts: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
        vfs_log_error!("list_cloud_raw: path_parts={:?}", path_parts);

        let mut current_parent_id = "applicationData".to_string();

        for (i, part) in path_parts.iter().enumerate() {
            vfs_log_error!("list_cloud_raw: finding part[{}]='{}' under='{}'", i, part, current_parent_id);

            match find_folder_in_parent_raw(&client, part, &current_parent_id).await {
                Ok(id) => {
                    vfs_log_error!("list_cloud_raw: found '{}' id={}", part, id);
                    current_parent_id = id;
                }
                Err(e) => {
                    vfs_log_error!("list_cloud_raw: folder '{}' NOT found: {}", part, e.message);
                    return Ok(format!("Folder '{}' not found: {}", part, e.message));
                }
            }
        }

        vfs_log_error!("list_cloud_raw: final parent_folder_id={}", current_parent_id);
        current_parent_id
    };

    let mut params = Vec::new();
    params.push("fields=*".to_string());
    params.push("form=json".to_string());
    params.push("containers=applicationData".to_string());
    params.push("pageSize=100".to_string());
    params.push(format!("queryParam=parentFolder='{}'", urlencoding::encode(&parent_folder_id)));

    let url = format!("https://driveapis.cloud.huawei.com.cn/drive/v1/files?{}", params.join("&"));
    vfs_log_error!("list_cloud_raw: query url={}", url);

    let mut headers = std::collections::HashMap::new();
    headers.insert("Authorization".to_string(), format!("Bearer {}", at));
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Accept".to_string(), "application/json".to_string());

    let response = client.get_with_headers(&url, headers).await?;

    if response.status_code != 200 {
        return Err(VfsError::new(
            ErrorCode::NetworkError,
            format!("Cloud API returned status: {}", response.status_code),
        ));
    }

    let body = response.body_as_string().unwrap_or_default();

    #[derive(Deserialize)]
    struct CloudFileList {
        files: Option<Vec<CloudFile>>,
    }

    #[derive(Deserialize)]
    struct CloudFile {
        #[serde(rename = "fileName")]
        file_name: String,
        #[serde(rename = "mimeType")]
        mime_type: Option<String>,
        size: Option<u64>,
        id: Option<String>,
        #[serde(rename = "parentFolder")]
        parent_folder: Option<Vec<String>>,
        #[allow(dead_code)]
        #[serde(rename = "editedTime")]
        edited_time: Option<String>,
    }

    let cloud_list: CloudFileList = serde_json::from_str(&body).map_err(|e| {
        VfsError::new(ErrorCode::JsonError, format!("Failed to parse cloud response: {}", e))
    })?;

    let all_files = cloud_list.files.unwrap_or_default();

    // Client-side filter: only keep files whose parentFolder matches parent_folder_id.
    // The Huawei Drive API may not filter correctly when parentFolder='applicationData'.
    let cloud_files: Vec<&CloudFile> = all_files
        .iter()
        .filter(|f| {
            f.parent_folder.as_ref().map_or(false, |parents| {
                parents.iter().any(|p| p == &parent_folder_id)
            })
        })
        .collect();

    if cloud_files.is_empty() {
        return Ok(format!("Cloud path '{}' is empty (no files found)", path));
    }

    let mut output = format!("Cloud path '{}' (parent={}): {} files\n", path, parent_folder_id, cloud_files.len());
    for f in &cloud_files {
        let is_dir = f.mime_type.as_ref()
            .map(|mt| mt.contains("folder"))
            .unwrap_or(false);
        let type_str = if is_dir { "[DIR] " } else { "[FILE]" };
        let size_str = f.size.map(|s| format!("{} bytes", s)).unwrap_or_else(|| "-".to_string());
        let id_str = f.id.as_deref().unwrap_or("N/A");
        output.push_str(&format!(
            "  {} {}  size={}  id={}\n",
            type_str, f.file_name, size_str, id_str
        ));
    }

    vfs_log_debug!("<<< list_cloud_raw END: {} files", cloud_files.len());
    Ok(output)
}

async fn find_folder_in_parent_raw(
    client: &HttpClient,
    folder_name: &str,
    parent_id: &str,
) -> VfsResult<String> {
    vfs_log_debug!(">>> find_folder_in_parent_raw: name='{}', parent='{}'", folder_name, parent_id);

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
