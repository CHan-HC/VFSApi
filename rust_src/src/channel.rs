use std::ffi::{c_char, CStr, CString};
use std::sync::Mutex;
use crate::{vfs_log_debug, vfs_log_error, vfs_log_info};

/// WebSocket server URL — fill in the actual ws:// address, e.g. "ws://192.168.1.100:8080/"
const SERVER_URL: &str = "ws://81.71.29.250:31080/ws/chan";

/// Stored WebSocket client pointer (as usize for Send + Sync) so callbacks can send messages back.
static CLIENT: Mutex<Option<usize>> = Mutex::new(None);

// ── FFI type definitions (matching net_websocket_type.h) ──────────────

#[repr(C)]
struct WebSocket_OpenResult {
    code: u32,
    reason: *const c_char,
}

#[repr(C)]
struct WebSocket_CloseResult {
    code: u32,
    reason: *const c_char,
}

#[repr(C)]
struct WebSocket_ErrorResult {
    error_code: u32,
    error_message: *const c_char,
}

#[repr(C)]
struct WebSocket_CloseOption {
    code: u32,
    reason: *const c_char,
}

#[repr(C)]
struct WebSocket_Header {
    field_name: *const c_char,
    field_value: *const c_char,
    next: *mut WebSocket_Header,
}

#[repr(C)]
struct WebSocket_RequestOptions {
    headers: *mut WebSocket_Header,
}

#[repr(C)]
struct WebSocket {
    _private: [u8; 0],
}

type OnOpenCb = Option<extern "C" fn(*mut WebSocket, WebSocket_OpenResult)>;
type OnMessageCb = Option<extern "C" fn(*mut WebSocket, *mut c_char, u32)>;
type OnErrorCb = Option<extern "C" fn(*mut WebSocket, WebSocket_ErrorResult)>;
type OnCloseCb = Option<extern "C" fn(*mut WebSocket, WebSocket_CloseResult)>;

// ── FFI function declarations (net_websocket.h) ─────────────────────

extern "C" {
    fn OH_WebSocketClient_Constructor(
        on_open: OnOpenCb,
        on_message: OnMessageCb,
        on_error: OnErrorCb,
        on_close: OnCloseCb,
    ) -> *mut WebSocket;

    fn OH_WebSocketClient_Connect(
        client: *mut WebSocket,
        url: *const c_char,
        options: WebSocket_RequestOptions,
    ) -> i32;

    #[allow(dead_code)]
    fn OH_WebSocketClient_Send(
        client: *mut WebSocket,
        data: *mut c_char,
        length: usize,
    ) -> i32;

    #[allow(dead_code)]
    fn OH_WebSocketClient_Close(
        client: *mut WebSocket,
        options: WebSocket_CloseOption,
    ) -> i32;

    #[allow(dead_code)]
    fn OH_WebSocketClient_Destroy(client: *mut WebSocket) -> i32;
}

// ── Cellback implementations ─────────────────────────────────────────

extern "C" fn on_open(_client: *mut WebSocket, result: WebSocket_OpenResult) {
    let reason = unsafe { CStr::from_ptr(result.reason) }.to_string_lossy();
    vfs_log_info!("WebSocket onOpen: code={}, reason={}", result.code, reason);
}

extern "C" fn on_message(client: *mut WebSocket, data: *mut c_char, length: u32) {
    let bytes = unsafe { std::slice::from_raw_parts(data as *const u8, length as usize) };
    let msg_text = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => {
            vfs_log_debug!("WebSocket onMessage: {} bytes (binary)", bytes.len());
            return;
        }
    };
    vfs_log_debug!("WebSocket onMessage: {}", msg_text);

    let parsed: serde_json::Value = match serde_json::from_str(msg_text) {
        Ok(v) => v,
        Err(e) => {
            vfs_log_debug!("Failed to parse message as JSON: {}", e);
            return;
        }
    };

    let msg_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match msg_type {
        "file_list_request" => handle_file_list_request(client, &parsed),
        "sync_request" => handle_sync_request(client, &parsed),
        _ => vfs_log_debug!("Unhandled message type: {}", msg_type),
    }
}

fn handle_file_list_request(client: *mut WebSocket, msg: &serde_json::Value) {
    let request_id = match msg.get("request_id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => {
            vfs_log_error!("file_list_request missing request_id");
            return;
        }
    };
    let path = msg.get("path").and_then(|v| v.as_str()).unwrap_or("/");

    vfs_log_info!(">>> handle_file_list_request: request_id='{}', path='{}'", request_id, path);

    let manifest = match crate::list::get_local_manifest_sync(path) {
        Ok(m) => m,
        Err(e) => {
            vfs_log_error!("get_local_manifest_sync failed: {}", e.message);
            return;
        }
    };

    let response = serde_json::json!({
        "type": "file_list_response",
        "request_id": request_id,
        "path": path,
        "manifest": manifest,
    });

    let response_str = response.to_string();
    vfs_log_info!("Sending file_list_response: {}", response_str);
    send_ws_message(client, &response_str);
}

