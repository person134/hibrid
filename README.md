# Hibrid

A lightweight cross-platform package manager wrapper. Hibrid gives you one CLI to install, remove, update, and search packages, no matter which package manager your system uses.

```
hibrid -I firefox       # install with apt/pacman/dnf/emerge/brew/winget
hibrid -If spotify      # install via Flatpak
hibrid -R vim           # remove a package
hibrid -U               # update everything
```

## Supported backends

| OS | Backends |
|----|----------|
| Windows | winget |
| macOS | Homebrew |
| Linux | apt, pacman, dnf, portage + Flatpak |

Hibrid auto-detects which package manager is installed on Linux. No config needed.

## Quick start

### Requirements
- Rust 1.56+ ([install](https://rustup.rs/))
- Your system's package manager must be installed (winget, brew, apt, pacman, dnf, or portage)

### Install
```bash
git clone https://github.com/person134/hibrid.git
cd hibrid
cargo build --release
```
The binary will be at `target/release/hibrid` (or `hibrid.exe` on Windows).

To install it system-wide (or uninstall later), run the script for your OS:

**Linux / macOS:**
```bash
cd install-uninstall
chmod +x install.sh
./install.sh
```

**Windows:** Right-click `install.bat` and select **Run as administrator**.

The scripts will ask whether to install or uninstall, check for existing installations, and auto-install Rust/Cargo if missing.

## Usage

```
hibrid <flags> [package]
```

### Commands

| Flag | Action |
|------|--------|
| `-I`  | Install a package |
| `-R`  | Remove a package |
| `-U`  | Update all packages or a specific one |
| `-L`  | List installed packages |
| `-S`  | Search for a package |
| `-V`  | Show version |

### Modifiers

| Modifier | Effect |
|----------|--------|
| `a` | Skip confirmation prompt |
| `q` | Quiet mode (show spinner instead of full output) |
| `f` | Use Flatpak backend (Linux only) |
| `d` | Dry run (preview changes without making them) |

Modifiers go right after the command flag with no space:

```
-Iaqf  →  Install + autoinstall + quiet + flatpak
-Id    →  Install + dry run
```

### Examples

```bash
hibrid -I vim           # Install vim
hibrid -Ia firefox      # Install firefox (skip confirmation)
hibrid -Iq vlc          # Install vlc (quiet mode)
hibrid -If spotify      # Install spotify via Flatpak
hibrid -R vim           # Remove vim
hibrid -U               # Update all system packages
hibrid -U mpv           # Update mpv only
hibrid -L               # List installed packages
hibrid -S git           # Search for git
hibrid -Id vim          # Preview installing vim (dry run)
hibrid -V               # Show version
```

## Development

```bash
cargo build              # debug build
cargo build --release    # release build
cargo test               # run tests
```

### Project structure

```
src/
  action.rs    - Action enum, Flags struct, CLI parsing
  backend.rs   - System and package manager detection
  runner.rs    - Command execution with spinner support
  search.rs    - Package search, info extraction, Flatpak helpers
  ui.rs        - Box formatting, confirmation prompts, output
  main.rs      - Dispatch logic (ties everything together)
tests/
examples/
```

## License

MIT. See [LICENSE](LICENSE)
