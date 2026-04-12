use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use dirs::home_dir;

// ─── Data structures ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SteamGame {
    #[allow(dead_code)]
    pub app_id: u64,
    pub name: String,
    pub cloud_saves: bool,
    pub installed: bool,
}

#[derive(Debug, Clone)]
pub struct WinePrefix {
    pub app_id: u64,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub game: Option<SteamGame>,
}

impl WinePrefix {
    pub fn size_human(&self) -> String { human_size(self.size_bytes) }

    pub fn game_name(&self) -> String {
        self.game.as_ref().map(|g| g.name.clone())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    pub fn is_installed(&self) -> bool {
        self.game.as_ref().map(|g| g.installed).unwrap_or(false)
    }
    pub fn has_cloud_saves(&self) -> bool {
        self.game.as_ref().map(|g| g.cloud_saves).unwrap_or(false)
    }
}

pub fn human_size(bytes: u64) -> String {
    const U: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    let mut s = bytes as f64; let mut i = 0;
    while s >= 1024.0 && i < U.len() - 1 { s /= 1024.0; i += 1; }
    if i == 0 { format!("{} B", bytes) } else { format!("{:.1} {}", s, U[i]) }
}

// ─── VDF text key-value parser ────────────────────────────────────────────────
// Handles lines of the form:  \t"key"\t\t"value"
fn parse_kv_line(line: &str) -> Option<(&str, &str)> {
    let t = line.trim();
    if !t.starts_with('"') { return None; }
    let mut parts = t.splitn(5, '"');
    parts.next()?;                   // empty before first quote
    let key = parts.next()?;
    parts.next()?;                   // whitespace between quotes
    let val = parts.next()?;
    if key.is_empty() { return None; }
    Some((key, val))
}

// ─── appinfo.vdf binary parser ────────────────────────────────────────────────
// Extracts a map of AppID → name from the Steam appcache binary file.
// Format reference: https://github.com/nicklvsa/go-appinfo (and SteamKit2)
//
// File layout:
//   4 bytes  magic (0x27/0x28/0x29  0x44 0x56 0x07)
//   4 bytes  universe (LE u32)
//   repeated app blocks until AppID == 0:
//     4 bytes  app_id  (LE u32)
//     4 bytes  blob size
//     1 byte   state
//     4 bytes  last_updated
//     8 bytes  access_token
//    20 bytes  sha1 of raw kv
//     4 bytes  change_number
//    20 bytes  sha1 of binary kv
//     N bytes  binary KeyValues blob
//
// Binary KeyValues types:
//   0x00  sub-object (recurse; terminated by 0x08)
//   0x01  string (NUL-terminated)
//   0x02  int32  (4 bytes LE)
//   0x03  float  (4 bytes)
//   0x07  uint64 (8 bytes)
//   0x08  end of object
//   0x0b  int32  (alternate tag, same size)
//
// We only care about:   common → name   (string, type 0x01)

pub fn parse_appinfo_vdf(steam_root: &Path) -> HashMap<u64, String> {
    let path = steam_root.join("appcache/appinfo.vdf");
    let data = match std::fs::read(&path) { Ok(d) => d, Err(_) => return HashMap::new() };

    let mut names = HashMap::new();
    if data.len() < 8 { return names; }

    // Validate magic: last 3 bytes of the 4-byte magic are 0x44 0x56 0x07
    if data[1] != 0x44 || data[2] != 0x56 || data[3] != 0x07 {
        return names;
    }

    // Header: 4 bytes magic + 4 bytes universe = 8 bytes.
    // Format version 0x29+ adds an extra 8 bytes (change number + padding) before app entries.
    let mut pos = if data[0] >= 0x29 { 16usize } else { 8usize };

    loop {
        if pos + 4 > data.len() { break; }
        let app_id = u32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as u64;
        pos += 4;
        if app_id == 0 { break; }

        if pos + 4 > data.len() { break; }
        let blob_size = u32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as usize;
        pos += 4;

        // Skip fixed metadata: state(1) + last_updated(4) + access_token(8) + sha1(20) + change_number(4) + sha1_kv(20) = 57 bytes
        const META_SIZE: usize = 57;
        if pos + META_SIZE > data.len() { break; }
        pos += META_SIZE;

        let kv_start = pos;
        let kv_end = (kv_start + blob_size.saturating_sub(META_SIZE)).min(data.len());

        if let Some(name) = extract_name_from_kv(&data[kv_start..kv_end]) {
            names.insert(app_id, name);
        }

        // Advance past the whole blob (blob_size includes the metadata we already skipped)
        pos = (kv_start + blob_size.saturating_sub(META_SIZE)).min(data.len());
    }

    names
}

/// Extract the game name from a binary KV blob in appinfo.vdf.
///
/// Steam's current appinfo.vdf uses a binary format with 4-byte integer keys.
/// The "name" field is encoded as: type=0x01 (string) + key=4 (LE u32) + NUL-terminated value.
/// We do a direct pattern search rather than a full KV traversal.
fn extract_name_from_kv(data: &[u8]) -> Option<String> {
    // Pattern: STRING type (0x01) followed by key 4 as LE u32
    const NEEDLE: &[u8] = &[0x01, 0x04, 0x00, 0x00, 0x00];
    let pos = data.windows(NEEDLE.len()).position(|w| w == NEEDLE)?;
    let val_start = pos + NEEDLE.len();
    let nul = data[val_start..].iter().position(|&b| b == 0)?;
    let s = std::str::from_utf8(&data[val_start..val_start + nul]).ok()?;
    let name = s.trim().to_string();
    if name.is_empty() { None } else { Some(name) }
}

// ─── Steam root discovery ─────────────────────────────────────────────────────

pub fn find_steam_roots(extra_dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut add = |p: PathBuf| {
        let canon = p.canonicalize().unwrap_or(p.clone());
        if p.exists() && seen.insert(canon) { roots.push(p); }
    };
    if let Some(home) = home_dir() {
        add(home.join(".steam/steam"));
        add(home.join(".local/share/Steam"));
        add(home.join(".var/app/com.valvesoftware.Steam/.steam/steam"));
        add(home.join(".var/app/com.valvesoftware.Steam/data/Steam"));
    }
    for d in extra_dirs { add(d.clone()); }
    roots
}

// ─── Library path discovery ───────────────────────────────────────────────────

pub fn find_library_paths(steam_root: &Path) -> Vec<PathBuf> {
    let mut libs: Vec<PathBuf> = vec![steam_root.to_path_buf()];
    let vdf = steam_root.join("steamapps/libraryfolders.vdf");
    if let Ok(content) = std::fs::read_to_string(&vdf) {
        for line in content.lines() {
            if let Some((key, val)) = parse_kv_line(line) {
                if key == "path" {
                    let p = PathBuf::from(val);
                    if p.exists() && !libs.contains(&p) { libs.push(p); }
                }
            }
        }
    }
    libs
}

// ─── Prefix discovery ─────────────────────────────────────────────────────────

pub fn find_prefix_dirs(library_path: &Path) -> Vec<(u64, PathBuf)> {
    let compat = library_path.join("steamapps/compatdata");
    if !compat.exists() { return vec![]; }
    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&compat) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }
            if let Some(Ok(app_id)) = path.file_name().and_then(|n| n.to_str()).map(|s| s.parse::<u64>()) {
                if path.join("pfx").exists() || path.join("pfx.lock").exists() {
                    result.push((app_id, path));
                }
            }
        }
    }
    result
}

