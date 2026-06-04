#!/bin/bash

set -e

BINARY_NAME="hibrid"
DEST_PATH="/usr/bin/$BINARY_NAME"

echo "Hibrid Uninstaller"
echo "=================="

if [ ! -f "$DEST_PATH" ]; then
    echo "Error: Hibrid is not installed at $DEST_PATH"
    exit 1
fi

echo "Removing $DEST_PATH..."

sudo rm "$DEST_PATH"

echo "Done! Hibrid has been uninstalled."
