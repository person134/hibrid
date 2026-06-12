#!/usr/bin/env bash
set -euo pipefail

NAME="hibrid"
DEST="/usr/local/bin"

case "${1:-}" in
  -u|--uninstall)
    sudo rm -f "$DEST/$NAME"
    echo "Uninstalled $NAME"
    exit 0
    ;;
  -h|--help)
    echo "Usage: ./install.sh [--uninstall]"
    exit 0
    ;;
esac

cargo build --release
sudo cp "target/release/$NAME" "$DEST/$NAME"
echo "Installed $NAME to $DEST/$NAME"
