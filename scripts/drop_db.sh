#!/bin/bash

HOME_DIR="${HOME:-$(getent passwd "$(whoami)" | cut -d: -f6)}"
TARGET_FILE="$HOME_DIR/.local/share/com.github.zoristaken-zmp/zmp.db"

if [ -f "$TARGET_FILE" ]; then
    rm "$TARGET_FILE"
    echo "Removed: $TARGET_FILE"
else
    echo "File not found: $TARGET_FILE"
fi