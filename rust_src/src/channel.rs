use std::ffi::{c_char, CStr};
use crate::{vfs_log_debug, vfs_log_error, vfs_log_info};

/// WebSocket server URL — fill in the actual ws:// address, e.g. "ws://192.168.1.100:8080/"
const SERVER_URL: &str = "";

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

extern "C" fn on_message(_client: *mut WebSocket, data: *mut c_char, length: u32) {
    let bytes = unsafe { std::slice::from_raw_parts(data as *const u8, length as usize) };
    match std::str::from_utf8(bytes) {
        Ok(msg) => vfs_log_debug!("WebSocket onMessage: {}", msg),
        Err(_) => vfs_log_debug!("WebSocket onMessage: {} bytes (binary)", bytes.len()),
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
