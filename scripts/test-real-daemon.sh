#!/bin/bash
set -euo pipefail

echo "=== Real Daemon E2E Tests ==="
echo "Requires LM Studio running at http://192.168.1.197:1234"
echo ""

# Verify LM Studio is reachable
if ! curl -sf http://192.168.1.197:1234/v1/models > /dev/null 2>&1; then
    echo "ERROR: LM Studio not reachable at 192.168.1.197:1234"
    echo "Start LM Studio and load a Qwen model first."
    exit 1
fi
echo "LM Studio: OK"

# Verify emulator/device is connected
if ! adb devices | grep -q "device$"; then
    echo "ERROR: No Android device/emulator connected"
    echo "Start an emulator: \$ANDROID_HOME/emulator/emulator -avd ZeroClaw_Test"
    exit 1
fi
echo "Device: OK"

# Install latest debug APK
echo "Installing debug APK..."
adb install -r app/build/outputs/apk/debug/app-debug.apk

# Run real-daemon flows
echo "Running real-daemon Maestro flows..."
maestro test maestro/flows/real-daemon/

echo ""
echo "=== All real-daemon tests passed ==="
