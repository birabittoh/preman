use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use crate::steam::{WinePrefix, discover_all_prefixes, find_steam_roots};

// ─── Sort column ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum SortColumn { Name, AppId, Size, Installed, Cloud }

impl SortColumn {
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::AppId,
            Self::AppId => Self::Size,
            Self::Size => Self::Installed,
            Self::Installed => Self::Cloud,
            Self::Cloud => Self::Name,
        }
    }
    pub fn prev(self) -> Self {
        match self {
            Self::Name => Self::Cloud,
            Self::AppId => Self::Name,
            Self::Size => Self::AppId,
            Self::Installed => Self::Size,
            Self::Cloud => Self::Installed,
        }
    }
    pub fn index(self) -> usize {
        match self { Self::Name=>0, Self::AppId=>1, Self::Size=>2, Self::Installed=>3, Self::Cloud=>4 }
    }
    /// Default sort direction for each column (true = ascending).
    pub fn default_asc(self) -> bool {
        match self {
            Self::Name      => true,
            Self::AppId     => true,
            Self::Size      => false,   // largest first by default
            Self::Installed => true,    // uninstalled first by default
            Self::Cloud     => false,   // with cloud saves first by default
        }
    }
}

// ─── App modes ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    FilterText,
    /// step 1 = first confirm, step 2 = second confirm (no cloud save)
    ConfirmDelete { step: u8 },
    /// Deletion is in progress. One item is removed per event-loop tick so the
    /// UI can show which prefix is currently being deleted.
    Deleting {
        /// Items still waiting to be removed from disk.
        pending: Vec<(std::path::PathBuf, String, u64)>,
        /// Name displayed in the loading overlay for the current item.
        current: String,
    },
    ManageDirs,
    Help,
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterMode { All, UninstalledOnly }

// ─── Manage-dirs modal state ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum DirModalFocus {
    List,
    Input,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DirModalConfirm { None, ResetToDefaults }

pub struct DirModalState {
    pub focus: DirModalFocus,
    pub selected: usize,
    pub input: String,
    pub confirm: DirModalConfirm,
    /// Which entries are "custom" (not auto-detected defaults)
    pub custom_indices: Vec<usize>,
}

impl DirModalState {
    pub fn new(custom_count: usize) -> Self {
        Self {
            focus: DirModalFocus::List,
            selected: 0,
            input: String::new(),
            confirm: DirModalConfirm::None,
            custom_indices: Vec::new(),
        }
    }
}

// ─── App state ────────────────────────────────────────────────────────────────

pub struct AppState {
    pub prefixes: Vec<WinePrefix>,
    pub filtered_indices: Vec<usize>,
    /// Cursor position (display index into filtered_indices).
    pub selected: usize,
    pub scroll_offset: usize,
    pub mode: AppMode,
    pub filter_mode: FilterMode,
    pub filter_text: String,

    /// Auto-detected roots (never removed by user)
    pub default_roots: Vec<PathBuf>,
    /// User-added roots
    pub custom_roots: Vec<PathBuf>,

    pub status_message: Option<String>,
    pub total_deleted_bytes: u64,

    pub sort_col: SortColumn,
    /// Per-column sort direction; initialized to each column's default.
    pub col_asc: HashMap<SortColumn, bool>,

    pub dir_modal: Option<DirModalState>,

    /// Multi-selection: real prefix indices that are explicitly selected.
    /// When empty, the effective selection is just the cursor item.
    pub selection: HashSet<usize>,
    /// Display index used as the fixed end of a shift-range selection.
    pub shift_anchor: usize,
    /// Display index where a mouse drag started, for drag-to-select.
    pub drag_anchor: Option<usize>,
}

