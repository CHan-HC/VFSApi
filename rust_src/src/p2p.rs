use std::sync::mpsc;
use std::sync::Mutex;
use std::sync::Arc;
use std::time::Duration;

use once_cell::sync::OnceCell;
use p2p_sdk::{Config, IceState, P2pClient};
use p2p_tokio::SyncHttpTransport;

use crate::{vfs_log_error, vfs_log_info};

static P2P_CLIENT: OnceCell<Mutex<P2pClient>> = OnceCell::new();

fn get_client() -> &'static Mutex<P2pClient> {
    P2P_CLIENT.get_or_init(|| Mutex::new(P2pClient::new()))
}

// ── Received message queue (bridge from on_data callback to ArkTS polling) ──

static RECEIVED_QUEUE: OnceCell<Arc<Mutex<Vec<String>>>> = OnceCell::new();

fn get_received_queue() -> &'static Arc<Mutex<Vec<String>>> {
    RECEIVED_QUEUE.get_or_init(|| Arc::new(Mutex::new(Vec::new())))
}

/// Drain and return all received messages from the on_data callback.
pub fn p2p_poll_messages() -> Vec<String> {
    let queue = get_received_queue();
    let mut guard = match queue.lock() {
        Ok(g) => g,
        Err(_) => return vec!["lock error".to_string()],
    };
    let msgs = guard.clone();
    guard.clear();
    msgs
}

/// Standalone register_ids call for testing.
/// Initializes the client then registers IDS.
/// The `extra` parameter is passed through to register_ids.
pub fn p2p_register_ids(
    ids_url: &str,
    nat_url: &str,
    app_id: &str,
    user_id: &str,
    odid: &str,
    extra: &str,
) -> Result<String, String> {
    vfs_log_info!("[P2P] p2p_register_ids start: app_id={}, user_id={}", app_id, user_id);

    let client = get_client();
    let mut guard = client.lock().map_err(|e| format!("lock: {e}"))?;

    guard.init(Config {
        ids_url: ids_url.to_string(),
        nat_url: nat_url.to_string(),
    });

    let http = SyncHttpTransport::new();

    vfs_log_info!("[P2P] registering IDS (standalone)...");
    guard
        .register_ids(&http, app_id, user_id, odid, extra)
        .map_err(|e| {
            vfs_log_error!("[P2P] register_ids failed: {}", e);
            format!("register_ids: {e}")
        })?;
    vfs_log_info!("[P2P] register_ids success");

    Ok("register_ids OK".to_string())
}

/// Single entry point for push message handling.
///
/// 1. Process the message via `handle_push_message` (file_list / sync request)
/// 2. If P2P ICE is already Connected/Completed → send result directly
/// 3. Otherwise → establish P2P connection, auto-send result when ICE ready
///
/// `on_data` is always registered during connection, so subsequent P2P
/// messages from the peer are handled automatically.
pub fn p2p_handle_push_message(
    ids_url: &str,
    nat_url: &str,
    app_id: &str,
    user_id: &str,
    odid: &str,
    push_token: &str,
    payload: &str,
) -> String {
    vfs_log_info!("[P2P] p2p_handle_push_message: payload_len={}", payload.len());

    // Step 1: Process the push message (file_list_request / sync_request)
    let result = handle_push_message(payload);
    vfs_log_info!("[P2P] p2p_handle_push_message: result_len={}", result.len());

    // Step 2: Check if P2P is already connected
    if p2p_is_ready() {
        vfs_log_info!("[P2P] already connected, sending result directly");
        match p2p_send_text(&result) {
            Ok(()) => vfs_log_info!("[P2P] direct send OK"),
            Err(e) => vfs_log_error!("[P2P] direct send failed: {}", e),
        }
    } else {
        vfs_log_info!("[P2P] not connected, establishing P2P connection with auto-send");
        match p2p_connect(ids_url, nat_url, app_id, user_id, odid, push_token, &result) {
            Ok(peer) => vfs_log_info!("[P2P] connect initiated, peer={}, will auto-send when ICE ready", peer),
            Err(e) => vfs_log_error!("[P2P] connect failed: {}", e),
        }
    }

    result
}

