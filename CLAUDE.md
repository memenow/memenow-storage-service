# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Rust microservice for concurrent file uploads to S3 and IPFS. Single endpoint: `POST /upload` (`multipart/form-data`, 5 MB default limit, configurable via `MAX_FILE_SIZE`). Uses Warp + Tokio.

## Architecture

Hexagonal (Ports & Adapters):
- `src/api/` — HTTP routes/handlers (Warp filters), per-request ID injection, rejection recovery (`upload.rs`)
- `src/domain/ports.rs` — Port traits: `ObjectStorage`, `ContentAddressedStorage`
- `src/domain/services.rs` — Business logic: `handle_upload` orchestrates concurrent S3+IPFS via `tokio::join!`
- `src/infrastructure/` — `S3Uploader` and `IpfsUploader` adapters
- `src/config.rs` — Env-based config loading + startup validation
- `src/error.rs` — `StorageError` mapped to HTTP status codes and stable error codes
- `src/utils/` — Standalone file I/O helpers (not used by the upload path)

`handle_upload` is generic over the port traits, which is what lets `tests/upload.rs` run the real handler against mock backends with no network or AWS. The upload uses `tokio::join!` (not `try_join!`) so that when one backend succeeds and the other fails, the success can be reported as a partial failure (502 `UPLOAD_PARTIAL_FAILURE`) instead of being discarded.

IPFS adapter wraps the synchronous `ipfs-api` crate with `spawn_blocking` + `block_on`, bounded by a semaphore (`IPFS_MAX_CONCURRENCY`) and a per-request timeout (aborted on timeout). The S3 adapter lazily initializes and caches its client (`OnceCell`) and applies a per-request timeout.

Full documentation lives in [`docs/`](docs/) — see `docs/architecture.md`, `docs/configuration.md`, `docs/api.md`, and `docs/development.md`. The HTTP contract is in `openapi.yaml`.

## Commands

```bash
cargo build                  # Debug build
cargo build --release        # Release build
cargo check                  # Fast type/syntax check
cargo clippy -- -D warnings  # Lint with warnings as errors
cargo fmt                    # Format
cargo test                   # Unit + integration tests (mock backends; no network/AWS)
```

`.cargo/config.toml` also defines aliases: `cargo check-all`, `cargo test-all`, `cargo clippy-all` (pedantic + nursery), `cargo fmt-check`.

## Environment Variables

Loaded from `.env` via dotenv; validated at startup by `Config::validate`. Required:
- `S3_BUCKET` — S3 bucket name (startup `ConfigError` if missing)
- `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` — AWS credentials (resolved by the AWS SDK credential chain)

Optional (defaults):
- `S3_KEY` — object key prefix (default: `uploads`)
- `AWS_REGION` — AWS region (default: `us-east-1`)
- `S3_UPLOAD_TIMEOUT_MS` (default: `30000`), `IPFS_UPLOAD_TIMEOUT_MS` (default: `60000`)
- `IPFS_MAX_CONCURRENCY` (default: `4`)
- `MAX_FILE_SIZE` bytes (default: `5242880`; upper limit 1 GiB)
- `TEMP_DIR` (default: `/tmp`), `SERVER_HOST` (default: `0.0.0.0`), `SERVER_PORT` (default: `8080`)

See `docs/configuration.md` for the full table and validation rules.

## Conventions

- **Commits**: Conventional Commits (`feat:`, `fix:`, `docs:`, `chore:`)
- **Error handling**: `anyhow::Result` for propagation in adapters; `StorageError` + `warp::reject::Reject` at the HTTP boundary
- **Async traits**: native `async fn` in traits (RPITIT); no `async_trait`
- **Doc comments**: All public functions have `///` doc comments with Args/Returns
- **PRs**: All changes go through PRs against `main`
