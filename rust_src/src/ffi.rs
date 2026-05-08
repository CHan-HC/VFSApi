use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use crate::filesystem::FileSystemAdapter;
use crate::{vfs_log_error};

#[repr(C)]
pub struct CFileInfo {
    name_ptr: *mut c_char,
    name_len: usize,
    size: u64,
    is_directory: c_int,
}

#[repr(C)]
pub struct CListDirResult {
    files_ptr: *mut CFileInfo,
    files_count: usize,
    error_code: c_int,
    error_message_ptr: *mut c_char,
    error_message_len: usize,
}

#[repr(C)]
pub struct CReadFileResult {
    content_ptr: *mut u8,
    content_len: usize,
    error_code: c_int,
    error_message_ptr: *mut c_char,
    error_message_len: usize,
}

#[no_mangle]
pub extern "C" fn vfs_set_at(at: *const c_char) -> c_int {
    if at.is_null() {
        return crate::error::ErrorCode::InvalidParameter.as_i32();
    }
    let at_str = unsafe { CStr::from_ptr(at) };
    let at_string = match at_str.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return crate::error::ErrorCode::InvalidParameter.as_i32(),
    };
    match crate::atmanager::set_at(&at_string) {
        Ok(_) => crate::error::ErrorCode::Success.as_i32(),
        Err(e) => {
            vfs_log_error!("vfs_set_at failed: {}", e.message);
            e.code.as_i32()
        }
    }
}

#[no_mangle]
pub extern "C" fn vfs_upload_file(path: *const c_char) -> c_int {
    if path.is_null() {
        return crate::error::ErrorCode::InvalidParameter.as_i32();
    }

    let path_str = unsafe { CStr::from_ptr(path) };
    let path_string = match path_str.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return crate::error::ErrorCode::InvalidParameter.as_i32(),
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(crate::upload::upload_file(&path_string)) {
        Ok(_) => crate::error::ErrorCode::Success.as_i32(),
        Err(e) => {
            vfs_log_error!("vfs_upload_file failed: {}", e.message);
            e.code.as_i32()
        }
    }
}

#[no_mangle]
pub extern "C" fn vfs_write_file(path: *const c_char, content_ptr: *const u8, content_len: usize) -> c_int {
    if path.is_null() || content_ptr.is_null() {
        return crate::error::ErrorCode::InvalidParameter.as_i32();
    }

    let path_str = unsafe { CStr::from_ptr(path) };
    let path_string = match path_str.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return crate::error::ErrorCode::InvalidParameter.as_i32(),
    };

    let content = unsafe { std::slice::from_raw_parts(content_ptr, content_len) };

    let absolute_path = match crate::workspace::resolve_path_sync(&path_string) {
        Ok(p) => p,
        Err(e) => {
            vfs_log_error!("vfs_write_file: resolve_path failed: {}", e.message);
            return e.code.as_i32();
        }
    };

    let fs = crate::harmonyappfilesystem::HarmonyAppFilesystem;

    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(fs.write_file(&absolute_path, content)) {
        Ok(_) => crate::error::ErrorCode::Success.as_i32(),
        Err(e) => {
            vfs_log_error!("vfs_write_file failed: {}", e.message);
            crate::error::ErrorCode::PathNotFound.as_i32()
        }
    }
}

#[no_mangle]
pub extern "C" fn vfs_rm_file(path: *const c_char) -> c_int {
    if path.is_null() {
        return crate::error::ErrorCode::InvalidParameter.as_i32();
    }

    let path_str = unsafe { CStr::from_ptr(path) };
    let path_string = match path_str.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return crate::error::ErrorCode::InvalidParameter.as_i32(),
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(crate::rm::rm_file(&path_string)) {
        Ok(success) => if success { crate::error::ErrorCode::Success.as_i32() } else { crate::error::ErrorCode::PathNotFound.as_i32() },
        Err(e) => {
            vfs_log_error!("vfs_rm_file failed: {}", e.message);
            e.code.as_i32()
        }
    }
}

#[no_mangle]
pub extern "C" fn vfs_mk_dir(path: *const c_char) -> c_int {
    if path.is_null() {
        return crate::error::ErrorCode::InvalidParameter.as_i32();
    }

    let path_str = unsafe { CStr::from_ptr(path) };
    let path_string = match path_str.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return crate::error::ErrorCode::InvalidParameter.as_i32(),
    };

    let absolute_path = match crate::workspace::resolve_path_sync(&path_string) {
        Ok(p) => p,
        Err(e) => {
            vfs_log_error!("vfs_mk_dir: resolve_path failed: {}", e.message);
            return e.code.as_i32();
        }
    };

    let fs = crate::harmonyappfilesystem::HarmonyAppFilesystem;

    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(fs.create_dir_all(&absolute_path)) {
        Ok(()) => crate::error::ErrorCode::Success.as_i32(),
        Err(e) => {
            vfs_log_error!("vfs_mk_dir failed: {}", e.message);
            crate::error::ErrorCode::PathNotFound.as_i32()
        }
    }
}

