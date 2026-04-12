pub mod vdf;
pub mod discovery;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
pub use discovery::{find_steam_roots, find_library_paths, find_prefix_dirs, parse_app_manifest, check_cloud_saves, dir_size};
pub use vdf::parse_appinfo_vdf;

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
