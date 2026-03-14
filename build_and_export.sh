#!/bin/bash

# Get the current date in YYYY-MM-DD format
TAG=$(date +%Y-%m-%d)
IMAGE_NAME="byrt/eredivisie-standing:$TAG"
SLIM_IMAGE_NAME="byrt/eredivisie-standing:$TAG-slim"

# Build the regular Docker image first
echo "Building regular Docker image..."
docker build --platform linux/amd64 -t $IMAGE_NAME .

# Use slimtoolkit to create an optimized version
echo "Creating optimized image with slimtoolkit..."
slim build --target $IMAGE_NAME \
    --tag $SLIM_IMAGE_NAME \
    --http-probe=false \
    --continue-after=5 \
    --expose=8000 \
    --include-path=/home/byrt/app \
    --preserve-path=/home/byrt/app \
    --show-clogs

# Export both images to tar files
echo "Exporting Docker images..."
docker save -o eredivisie-standing.$TAG.tar $IMAGE_NAME
docker save -o eredivisie-standing.$TAG-slim.tar $SLIM_IMAGE_NAME

echo "Docker images have been built and exported:"
echo "Regular image: eredivisie-standing.$TAG.tar"
echo "Slim image: eredivisie-standing.$TAG-slim.tar"