#[no_mangle]
pub extern "C" fn vfs_set_workspace(path: *const c_char) -> c_int {
    if path.is_null() {
        return crate::error::ErrorCode::InvalidParameter.as_i32();
    }
    
    let path_str = unsafe { CStr::from_ptr(path) };
    let path_string = match path_str.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return crate::error::ErrorCode::InvalidParameter.as_i32(),
    };
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(crate::workspace::set_workspace(&path_string)) {
        Ok(_) => crate::error::ErrorCode::Success.as_i32(),
        Err(e) => {
            vfs_log_error!("vfs_set_workspace failed: {}", e.message);
            e.code.as_i32()
        }
    }
}

#[repr(C)]
pub struct CHttpResponse {
    status_code: c_int,
    body_ptr: *mut c_char,
    body_len: usize,
    error_code: c_int,
}

#[no_mangle]
pub extern "C" fn vfs_http_get(url: *const c_char) -> CHttpResponse {
    if url.is_null() {
        return CHttpResponse {
            status_code: 0,
            body_ptr: std::ptr::null_mut(),
            body_len: 0,
            error_code: crate::error::ErrorCode::InvalidParameter.as_i32(),
        };
    }
    
    let url_str = unsafe { CStr::from_ptr(url) };
    let url_string = match url_str.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return CHttpResponse {
            status_code: 0,
            body_ptr: std::ptr::null_mut(),
            body_len: 0,
            error_code: crate::error::ErrorCode::InvalidParameter.as_i32(),
        },
    };
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(crate::rcp::http_get(&url_string)) {
        Ok(response) => {
            let (body_ptr, body_len) = if let Some(body) = response.body {
                let mut vec = body;
                vec.push(0);
                let ptr = vec.as_mut_ptr() as *mut c_char;
                let len = vec.len() - 1;
                std::mem::forget(vec);
                (ptr, len)
            } else {
                (std::ptr::null_mut(), 0)
            };
            
            CHttpResponse {
                status_code: response.status_code,
                body_ptr,
                body_len,
                error_code: crate::error::ErrorCode::Success.as_i32(),
            }
        }
        Err(e) => {
            vfs_log_error!("vfs_http_get failed: {}", e.message);
            CHttpResponse {
                status_code: 0,
                body_ptr: std::ptr::null_mut(),
                body_len: 0,
                error_code: e.code.as_i32(),
            }
        },
    }
}

#[no_mangle]
pub extern "C" fn vfs_free_response(response: CHttpResponse) {
    if !response.body_ptr.is_null() && response.body_len > 0 {
        unsafe {
            let _ = Vec::from_raw_parts(
                response.body_ptr as *mut u8,
                response.body_len + 1,
                response.body_len + 1,
            );
        }
    }
}

#[no_mangle]
pub extern "C" fn vfs_list_dir(path: *const c_char) -> CListDirResult {
    if path.is_null() {
        return CListDirResult {
            files_ptr: std::ptr::null_mut(),
            files_count: 0,
            error_code: crate::error::ErrorCode::InvalidParameter.as_i32(),
            error_message_ptr: std::ptr::null_mut(),
            error_message_len: 0,
        };
    }

    let path_str = unsafe { CStr::from_ptr(path) };
    let path_string = match path_str.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return CListDirResult {
            files_ptr: std::ptr::null_mut(),
            files_count: 0,
            error_code: crate::error::ErrorCode::InvalidParameter.as_i32(),
            error_message_ptr: std::ptr::null_mut(),
            error_message_len: 0,
        },
    };

    let absolute_path = match crate::workspace::resolve_path_sync(&path_string) {
        Ok(p) => p,
        Err(e) => {
            vfs_log_error!("vfs_list_dir: resolve_path failed: {}", e.message);
            return build_list_dir_error(crate::error::ErrorCode::WorkspaceNotSet, &e.message);
        }
    };

    let fs = crate::harmonyappfilesystem::HarmonyAppFilesystem;

    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(fs.list_dir(&absolute_path)) {
        Ok(entries) => {
            let mut c_files: Vec<CFileInfo> = Vec::new();

            for entry in entries {
                let name_c = match CString::new(entry.name) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let bytes = name_c.into_bytes_with_nul();
                let name_ptr = bytes.as_ptr() as *mut c_char;
                let name_len = bytes.len() - 1;
                std::mem::forget(bytes);

                c_files.push(CFileInfo {
                    name_ptr,
                    name_len,
                    size: entry.stat.size,
                    is_directory: if entry.stat.is_dir { 1 } else { 0 },
                });
            }

            let files_count = c_files.len();
            let files_ptr = if files_count > 0 {
                let mut vec = c_files;
                let ptr = vec.as_mut_ptr();
                std::mem::forget(vec);
                ptr
            } else {
                std::ptr::null_mut()
            };

            CListDirResult {
                files_ptr,
                files_count,
                error_code: crate::error::ErrorCode::Success.as_i32(),
                error_message_ptr: std::ptr::null_mut(),
                error_message_len: 0,
            }
        }
        Err(e) => {
            vfs_log_error!("vfs_list_dir failed: {}", e.message);
            build_list_dir_error(crate::error::ErrorCode::PathNotFound, &e.message)
        }
    }
}

