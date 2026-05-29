# Development

## Prerequisites

- Rust toolchain (edition 2021, `rust-version = 1.75.0` or newer).
- A running IPFS daemon reachable at `127.0.0.1:5001` for live uploads
  (`IpfsClient::default()`). Unit and integration tests use mocks and do not need it.
- AWS credentials and an S3 bucket for live uploads. Tests do not need them.

## Commands

```bash
cargo build                  # Debug build
cargo build --release        # Release build (LTO, single codegen unit, stripped)
cargo check                  # Fast type/syntax check
cargo run                    # Run the service (reads .env)

cargo test                   # Unit + integration tests (no network/AWS needed)
cargo test <name>            # Run a single test
cargo test -- --nocapture    # Show test stdout/stderr

cargo clippy -- -D warnings  # Lint, warnings as errors
cargo fmt                    # Format
cargo fmt -- --check         # Verify formatting
```

`.cargo/config.toml` sets `rustflags = ["-D", "warnings"]`, so warnings fail every
build. It also defines convenience aliases:

```bash
cargo check-all              # check --all-targets --all-features
cargo test-all               # test --all-targets --all-features
cargo clippy-all             # clippy with pedantic + nursery lints denied
cargo fmt-check              # fmt -- --check
```

Formatting is governed by `rustfmt.toml` (max width 100, crate-granularity imports,
`StdExternalCrate` grouping) and linting by `clippy.toml`.

## Project layout

```
src/
  main.rs              Binary entry point: load + validate config, build adapters, serve
  lib.rs               Library re-exports (enables integration tests under tests/)
  config.rs            Config loading and validation
  error.rs             StorageError, HTTP status/code mapping, error envelope
  api/
    mod.rs
    upload.rs          Warp routes, request-ID filter, rejection recovery
  domain/
    mod.rs
    ports.rs           ObjectStorage, ContentAddressedStorage traits
    services.rs        handle_upload orchestration
  infrastructure/
    mod.rs
    s3.rs              S3Uploader (ObjectStorage adapter)
    ipfs.rs            IpfsUploader (ContentAddressedStorage adapter)
  utils/
    mod.rs
    file.rs            Standalone file-system helpers
tests/
  upload.rs            Integration tests against mock backends
openapi.yaml           HTTP contract
docs/                  This documentation
```

## Testing approach

Tests rely on the port traits rather than real backends:

- `tests/upload.rs` builds the real `upload_routes` filter with `MockS3` / `MockIpfs`
  implementations of `ObjectStorage` / `ContentAddressedStorage`, then drives it with
  `warp::test::request()`. It covers success, S3-only failure, IPFS-only failure,
  both-fail, missing `file` part, oversized body (asserting the temp file is cleaned
  up), filename sanitization, wrong method (405), and unknown path (404).
- Unit tests live alongside their modules: `config.rs` (validation, host and temp-dir
  rules), `error.rs` (status/code mapping), `api/upload.rs` (route method/path),
  `services.rs` (sanitization, key generation, response serialization), `s3.rs`
  (URL formatting), and `utils/file.rs`.

This is the reason for the hexagonal split: business logic is verifiable without
network access, AWS credentials, or a local IPFS daemon.

## Adding a storage backend

1. Implement the relevant port trait from `src/domain/ports.rs` (`ObjectStorage` for
   URL-addressed stores, `ContentAddressedStorage` for hash-addressed stores) on a new
   adapter under `src/infrastructure/`.
2. Construct the adapter in `main.rs` and pass it into `upload_routes`. The routes and
   `handle_upload` are generic over the traits, so no domain changes are required to
   substitute an implementation.
3. If the new backend participates in the concurrent upload, extend the result
   classification in `handle_upload` and the corresponding `StorageError` variants.

## Conventions

- Conventional Commits (`feat:`, `fix:`, `docs:`, `chore:`).
- All changes land on `main` through pull requests.
- Public items carry `///` doc comments with `# Arguments` / `# Returns` where relevant.
- `anyhow::Result` for propagation in adapters; `StorageError` + `warp::reject` at the
  HTTP boundary.
- Native `async fn` in traits (RPITIT); no `async_trait`.
