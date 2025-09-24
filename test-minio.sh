#!/bin/bash

# MinIO connectivity test script for Scribe Ledger

echo "🧪 Testing MinIO connectivity..."

# Test 1: Check if MinIO is running
if ! docker-compose ps minio | grep -q "Up"; then
    echo "❌ MinIO is not running. Start it with: ./dev.sh start-minio"
    exit 1
fi

echo "✅ MinIO container is running"

# Test 2: Test S3 API endpoint
if curl -s -f http://localhost:9000/minio/health/live > /dev/null; then
    echo "✅ MinIO S3 API is healthy"
else
    echo "❌ MinIO S3 API is not responding"
    exit 1
fi

# Test 3: List buckets using mc client
echo "📋 Testing bucket access..."
BUCKETS=$(docker run --rm --network scribe-ledger_scribe-network \
    -e MC_HOST_local=http://scribe-admin:scribe-password-123@minio:9000 \
    minio/mc ls local/ | wc -l)

if [ "$BUCKETS" -ge 3 ]; then
    echo "✅ All buckets are accessible"
    docker run --rm --network scribe-ledger_scribe-network \
        -e MC_HOST_local=http://scribe-admin:scribe-password-123@minio:9000 \
        minio/mc ls local/
else
    echo "❌ Some buckets are missing"
    exit 1
fi

# Test 4: Test file upload/download
echo "📁 Testing file operations..."
echo "test-content-$(date)" > /tmp/test-file.txt

if docker run --rm --network scribe-ledger_scribe-network \
    -e MC_HOST_local=http://scribe-admin:scribe-password-123@minio:9000 \
    -v /tmp/test-file.txt:/tmp/test-file.txt \
    minio/mc cp /tmp/test-file.txt local/scribe-ledger-test/test-file.txt; then
    echo "✅ File upload successful"
else
    echo "❌ File upload failed"
    exit 1
fi

DOWNLOADED_CONTENT=$(docker run --rm --network scribe-ledger_scribe-network \
    -e MC_HOST_local=http://scribe-admin:scribe-password-123@minio:9000 \
    minio/mc cat local/scribe-ledger-test/test-file.txt)

if echo "$DOWNLOADED_CONTENT" | grep -q "test-content"; then
    echo "✅ File download successful"
else
    echo "❌ File download failed"
    echo "Downloaded content: $DOWNLOADED_CONTENT"
    exit 1
fi

# Cleanup
docker run --rm --network scribe-ledger_scribe-network \
    -e MC_HOST_local=http://scribe-admin:scribe-password-123@minio:9000 \
    minio/mc rm local/scribe-ledger-test/test-file.txt > /dev/null
rm -f /tmp/test-file.txt

echo ""
echo "🎉 All MinIO tests passed!"
echo "📊 MinIO Console: http://localhost:9001"
echo "🔐 Username: scribe-admin"
echo "🔐 Password: scribe-password-123"
echo ""
echo "Ready for S3 development! Use: ./dev.sh run-dev"