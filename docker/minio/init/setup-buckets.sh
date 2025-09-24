#!/bin/bash
set -e

echo "Waiting for MinIO to be ready..."
sleep 10

# Configure MinIO client
mc alias set local http://minio:9000 scribe-admin scribe-password-123

# Create buckets for Scribe Ledger
echo "Creating buckets..."

# Development bucket
mc mb local/scribe-ledger-dev --ignore-existing
echo "✅ Created bucket: scribe-ledger-dev"

# Test bucket
mc mb local/scribe-ledger-test --ignore-existing
echo "✅ Created bucket: scribe-ledger-test"

# Production bucket (for future use)
mc mb local/scribe-ledger-prod --ignore-existing
echo "✅ Created bucket: scribe-ledger-prod"

# Set bucket policies (public read for development)
mc anonymous set public local/scribe-ledger-dev
mc anonymous set public local/scribe-ledger-test

# Create some sample directories with placeholder files
echo "placeholder" | mc pipe local/scribe-ledger-dev/segments/.keep
echo "placeholder" | mc pipe local/scribe-ledger-dev/manifests/.keep
echo "placeholder" | mc pipe local/scribe-ledger-test/segments/.keep

echo "✅ MinIO setup completed successfully!"
echo "📊 MinIO Console: http://localhost:9001"
echo "🔐 Username: scribe-admin"
echo "🔐 Password: scribe-password-123"
echo "🚀 S3 API Endpoint: http://localhost:9000"

# List buckets to verify
echo "📋 Available buckets:"
mc ls local/