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
    RunExe { prefix_idx: usize, input: String },
    Help,
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterMode { All, UninstalledOnly }
