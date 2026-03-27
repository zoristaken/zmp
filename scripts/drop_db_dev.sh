#!/bin/bash

HOME_DIR="${HOME:-$(getent passwd "$(whoami)" | cut -d: -f6)}"
TARGET_FILE="$HOME_DIR/.config/zmp/zmp_dev.db"

if [ -f "$TARGET_FILE" ]; then
    rm "$TARGET_FILE"
    echo "Removed: $TARGET_FILE"
else
    echo "File not found: $TARGET_FILE"
fi