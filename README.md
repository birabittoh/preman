# preman

A TUI tool to manage and cleanup unused Steam Wine/Proton prefixes; works completely offline with multiple Steam directories.

![demo](https://github.com/user-attachments/assets/86f78c3e-901f-4a01-97b6-2ba88f722ae7)

## Installation

### Pre-built binary

Download the stable binary for your architecture from the [latest release](../../releases/latest). Nightly development releases can be found [here](https://nightly.link/birabittoh/preman/workflows/dev/main?preview).

```bash
chmod +x preman-*
mv preman-* ~/.local/bin/preman
```

### Build from source

```bash
make install        # build release binary and install to ~/.local/bin
make INSTALL_DIR=/usr/local/bin install  # custom install path
```

## Usage

```bash
# Auto-detect all Steam installs
preman

# Also scan custom directories
preman /mnt/games/SteamLibrary /opt/steam
```

## Key Bindings

| Key             | Action                                      |
|-----------------|---------------------------------------------|
| `↑/↓` or `j/k`  | Navigate list                               |
| `←/→` or `h/l`  | Sort list                                   |
| `PgUp/PgDn`     | Scroll by page                              |
| `Home/End`      | Jump to first/last                          |
| `Del`           | Delete selected prefix                      |
| `F` or `/`      | Enter text filter mode                      |
| `I`             | Invert sorting order                        |
| `A`             | Toggle Uninstalled-only / All view          |
| `R`             | Rescan all Steam directories                |
| `D`             | Manage scanned directories                  |
| `?`             | Show help overlay                           |
| `Q` or `Esc`    | Quit                                        |

## AI Notice
AI tools such as Claude Code and Jules were used in this project.
