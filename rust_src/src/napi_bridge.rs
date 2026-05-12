use std::ffi::{c_char, c_int, c_uint, c_void};
use std::ptr;

use crate::vfs_log_error;

// ── NAPI type aliases ──────────────────────────────────────────────
type NapiEnv = *mut c_void;
type NapiValue = *mut c_void;
type NapiCallbackInfo = *mut c_void;
type NapiStatus = c_int;

type NapiCallback = extern "C" fn(env: NapiEnv, info: NapiCallbackInfo) -> NapiValue;

#[repr(C)]
struct NapiPropertyDescriptor {
    utf8name: *const c_char,
    name: NapiValue,
    method: Option<NapiCallback>,
    getter: Option<NapiCallback>,
    setter: Option<NapiCallback>,
    value: NapiValue,
    attributes: u32,
    data: *mut c_void,
}

#[repr(C)]
struct NapiModule {
    nm_version: c_int,
    nm_flags: c_uint,
    nm_filename: *const c_char,
    nm_register_func: Option<extern "C" fn(env: NapiEnv, exports: NapiValue) -> NapiValue>,
    nm_modname: *const c_char,
    nm_priv: *mut c_void,
    reserved: [*mut c_void; 4],
}

unsafe impl Sync for NapiModule {}

// ── FFI to libace_napi.z.so ────────────────────────────────────────
extern "C" {
    fn napi_module_register(module: *const NapiModule);
    fn napi_define_properties(
        env: NapiEnv,
        object: NapiValue,
        property_count: usize,
        properties: *const NapiPropertyDescriptor,
    ) -> NapiStatus;
    fn napi_get_cb_info(
        env: NapiEnv,
        cbinfo: NapiCallbackInfo,
        argc: *mut usize,
        argv: *mut NapiValue,
        this_arg: *mut NapiValue,
        data: *mut *mut c_void,
    ) -> NapiStatus;
    fn napi_get_value_string_utf8(
        env: NapiEnv,
        value: NapiValue,
        buf: *mut c_char,
        bufsize: usize,
        result: *mut usize,
    ) -> NapiStatus;
    fn napi_create_string_utf8(
        env: NapiEnv,
        str_: *const c_char,
        length: usize,
        result: *mut NapiValue,
    ) -> NapiStatus;
    fn napi_create_int32(env: NapiEnv, value: c_int, result: *mut NapiValue) -> NapiStatus;
    fn napi_get_array_length(env: NapiEnv, value: NapiValue, result: *mut u32) -> NapiStatus;
    fn napi_get_element(
        env: NapiEnv,
        object: NapiValue,
        index: u32,
        result: *mut NapiValue,
    ) -> NapiStatus;
    fn napi_get_value_uint32(env: NapiEnv, value: NapiValue, result: *mut u32) -> NapiStatus;
}

// ── Helpers ────────────────────────────────────────────────────────

unsafe fn get_cb_args(env: NapiEnv, info: NapiCallbackInfo) -> (usize, [NapiValue; 2]) {
    let mut argc: usize = 2;
    let mut args: [NapiValue; 2] = [ptr::null_mut(); 2];
    napi_get_cb_info(env, info, &mut argc, args.as_mut_ptr(), ptr::null_mut(), ptr::null_mut());
    (argc, args)
}

fn read_napi_string(env: NapiEnv, val: NapiValue) -> Option<String> {
    let mut len: usize = 0;
    unsafe { napi_get_value_string_utf8(env, val, ptr::null_mut(), 0, &mut len); }
    let mut buf: Vec<u8> = vec![0u8; len + 1];
    unsafe {
        napi_get_value_string_utf8(
            env, val,
            buf.as_mut_ptr() as *mut c_char,
            len + 1,
            &mut len,
        );
    }
    buf.truncate(len);
    String::from_utf8(buf).ok()
}

fn read_byte_array(env: NapiEnv, array: NapiValue) -> Vec<u8> {
    let mut len: u32 = 0;
    unsafe { napi_get_array_length(env, array, &mut len); }
    let mut bytes = Vec::with_capacity(len as usize);
    for i in 0..len {
        let mut element: NapiValue = ptr::null_mut();
        unsafe { napi_get_element(env, array, i, &mut element); }
        let mut byte_val: u32 = 0;
        unsafe { napi_get_value_uint32(env, element, &mut byte_val); }
        bytes.push(byte_val as u8);
    }
    bytes
}

