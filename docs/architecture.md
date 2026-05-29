# Architecture

The service follows a hexagonal (Ports & Adapters) architecture. The domain layer
defines port traits; the infrastructure layer provides concrete adapters; the API
layer wires everything to HTTP. This keeps business logic independent of S3, IPFS,
and Warp, and makes the upload flow testable with in-memory mocks.

## Layers

| Layer | Path | Responsibility |
|-------|------|----------------|
| API | `src/api/` | Warp filters, request-ID injection, multipart intake, rejection recovery |
| Domain | `src/domain/` | Port traits (`ports.rs`) and upload orchestration (`services.rs`) |
| Infrastructure | `src/infrastructure/` | `S3Uploader` and `IpfsUploader` adapters |
| Configuration | `src/config.rs` | Loads and validates configuration from the environment |
| Errors | `src/error.rs` | `StorageError` enum mapped to HTTP status codes and stable error codes |
| Utilities | `src/utils/` | Standalone file-system helpers |
| Entry points | `src/main.rs`, `src/lib.rs` | Binary bootstrap; library re-exports for integration tests |

### Ports

`src/domain/ports.rs` defines two traits using native `async fn` in traits (RPITIT,
no `async_trait` crate):

- `ObjectStorage` — `upload(filepath, bucket, key) -> Result<String>` (returns a URL).
- `ContentAddressedStorage` — `upload(filepath) -> Result<String>` (returns a CID).

Both are bound `Send + Sync + 'static`. The domain depends only on these traits, so
backends can be swapped or mocked without touching business logic.

### Adapters

- `S3Uploader` (`infrastructure/s3.rs`) implements `ObjectStorage`. The AWS client and
  resolved region are initialized lazily and cached in a `tokio::sync::OnceCell`, so
  the first request pays the setup cost and later requests reuse the client. Each
  `put_object` call is wrapped in a `tokio::time::timeout`. The returned URL is
  region-aware (`get_s3_url`): `us-east-1` uses the bare `s3.amazonaws.com` host;
  other regions use `s3.<region>.amazonaws.com`.
- `IpfsUploader` (`infrastructure/ipfs.rs`) implements `ContentAddressedStorage`. The
  `ipfs-api` crate is synchronous, so each upload runs inside `tokio::task::spawn_blocking`
  with `futures::executor::block_on(client.add_path(...))`. A `tokio::sync::Semaphore`
  caps concurrent IPFS uploads (`IPFS_MAX_CONCURRENCY`). The blocking task is bounded
  by `tokio::time::timeout`; on timeout the join handle is aborted and the permit is
  released when the function returns. `IpfsClient::default()` targets a local daemon
  (`127.0.0.1:5001`).

## Upload flow

`domain::services::handle_upload` is the orchestrator. For one request:

1. **Receive** — a short request ID (8 hex chars from a UUID) is injected by the API
   filter and threaded through every log line as `request_id=...`.
2. **Extract & save** (`extract_and_save_file`) — the multipart stream is scanned for
   the part named `file`. Its filename is sanitized (see below). Bytes are streamed to
   a temp file at `<TEMP_DIR>/upload-<uuid>`, counting bytes as they arrive. If the
   running total exceeds `MAX_FILE_SIZE`, the temp file is deleted and the request
   fails with `PayloadTooLarge`. A missing `file` part yields `NoFileError`.
3. **Key generation** — the S3 object key is `<S3_KEY>/<uuid>_<sanitized-filename>`.
4. **Concurrent upload** — S3 and IPFS uploads run together via `tokio::join!`. Unlike
   `try_join!`, this awaits *both* futures even when one fails, so a side that succeeds
   is still observable and can be reported in a partial-failure response.
5. **Result classification** — the `(s3_result, ipfs_result)` pair maps to:
   - both `Ok` → success;
   - exactly one `Ok` → `PartialUploadFailure` carrying an `UploadSuccessSnapshot` of the
     side that succeeded;
   - both `Err` → `StorageBackendError` with backend `s3_and_ipfs`.
6. **Cleanup** — the temp file is always removed, regardless of upload outcome. A failed
   removal becomes a `CleanupError` (and is logged if it coincides with an upload error).
7. **Respond** — on full success the handler returns `200 OK` with an `UploadResponse`
   (`s3_url`, `ipfs_hash`, `filename`, `size`); otherwise it converts the captured
   `StorageError` into a Warp rejection.

### Filename sanitization

`sanitize_filename` replaces every character that is not ASCII alphanumeric, `.`, `-`,
or `_` with `_`, then trims the result. This neutralizes path-traversal input — for
example `../../../etc/passwd` becomes `.._.._.._etc_passwd`. An empty result, or a
name longer than 255 characters, is rejected as a `ValidationError`.

## Concurrency model

- The Tokio multi-threaded runtime (`#[tokio::main]`) drives Warp.
- Per request, S3 and IPFS run concurrently via `tokio::join!`.
- IPFS work is offloaded to the blocking pool (`spawn_blocking`) because the client is
  synchronous, and globally throttled by a semaphore.
- Per-backend timeouts (`S3_UPLOAD_TIMEOUT_MS`, `IPFS_UPLOAD_TIMEOUT_MS`) bound how long
  a single request can wait on each backend.

## Error model

`StorageError` (`src/error.rs`) is the single error type. Each variant maps to:

- a stable HTTP status (`status_code`),
- a stable machine-readable code (`error_code`, e.g. `UPLOAD_PARTIAL_FAILURE`),
- an optional `details` object (`error_details`).

Errors implement `warp::reject::Reject`. The API layer's `handle_rejection` renders any
`StorageError` — plus framework rejections (payload too large, method not allowed,
unsupported media type, not found) — into the uniform `ErrorResponse` JSON envelope.
See [api.md](api.md) for the full matrix.

## Testing seams

Because the domain is generic over the port traits, `tests/upload.rs` exercises the real
`handle_upload` and Warp wiring against `MockS3` / `MockIpfs` implementations, with no
network or AWS credentials required. This is the primary reason the ports exist.
