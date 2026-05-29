# Documentation

Reference documentation for the MemeNow Storage Service — a Rust microservice that
accepts a file over HTTP and stores it concurrently on Amazon S3 and IPFS.

## Contents

- [Architecture](architecture.md) — Hexagonal layering, the upload flow, the
  concurrency model, and how partial failures are handled.
- [Configuration](configuration.md) — Every environment variable, its default, and
  the validation rules applied at startup.
- [API](api.md) — The `POST /upload` endpoint, response shapes, and the full error
  matrix. The machine-readable contract lives in [`../openapi.yaml`](../openapi.yaml).
- [Development](development.md) — Building, testing, linting, the project layout, and
  how to add a new storage backend.

## Quick reference

| Item | Value |
|------|-------|
| Endpoint | `POST /upload` (`multipart/form-data`, field name `file`) |
| Default bind address | `0.0.0.0:8080` |
| Default max file size | 5 MiB (`MAX_FILE_SIZE`, hard upper limit 1 GiB) |
| Backends | Amazon S3 + IPFS, uploaded concurrently |
| Success response | `200 OK` with `s3_url`, `ipfs_hash`, `filename`, `size` |

For a top-level overview and setup steps, see the [project README](../README.md).
