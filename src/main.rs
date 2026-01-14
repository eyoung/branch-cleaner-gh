use core::fmt;

#[cfg(feature = "github-api")]
mod error;
#[cfg(feature = "github-api")]
mod git;
#[cfg(feature = "github-api")]
mod github;
mod store;
mod tui;

#[cfg(feature = "in-memory")]
use store::{BranchStore, InMemoryBranchStore};

#[cfg(feature = "github-api")]
use store::GitHubBranchStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "github-api")]
    {
        // Use real GitHub API
        let store = GitHubBranchStore::new(".")?;
        // load() returns immediately with LOADING status and spawns async task
        let (initial_branches, update_rx) = store.load()?;
        // Use slow animation for better readability
        let animation_config = tui::AnimationConfig::slow();
        tui::run_branch_tui(store, initial_branches, update_rx, animation_config)?;
    }

    #[cfg(feature = "in-memory")]
    {
        // Use in-memory store for testing (no async loading needed)
        let store = InMemoryBranchStore::default();
        let branches = store.list_branches();
        // Create a dummy channel that never sends (in-memory has no async loading)
        let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let animation_config = tui::AnimationConfig::slow();
        tui::run_branch_tui(store, branches, rx, animation_config)?;
    }

    Ok(())
}

// Branch information structures
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PrStatus {
    OPEN,
    MERGED,
    NONE,
    LOADING,
}

impl PrStatus {
    pub fn to_string(&self) -> String {
        match self {
            PrStatus::OPEN => "open",
            PrStatus::MERGED => "merged",
            PrStatus::NONE => "No PR",
            PrStatus::LOADING => "Loading",
        }
        .to_owned()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct BCBranch {
    pub name: String,
    pub pr_status: PrStatus,
    pub pr_number: Option<u32>,
    pub pr_title: Option<String>,
}

impl BCBranch {
    pub fn new(name: &str, pr_status: PrStatus) -> Self {
        Self {
            name: name.to_owned(),
            pr_status,
            pr_number: None,
            pr_title: None,
        }
    }

    pub fn with_pr(name: &str, pr_status: PrStatus, pr_number: u32, pr_title: &str) -> Self {
        Self {
            name: name.to_owned(),
            pr_status,
            pr_number: Some(pr_number),
            pr_title: Some(pr_title.to_owned()),
        }
    }
}

impl fmt::Display for BCBranch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} | {}", self.name, self.pr_status.to_string())
    }
}
