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
    pub fn new() -> Self {
        Self {
            focus: DirModalFocus::List,
            selected: 0,
            input: String::new(),
            confirm: DirModalConfirm::None,
            custom_indices: Vec::new(),
        }
    }
}
