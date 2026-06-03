# Exceedingly important information you should never ever ignore

This project is still in development. Things do and will eventually break.

# Hibrid

Hibrid is a minimal cross-platform package manager wrapper for Windows and Linux. <br />
It provides a unified command interface over multiple system package managers, allowing you to install and remove software using a single consistent CLI. <br />
Supported backends:

- Windows: winget <br />
- Linux: apt, pacman, dnf <br />
- Linux (optional): flatpak <br />

---

# Features

- Cross-platform <br />
- Automatic Linux package manager detection <br />
- Optional Flatpak support <br />
- Lightweight and fast <br />
- No external dependencies <br />
- Clean and extendable Rust codebase <br />

---

# Installation

There is currently no installer and no plans to create one in the future. <br />
Hibrid is intentionally lightweight and minimal. Building the project takes only a few seconds and requires no external dependencies beyond Rust itself.
The same build commands work on both Linux and Windows. <br />

## Build from source

git clone https://github.com/person134/hibrid.git <br />
cd hibrid <br />
cargo build --release <br />
