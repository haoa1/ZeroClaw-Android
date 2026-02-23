#!/bin/bash
set -euo pipefail

echo "=== Fresh Install Test ==="

PACKAGE="com.zeroclaw.android"

# Uninstall if present
adb uninstall "$PACKAGE" 2>/dev/null || true

# Install fresh
adb install app/build/outputs/apk/debug/app-debug.apk

# Run onboarding flow (verifies clean state)
maestro test maestro/flows/onboarding.yaml

echo "=== Fresh install test passed ==="
