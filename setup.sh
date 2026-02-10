#!/bin/bash

set -e

echo "🚀 Starting deployment..."
echo "--------------------------------"

echo "📥 Pulling latest code..."
git pull

echo "🦀 Building Rust project..."
cargo build --release

echo "🔄 Restarting PM2 process..."
pm2 restart 0

echo "✅ Deployment finished successfully!"
echo "--------------------------------"
