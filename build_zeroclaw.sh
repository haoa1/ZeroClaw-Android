#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64

echo "Building..."

cd "$SCRIPT_DIR"

./gradlew :lib:assembleDebug :app:assembleDebug

echo "Done! APK ready at app/build/outputs/apk/debug/"
