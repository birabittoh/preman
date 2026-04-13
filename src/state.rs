use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use crate::steam::{WinePrefix, discover_all_prefixes, find_steam_roots};
pub use crate::app_types::{SortColumn, AppMode, FilterMode, DirModalState};

pub struct AppState {
    pub prefixes: Vec<WinePrefix>,
    pub filtered_indices: Vec<usize>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub mode: AppMode,
    pub filter_mode: FilterMode,
    pub filter_text: String,

    pub default_roots: Vec<PathBuf>,
    pub custom_roots: Vec<PathBuf>,

    pub status_message: Option<String>,
    pub total_deleted_bytes: u64,

    pub sort_col: SortColumn,
    pub col_asc: HashMap<SortColumn, bool>,

    pub dir_modal: Option<DirModalState>,

    pub selection: HashSet<usize>,
    pub shift_anchor: usize,
    pub drag_anchor: Option<usize>,
    pub last_click: Option<(usize, std::time::Instant)>,
}

impl AppState {
    pub fn new(extra_dirs: Vec<PathBuf>) -> Self {
        let default_roots = find_steam_roots(&[]);
        let custom_roots = extra_dirs;
        AppState {
            prefixes: Vec::new(),
            filtered_indices: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            mode: AppMode::Startup,
            filter_mode: FilterMode::UninstalledOnly,
            filter_text: String::new(),
            default_roots,
            custom_roots,
            status_message: None,
            total_deleted_bytes: 0,
            sort_col: SortColumn::Name,
            col_asc: {
                let mut m: HashMap<SortColumn, bool> = HashMap::new();
                for col in [SortColumn::Name, SortColumn::AppId, SortColumn::Size,
                            SortColumn::Installed, SortColumn::Cloud] {
                    m.insert(col, col.default_asc());
                }
                m
            },
            dir_modal: None,
            selection: HashSet::new(),
            shift_anchor: 0,
            drag_anchor: None,
            last_click: None,
        }
    }

    fn merge_roots(defaults: &[PathBuf], custom: &[PathBuf]) -> Vec<PathBuf> {
        let mut all = defaults.to_vec();
        for c in custom {
            if !all.contains(c) { all.push(c.clone()); }
        }
        all
    }

    pub fn all_roots(&self) -> Vec<PathBuf> {
        Self::merge_roots(&self.default_roots, &self.custom_roots)
    }

