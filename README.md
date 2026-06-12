# Hibrid

A lightweight cross-platform package manager wrapper. Hibrid gives you one CLI to install, remove, update, list, and search packages, no matter which package manager your system uses.

```
hibrid install firefox    # install with apt/pacman/dnf/emerge/brew/winget
hibrid -If spotify        # install via Flatpak (legacy style also works)
hibrid remove vim         # remove a package
hibrid update             # update everything
```

## Features

- **Unified CLI** over apt, pacman, dnf, emerge, brew, and winget
- **Auto-detects** your Linux package manager -- no config
- **Word commands** (`install`, `remove`, `update`, `list`, `search`) and legacy short flags (`-I`, `-R`, `-U`, `-L`, `-S`)
- **Flatpak** support on Linux
- **Batched** installs/removes/updates (single package manager invocation)
- **Dry-run mode** (`-d` / `--dry-run`) preview changes without applying them
- **Non-blocking validation** -- warns if a package is not found but continues
- **Quiet mode** (`-q` / `--quiet`) suppresses spinner and package manager output
- **Auto-cache update** refreshes apt cache before upgrade

## Supported backends

| OS | Backends |
|----|----------|
| Windows | winget |
| macOS | Homebrew |
| Linux | apt, pacman, dnf, portage + Flatpak (AUR via yay/paru) |

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

To install it system-wide (or uninstall later):

**Linux / macOS:**
```bash
sudo ./install-uninstall/install.sh            # install
sudo ./install-uninstall/install.sh --uninstall # uninstall
```

**Windows:** Run as administrator:
```batch
install-uninstall\install.bat                  # install
install-uninstall\install.bat --uninstall      # uninstall
```

## Usage

Two styles are supported:

- **Word commands** (recommended): `hibrid install vim -y`
- **Short flags**: `hibrid -Ia vim`

### Commands

| Style | Short flag | Description |
|-------|------------|-------------|
| `install <pkg>` | `-I` | Install a package |
| `remove <pkg>` | `-R` | Remove a package |
| `update [pkg]` | `-U` | Update all packages or a specific one |
| `list` | `-L` | List installed packages |
| `search <pkg>` | `-S` | Search for a package |

### Modifiers

| Modifier | Effect |
|----------|--------|
| `-y`, `--yes` | Skip confirmation prompt |
| `-q`, `--quiet` | Quiet mode (show spinner instead of full output) |
| `-f`, `--flatpak` | Use Flatpak backend (Linux only) |
| `-d`, `--dry-run` | Preview changes without making them |
| `-V`, `--version` | Show version |
| `-h`, `--help` | Show help message |

Modifiers can be passed as separate flags (`-y -d`) or combined (`-yd`). With the short-flag style, modifiers can be appended to the flag or passed as separate arguments after it:

```
hibrid -Iaqf vim    →  Install + autoinstall + quiet + flatpak
hibrid -Id vim      →  Install + dry run
hibrid -I vim -d    →  Install + dry run (modifier after package)
```

### Examples

```bash
hibrid install vim           # Install vim
hibrid -I vim                # Same, using short flag
hibrid install vim -y        # Install vim (skip confirmation)
hibrid install firefox -q    # Install firefox (quiet mode)
hibrid -If spotify           # Install spotify via Flatpak
hibrid remove vim -y         # Remove vim
hibrid update                # Update all system packages
hibrid update mpv            # Update mpv only
hibrid list                  # List installed packages
hibrid search git            # Search for git
hibrid install -d vim        # Preview installing vim (dry run)
hibrid -V                    # Show version
hibrid -h                    # Show help
```

## Development

```bash
cargo build              # debug build
cargo build --release    # release build
cargo test               # run tests
cargo clippy             # lint
cargo fmt --check        # check formatting
```

### Project structure

```
src/
  action.rs    - Action enum, Flags struct, CLI parsing (word + short-flag)
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
