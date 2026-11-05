#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOTFS="$SCRIPT_DIR/rootfs"
OUTPUT="$SCRIPT_DIR/initramfs.cpio.gz"

# Check dependencies and rootfs
for tool in cpio gzip; do
    command -v "$tool" >/dev/null || {
        echo "Error: $tool is not installed"
        exit 1
    }
done

if [[ ! -d "$ROOTFS" ]]; then
    echo "Error: rootfs directory not found"
    exit 1
fi

# Create required directories
sudo mkdir -p "$ROOTFS"/{dev,proc,sys}

# Create device nodes if they don't exist
[[ -c "$ROOTFS/dev/console" ]] || sudo mknod -m 600 "$ROOTFS/dev/console" c 5 1
[[ -c "$ROOTFS/dev/null" ]] || sudo mknod -m 666 "$ROOTFS/dev/null" c 1 3

# Build compressed initramfs
sudo bash -c 'cd "$1" && find . -print0 | cpio --null -o -H newc | gzip -9 > "$2"' _ "$ROOTFS" "$OUTPUT"

# Give archive ownership back to current user
sudo chown "$(id -u):$(id -g)" "$OUTPUT"

echo "Initramfs created: $OUTPUT"