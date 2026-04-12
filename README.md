# steam-prefix-manager

A terminal UI tool to manage and clean up Steam Wine/Proton prefixes (`~/.steam/steam/steamapps/compatdata`).

```
┌─ STEAM PREFIX MANAGER  ALL  42 prefixes  18.3 GiB total  Roots: Native, Flatpak ──────────────────────┐
│  Game Name                     App ID      Size       Installed   Cloud  │  Details                   │
│▶ Cyberpunk 2077                1091500     3.2 GiB    ✓           ☁      │  Cyberpunk 2077             │
│  Dark Souls III                374320      1.1 GiB    ✗           ✗      │                             │
│  Elden Ring                    1245620     890.4 MiB  ✓           ☁      │  App ID  1091500            │
│  ...                                                                      │  Size    3.2 GiB            │
└───────────────────────────────────────────────────────────────────────────┘  Status  Installed ✓       │
  ↑/↓ Navigate  [Del] Delete  [F] Filter  [U] Uninstalled  [R] Reload  [?] Help  [Q] Quit               │
```

## Features

- **Auto-discovers** Steam roots: native (`~/.steam/steam`, `~/.local/share/Steam`) and **Flatpak** (`~/.var/app/com.valvesoftware.Steam`)
- **Custom directories**: add extra Steam roots at runtime with `[A]` or as CLI arguments
- **Game association**: matches each prefix to its game name, install status, and app ID via `appmanifest_*.acf`
- **Cloud save detection**: checks `userdata/<uid>/<appid>/remote/` — warns **twice** before deleting prefixes with no detected cloud saves
- **Filter by text**: live search on game name or app ID
- **Uninstalled filter**: toggle with `[U]` to show only prefixes for games no longer installed — great for cleaning up
- **Safe deletion**: single confirm for games with cloud saves; **double confirm** for games without
- **Freed space counter**: tracks total bytes deleted in the session
- **Size reporting**: human-readable sizes (MiB/GiB), highlights large prefixes in amber

## Installation

### Pre-built binary

```bash
cp steam-prefix-manager ~/.local/bin/
chmod +x ~/.local/bin/steam-prefix-manager
```

### Build from source

```bash
cargo build --release
cp target/release/steam-prefix-manager ~/.local/bin/
```

Requires Rust 1.75+.

## Usage

```bash
# Auto-detect all Steam installs
steam-prefix-manager

# Also scan custom directories
steam-prefix-manager /mnt/games/SteamLibrary /opt/steam
```

## Key Bindings

| Key             | Action                                      |
|-----------------|---------------------------------------------|
| `↑/↓` or `j/k` | Navigate list                               |
| `PgUp/PgDn`     | Scroll by page                              |
| `Home/End`      | Jump to first/last                          |
| `Del` or `d`    | Delete selected prefix                      |
| `F` or `/`      | Enter text filter mode                      |
| `U`             | Toggle All / Uninstalled-only view          |
| `R`             | Reload — rescan all Steam directories       |
| `A`             | Add a custom Steam root directory           |
| `?`             | Help overlay                                |
| `Q` or `Esc`    | Quit                                        |

## Cloud Save Warning

If a game has **no detected cloud saves** (no `userdata/<uid>/<appid>/remote/` directory), you will be asked to confirm deletion **twice** with escalating warnings, since deleting the prefix will permanently erase local save data.

## How It Works

1. Scans all Steam roots for `steamapps/compatdata/<appid>/pfx/` directories
2. Reads `steamapps/appmanifest_<appid>.acf` for game name and install state
3. Reads `steamapps/libraryfolders.vdf` to find additional library paths
4. Checks `userdata/*/appid/remote/` or `remotecache.vdf` for cloud save evidence
5. Computes directory sizes recursively

## Notes

- Only prefixes with a `pfx/` subdirectory are shown (real Proton prefixes)
- Symlinks are resolved to avoid counting the same prefix twice
- The "Installed" column uses `StateFlags & 4` from the ACF manifest
- Cloud save detection is heuristic — absence of a remote directory doesn't guarantee no cloud saves exist on Steam's servers
