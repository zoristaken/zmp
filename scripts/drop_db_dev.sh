#!/bin/bash

HOME_DIR="${HOME:-$(getent passwd "$(whoami)" | cut -d: -f6)}"
TARGET_FILE="$HOME_DIR/.local/share/zmp/zmp_dev.db"

if [ -f "$TARGET_FILE" ]; then
    rm "$TARGET_FILE"
    echo "Removed: $TARGET_FILE"
else
    echo "File not found: $TARGET_FILE"
fi