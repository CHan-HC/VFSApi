# VFSApi

Rust SDK providing a virtual file system that fuses local workspace with Huawei Cloud Drive.  Applications operate on a unified path space — the SDK transparently merges local and cloud files under a single set of `read/write/list_dir/rm/mkdir/stat` APIs.

## Architecture

```
┌────────────────────────────┐
│  Rust Caller (downstream)  │
├────────────────────────────┤
│  Public API (lib.rs)       │  read_file / write_file / list_dir / rm_file / mk_dir / stat_file
├────────────────────────────┤
│  Fusion Logic              │  read.rs / write.rs / list.rs / rm.rs / mkdir.rs / stat.rs
├────────────────────────────┤
│  Internal Tools            │  upload.rs (cloud sync) / workspace.rs (path resolution)
├────────────────────────────┤
│  Platform Layer            │  rcp.rs (HTTP via HarmonyOS RCP C API)
│                            │  hilog.rs (logging via HarmonyOS HiLog C API)
│                            │  channel.rs (WebSocket via HarmonyOS net_websocket C API)
├────────────────────────────┤
│  FFI Layer (ffi.rs)        │  extern "C" → loaded by HarmonyOS app via NAPI
└────────────────────────────┘
```

## Directory Structure

```
VFSApi/
├── rust_src/                  # Rust SDK
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs             # Public exports
│       ├── atmanager.rs       # AT token management
│       ├── channel.rs         # WebSocket client (Phase 9)
│       ├── error.rs           # Error types
│       ├── ffi.rs             # C FFI layer → HarmonyOS
│       ├── hilog.rs           # HiLog FFI bindings
│       ├── list.rs            # list_dir
│       ├── mkdir.rs           # mk_dir
│       ├── rcp.rs             # RCP HTTP client
│       ├── read.rs            # read_file
│       ├── rm.rs              # rm_file
│       ├── stat.rs            # stat_file
│       ├── upload.rs          # upload_file (internal)
│       ├── workspace.rs       # Workspace path management
│       └── write.rs           # write_file
├── harmony/                   # HarmonyOS test app
│   └── entry/src/main/
│       ├── cpp/
│       │   ├── napi_init.cpp  # NAPI → Rust FFI bridge
│       │   ├── CMakeLists.txt
│       │   └── types/libentry/Index.d.ts
│       └── ets/pages/
│           └── Index.ets      # Test UI
├── build_rust.sh              # Build Rust → HarmonyOS .so
└── project_spec.md            # Full spec
```

## Public API

| Function | Signature | Description |
|----------|-----------|-------------|
| `set_workspace` | `(path: &str) -> VfsResult<()>` | Set local workspace root directory |
| `set_at` | `(at: &str) -> VfsResult<()>` | Set Huawei Cloud AT token |
| `list_dir` | `(path: &str) -> VfsResult<ListDirResult>` | List files at path (merged local + cloud) |
| `read_file` | `(path: &str) -> VfsResult<ReadFileResult>` | Read file content (fusion strategy) |
| `write_file` | `(path: &str, content: &[u8]) -> VfsResult<()>` | Write bytes to file (local + cloud sync) |
| `rm_file` | `(path: &str) -> VfsResult<bool>` | Delete file (local + cloud) |
| `mk_dir` | `(path: &str) -> VfsResult<bool>` | Create directory (local + cloud) |
| `stat_file` | `(path: &str) -> VfsResult<StatFileResult>` | Get file metadata |
| `bind_server` | `() -> Result<(), String>` | Connect WebSocket server and listen |

## Fusion Strategy

The SDK maintains a merged view of local workspace and cloud storage (`applicationData` container).

### read_file

| Local | Cloud | Behavior |
|-------|-------|----------|
| ✓ | ✓ | Compare modified time, use the latest; sync the outdated side |
| ✓ | ✗ | Read from local |
| ✗ | ✓ | Download from cloud, save to local |
| ✗ | ✗ | Return error: `PathNotFound` |

### write_file

| Local | Cloud | Behavior |
|-------|-------|----------|
| ✓ | ✓ | Write local → upload to cloud |
| ✓ | ✗ | Write local → upload to cloud |
| ✗ | ✓ | Download cloud → write new content → upload to cloud |
| ✗ | ✗ | Write local → upload to cloud |

### list_dir

Merge local and cloud file lists. If a file exists in both, keep the entry with the newer timestamp.  Local-only and cloud-only files are both included.

### Path Alignment

Local paths mirror cloud paths: a file at `workspace/xxx/yyy/1.txt` corresponds to `applicationData/xxx/yyy/1.txt` on Huawei Cloud Drive.  Intermediate cloud folders are created automatically when uploading.

## Build

### Prerequisites

- Rust with `aarch64-unknown-linux-ohos` target
- DevEco Studio (for HarmonyOS test app)

### Build Rust Library

```bash
./build_rust.sh
```

This compiles the Rust crate to `aarch64-unknown-linux-ohos` and copies `libvfs_apis.so` to `harmony/entry/libs/arm64-v8a/`.

### Build HarmonyOS Test App

Open the `harmony/` directory in DevEco Studio and run on device, or use:

```bash
cd harmony
<DevEco-Studio-path>/tools/node/bin/node <DevEco-Studio-path>/tools/hvigor/bin/hvigorw.js assembleHap
```

## Testing

Each API has a corresponding button in the HarmonyOS test app (`Index.ets`):

- **get at** — OAuth login and workspace init
- **List Dir** — `list_dir("/xxx/zzz")`
- **Read File** — `read_file("/xxx/zzz/write_test.txt")`
- **Upload File** — `upload_file("/xxx/zzz/upload-0507")`
- **Write File** — `write_file` with byte array
- **Rm File** — `rm_file` deletion
- **MkDir** — `mk_dir` directory creation
- **Stat File** — `stat_file` metadata query
- **startBinding** — WebSocket bind server

## Dependencies

### Rust
- `tokio` (async runtime)
- `serde` / `serde_json` (JSON parsing)
- `chrono` (timestamp parsing)
- `urlencoding` (URL encoding)
- `libc` (FFI)

### HarmonyOS System Libraries
- `libace_napi.z.so` — NAPI bridge
- `libhilog_ndk.z.so` — Logging
- `librcp_c.so` — HTTP networking
- `libnet_websocket.so` — WebSocket client
