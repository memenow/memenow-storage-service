# memenow-storage-service

A Rust-based microservice for handling file uploads to Amazon S3 and IPFS.

## Description

The MemeNow Storage Service is a high-performance, asynchronous file upload service built with Rust. It provides an API endpoint for uploading files, which are then stored both on Amazon S3 and IPFS, ensuring redundancy and decentralized access.

## Features

- Single `POST /upload` endpoint accepting `multipart/form-data`
- Concurrent uploads to Amazon S3 and IPFS on the Tokio runtime
- Partial-failure reporting: if one backend succeeds and the other fails, the response carries the successful result so it need not be re-uploaded
- Per-backend timeouts and a bounded IPFS concurrency limit
- Streaming size enforcement and filename sanitization (path-traversal safe)
- Structured JSON errors with stable codes mapped to HTTP statuses
- Per-request IDs in logs; configuration validated at startup
- Hexagonal architecture (ports + adapters), testable against mock backends

## Architecture

The service uses a hexagonal (Ports & Adapters) layout: the domain defines port traits
(`ObjectStorage`, `ContentAddressedStorage`), infrastructure provides the `S3Uploader`
and `IpfsUploader` adapters, and the API layer wires them to Warp. `handle_upload` runs
both backends concurrently via `tokio::join!` and classifies the combined outcome. See
[docs/architecture.md](docs/architecture.md) for details.

## Installation

1. Ensure you have Rust and Cargo installed on your system.
2. Clone the repository:
   ```
   git clone https://github.com/memenow/memenow-storage-service.git
   cd memenow-storage-service
   ```
3. Install dependencies:
   ```
   cargo build
   ```

## Configuration

Create a `.env` file in the project root. Only `S3_BUCKET` and AWS credentials are
required; everything else has a default.

```dotenv
# Required
S3_BUCKET=your-s3-bucket-name
AWS_ACCESS_KEY_ID=your-aws-access-key
AWS_SECRET_ACCESS_KEY=your-aws-secret-key

# Optional (defaults shown)
S3_KEY=uploads
AWS_REGION=us-east-1
```

Ensure you have IPFS installed and running locally (default `127.0.0.1:5001`), or configure the IPFS API endpoint if using a remote node.

The service also reads the following optional values:

- `S3_UPLOAD_TIMEOUT_MS` (default: `30000`)
- `IPFS_UPLOAD_TIMEOUT_MS` (default: `60000`)
- `IPFS_MAX_CONCURRENCY` (default: `4`)
- `MAX_FILE_SIZE` (default: `5242880`; upper limit 1 GiB)
- `TEMP_DIR` (default: `/tmp`)
- `SERVER_HOST` (default: `0.0.0.0`)
- `SERVER_PORT` (default: `8080`)

AWS credentials are resolved by the AWS SDK's default credential chain. See
[docs/configuration.md](docs/configuration.md) for the full table and validation rules.

## Usage

1. Start the service:
   ```
   cargo run
   ```
2. The service will start on `http://0.0.0.0:8080`.
3. To upload a file, send a POST request to `http://0.0.0.0:8080/upload` with the file in the multipart form data:
   ```
   curl -F "file=@./example.bin" http://localhost:8080/upload
   ```

## Development

```bash
cargo build                  # Debug build
cargo build --release        # Release build
cargo check                  # Fast type/syntax check
cargo clippy -- -D warnings  # Lint with warnings as errors
cargo fmt                    # Format
cargo test                   # Unit + integration tests (mock backends; no network/AWS)
```

See [docs/development.md](docs/development.md) for the project layout, testing approach,
and how to add a new storage backend.

## API

### POST /upload

Upload a file to both S3 and IPFS.

**Request:**
- Method: POST
- Content-Type: multipart/form-data
- Body: Form data with a "file" field containing the file to upload
- The service expects one form part named `file`. If multiple parts exist, only the `file` part is used.

**Response:**
- Status: 200 OK
- Content-Type: application/json
- Body:
  ```json
  {
    "s3_url": "https://your-bucket.s3.amazonaws.com/your-file-key",
    "ipfs_hash": "QmHashOfYourFileOnIPFS",
    "filename": "sanitized-name.jpg",
    "size": 12345
  }
  ```

## Error response

All failures return:

```json
{
  "error": {
    "code": "STRING_CODE",
    "message": "human-readable message",
    "details": {},
    "request_id": null
  }
}
```

`request_id` is optional and currently omitted for now because the filter-level recovery layer does not attach request metadata to the current rejection object.

### Error matrix

| HTTP status | code                  | Meaning                                     | Details                                 |
|------------|-----------------------|---------------------------------------------|-----------------------------------------|
| 400        | INVALID_REQUEST       | Missing `file` field, malformed multipart data, or invalid filename | none                                    |
| 413        | PAYLOAD_TOO_LARGE     | Uploaded body exceeds `MAX_FILE_SIZE`         | `max_bytes`, `actual_bytes`             |
| 502        | UPLOAD_PARTIAL_FAILURE | One backend failed, the other succeeded       | `failed_backend`, `partial_success`      |
| 502        | STORAGE_BACKEND_ERROR  | Both backends failed                          | `backend`                               |
| 500        | CLEANUP_ERROR         | Temp file cleanup failed after upload attempt   | `path`                                  |
| 500        | CONFIG_ERROR          | Invalid configuration                         | none                                    |
| 500        | IO_ERROR / AWS_ERROR / S3_ERROR / INTERNAL_ERROR | Internal runtime failures | none or backend-specific details |

## OpenAPI

The OpenAPI 3.0 specification is available at [openapi.yaml](openapi.yaml).

## Documentation

Detailed documentation lives in the [`docs/`](docs/) directory:

- [Architecture](docs/architecture.md) — layering, upload flow, concurrency, error model
- [Configuration](docs/configuration.md) — environment variables and validation rules
- [API](docs/api.md) — endpoint reference and error matrix
- [Development](docs/development.md) — building, testing, and extending the service

## Dependencies

- warp: Web framework for Rust
- tokio: Asynchronous runtime
- aws-sdk-s3: AWS SDK for S3 operations
- ipfs-api: IPFS API client
- anyhow: Error handling
- dotenv: Environment variable management
- log: Logging facade

## License

This project is licensed under the Apache License, Version 2.0. See the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
