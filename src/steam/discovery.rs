use std::collections::HashSet;
use std::path::{Path, PathBuf};
use dirs::home_dir;
use crate::steam::vdf::parse_kv_line;

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
