# MinIO Docker Setup for Scribe Ledger

This directory contains Docker Compose configuration for running MinIO as an S3-compatible storage backend during development.

## Quick Start

```bash
# Start MinIO
./dev.sh start-minio

# Access MinIO Console
open http://localhost:9001
# Username: scribe-admin
# Password: scribe-password-123
```

## Configuration

### Ports
- **9000**: MinIO S3 API
- **9001**: MinIO Web Console

### Credentials
- **Access Key**: `scribe-admin`
- **Secret Key**: `scribe-password-123`

### Buckets
- `scribe-ledger-dev` - Development environment
- `scribe-ledger-test` - Testing environment
- `scribe-ledger-prod` - Production-like environment

## Volume Mounts

- `minio_data`: Persistent storage for MinIO data
- `./docker/minio/config`: MinIO configuration directory
- `./docker/minio/init`: Initialization scripts

## Scripts

### setup-buckets.sh
Automatically runs when MinIO starts up to:
- Create required buckets
- Set appropriate permissions
- Create directory structure
- Verify setup

## Docker Compose Services

### minio
Main MinIO server service with:
- Health checks
- Persistent volumes
- Environment configuration
- Network isolation

### minio-init
One-time initialization container that:
- Waits for MinIO to be ready
- Creates buckets and directories
- Sets up development environment
- Exits after completion

## Development Integration

The MinIO setup integrates with the Scribe Ledger development workflow:

```bash
# Development with MinIO backend
./dev.sh run-dev

# Monitor MinIO logs
./dev.sh minio-logs

# Reset development data
./dev.sh minio-reset
```

## Troubleshooting

### Port Conflicts
If ports 9000 or 9001 are in use:
```bash
# Check what's using the ports
lsof -i :9000
lsof -i :9001
```

### Container Issues
```bash
# View container logs
docker-compose logs minio

# Restart containers
docker-compose restart

# Rebuild containers
docker-compose up --build
```

### Data Persistence
MinIO data is stored in the `minio_data` Docker volume. To completely reset:
```bash
./dev.sh minio-reset
```