impl AppState {
    pub fn new(extra_dirs: Vec<PathBuf>) -> Self {
        let default_roots = find_steam_roots(&[]);
        let custom_roots = extra_dirs;
        let all_roots = Self::merge_roots(&default_roots, &custom_roots);
        let prefixes = discover_all_prefixes(&all_roots);
        let count = prefixes.len();
        let mut s = AppState {
            prefixes,
            filtered_indices: (0..count).collect(),
            selected: 0,
            scroll_offset: 0,
            mode: AppMode::Normal,
            filter_mode: FilterMode::All,
            filter_text: String::new(),
            default_roots,
            custom_roots,
            status_message: None,
            total_deleted_bytes: 0,
            sort_col: SortColumn::Name,
            col_asc: {
                let mut m = HashMap::new();
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
        };
        s.apply_sort_and_filter();
        s
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

    /// Current sort direction for the active column.
    pub fn sort_asc(&self) -> bool {
        *self.col_asc.get(&self.sort_col).unwrap()
    }

    pub fn apply_sort_and_filter(&mut self) {
        let text = self.filter_text.to_lowercase();

        // Filter
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

        // Sort
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

    /// Shift+Up: extend/contract the shift-range selection upward.
    pub fn extend_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.scroll_offset {
                self.scroll_offset = self.selected;
            }
        }
        self.apply_shift_selection();
    }

    /// Shift+Down: extend/contract the shift-range selection downward.
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

    /// Ctrl+Click: toggle a display-index item in/out of the multi-selection.
    pub fn ctrl_toggle(&mut self, display_idx: usize) {
        if display_idx >= self.filtered_indices.len() { return; }
        let real_idx = self.filtered_indices[display_idx];
        // If entering multi-select for the first time, seed with the cursor item.
        if self.selection.is_empty() {
            if let Some(&cursor_real) = self.filtered_indices.get(self.selected) {
                self.selection.insert(cursor_real);
            }
        }
        if self.selection.contains(&real_idx) {
            self.selection.remove(&real_idx);
            if self.selection.is_empty() {
                // Collapsed back to single-select; keep cursor at clicked item.
                self.selected = display_idx;
                self.shift_anchor = display_idx;
            }
        } else {
            self.selection.insert(real_idx);
            self.selected = display_idx;
            self.shift_anchor = display_idx;
        }
    }

    /// Start a drag selection from a display index.
    pub fn drag_start(&mut self, display_idx: usize) {
        if display_idx >= self.filtered_indices.len() { return; }
        self.selection.clear();
        self.selected = display_idx;
        self.shift_anchor = display_idx;
        self.drag_anchor = Some(display_idx);
        self.apply_shift_selection();
    }

    /// Continue a drag selection to a display index.
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

    /// The real prefix indices that are effectively selected (multi-select set,
    /// or just the cursor item if nothing is explicitly selected).
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

    /// True if any effectively-selected prefix lacks confirmed cloud saves.
    pub fn any_selected_unsafe(&self) -> bool {
        self.effective_selection().iter()
            .any(|&i| !self.prefixes[i].has_cloud_saves())
    }

    pub fn sort_by_col(&mut self, col: SortColumn) {
        if self.sort_col == col {
            // Toggle and remember for this column
            let asc = self.col_asc.get_mut(&col).unwrap();
            *asc = !*asc;
        } else {
            // Switch to new column, restoring its remembered direction
            self.sort_col = col;
        }
        self.apply_sort_and_filter();
    }

    /// Reverse the sort direction of the currently active column and remember it.
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

    /// Build the deletion queue and transition to `AppMode::Deleting`.
    /// The actual filesystem removal is driven one-by-one in the event loop.
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

    /// Remove all prefixes whose paths no longer exist and rebuild display state.
    /// Called after the deletion queue has been fully processed.
    pub fn finish_delete(&mut self) {
        self.prefixes.retain(|p| p.path.exists());
        self.selection.clear();
        self.apply_sort_and_filter();
        if self.selected >= self.filtered_indices.len() && self.selected > 0 {
            self.selected -= 1;
        }
        self.shift_anchor = self.selected;
    }

    // ── Dir modal helpers ────────────────────────────────────────────────────

    pub fn open_dir_modal(&mut self) {
        let mut modal = DirModalState::new(self.custom_roots.len());
        // custom_indices: indices in all_roots() that are custom
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
