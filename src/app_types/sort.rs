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
    #[allow(dead_code)]
    pub fn index(self) -> usize {
        match self { Self::Name=>0, Self::AppId=>1, Self::Size=>2, Self::Installed=>3, Self::Cloud=>4 }
    }
    /// Default sort direction for each column (true = ascending).
    pub fn default_asc(self) -> bool {
        match self {
            Self::Name      => true,
            Self::AppId     => true,
            Self::Size      => false,   // largest first by default
            Self::Installed => false,   // installed first by default
            Self::Cloud     => false,   // with cloud saves first by default
        }
    }
}
