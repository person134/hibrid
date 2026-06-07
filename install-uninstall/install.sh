#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'; GREEN='\033[0;32m'
YELLOW='\033[1;33m'; CYAN='\033[0;36m'
NC='\033[0m'

BINARY_NAME="hibrid"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SOURCE_PATH="$PROJECT_DIR/target/release/$BINARY_NAME"

OS="$(uname -s)"
case "$OS" in
    Linux)  PLATFORM="linux"  ;;
    Darwin) PLATFORM="macos"  ;;
    *)
        echo -e "${RED}This script is for Linux and macOS only.${NC}"
        echo "For Windows, use install.bat instead."
        exit 1
        ;;
esac

echo -e "${CYAN}━━━ Hibrid Installer / Uninstaller (${PLATFORM}) ━━━${NC}"
echo ""

echo "Select action:"
echo "  1) Install"
echo "  2) Uninstall"
read -p "Choice [1/2]: " ac
case $ac in
    1|install|Install)   ACTION="install"   ;;
    2|uninstall|Uninstall) ACTION="uninstall" ;;
    *) echo -e "${RED}Invalid choice${NC}"; exit 1 ;;
esac
echo ""

# ── Install ───────────────────────────────────────────────────
if [ "$ACTION" = "install" ]; then

    if command -v "$BINARY_NAME" &>/dev/null; then
        echo -e "${YELLOW}Hibrid is already installed at $(command -v $BINARY_NAME)${NC}"
        exit 0
    fi

    if ! command -v cargo &>/dev/null; then
        echo -e "${YELLOW}Rust/Cargo not found. Installing Rust via rustup...${NC}"
        case "$PLATFORM" in
            linux)
                if command -v apt &>/dev/null; then
                    sudo apt update && sudo apt install -y build-essential
                elif command -v pacman &>/dev/null; then
                    sudo pacman -S --noconfirm base-devel
                elif command -v dnf &>/dev/null; then
                    sudo dnf install -y gcc
                fi
                ;;
            macos)
                if ! xcode-select -p &>/dev/null 2>&1; then
                    xcode-select --install
                fi
                ;;
        esac
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
        echo -e "${GREEN}Rust installed successfully.${NC}"
    fi

    echo -e "${CYAN}Building $BINARY_NAME...${NC}"
    cargo build --release --manifest-path "$PROJECT_DIR/Cargo.toml"

    if [ ! -f "$SOURCE_PATH" ]; then
        echo -e "${RED}Build failed — binary not found at $SOURCE_PATH${NC}"
        exit 1
    fi

    echo -e "${CYAN}Installing $BINARY_NAME...${NC}"
    sudo cp "$SOURCE_PATH" "/usr/local/bin/$BINARY_NAME"
    sudo chmod +x "/usr/local/bin/$BINARY_NAME"
    echo -e "${GREEN}Installed to /usr/local/bin/$BINARY_NAME${NC}"
    echo -e "${GREEN}You can now run '$BINARY_NAME' from anywhere.${NC}"
fi

# ── Uninstall ─────────────────────────────────────────────────
if [ "$ACTION" = "uninstall" ]; then

    INSTALLED=""
    for p in "/usr/local/bin/$BINARY_NAME" "/usr/bin/$BINARY_NAME"; do
        [ -f "$p" ] && INSTALLED="$p" && break
    done

    if [ -z "$INSTALLED" ]; then
        echo -e "${YELLOW}Hibrid is not installed.${NC}"
        exit 0
    fi

    echo -e "${CYAN}Removing $INSTALLED...${NC}"
    sudo rm "$INSTALLED"
    echo -e "${GREEN}Hibrid has been uninstalled.${NC}"
fi
