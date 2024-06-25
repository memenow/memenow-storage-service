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

**Response:**
- Status: 200 OK
- Content-Type: application/json
- Body:
  ```json
  {
    "s3_url": "https://your-bucket.s3.amazonaws.com/your-file-key",
    "ipfs_hash": "QmHashOfYourFileOnIPFS"
  }
  ```

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
