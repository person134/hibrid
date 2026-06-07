#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'; GREEN='\033[0;32m'
YELLOW='\033[1;33m'; CYAN='\033[0;36m'
NC='\033[0m'

BINARY_NAME="hibrid"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SOURCE_PATH="$PROJECT_DIR/target/release/$BINARY_NAME"

echo -e "${CYAN}━━━ Hibrid Installer / Uninstaller ━━━${NC}"
echo ""

detect_platform() {
    case "$(uname -s)" in
        Linux)   echo "linux"  ;;
        Darwin)  echo "macos"  ;;
        *)       echo ""       ;;
    esac
}

detected=$(detect_platform)
if [ -z "$detected" ] && [ -n "$WINDIR" ]; then
    detected="windows"
fi

echo "Select platform:"
echo "  1) Linux"
echo "  2) macOS"
echo "  3) Windows"
[ -n "$detected" ] && echo "  4) Auto-detected ($detected)" || echo "  4) Auto-detect"
read -p "Choice [1-4] (default: 4): " pc
pc=${pc:-4}

case $pc in
    1) PLATFORM="linux"   ;;
    2) PLATFORM="macos"   ;;
    3) PLATFORM="windows" ;;
    4) PLATFORM="$detected" ;;
    *) echo -e "${RED}Invalid choice${NC}"; exit 1 ;;
esac

if [ -z "$PLATFORM" ]; then
    echo -e "${RED}Could not detect platform. Select manually.${NC}"
    exit 1
fi
echo -e "Platform: ${CYAN}$PLATFORM${NC}"
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
            windows)
                if command -v winget &>/dev/null; then
                    winget install Rustlang.Rustup
                else
                    echo -e "${RED}Install Rust from https://rustup.rs first, then re-run this script.${NC}"
                    exit 1
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
    case "$PLATFORM" in
        linux|macos)
            sudo cp "$SOURCE_PATH" "/usr/local/bin/$BINARY_NAME"
            sudo chmod +x "/usr/local/bin/$BINARY_NAME"
            echo -e "${GREEN}Installed to /usr/local/bin/$BINARY_NAME${NC}"
            ;;
        windows)
            if command -v powershell &>/dev/null; then
                powershell -Command "
                    Start-Process -Verb RunAs -Wait \
                        -FilePath 'cmd.exe' \
                        -ArgumentList '/c copy /Y \"$SOURCE_PATH\" \"%SystemRoot%\\System32\\$BINARY_NAME.exe\"'
                "
            else
                cp "$SOURCE_PATH" "/c/Windows/System32/$BINARY_NAME.exe"
            fi
            echo -e "${GREEN}Installed to %SystemRoot%\\System32\\$BINARY_NAME.exe${NC}"
            ;;
    esac
    echo -e "${GREEN}You can now run '$BINARY_NAME' from anywhere.${NC}"
fi

# ── Uninstall ─────────────────────────────────────────────────
if [ "$ACTION" = "uninstall" ]; then

    INSTALLED=""
    case "$PLATFORM" in
        linux|macos)
            for p in "/usr/local/bin/$BINARY_NAME" "/usr/bin/$BINARY_NAME"; do
                [ -f "$p" ] && INSTALLED="$p" && break
            done
            ;;
        windows)
            for p in "/c/Windows/System32/$BINARY_NAME.exe" "/c/Windows/SysWOW64/$BINARY_NAME.exe"; do
                [ -f "$p" ] && INSTALLED="$p" && break
            done
            if command -v powershell &>/dev/null; then
                winpath=$(powershell -Command "if (Test-Path \"\$env:SystemRoot\\System32\\$BINARY_NAME.exe\") { Write-Output \"found\" }")
                [ "$winpath" = "found" ] && INSTALLED="${INSTALLED:-found}"
            fi
            ;;
    esac

    if [ -z "$INSTALLED" ]; then
        echo -e "${YELLOW}Hibrid is not installed.${NC}"
        exit 0
    fi

    case "$PLATFORM" in
        linux|macos)
            echo -e "${CYAN}Removing $INSTALLED...${NC}"
            sudo rm "$INSTALLED"
            ;;
        windows)
            echo -e "${CYAN}Removing $BINARY_NAME...${NC}"
            if command -v powershell &>/dev/null; then
                powershell -Command "
                    Start-Process -Verb RunAs -Wait \
                        -FilePath 'cmd.exe' \
                        -ArgumentList '/c del /F \"%SystemRoot%\\System32\\$BINARY_NAME.exe\"'
                "
            else
                rm -f "/c/Windows/System32/$BINARY_NAME.exe"
            fi
            ;;
    esac
    echo -e "${GREEN}Hibrid has been uninstalled.${NC}"
fi
