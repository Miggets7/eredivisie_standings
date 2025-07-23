#!/bin/bash

# Get the current date in YYYY-MM-DD format
TAG=$(date +%Y-%m-%d)
IMAGE_NAME="eredivisie-standing:$TAG"
SLIM_IMAGE_NAME="eredivisie-standing:$TAG-slim"

# Build the regular Docker image first
echo "Building regular Docker image..."
docker build --platform linux/amd64 -t $IMAGE_NAME .

# Use slimtoolkit to create an optimized version
echo "Creating optimized image with slimtoolkit..."
slim build --target $IMAGE_NAME \
    --tag $SLIM_IMAGE_NAME \
    --http-probe=false \
    --continue-after=3 \
    --expose=8000 \
    --include-path=/usr/local/lib/python3.13 \
    --include-path=/app \
    --preserve-path=/usr/local/bin \
    --preserve-path=/app \
    --include-bin=/usr/local/bin/python3.13 \
    --include-bin=/usr/local/bin/uvicorn \
    --include-shell \
    --env PYTHONPATH=/app:/usr/local/lib/python3.13/site-packages \
    --show-clogs

# Export both images to tar files
echo "Exporting Docker images..."
docker save -o eredivisie-standing.$TAG.tar $IMAGE_NAME
docker save -o eredivisie-standing.$TAG-slim.tar $SLIM_IMAGE_NAME

echo "Docker images have been built and exported:"
echo "Regular image: eredivisie-standing.$TAG.tar"
echo "Slim image: eredivisie-standing.$TAG-slim.tar"