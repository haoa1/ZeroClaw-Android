#!/bin/bash
set -euo pipefail

echo "=== Upgrade Test ==="

PACKAGE="com.zeroclaw.android"

# Get previous release APK
PREV_TAG=$(gh release list --limit 2 --json tagName -q '.[1].tagName')
echo "Previous release: $PREV_TAG"

PREV_APK="/tmp/zeroclaw-prev.apk"
gh release download "$PREV_TAG" -p '*.apk' -D /tmp --clobber
mv /tmp/app-release.apk "$PREV_APK" 2>/dev/null || mv /tmp/app-release-unsigned.apk "$PREV_APK"

# Clean install previous version
adb uninstall "$PACKAGE" 2>/dev/null || true
adb install "$PREV_APK"

# Run onboarding on previous version
maestro test maestro/flows/onboarding.yaml || echo "WARN: onboarding may differ on older version"

# Upgrade to current version
echo "Upgrading to current version..."
adb install -r app/build/outputs/apk/debug/app-debug.apk

# Verify app still works after upgrade
maestro test maestro/flows/settings-roundtrip.yaml

echo "=== Upgrade test passed ==="