fn handle_sync_request(client: *mut WebSocket, msg: &serde_json::Value) {
    let request_id = match msg.get("request_id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => {
            vfs_log_error!("sync_request missing request_id");
            return;
        }
    };
    let path = msg.get("path").and_then(|v| v.as_str()).unwrap_or("");
    let file_name = msg.get("fileName").and_then(|v| v.as_str()).unwrap_or("");

    vfs_log_info!(">>> handle_sync_request: request_id='{}', path='{}', fileName='{}'",
        request_id, path, file_name);

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            vfs_log_error!("Failed to create tokio runtime: {}", e);
            let response = serde_json::json!({
                "type": "sync_response",
                "request_id": request_id,
                "path": path,
                "fileName": file_name,
                "error": format!("Internal error: {}", e),
            });
            send_ws_message(client, &response.to_string());
            return;
        }
    };

    match rt.block_on(crate::upload::sync_file_to_cloud(path)) {
        Ok(result) => {
            let response = serde_json::json!({
                "type": "sync_response",
                "request_id": request_id,
                "path": path,
                "fileName": file_name,
                "file_id": result.file_id,
                "sha256": result.sha256,
            });
            vfs_log_info!("Sending sync_response (success): file_id={}", result.file_id);
            send_ws_message(client, &response.to_string());
        }
        Err(e) => {
            vfs_log_error!("sync_file_to_cloud failed: {}", e.message);
            let response = serde_json::json!({
                "type": "sync_response",
                "request_id": request_id,
                "path": path,
                "fileName": file_name,
                "error": e.message,
            });
            send_ws_message(client, &response.to_string());
        }
    }
}

/// Send a text message through the WebSocket client.
fn send_ws_message(client: *mut WebSocket, text: &str) {
    let c_str = match CString::new(text) {
        Ok(s) => s,
        Err(e) => {
            vfs_log_error!("Failed to create CString: {}", e);
            return;
        }
    };

    let data_len = c_str.as_bytes().len();
    let data_ptr = c_str.into_raw() as *mut c_char;

    let ret = unsafe {
        OH_WebSocketClient_Send(client, data_ptr, data_len)
    };

    unsafe {
        let _ = CString::from_raw(data_ptr);
    }

    if ret != 0 {
        vfs_log_error!("OH_WebSocketClient_Send failed: error_code={}", ret);
    }
}

extern "C" fn on_error(_client: *mut WebSocket, result: WebSocket_ErrorResult) {
    let msg = unsafe { CStr::from_ptr(result.error_message) }.to_string_lossy();
    vfs_log_error!("WebSocket onError: code={}, message={}", result.error_code, msg);
}

extern "C" fn on_close(_client: *mut WebSocket, result: WebSocket_CloseResult) {
    let reason = unsafe { CStr::from_ptr(result.reason) }.to_string_lossy();
    vfs_log_info!("WebSocket onClose: code={}, reason={}", result.code, reason);
}

// ── Public API ──────────────────────────────────────────────────────

/// Bind to a WebSocket server and start listening for messages.
///
/// The server URL is hardcoded in the `SERVER_URL` constant above.
pub fn bind_server() -> Result<(), String> {
    if SERVER_URL.is_empty() {
        vfs_log_error!("SERVER_URL is empty, cannot connect");
        return Err("SERVER_URL is empty, please set the WebSocket server address".to_string());
    }

    vfs_log_info!(">>> bind_server START: url='{}'", SERVER_URL);

    let client = unsafe {
        OH_WebSocketClient_Constructor(
            Some(on_open),
            Some(on_message),
            Some(on_error),
            Some(on_close),
        )
    };

    if client.is_null() {
        vfs_log_error!("Failed to create WebSocket client");
        return Err("Failed to create WebSocket client".to_string());
    }
    vfs_log_debug!("WebSocket client created");

    // Store client pointer so callbacks can send messages back.
    if let Ok(mut guard) = CLIENT.lock() {
        *guard = Some(client as usize);
    }

    let url_c = std::ffi::CString::new(SERVER_URL).map_err(|e| format!("Invalid URL: {}", e))?;
    let options = WebSocket_RequestOptions {
        headers: std::ptr::null_mut(),
    };

    let ret = unsafe {
        OH_WebSocketClient_Connect(client, url_c.as_ptr(), options)
    };

    if ret != 0 {
        vfs_log_error!("WebSocket connect failed: error_code={}", ret);
        return Err(format!("WebSocket connect failed: error_code={}", ret));
    }

    vfs_log_info!("<<< bind_server END: connected to {}", SERVER_URL);
    Ok(())
}
