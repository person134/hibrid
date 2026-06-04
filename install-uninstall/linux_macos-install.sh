#!/bin/bash

set -e

BINARY_NAME="hibrid"
SOURCE_PATH="$HOME/hibrid/target/release/$BINARY_NAME"
DEST_PATH="/usr/bin/$BINARY_NAME"

echo "Hibrid Installer"
echo "================"

if [ ! -f "$SOURCE_PATH" ]; then
    echo "Error: Binary not found at $SOURCE_PATH"
    echo "Make sure you have run 'cargo build --release' first."
    exit 1
fi

echo "Installing $BINARY_NAME to $DEST_PATH..."

sudo cp "$SOURCE_PATH" "$DEST_PATH"
sudo chmod +x "$DEST_PATH"

echo "Done! Hibrid has been installed to $DEST_PATH"
echo "You can now run 'hibrid' from anywhere."
