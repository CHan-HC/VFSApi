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

/// Establish a P2P connection following the full flow:
/// init → register_ids → query_ids → connect.
///
/// Returns the connected peer token on success.
pub fn p2p_connect(
    ids_url: &str,
    nat_url: &str,
    app_id: &str,
    user_id: &str,
    odid: &str,
) -> Result<String, String> {
    vfs_log_info!("[P2P] p2p_connect start: app_id={}, user_id={}", app_id, user_id);

    let client = get_client();
    let mut guard = client.lock().map_err(|e| format!("lock: {e}"))?;

    guard.init(Config {
        ids_url: ids_url.to_string(),
        nat_url: nat_url.to_string(),
    });

    // Register on_data callback → pushes received messages to the shared queue
    {
        let queue = get_received_queue().clone();
        guard.on_data(Box::new(move |payload: Vec<u8>| {
            let text = String::from_utf8_lossy(&payload).to_string();
            vfs_log_info!("[P2P] on_data received: {}", text);
            if let Ok(mut q) = queue.lock() {
                q.push(text);
            }
        }));
    }

    let http = SyncHttpTransport::new();

    // Step 1: Register IDS
    vfs_log_info!("[P2P] registering IDS...");
    guard
        .register_ids(&http, app_id, user_id, odid, "")
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
    vfs_log_info!("[P2P] connect initiated, waiting for ICE...");

    Ok(peer.token)
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
    let client = get_client();
    let guard = client.lock().map_err(|e| format!("lock: {e}"))?;
    guard.send_text(text).map_err(|e| format!("send_text: {e}"))
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