    pub fn reload(&mut self) {
        let roots = self.all_roots();
        self.prefixes = discover_all_prefixes(&roots);
        self.apply_sort_and_filter();
        if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len().saturating_sub(1);
        }
    }

    pub fn sort_asc(&self) -> bool {
        *self.col_asc.get(&self.sort_col).unwrap()
    }

    pub fn apply_sort_and_filter(&mut self) {
        let text = self.filter_text.to_lowercase();

        let mut indices: Vec<usize> = self.prefixes.iter().enumerate()
            .filter(|(_, p)| {
                let mode_ok = match self.filter_mode {
                    FilterMode::All => true,
                    FilterMode::UninstalledOnly => !p.is_installed(),
                };
                let text_ok = text.is_empty()
                    || p.game_name().to_lowercase().contains(&text)
                    || p.app_id.to_string().contains(&text);
                mode_ok && text_ok
            })
            .map(|(i, _)| i)
            .collect();

        let col = self.sort_col;
        let asc = self.sort_asc();
        let prefixes = &self.prefixes;
        indices.sort_by(|&a, &b| {
            let ord = match col {
                SortColumn::Name => prefixes[a].game_name().to_lowercase()
                    .cmp(&prefixes[b].game_name().to_lowercase()),
                SortColumn::AppId => prefixes[a].app_id.cmp(&prefixes[b].app_id),
                SortColumn::Size => prefixes[a].size_bytes.cmp(&prefixes[b].size_bytes),
                SortColumn::Installed => prefixes[a].is_installed().cmp(&prefixes[b].is_installed()),
                SortColumn::Cloud => prefixes[a].has_cloud_saves().cmp(&prefixes[b].has_cloud_saves()),
            };
            if asc { ord } else { ord.reverse() }
        });

        self.filtered_indices = indices;
        if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len().saturating_sub(1);
        }
    }

    pub fn selected_prefix(&self) -> Option<&WinePrefix> {
        self.filtered_indices.get(self.selected).and_then(|&i| self.prefixes.get(i))
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.scroll_offset {
                self.scroll_offset = self.selected;
            }
        }
        self.selection.clear();
        self.shift_anchor = self.selected;
    }

    pub fn move_down(&mut self, visible_height: usize) {
        if self.selected + 1 < self.filtered_indices.len() {
            self.selected += 1;
            if self.selected >= self.scroll_offset + visible_height {
                self.scroll_offset = self.selected + 1 - visible_height;
            }
        }
        self.selection.clear();
        self.shift_anchor = self.selected;
    }

    pub fn extend_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.scroll_offset {
                self.scroll_offset = self.selected;
            }
        }
        self.apply_shift_selection();
    }

    pub fn extend_down(&mut self, visible_height: usize) {
        if self.selected + 1 < self.filtered_indices.len() {
            self.selected += 1;
            if self.selected >= self.scroll_offset + visible_height {
                self.scroll_offset = self.selected + 1 - visible_height;
            }
        }
        self.apply_shift_selection();
    }

    fn apply_shift_selection(&mut self) {
        let lo = self.shift_anchor.min(self.selected);
        let hi = self.shift_anchor.max(self.selected);
        self.selection = (lo..=hi)
            .filter_map(|di| self.filtered_indices.get(di).copied())
            .collect();
    }

    pub fn ctrl_toggle(&mut self, display_idx: usize) {
        if display_idx >= self.filtered_indices.len() { return; }
        let real_idx = self.filtered_indices[display_idx];
        if self.selection.is_empty() {
            if let Some(&cursor_real) = self.filtered_indices.get(self.selected) {
                self.selection.insert(cursor_real);
            }
        }
        if self.selection.contains(&real_idx) {
            self.selection.remove(&real_idx);
            if self.selection.is_empty() {
                self.selected = display_idx;
                self.shift_anchor = display_idx;
            }
        } else {
            self.selection.insert(real_idx);
            self.selected = display_idx;
            self.shift_anchor = display_idx;
        }
    }

    pub fn drag_start(&mut self, display_idx: usize) {
        if display_idx >= self.filtered_indices.len() { return; }
        self.selection.clear();
        self.selected = display_idx;
        self.shift_anchor = display_idx;
        self.drag_anchor = Some(display_idx);
        self.apply_shift_selection();
    }

    pub fn drag_to(&mut self, display_idx: usize) {
        if display_idx >= self.filtered_indices.len() { return; }
        let anchor = match self.drag_anchor { Some(a) => a, None => return };
        self.selected = display_idx;
        let lo = anchor.min(display_idx);
        let hi = anchor.max(display_idx);
        self.selection = (lo..=hi)
            .filter_map(|di| self.filtered_indices.get(di).copied())
            .collect();
    }

    pub fn effective_selection(&self) -> Vec<usize> {
        if self.selection.is_empty() {
            self.filtered_indices.get(self.selected).copied()
                .map(|i| vec![i])
                .unwrap_or_default()
        } else {
            let mut v: Vec<usize> = self.selection.iter().copied().collect();
            v.sort_unstable();
            v
        }
    }

    pub fn any_selected_unsafe(&self) -> bool {
        self.effective_selection().iter()
            .any(|&i| !self.prefixes[i].has_cloud_saves())
    }

    pub fn sort_by_col(&mut self, col: SortColumn) {
        if self.sort_col == col {
            let asc = self.col_asc.get_mut(&col).unwrap();
            *asc = !*asc;
        } else {
            self.sort_col = col;
        }
        self.apply_sort_and_filter();
    }

    pub fn reverse_sort(&mut self) {
        let asc = self.col_asc.get_mut(&self.sort_col).unwrap();
        *asc = !*asc;
        self.apply_sort_and_filter();
    }

    pub fn toggle_filter_mode(&mut self) {
        self.filter_mode = match self.filter_mode {
            FilterMode::All => FilterMode::UninstalledOnly,
            FilterMode::UninstalledOnly => FilterMode::All,
        };
        self.apply_sort_and_filter();
        self.selected = 0; self.scroll_offset = 0;
    }

    pub fn begin_delete(&mut self) {
        let sel = self.effective_selection();
        let pending: Vec<(std::path::PathBuf, String, u64)> = sel.iter()
            .map(|&i| (
                self.prefixes[i].path.clone(),
                self.prefixes[i].game_name().to_string(),
                self.prefixes[i].size_bytes,
            ))
            .collect();
        let current = pending.first().map(|(_, n, _)| n.clone()).unwrap_or_default();
        self.mode = AppMode::Deleting { pending, current };
    }

    pub fn finish_delete(&mut self) {
        self.prefixes.retain(|p| p.path.exists());
        self.selection.clear();
        self.apply_sort_and_filter();
        if self.selected >= self.filtered_indices.len() && self.selected > 0 {
            self.selected -= 1;
        }
        self.shift_anchor = self.selected;
    }

    pub fn open_dir_modal(&mut self) {
        let mut modal = DirModalState::new();
        let defaults = &self.default_roots;
        modal.custom_indices = self.all_roots().iter().enumerate()
            .filter(|(_, p)| !defaults.contains(p))
            .map(|(i, _)| i)
            .collect();
        self.dir_modal = Some(modal);
        self.mode = AppMode::ManageDirs;
    }

    pub fn add_custom_root(&mut self, path_str: &str) -> Result<(), String> {
        let path = PathBuf::from(path_str.trim());
        if !path.exists() {
            return Err(format!("Path does not exist: {}", path.display()));
        }
        if !self.custom_roots.contains(&path) && !self.default_roots.contains(&path) {
            self.custom_roots.push(path);
            self.reload();
        }
        Ok(())
    }

    pub fn remove_custom_root(&mut self, all_roots_index: usize) -> Result<(), String> {
        let all = self.all_roots();
        let path = all.get(all_roots_index)
            .ok_or_else(|| "Invalid index".to_string())?
            .clone();
        if self.default_roots.contains(&path) {
            return Err("Cannot remove an auto-detected Steam directory.".to_string());
        }
        self.custom_roots.retain(|p| p != &path);
        self.reload();
        Ok(())
    }

    pub fn reset_to_default_roots(&mut self) {
        self.custom_roots.clear();
        self.default_roots = find_steam_roots(&[]);
        self.reload();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_types::SortColumn;

    #[test]
    fn test_app_state_sort_and_filter() {
        let mut app = AppState::new(vec![]);
        // Clear everything to have a clean slate
        app.prefixes.clear();
        app.filtered_indices.clear();

        // Add some dummy prefixes
        app.prefixes.push(crate::steam::WinePrefix {
            app_id: 1,
            path: PathBuf::from("/p1"),
            size_bytes: 100,
            game: Some(crate::steam::SteamGame {
                app_id: 1,
                name: "Game B".to_string(),
                cloud_saves: true,
                installed: false,
            }),
        });
        app.prefixes.push(crate::steam::WinePrefix {
            app_id: 2,
            path: PathBuf::from("/p2"),
            size_bytes: 50,
            game: Some(crate::steam::SteamGame {
                app_id: 2,
                name: "Game A".to_string(),
                cloud_saves: false,
                installed: true,
            }),
        });

        app.filter_mode = FilterMode::All;
        app.sort_col = SortColumn::Name;
        *app.col_asc.get_mut(&SortColumn::Name).unwrap() = true;
        app.apply_sort_and_filter();

        assert_eq!(app.filtered_indices.len(), 2);
        // Sorted by name: Game A (idx 1) then Game B (idx 0)
        assert_eq!(app.filtered_indices[0], 1);
        assert_eq!(app.filtered_indices[1], 0);

        // Filter uninstalled only
        app.filter_mode = FilterMode::UninstalledOnly;
        app.apply_sort_and_filter();
        assert_eq!(app.filtered_indices.len(), 1);
        assert_eq!(app.filtered_indices[0], 0); // Game B is uninstalled
    }

    #[test]
    fn test_selection_logic() {
        let mut app = AppState::new(vec![]);
        app.filtered_indices = vec![0, 1, 2, 3];
        app.selected = 1;

        // Effective selection should be just the cursor (filtered_indices[1] == 1)
        assert_eq!(app.effective_selection(), vec![1]);

        // Multi-selection
        app.selection.insert(2);
        app.selection.insert(3);
        let sel = app.effective_selection();
        assert_eq!(sel.len(), 2);
        assert!(sel.contains(&2));
        assert!(sel.contains(&3));
    }
}
