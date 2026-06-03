# Development Notice

This project is still in development. Things do and will eventually break.

---

# Hibrid

A minimal cross-platform package manager wrapper for Windows and Linux. Hibrid provides a unified command interface over multiple system package managers, allowing you to install and remove software using a single consistent CLI.

**Supported backends:**
- **Windows:** winget
- **Linux:** apt, pacman, dnf
- **Linux (optional):** flatpak

---

## Features

- Cross-platform (Windows & Linux)
- Automatic Linux package manager detection
- Optional Flatpak support
- Lightweight and fast
- No external dependencies (Rust only)
- Clean and extendable codebase

---

## Requirements

- **Rust 1.56+** ([Install Rust](https://rustup.rs/))
- For Linux: One of apt, pacman, or dnf must be installed
- For Windows: winget must be available

---

## Installation

Hibrid is intentionally lightweight with no installer. Building takes seconds and requires only Rust.

### Build from source

```bash
git clone https://github.com/person134/hibrid.git
cd hibrid
cargo build --release
```

The binary will be at `target/release/hibrid` (or `hibrid.exe` on Windows).

---

## Usage

```bash
hibrid [-I|-R|-V][a][q][f] package
```

**Main commands:**
- `-I` Install a package
- `-R` Remove a package
- `-V` Show version
- `-S` Search a package and list information

**Modifiers:**
- `a` Autoinstall (skip confirmation prompt)
- `q` Quiet output (suppress package manager output)
- `f` Use Flatpak backend

**Examples:**
```bash
hibrid -I vim
hibrid -Ia firefox
hibrid -R vim
hibrid -If spotify
hibrid -Iq vlc
hibrid -V
hibrid -S mpv
```

---

## Development

### Running tests
```bash
cargo test
```

### Building debug version
```bash
cargo build
```

### Code structure
- `src/main.rs` — Main CLI application entry point
- `examples/` — Usage examples
- `tests/` — Integration tests

---

## License

MIT License — see [LICENSE](LICENSE) file for details

---

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.