fn build_list_dir_error(code: crate::error::ErrorCode, message: &str) -> CListDirResult {
    let msg_c = CString::new(message).unwrap_or_default();
    let bytes = msg_c.into_bytes_with_nul();
    let msg_ptr = bytes.as_ptr() as *mut c_char;
    let msg_len = bytes.len() - 1;
    std::mem::forget(bytes);

    CListDirResult {
        files_ptr: std::ptr::null_mut(),
        files_count: 0,
        error_code: code.as_i32(),
        error_message_ptr: msg_ptr,
        error_message_len: msg_len,
    }
}

#[no_mangle]
pub extern "C" fn vfs_free_list_dir_result(result: CListDirResult) {
    if !result.files_ptr.is_null() && result.files_count > 0 {
        unsafe {
            let files = Vec::from_raw_parts(result.files_ptr, result.files_count, result.files_count);
            for file in files {
                if !file.name_ptr.is_null() && file.name_len > 0 {
                    let _ = Vec::from_raw_parts(file.name_ptr as *mut u8, file.name_len + 1, file.name_len + 1);
                }
            }
        }
    }
    
    if !result.error_message_ptr.is_null() && result.error_message_len > 0 {
        unsafe {
            let _ = Vec::from_raw_parts(result.error_message_ptr as *mut u8, result.error_message_len + 1, result.error_message_len + 1);
        }
    }
}

#[no_mangle]
pub extern "C" fn vfs_read_file(path: *const c_char) -> CReadFileResult {
    if path.is_null() {
        return CReadFileResult {
            content_ptr: std::ptr::null_mut(),
            content_len: 0,
            error_code: crate::error::ErrorCode::InvalidParameter.as_i32(),
            error_message_ptr: std::ptr::null_mut(),
            error_message_len: 0,
        };
    }

    let path_str = unsafe { CStr::from_ptr(path) };
    let path_string = match path_str.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return CReadFileResult {
            content_ptr: std::ptr::null_mut(),
            content_len: 0,
            error_code: crate::error::ErrorCode::InvalidParameter.as_i32(),
            error_message_ptr: std::ptr::null_mut(),
            error_message_len: 0,
        },
    };

    let absolute_path = match crate::workspace::resolve_path_sync(&path_string) {
        Ok(p) => p,
        Err(e) => {
            vfs_log_error!("vfs_read_file: resolve_path failed: {}", e.message);
            return build_read_error(crate::error::ErrorCode::WorkspaceNotSet, &e.message);
        }
    };

    let fs = crate::harmonyappfilesystem::HarmonyAppFilesystem;

    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(fs.read_file(&absolute_path)) {
        Ok(content) => {
            let (content_ptr, content_len) = if !content.is_empty() {
                let mut vec = content;
                let ptr = vec.as_mut_ptr();
                let len = vec.len();
                std::mem::forget(vec);
                (ptr, len)
            } else {
                (std::ptr::null_mut(), 0)
            };

            CReadFileResult {
                content_ptr,
                content_len,
                error_code: crate::error::ErrorCode::Success.as_i32(),
                error_message_ptr: std::ptr::null_mut(),
                error_message_len: 0,
            }
        }
        Err(e) => {
            vfs_log_error!("vfs_read_file failed: {}", e.message);
            build_read_error(crate::error::ErrorCode::PathNotFound, &e.message)
        }
    }
}