/// Establish a P2P connection following the full flow:
/// init → register_ids → query_ids → connect.
///
/// If `send_on_ready` is non-empty, an `on_state_change` callback is registered
/// that watches for ICE Connected/Completed, then auto-sends the message via
/// `p2p_send_text`. The callback fires on the SDK's ICE tick thread, so a
/// channel + background thread pattern is used to avoid locking issues.
///
/// Returns the connected peer token on success (non-blocking).
pub fn p2p_connect(
    ids_url: &str,
    nat_url: &str,
    app_id: &str,
    user_id: &str,
    odid: &str,
    push_token: &str,
    send_on_ready: &str,
) -> Result<String, String> {
    vfs_log_info!("[P2P] p2p_connect start: app_id={}, user_id={}, push_token={}", app_id, user_id, push_token);

    let client = get_client();
    let mut guard = client.lock().map_err(|e| format!("lock: {e}"))?;

    guard.init(Config {
        ids_url: ids_url.to_string(),
        nat_url: nat_url.to_string(),
    });

    // Register on_data callback → process messages like handlePushMessage and reply via P2P.
    // fire_on_data now drops the lock before calling the callback, so p2p_send_text is safe.
    {
        let queue = get_received_queue().clone();
        guard.on_data(Box::new(move |payload: Vec<u8>| {
            let text = String::from_utf8_lossy(&payload).to_string();
            vfs_log_info!("[P2P] on_data received: {}", text);
            // Also push to queue for p2p_poll_messages compatibility
            if let Ok(mut q) = queue.lock() {
                q.push(text.clone());
            }
            // Process the message with the same logic as handlePushMessage
            let response = crate::p2p::handle_push_message(&text);
            vfs_log_info!("[P2P] on_data response: {}", response);
            // Send response back via P2P
            match crate::p2p::p2p_send_text(&response) {
                Ok(()) => vfs_log_info!("[P2P] on_data reply OK"),
                Err(e) => vfs_log_error!("[P2P] on_data reply failed: {}", e),
            }
        }));
    }

    // Register on_state_change callback → auto-send when ICE is ready.
    // fire_state_change now drops the lock before calling the callback,
    // so it's safe to call p2p_send_text directly here (no deadlock).
    let send_msg = send_on_ready.to_string();
    if !send_msg.is_empty() {
        vfs_log_info!("[P2P] registering on_state_change for auto-send, msg_len={}", send_msg.len());
        guard.on_state_change(Box::new(move |state: IceState| {
            vfs_log_info!("[P2P] on_state_change callback: state={}", state);
            if state == IceState::Connected || state == IceState::Completed {
                vfs_log_info!("[P2P] ICE ready, auto-sending text: {}", send_msg);
                match crate::p2p::p2p_send_text(&send_msg) {
                    Ok(()) => vfs_log_info!("[P2P] auto-send OK"),
                    Err(e) => vfs_log_error!("[P2P] auto-send failed: {}", e),
                }
            } else if state == IceState::Failed || state == IceState::Closed {
                vfs_log_error!("[P2P] ICE {}, abort auto-send", state);
            }
        }));
    }

    let http = SyncHttpTransport::new();

    // Step 1: Register IDS
    vfs_log_info!("[P2P] registering IDS...");
    guard
        .register_ids(&http, app_id, user_id, odid, push_token)
        .map_err(|e| {
            vfs_log_error!("[P2P] register_ids failed: {}", e);
            format!("register_ids: {e}")
        })?;
    vfs_log_info!("[P2P] register_ids success");

    // Step 2: Query IDS
    vfs_log_info!("[P2P] querying IDS...");
    let peer = guard
        .query_ids(&http, app_id, user_id)
        .map_err(|e| {
            vfs_log_error!("[P2P] query_ids failed: {}", e);
            format!("query_ids: {e}")
        })?;

    if peer.token.is_empty() {
        vfs_log_error!("[P2P] no peer found");
        return Err("no peer found".to_string());
    }
    vfs_log_info!("[P2P] peer found: {}", peer.token);

    // Step 3: One-click connect (non-blocking, background thread)
    vfs_log_info!("[P2P] connecting to peer...");
    guard.connect(&peer.token, odid, 30).map_err(|e| {
        vfs_log_error!("[P2P] connect failed: {}", e);
        format!("connect: {e}")
    })?;
    drop(guard);
    vfs_log_info!("[P2P] connect initiated (non-blocking), peer={}", peer.token);

    Ok(peer.token)
}