// ─── Manifest parsing ─────────────────────────────────────────────────────────

pub fn parse_app_manifest(library_path: &Path, app_id: u64) -> Option<(String, bool)> {
    let manifest = library_path.join(format!("steamapps/appmanifest_{}.acf", app_id));
    let content = std::fs::read_to_string(&manifest).ok()?;
    let mut name: Option<String> = None;
    let mut state_flags: Option<u32> = None;
    for line in content.lines() {
        if let Some((key, val)) = parse_kv_line(line) {
            match key {
                "name"       => name = Some(val.to_string()),
                "StateFlags" => state_flags = val.parse().ok(),
                _ => {}
            }
        }
    }
    let name = name?;
    let installed = state_flags.map(|f| (f & 4) != 0).unwrap_or(false);
    Some((name, installed))
}

// ─── Cloud save detection ─────────────────────────────────────────────────────

pub fn check_cloud_saves(steam_root: &Path, app_id: u64) -> bool {
    let userdata = steam_root.join("userdata");
    if let Ok(entries) = std::fs::read_dir(&userdata) {
        for user_entry in entries.flatten() {
            let base = user_entry.path().join(app_id.to_string());
            if base.join("remote").exists() || base.join("remotecache.vdf").exists() {
                return true;
            }
        }
    }
    false
}

// ─── Directory size ───────────────────────────────────────────────────────────

pub fn dir_size(path: &Path) -> u64 {
    walkdir::WalkDir::new(path).into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .sum()
}

// ─── Full discovery ───────────────────────────────────────────────────────────

pub fn discover_all_prefixes(steam_roots: &[PathBuf]) -> Vec<WinePrefix> {
    let mut prefixes: Vec<WinePrefix> = Vec::new();
    let mut seen_paths: HashSet<PathBuf> = HashSet::new();

    // 1. Collect all library paths across all roots
    let all_libraries: Vec<PathBuf> = steam_roots.iter()
        .flat_map(|r| find_library_paths(r))
        .collect::<HashSet<_>>().into_iter().collect();

    // 2. Build appinfo name cache (handles recently-uninstalled games)
    let mut appinfo_names: HashMap<u64, String> = HashMap::new();
    for root in steam_roots {
        for (id, name) in parse_appinfo_vdf(root) {
            appinfo_names.entry(id).or_insert(name);
        }
    }


    for steam_root in steam_roots {
        for lib in &find_library_paths(steam_root) {
            for (app_id, prefix_path) in find_prefix_dirs(lib) {
                let canon = prefix_path.canonicalize().unwrap_or(prefix_path.clone());
                if !seen_paths.insert(canon) { continue; }

                let size_bytes = dir_size(&prefix_path);

                // Try appmanifest first (gives installed status too)
                let mut game_name: Option<String> = None;
                let mut installed = false;
                for search_lib in &all_libraries {
                    if let Some((n, inst)) = parse_app_manifest(search_lib, app_id) {
                        game_name = Some(n);
                        installed = inst;
                        break;
                    }
                }

                // Fall back to appinfo.vdf cache for uninstalled games
                if game_name.is_none() {
                    game_name = appinfo_names.get(&app_id).cloned();
                }


                let cloud_saves = game_name.is_some()
                    && steam_roots.iter().any(|r| check_cloud_saves(r, app_id));

                let game = game_name.map(|name| SteamGame {
                    app_id, name, cloud_saves, installed,
                });

                prefixes.push(WinePrefix { app_id, path: prefix_path, size_bytes, game });
            }
        }
    }

    prefixes.sort_by(|a, b| a.game_name().to_lowercase().cmp(&b.game_name().to_lowercase()));
    prefixes
}