fn build_read_error(code: crate::error::ErrorCode, message: &str) -> CReadFileResult {
    let msg_c = CString::new(message).unwrap_or_default();
    let bytes = msg_c.into_bytes_with_nul();
    let msg_ptr = bytes.as_ptr() as *mut c_char;
    let msg_len = bytes.len() - 1;
    std::mem::forget(bytes);

    CReadFileResult {
        content_ptr: std::ptr::null_mut(),
        content_len: 0,
        error_code: code.as_i32(),
        error_message_ptr: msg_ptr,
        error_message_len: msg_len,
    }
}

#[no_mangle]
pub extern "C" fn vfs_free_read_file_result(result: CReadFileResult) {
    if !result.content_ptr.is_null() && result.content_len > 0 {
        unsafe {
            let _ = Vec::from_raw_parts(result.content_ptr, result.content_len, result.content_len);
        }
    }

    if !result.error_message_ptr.is_null() && result.error_message_len > 0 {
        unsafe {
            let _ = Vec::from_raw_parts(result.error_message_ptr as *mut u8, result.error_message_len + 1, result.error_message_len + 1);
        }
    }
}

#[repr(C)]
pub struct CStatFileResult {
    pub size: u64,
    pub is_file: c_int,
    pub is_dir: c_int,
    pub modified_time: u64,
    pub error_code: c_int,
    pub error_message_ptr: *mut c_char,
    pub error_message_len: usize,
}

#[no_mangle]
pub extern "C" fn vfs_stat_file(path: *const c_char) -> CStatFileResult {
    if path.is_null() {
        return CStatFileResult {
            size: 0,
            is_file: 0,
            is_dir: 0,
            modified_time: 0,
            error_code: crate::error::ErrorCode::InvalidParameter.as_i32(),
            error_message_ptr: std::ptr::null_mut(),
            error_message_len: 0,
        };
    }

    let path_str = unsafe { CStr::from_ptr(path) };
    let path_string = match path_str.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return CStatFileResult {
            size: 0,
            is_file: 0,
            is_dir: 0,
            modified_time: 0,
            error_code: crate::error::ErrorCode::InvalidParameter.as_i32(),
            error_message_ptr: std::ptr::null_mut(),
            error_message_len: 0,
        },
    };

    let absolute_path = match crate::workspace::resolve_path_sync(&path_string) {
        Ok(p) => p,
        Err(e) => {
            vfs_log_error!("vfs_stat_file: resolve_path failed: {}", e.message);
            return build_stat_error(crate::error::ErrorCode::WorkspaceNotSet, &e.message);
        }
    };

    let fs = crate::harmonyappfilesystem::HarmonyAppFilesystem;

    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(fs.stat(&absolute_path)) {
        Ok(Some(stat)) => CStatFileResult {
            size: stat.size,
            is_file: if stat.is_file { 1 } else { 0 },
            is_dir: if stat.is_dir { 1 } else { 0 },
            modified_time: 0,
            error_code: crate::error::ErrorCode::Success.as_i32(),
            error_message_ptr: std::ptr::null_mut(),
            error_message_len: 0,
        },
        Ok(None) => {
            let msg = format!("File not found: {:?}", absolute_path);
            build_stat_error(crate::error::ErrorCode::PathNotFound, &msg)
        }
        Err(e) => {
            vfs_log_error!("vfs_stat_file failed: {}", e.message);
            build_stat_error(crate::error::ErrorCode::PathNotFound, &e.message)
        }
    }
}

fn build_stat_error(code: crate::error::ErrorCode, message: &str) -> CStatFileResult {
    let msg_c = CString::new(message).unwrap_or_default();
    let bytes = msg_c.into_bytes_with_nul();
    let msg_ptr = bytes.as_ptr() as *mut c_char;
    let msg_len = bytes.len() - 1;
    std::mem::forget(bytes);

    CStatFileResult {
        size: 0,
        is_file: 0,
        is_dir: 0,
        modified_time: 0,
        error_code: code.as_i32(),
        error_message_ptr: msg_ptr,
        error_message_len: msg_len,
    }
}

#[no_mangle]
pub extern "C" fn vfs_bind_server() -> c_int {
    match crate::channel::bind_server() {
        Ok(_) => crate::error::ErrorCode::Success.as_i32(),
        Err(e) => {
            vfs_log_error!("vfs_bind_server failed: {}", e);
            crate::error::ErrorCode::NetworkError.as_i32()
        }
    }
}

#[no_mangle]
pub extern "C" fn vfs_free_stat_file_result(result: CStatFileResult) {
    if !result.error_message_ptr.is_null() && result.error_message_len > 0 {
        unsafe {
            let _ = Vec::from_raw_parts(
                result.error_message_ptr as *mut u8,
                result.error_message_len + 1,
                result.error_message_len + 1,
            );
        }
    }
}