fn return_int32(env: NapiEnv, value: c_int) -> NapiValue {
    let mut result: NapiValue = ptr::null_mut();
    unsafe { napi_create_int32(env, value, &mut result); }
    result
}

fn return_string(env: NapiEnv, s: &str) -> NapiValue {
    let mut result: NapiValue = ptr::null_mut();
    unsafe {
        napi_create_string_utf8(env, s.as_ptr() as *const c_char, s.len(), &mut result);
    }
    result
}

fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    const K: f64 = 1024.0;
    let sizes = ["B", "KB", "MB", "GB", "TB"];
    let mut i = 0usize;
    let mut size = bytes as f64;
    while size >= K && i < sizes.len() - 1 {
        size /= K;
        i += 1;
    }
    format!("{:.2} {}", size, sizes[i])
}

fn format_time(timestamp: u64) -> String {
    if timestamp == 0 {
        return "N/A".to_string();
    }
    use chrono::TimeZone;
    match chrono::Utc.timestamp_opt(timestamp as i64, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => "N/A".to_string(),
    }
}

fn is_text(data: &[u8], max_check: usize) -> bool {
    let check_len = data.len().min(max_check);
    for &b in &data[..check_len] {
        if b < 32 && b != b'\n' && b != b'\r' && b != b'\t' {
            return false;
        }
    }
    true
}

// ── NAPI callbacks ─────────────────────────────────────────────────

