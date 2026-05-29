# Configuration

All configuration is loaded from environment variables by `Config::from_env`
(`src/config.rs`). A `.env` file in the working directory is loaded automatically via
`dotenv`. After loading, `Config::validate` runs at startup and the process exits if any
value is invalid.

## Environment variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `S3_BUCKET` | Yes | — | Target S3 bucket. Missing value is a startup `ConfigError`. |
| `AWS_ACCESS_KEY_ID` | Yes¹ | — | AWS credential, read by the AWS SDK credential chain. |
| `AWS_SECRET_ACCESS_KEY` | Yes¹ | — | AWS credential, read by the AWS SDK credential chain. |
| `S3_KEY` | No | `uploads` | Object key prefix. Final key is `<S3_KEY>/<uuid>_<filename>`. |
| `AWS_REGION` | No | `us-east-1` | AWS region. Also resolvable through the standard AWS region chain. |
| `S3_UPLOAD_TIMEOUT_MS` | No | `30000` | Per-request S3 upload timeout, in milliseconds. |
| `IPFS_UPLOAD_TIMEOUT_MS` | No | `60000` | Per-request IPFS upload timeout, in milliseconds. |
| `IPFS_MAX_CONCURRENCY` | No | `4` | Maximum IPFS uploads running in parallel (semaphore permits). |
| `MAX_FILE_SIZE` | No | `5242880` (5 MiB) | Maximum accepted upload size, in bytes. Hard upper limit 1 GiB. |
| `TEMP_DIR` | No | `/tmp` | Directory for streaming uploads to disk before forwarding. |
| `SERVER_HOST` | No | `0.0.0.0` | Bind address. An IP literal or a resolvable hostname. |
| `SERVER_PORT` | No | `8080` | Bind port. |
| `RUST_LOG` | No | `info` | Log filter (via `env_logger`), e.g. `debug`, `memenow_storage_service=debug`. |

¹ AWS credentials are not read directly by this service; they are resolved by the AWS
SDK's default credential provider chain (environment variables, shared config/credential
files, instance/role providers). Environment variables are the simplest option for local
runs.

## Example `.env`

```dotenv
# Required
S3_BUCKET=your-s3-bucket-name
AWS_ACCESS_KEY_ID=your-aws-access-key
AWS_SECRET_ACCESS_KEY=your-aws-secret-key

# Optional (defaults shown)
S3_KEY=uploads
AWS_REGION=us-east-1
S3_UPLOAD_TIMEOUT_MS=30000
IPFS_UPLOAD_TIMEOUT_MS=60000
IPFS_MAX_CONCURRENCY=4
MAX_FILE_SIZE=5242880
TEMP_DIR=/tmp
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
RUST_LOG=info
```

## Validation rules

`Config::validate` enforces, and the service refuses to start otherwise:

- `S3_BUCKET` is non-empty.
- `SERVER_PORT` is greater than 0.
- `SERVER_HOST` is a valid IP literal or hostname (see [host validation](#host-validation)).
- `MAX_FILE_SIZE` is greater than 0 and at most 1 GiB (`1073741824` bytes).
- `S3_UPLOAD_TIMEOUT_MS` is greater than 0.
- `IPFS_UPLOAD_TIMEOUT_MS` is greater than 0.
- `IPFS_MAX_CONCURRENCY` is greater than 0.
- `TEMP_DIR` is non-empty, exists or can be created, is a directory, and is writable
  (verified with a temporary write probe that is removed immediately).

Numeric variables that fail to parse (`SERVER_PORT`, the `*_MS` timeouts,
`MAX_FILE_SIZE`, `IPFS_MAX_CONCURRENCY`) produce a `ConfigError` naming the variable.

## Host validation

`SERVER_HOST` is validated in two places:

- `Config::validate` rejects empty hosts, hosts containing `://`, `/`, or spaces,
  out-of-range numeric IPs (e.g. `256.1.1.1`), malformed dotted-numeric forms
  (e.g. `1.2.3.4.5`, `1.2.3.4.`), and invalid DNS labels.
- `main.rs::resolve_server_host` then parses the host as an `IpAddr`, or resolves it via
  DNS (`to_socket_addrs`) to obtain the bind address.

## Size enforcement

`MAX_FILE_SIZE` is enforced twice for defense in depth:

1. Warp's multipart filter is configured with `max_length(MAX_FILE_SIZE)`, rejecting
   oversized bodies at the framework boundary.
2. `extract_and_save_file` counts bytes while streaming to disk and aborts with
   `PayloadTooLarge` (deleting the partial temp file) if the limit is crossed.
