#!/bin/bash
set -euo pipefail

echo "=== Uninstall/Reinstall Test ==="

PACKAGE="com.zeroclaw.android"

# Install and complete setup
adb install -r app/build/outputs/apk/debug/app-debug.apk
maestro test maestro/flows/onboarding.yaml

# Uninstall completely
echo "Uninstalling..."
adb uninstall "$PACKAGE"

# Reinstall
echo "Reinstalling..."
adb install app/build/outputs/apk/debug/app-debug.apk

# Verify clean slate (onboarding should appear again)
maestro test maestro/flows/onboarding.yaml

echo "=== Uninstall/reinstall test passed ==="