/// Check whether ICE is ready for sending data (Connected or Completed).
pub fn p2p_is_ready() -> bool {
    let state = p2p_ice_state();
    matches!(state.as_str(), "Connected" | "Completed")
}

/// Get current ICE state. Returns "NONE" if not initialized.
pub fn p2p_ice_state() -> String {
    match get_client().lock() {
        Ok(guard) => match guard.ice_state() {
            Some(state) => format!("{state}"),
            None => "NONE".to_string(),
        },
        Err(_) => "LOCK_ERROR".to_string(),
    }
}

/// Close the P2P connection and release resources.
pub fn p2p_close() -> Result<(), String> {
    let client = get_client();
    let guard = client.lock().map_err(|e| format!("lock: {e}"))?;
    guard.close().map_err(|e| format!("close: {e}"))
}

/// Send text through the established P2P channel.
pub fn p2p_send_text(text: &str) -> Result<(), String> {
    vfs_log_info!("[P2P] p2p_send_text: text_len={}", text.len());
    let client = get_client();
    let guard = client.lock().map_err(|e| {
        vfs_log_error!("[P2P] p2p_send_text: client lock error: {}", e);
        format!("lock: {e}")
    })?;
    let ice_state = guard.ice_state().map(|s| format!("{s}")).unwrap_or_else(|| "NONE".to_string());
    vfs_log_info!("[P2P] p2p_send_text: ICE state before send={}", ice_state);
    match guard.send_text(text) {
        Ok(()) => {
            vfs_log_info!("[P2P] p2p_send_text: OK");
            Ok(())
        }
        Err(e) => {
            vfs_log_error!("[P2P] p2p_send_text failed: {}", e);
            Err(format!("send_text: {e}"))
        }
    }
}

/// Full integration test: init → register_ids → query_ids → connect → wait for ICE.
/// Returns a formatted string with the test results.
pub fn p2p_integration_test() -> String {
    vfs_log_info!("[P2P] integration test start");

    let config = Config {
        ids_url: String::new(),
        nat_url: String::new(),
    };

    let mut client = P2pClient::new();
    client.init(config);

    // Set up channels for state and data callbacks
    let (state_tx, state_rx) = mpsc::channel::<IceState>();
    let (data_tx, data_rx) = mpsc::channel::<String>();

    client.on_state_change(Box::new(move |state: IceState| {
        let _ = state_tx.send(state);
    }));

    client.on_data(Box::new(move |payload: Vec<u8>| {
        let text = String::from_utf8_lossy(&payload).to_string();
        let _ = data_tx.send(text);
    }));

    let http = SyncHttpTransport::new();

    // Placeholder config — replace with real values for actual testing
    let app_id = "test_app";
    let user_id = "test_user";
    let odid = "test_odid";

    let mut output = String::new();
    output.push_str("=== P2P Integration Test ===\n\n");

    // Step 1: Register IDS
    output.push_str("[1/3] Register IDS... ");
    match client.register_ids(&http, app_id, user_id, odid, "") {
        Ok(()) => output.push_str("OK\n"),
        Err(e) => {
            output.push_str(&format!("FAILED: {e}\n"));
            output.push_str("\nNote: IDS URL not configured. Set ids_url/nat_url for real P2P test.\n");
            return output;
        }
    }

    // Step 2: Query IDS
    output.push_str("[2/3] Query IDS... ");
    match client.query_ids(&http, app_id, user_id) {
        Ok(peer) => {
            if peer.token.is_empty() {
                output.push_str("no peer found\n");
                output.push_str("\nNote: No peer registered. Ensure both ends are configured.\n");
                return output;
            }
            output.push_str(&format!("found peer: {}\n", peer.token));

            // Step 3: Connect
            output.push_str("[3/3] Connect... ");
            match client.connect(&peer.token, odid, 30) {
                Ok(()) => output.push_str("initiated, waiting for ICE...\n"),
                Err(e) => {
                    output.push_str(&format!("FAILED: {e}\n"));
                    return output;
                }
            }

            // Wait for ICE state
            output.push_str("\nWaiting for ICE negotiation (30s timeout)...\n");
            match state_rx.recv_timeout(Duration::from_secs(30)) {
                Ok(state) => {
                    output.push_str(&format!("ICE State: {state}\n"));
                    if state == IceState::Completed || state == IceState::Connected {
                        output.push_str("P2P connection established!\n");

                        // Check for any received data
                        match data_rx.try_recv() {
                            Ok(data) => output.push_str(&format!("Received: {data}\n")),
                            Err(_) => {}
                        }
                    }
                }
                Err(_) => output.push_str("ICE negotiation timed out\n"),
            }
        }
        Err(e) => {
            output.push_str(&format!("FAILED: {e}\n"));
        }
    }

    let _ = client.close();
    output.push_str("\n=== Test Complete ===");
    output
}