extern "C" fn set_workspace(env: NapiEnv, info: NapiCallbackInfo) -> NapiValue {
    let (argc, args) = unsafe { get_cb_args(env, info) };
    if argc < 1 {
        return return_int32(env, crate::error::ErrorCode::InvalidParameter.as_i32());
    }
    let path = match read_napi_string(env, args[0]) {
        Some(p) => p,
        None => return return_int32(env, crate::error::ErrorCode::InvalidParameter.as_i32()),
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    let code = match rt.block_on(crate::workspace::set_workspace(&path)) {
        Ok(_) => crate::error::ErrorCode::Success,
        Err(e) => {
            vfs_log_error!("set_workspace failed: {}", e.message);
            e.code
        }
    };
    return_int32(env, code.as_i32())
}

extern "C" fn set_base_path(env: NapiEnv, info: NapiCallbackInfo) -> NapiValue {
    let (argc, args) = unsafe { get_cb_args(env, info) };
    if argc < 1 {
        return return_int32(env, crate::error::ErrorCode::InvalidParameter.as_i32());
    }
    let path = match read_napi_string(env, args[0]) {
        Some(p) => p,
        None => return return_int32(env, crate::error::ErrorCode::InvalidParameter.as_i32()),
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    let code = match rt.block_on(crate::workspace::set_base_path(&path)) {
        Ok(_) => crate::error::ErrorCode::Success,
        Err(e) => {
            vfs_log_error!("set_base_path failed: {}", e.message);
            e.code
        }
    };
    return_int32(env, code.as_i32())
}

extern "C" fn set_at(env: NapiEnv, info: NapiCallbackInfo) -> NapiValue {
    let (argc, args) = unsafe { get_cb_args(env, info) };
    if argc < 1 {
        return return_int32(env, crate::error::ErrorCode::InvalidParameter.as_i32());
    }
    let at = match read_napi_string(env, args[0]) {
        Some(a) => a,
        None => return return_int32(env, crate::error::ErrorCode::InvalidParameter.as_i32()),
    };
    let code = match crate::atmanager::set_at(&at) {
        Ok(_) => crate::error::ErrorCode::Success,
        Err(e) => {
            vfs_log_error!("set_at failed: {}", e.message);
            e.code
        }
    };
    return_int32(env, code.as_i32())
}

extern "C" fn list_dir(env: NapiEnv, info: NapiCallbackInfo) -> NapiValue {
    let (argc, args) = unsafe { get_cb_args(env, info) };
    if argc < 1 {
        return return_string(env, "Error: invalid parameter");
    }
    let path = match read_napi_string(env, args[0]) {
        Some(p) => p,
        None => return return_string(env, "Error: invalid parameter"),
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(crate::list::list_dir(&path)) {
        Ok(result) => {
            let files = result.files;
            if files.is_empty() {
                return return_string(env, "No files found");
            }
            let mut output = format!("Found {} files:\n", files.len());
            for (i, f) in files.iter().enumerate() {
                let type_str = if f.is_directory { "Directory" } else { "File" };
                output.push_str(&format!(
                    "{}. {}\n   Type: {}, Size: {}\n",
                    i + 1,
                    f.name,
                    type_str,
                    format_size(f.size)
                ));
            }
            return_string(env, &output)
        }
        Err(e) => {
            vfs_log_error!("list_dir failed: {}", e.message);
            return_string(env, &format!("Error: {} - {}", e.code.as_i32(), e.message))
        }
    }
}

extern "C" fn list_cloud_raw(env: NapiEnv, info: NapiCallbackInfo) -> NapiValue {
    let (argc, args) = unsafe { get_cb_args(env, info) };
    if argc < 1 {
        return return_string(env, "Error: invalid parameter");
    }
    let path = match read_napi_string(env, args[0]) {
        Some(p) => p,
        None => return return_string(env, "Error: invalid parameter"),
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(crate::test::list_cloud_raw(&path)) {
        Ok(result) => return_string(env, &result),
        Err(e) => {
            vfs_log_error!("list_cloud_raw failed: {}", e.message);
            return_string(env, &format!("Error: {} - {}", e.code.as_i32(), e.message))
        }
    }
}

extern "C" fn read_file(env: NapiEnv, info: NapiCallbackInfo) -> NapiValue {
    let (argc, args) = unsafe { get_cb_args(env, info) };
    if argc < 1 {
        return return_string(env, "Error: invalid parameter");
    }
    let path = match read_napi_string(env, args[0]) {
        Some(p) => p,
        None => return return_string(env, "Error: invalid parameter"),
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(crate::read::read_file(&path)) {
        Ok(result) => {
            let content = result.content;
            let mut output = format!(
                "Read {} bytes successfully\nSize: {}\n\n",
                content.len(),
                format_size(content.len() as u64)
            );
            if !content.is_empty() {
                output.push_str("Content:\n");
                const MAX_DISPLAY: usize = 1000;
                let display_len = content.len().min(MAX_DISPLAY);
                if is_text(&content, display_len) {
                    if let Ok(text) = std::str::from_utf8(&content[..display_len]) {
                        output.push_str(text);
                        if content.len() > MAX_DISPLAY {
                            output.push_str(&format!(
                                "\n... (truncated, total {} bytes)",
                                content.len()
                            ));
                        }
                    } else {
                        output.push_str("[Binary data - invalid UTF-8]\n");
                    }
                } else {
                    output.push_str("[Binary data]\n");
                    let hex_len = display_len.min(256);
                    for (i, &b) in content[..hex_len].iter().enumerate() {
                        output.push_str(&format!("{:02x} ", b));
                        if (i + 1) % 16 == 0 {
                            output.push('\n');
                        }
                    }
                    if content.len() > hex_len {
                        output.push_str(&format!(
                            "\n... (truncated, total {} bytes)",
                            content.len()
                        ));
                    }
                }
            }
            return_string(env, &output)
        }
        Err(e) => {
            vfs_log_error!("read_file failed: {}", e.message);
            return_string(env, &format!("Error: {} - {}", e.code.as_i32(), e.message))
        }
    }
}

extern "C" fn upload_file(env: NapiEnv, info: NapiCallbackInfo) -> NapiValue {
    let (argc, args) = unsafe { get_cb_args(env, info) };
    if argc < 1 {
        return return_string(env, "Error: invalid parameter");
    }
    let path = match read_napi_string(env, args[0]) {
        Some(p) => p,
        None => return return_string(env, "Error: invalid parameter"),
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(crate::upload::upload_file(&path)) {
        Ok(_) => return_string(env, "Upload file successfully!"),
        Err(e) => {
            vfs_log_error!("upload_file failed: {}", e.message);
            return_string(env, &format!("Upload failed with error code: {}", e.code.as_i32()))
        }
    }
}

extern "C" fn write_file(env: NapiEnv, info: NapiCallbackInfo) -> NapiValue {
    let (argc, args) = unsafe { get_cb_args(env, info) };
    if argc < 2 {
        return return_string(env, "Error: need path and content arguments");
    }
    let path = match read_napi_string(env, args[0]) {
        Some(p) => p,
        None => return return_string(env, "Error: invalid path"),
    };
    let content = read_byte_array(env, args[1]);
    let content_len = content.len();
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(crate::write::write_file(&path, &content)) {
        Ok(_) => return_string(
            env,
            &format!("Write file successfully! Wrote {} bytes.", content_len),
        ),
        Err(e) => {
            vfs_log_error!("write_file failed: {}", e.message);
            return_string(env, &format!("Write failed with error code: {}", e.code.as_i32()))
        }
    }
}

extern "C" fn rm_file(env: NapiEnv, info: NapiCallbackInfo) -> NapiValue {
    let (argc, args) = unsafe { get_cb_args(env, info) };
    if argc < 1 {
        return return_string(env, "Error: invalid parameter");
    }
    let path = match read_napi_string(env, args[0]) {
        Some(p) => p,
        None => return return_string(env, "Error: invalid parameter"),
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(crate::rm::rm_file(&path)) {
        Ok(true) => return_string(env, "Delete file successfully!"),
        Ok(false) => return_string(
            env,
            &format!("Delete failed with error code: {}", crate::error::ErrorCode::PathNotFound.as_i32()),
        ),
        Err(e) => {
            vfs_log_error!("rm_file failed: {}", e.message);
            return_string(env, &format!("Delete failed with error code: {}", e.code.as_i32()))
        }
    }
}

extern "C" fn mk_dir(env: NapiEnv, info: NapiCallbackInfo) -> NapiValue {
    let (argc, args) = unsafe { get_cb_args(env, info) };
    if argc < 1 {
        return return_string(env, "Error: invalid parameter");
    }
    let path = match read_napi_string(env, args[0]) {
        Some(p) => p,
        None => return return_string(env, "Error: invalid parameter"),
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(crate::mkdir::mk_dir(&path)) {
        Ok(_) => return_string(env, "Create directory successfully!"),
        Err(e) => {
            vfs_log_error!("mk_dir failed: {}", e.message);
            return_string(env, &format!("Create directory failed with error code: {}", e.code.as_i32()))
        }
    }
}

extern "C" fn stat_file(env: NapiEnv, info: NapiCallbackInfo) -> NapiValue {
    let (argc, args) = unsafe { get_cb_args(env, info) };
    if argc < 1 {
        return return_string(env, "Error: invalid parameter");
    }
    let path = match read_napi_string(env, args[0]) {
        Some(p) => p,
        None => return return_string(env, "Error: invalid parameter"),
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(crate::stat::stat_file(&path)) {
        Ok(result) => {
            if result.error_code == crate::error::ErrorCode::Success {
                let output = format!(
                    "File Stats:\n  Size: {}\n  Is File: {}\n  Is Dir: {}\n  Modified: {}",
                    format_size(result.size),
                    if result.is_file { "Yes" } else { "No" },
                    if result.is_dir { "Yes" } else { "No" },
                    format_time(result.modified_time)
                );
                return_string(env, &output)
            } else {
                let msg = result.error_message.as_deref().unwrap_or("Unknown error");
                return_string(env, &format!("Error: {} - {}", result.error_code.as_i32(), msg))
            }
        }
        Err(e) => {
            vfs_log_error!("stat_file failed: {}", e.message);
            return_string(env, &format!("Error: {} - {}", e.code.as_i32(), e.message))
        }
    }
}

extern "C" fn bind_server(env: NapiEnv, _info: NapiCallbackInfo) -> NapiValue {
    match crate::channel::bind_server() {
        Ok(_) => return_string(env, "WebSocket bind server successfully!"),
        Err(e) => {
            vfs_log_error!("bind_server failed: {}", e);
            return_string(
                env,
                &format!("Bind server failed with error code: {}", crate::error::ErrorCode::NetworkError.as_i32()),
            )
        }
    }
}

// ── Module registration ────────────────────────────────────────────

extern "C" fn init_module(env: NapiEnv, exports: NapiValue) -> NapiValue {
    vfs_log_error!("INIT_MODULE: called, env={:p}, exports={:p}", env, exports);
    let descriptors: [NapiPropertyDescriptor; 12] = [
        NapiPropertyDescriptor {
            utf8name: b"setWorkspace\0".as_ptr() as *const c_char,
            name: ptr::null_mut(), method: Some(set_workspace),
            getter: None, setter: None, value: ptr::null_mut(),
            attributes: 0, data: ptr::null_mut(),
        },
        NapiPropertyDescriptor {
            utf8name: b"setBasePath\0".as_ptr() as *const c_char,
            name: ptr::null_mut(), method: Some(set_base_path),
            getter: None, setter: None, value: ptr::null_mut(),
            attributes: 0, data: ptr::null_mut(),
        },
        NapiPropertyDescriptor {
            utf8name: b"setAt\0".as_ptr() as *const c_char,
            name: ptr::null_mut(), method: Some(set_at),
            getter: None, setter: None, value: ptr::null_mut(),
            attributes: 0, data: ptr::null_mut(),
        },
        NapiPropertyDescriptor {
            utf8name: b"listDir\0".as_ptr() as *const c_char,
            name: ptr::null_mut(), method: Some(list_dir),
            getter: None, setter: None, value: ptr::null_mut(),
            attributes: 0, data: ptr::null_mut(),
        },
        NapiPropertyDescriptor {
            utf8name: b"listCloudRaw\0".as_ptr() as *const c_char,
            name: ptr::null_mut(), method: Some(list_cloud_raw),
            getter: None, setter: None, value: ptr::null_mut(),
            attributes: 0, data: ptr::null_mut(),
        },
        NapiPropertyDescriptor {
            utf8name: b"readFile\0".as_ptr() as *const c_char,
            name: ptr::null_mut(), method: Some(read_file),
            getter: None, setter: None, value: ptr::null_mut(),
            attributes: 0, data: ptr::null_mut(),
        },
        NapiPropertyDescriptor {
            utf8name: b"uploadFile\0".as_ptr() as *const c_char,
            name: ptr::null_mut(), method: Some(upload_file),
            getter: None, setter: None, value: ptr::null_mut(),
            attributes: 0, data: ptr::null_mut(),
        },
        NapiPropertyDescriptor {
            utf8name: b"writeFile\0".as_ptr() as *const c_char,
            name: ptr::null_mut(), method: Some(write_file),
            getter: None, setter: None, value: ptr::null_mut(),
            attributes: 0, data: ptr::null_mut(),
        },
        NapiPropertyDescriptor {
            utf8name: b"rmFile\0".as_ptr() as *const c_char,
            name: ptr::null_mut(), method: Some(rm_file),
            getter: None, setter: None, value: ptr::null_mut(),
            attributes: 0, data: ptr::null_mut(),
        },
        NapiPropertyDescriptor {
            utf8name: b"mkDir\0".as_ptr() as *const c_char,
            name: ptr::null_mut(), method: Some(mk_dir),
            getter: None, setter: None, value: ptr::null_mut(),
            attributes: 0, data: ptr::null_mut(),
        },
        NapiPropertyDescriptor {
            utf8name: b"statFile\0".as_ptr() as *const c_char,
            name: ptr::null_mut(), method: Some(stat_file),
            getter: None, setter: None, value: ptr::null_mut(),
            attributes: 0, data: ptr::null_mut(),
        },
        NapiPropertyDescriptor {
            utf8name: b"bindServer\0".as_ptr() as *const c_char,
            name: ptr::null_mut(), method: Some(bind_server),
            getter: None, setter: None, value: ptr::null_mut(),
            attributes: 0, data: ptr::null_mut(),
        },
    ];
    let status = unsafe { napi_define_properties(env, exports, descriptors.len(), descriptors.as_ptr()) };
    vfs_log_error!("INIT_MODULE: napi_define_properties returned status={}", status);
    vfs_log_error!("INIT_MODULE: returning exports={:p}", exports);
    exports
}

const MODULE_NAME: &[u8] = b"libvfs_apis.so\0";

static MODULE: NapiModule = NapiModule {
    nm_version: 1,
    nm_flags: 0,
    nm_filename: ptr::null(),
    nm_register_func: Some(init_module),
    nm_modname: MODULE_NAME.as_ptr() as *const c_char,
    nm_priv: ptr::null_mut(),
    reserved: [ptr::null_mut(); 4],
};

extern "C" fn napi_ctor() {
    vfs_log_error!("NAPI_CTOR: registering module, addr={:p}", &MODULE as *const _);
    unsafe { napi_module_register(&MODULE); }
    vfs_log_error!("NAPI_CTOR: module registered");
}

#[used]
#[link_section = ".init_array"]
static NAPI_CTOR: extern "C" fn() = napi_ctor;
