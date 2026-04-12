# preman

A terminal UI tool to manage and clean up Steam Wine/Proton prefixes.

```
┌─ PREMAN  ALL  42 prefixes  18.3 GiB total  Roots: Native, Flatpak ──────────────────────────────────────┐
│  Game Name                     App ID      Size       Installed   Cloud   │  Details                    │
│▶ Cyberpunk 2077                1091500     3.2 GiB    ✓           ☁       │  Cyberpunk 2077             │
│  Dark Souls III                374320      1.1 GiB    ✗           ✗       │                             │
│  Elden Ring                    1245620     890.4 MiB  ✓           ☁       │  App ID  1091500            │
│  ...                                                                      │  Size    3.2 GiB            │
└───────────────────────────────────────────────────────────────────────────┘  Status  Installed ✓        │
  ↑/↓ Navigate  [Del] Delete  [F] Filter  [U] Uninstalled  [R] Reload  [?] Help  [Q] Quit                 │
```

## Features

- **Auto-discovers** Steam roots: native (`~/.steam/steam`, `~/.local/share/Steam`) and **Flatpak** (`~/.var/app/com.valvesoftware.Steam`)
- **Custom directories**: add extra Steam roots at runtime or as CLI arguments
- **Game association**: matches each prefix to its game name, install status, and app ID via `appmanifest_*.acf`
- **Cloud save detection**: checks `userdata/<uid>/<appid>/remote/`
- **Filter by text**: live search on game name or app ID

## Installation

### Pre-built binary

Download the latest binary for your architecture from the [releases page](../../releases/latest):

| Architecture | File |
|---|---|
| x86_64 (most PCs) | `preman-<version>-x86_64` |
| aarch64 (ARM 64-bit) | `preman-<version>-aarch64` |
| armv7 (ARM 32-bit) | `preman-<version>-armv7` |
| i686 (32-bit x86) | `preman-<version>-i686` |

```bash
chmod +x preman-*
mv preman-* ~/.local/bin/preman
```

### Build from source

```bash
make install        # build release binary and install to ~/.local/bin
make INSTALL_DIR=/usr/local/bin install  # custom install path
```

Requires Rust 1.75+.

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
| `PgUp/PgDn`     | Scroll by page                              |
| `Home/End`      | Jump to first/last                          |
| `Del`           | Delete selected prefix                      |
| `F` or `/`      | Enter text filter mode                      |
| `U`             | Toggle All / Uninstalled-only view          |
| `R`             | Reload — rescan all Steam directories       |
| `A`             | Add a custom Steam root directory           |
| `?`             | Help overlay                                |
| `Q` or `Esc`    | Quit                                        |