// ── Push message handler (mimics channel::on_message, but returns result via P2P) ──

/// Handle a push message: parse JSON, route by `type` field, and return the response
/// string (to be sent via P2P by the caller). Logic mirrors `channel::on_message`
/// but the response goes through the P2P channel instead of WebSocket.
pub fn handle_push_message(payload: &str) -> String {
    vfs_log_info!("[P2P] handle_push_message received: {}", payload);

    let parsed: serde_json::Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(e) => {
            vfs_log_error!("[P2P] handle_push_message: failed to parse JSON: {}", e);
            return format!("{{\"error\": \"Invalid JSON: {}\"}}", e);
        }
    };

    let msg_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match msg_type {
        "file_list_request" => handle_push_file_list_request(&parsed),
        "sync_request" => handle_push_sync_request(&parsed),
        _ => {
            vfs_log_info!("[P2P] handle_push_message: unhandled type: {}", msg_type);
            format!("{{\"error\": \"Unhandled message type: {}\"}}", msg_type)
        }
    }
}

fn handle_push_file_list_request(msg: &serde_json::Value) -> String {
    let request_id = match msg.get("request_id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => {
            vfs_log_error!("[P2P] file_list_request missing request_id");
            return r#"{"error": "missing request_id"}"#.to_string();
        }
    };
    let path = msg.get("path").and_then(|v| v.as_str()).unwrap_or("/");

    vfs_log_info!("[P2P] handle_push_file_list_request: request_id='{}', path='{}'", request_id, path);

    let manifest = match crate::list::get_local_manifest_sync(path) {
        Ok(m) => m,
        Err(e) => {
            vfs_log_error!("[P2P] get_local_manifest_sync failed: {}", e.message);
            return serde_json::json!({
                "type": "file_list_response",
                "request_id": request_id,
                "path": path,
                "error": e.message,
            }).to_string();
        }
    };

    let response = serde_json::json!({
        "type": "file_list_response",
        "request_id": request_id,
        "path": path,
        "manifest": manifest,
    });

    vfs_log_info!("[P2P] handle_push_file_list_request: response ready");
    response.to_string()
}

fn handle_push_sync_request(msg: &serde_json::Value) -> String {
    let request_id = match msg.get("request_id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => {
            vfs_log_error!("[P2P] sync_request missing request_id");
            return r#"{"error": "missing request_id"}"#.to_string();
        }
    };
    let path = msg.get("path").and_then(|v| v.as_str()).unwrap_or("");
    let file_name = msg.get("fileName").and_then(|v| v.as_str()).unwrap_or("");

    vfs_log_info!("[P2P] handle_push_sync_request: request_id='{}', path='{}', fileName='{}'",
        request_id, path, file_name);

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            vfs_log_error!("[P2P] Failed to create tokio runtime: {}", e);
            return serde_json::json!({
                "type": "sync_response",
                "request_id": request_id,
                "path": path,
                "fileName": file_name,
                "error": format!("Internal error: {}", e),
            }).to_string();
        }
    };

    match rt.block_on(crate::upload::sync_file_to_cloud(path)) {
        Ok(result) => {
            vfs_log_info!("[P2P] handle_push_sync_request success: file_id={}", result.file_id);
            serde_json::json!({
                "type": "sync_response",
                "request_id": request_id,
                "path": path,
                "fileName": file_name,
                "file_id": result.file_id,
                "sha256": result.sha256,
            }).to_string()
        }
        Err(e) => {
            vfs_log_error!("[P2P] sync_file_to_cloud failed: {}", e.message);
            serde_json::json!({
                "type": "sync_response",
                "request_id": request_id,
                "path": path,
                "fileName": file_name,
                "error": e.message,
            }).to_string()
        }
    }
}
