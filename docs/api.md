# API

The service exposes a single endpoint. The machine-readable contract is
[`../openapi.yaml`](../openapi.yaml) (OpenAPI 3.0); this page is the narrative reference.

## `POST /upload`

Uploads one file to Amazon S3 and IPFS concurrently.

**Request**

- Method: `POST`
- Content-Type: `multipart/form-data`
- Body: a form part named `file` containing the file bytes. Only the `file` part is
  read; any other parts are ignored.
- Size limit: `MAX_FILE_SIZE` bytes (default 5 MiB). See
  [configuration](configuration.md#size-enforcement).

```bash
curl -F "file=@./example.bin" http://localhost:8080/upload
```

**Success — `200 OK`**

```json
{
  "s3_url": "https://your-bucket.s3.amazonaws.com/uploads/3f5e12a1_example.bin",
  "ipfs_hash": "QmExampleHash",
  "filename": "example.bin",
  "size": 1024
}
```

| Field | Type | Description |
|-------|------|-------------|
| `s3_url` | string (URI) | Region-aware public S3 URL of the stored object. |
| `ipfs_hash` | string | IPFS Content Identifier (CID). |
| `filename` | string | Sanitized filename used in the S3 key and the response. |
| `size` | integer (int64) | Stored byte count. |

A `200` is returned only when **both** backends succeed.

## Error envelope

Every failure returns the same JSON shape:

```json
{
  "error": {
    "code": "STRING_CODE",
    "message": "human-readable message",
    "details": null,
    "request_id": null
  }
}
```

- `code` and `message` are always present.
- `details` is always present; it is `null` unless the error supplies structured
  fields (the Details column below — a `—` there means `null`).
- `request_id` is always present but currently always `null`. A short request ID is
  generated per request and written to the server logs (`request_id=...`), but the
  rejection-recovery layer does not yet attach it to the response body.

## Error matrix

| HTTP | code | Meaning | `details` |
|------|------|---------|-----------|
| 400 | `INVALID_REQUEST` | Missing `file` part, malformed multipart, or invalid/empty/too-long filename | — |
| 413 | `PAYLOAD_TOO_LARGE` | Body exceeds `MAX_FILE_SIZE` | `max_bytes`, `actual_bytes` |
| 502 | `UPLOAD_PARTIAL_FAILURE` | Exactly one backend failed; the other succeeded | `failed_backend`, `partial_success` |
| 502 | `STORAGE_BACKEND_ERROR` | Both backends failed | `backend` |
| 500 | `CLEANUP_ERROR` | Temp-file removal failed after the upload stage | `path` |
| 500 | `CONFIG_ERROR` | Invalid configuration surfaced at runtime | — |
| 500 | `IO_ERROR` / `AWS_ERROR` / `S3_ERROR` / `INTERNAL_ERROR` | Internal runtime failures | — |

The rejection-recovery layer also returns these framework-level responses:

| HTTP | code | When |
|------|------|------|
| 404 | `NOT_FOUND` | Unknown path |
| 405 | `METHOD_NOT_ALLOWED` | Wrong method on `/upload` (e.g. `GET`) |
| 413 | `PAYLOAD_TOO_LARGE` | Multipart body rejected by the framework size guard |
| 415 | `INVALID_MEDIA_TYPE` | Body is not `multipart/form-data` |

### Partial failure

When one backend succeeds and the other fails, the response is `502` with
`code: UPLOAD_PARTIAL_FAILURE`. `details.failed_backend` is `s3` or `ipfs`, and
`details.partial_success` carries the successful side so the caller can recover without
re-uploading it:

```json
{
  "error": {
    "code": "UPLOAD_PARTIAL_FAILURE",
    "message": "Partial upload failure on 'ipfs': IPFS upload failed: ...",
    "details": {
      "failed_backend": "ipfs",
      "partial_success": { "s3_url": "https://your-bucket.s3.amazonaws.com/uploads/..." }
    }
  }
}
```

`partial_success` contains exactly one of `s3_url` or `ipfs_hash`; the absent field is
omitted. The temporary file is removed in all cases, including partial and total failure.
