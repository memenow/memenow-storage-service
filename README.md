# memenow-storage-service

A Rust-based microservice for handling file uploads to Amazon S3 and IPFS.

## Description

The MemeNow Storage Service is a high-performance, asynchronous file upload service built with Rust. It provides an API endpoint for uploading files, which are then stored both on Amazon S3 and IPFS, ensuring redundancy and decentralized access.

## Features

- File upload endpoint
- Concurrent uploads to Amazon S3 and IPFS
- Asynchronous processing using Tokio runtime
- Error handling and logging
- Environment variable configuration

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

Create a `.env` file in the project root with the following variables:

```
S3_BUCKET=your-s3-bucket-name
S3_KEY=your-s3-key-prefix
AWS_ACCESS_KEY_ID=your-aws-access-key
AWS_SECRET_ACCESS_KEY=your-aws-secret-key
AWS_REGION=your-aws-region
```

Ensure you have IPFS installed and running locally, or configure the IPFS API endpoint if using a remote node.

The service also reads the following optional values:

- `S3_UPLOAD_TIMEOUT_MS` (default: `30000`)
- `IPFS_UPLOAD_TIMEOUT_MS` (default: `60000`)
- `IPFS_MAX_CONCURRENCY` (default: `4`)
- `MAX_FILE_SIZE` (default: `5242880`)
- `TEMP_DIR` (default: `/tmp`)
- `SERVER_HOST` (default: `0.0.0.0`)
- `SERVER_PORT` (default: `8080`)

## Usage

1. Start the service:
   ```
   cargo run
   ```
2. The service will start on `http://0.0.0.0:8080`.
3. To upload a file, send a POST request to `http://0.0.0.0:8080/upload` with the file in the multipart form data.

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

The OpenAPI 3.0 specification is available at [openapi.yaml](/home/billduke/Documents/memenow/memenow-storage-service/openapi.yaml).

## Environment variables

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
