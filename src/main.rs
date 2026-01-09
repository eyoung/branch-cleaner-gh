use core::fmt;

mod error;
mod git;
mod github;
mod store;
mod tui;

// Re-export for convenience
pub use error::{BranchCleanerError, Result};
pub use store::{BranchStore, GitHubBranchStore, InMemoryBranchStore};

/// Enum wrapper to allow either GitHub or In-Memory store
#[derive(Debug, Clone)]
pub enum AppBranchStore {
    GitHub(GitHubBranchStore),
    InMemory(InMemoryBranchStore),
}

impl BranchStore for AppBranchStore {
    fn list_branches(&self) -> Vec<BCBranch> {
        match self {
            AppBranchStore::GitHub(store) => store.list_branches(),
            AppBranchStore::InMemory(store) => store.list_branches(),
        }
    }

    fn delete_branches(&mut self, names: &[String]) {
        match self {
            AppBranchStore::GitHub(store) => store.delete_branches(names),
            AppBranchStore::InMemory(store) => store.delete_branches(names),
        }
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Detect repository mode and create appropriate store
    let store = detect_repository_mode().await;

    // Run the TUI application with the store
    tui::run_branch_tui(store).await?;
    Ok(())
}

async fn detect_repository_mode() -> AppBranchStore {
    match GitHubBranchStore::new(".") {
        Ok(store) => {
            // Pre-fetch GitHub data asynchronously
            if let Err(e) = store.load().await {
                eprintln!("Warning: Failed to load GitHub data: {}", e);
            }
            AppBranchStore::GitHub(store)
        }
        Err(e) => {
            eprintln!("Could not initialize GitHub store: {}", e);
            eprintln!("Running in demo mode with fake data.");
            AppBranchStore::InMemory(InMemoryBranchStore::default())
        }
    }
}

// Tests have been moved to their respective modules (git.rs, github.rs, store.rs, tui.rs)


// Branch information structures
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PrStatus {
    OPEN,
    MERGED,
    NONE,
}

impl PrStatus {
    pub fn to_string(&self) -> String {
        match self {
            PrStatus::OPEN => "open",
            PrStatus::MERGED => "merged",
            PrStatus::NONE => "No PR",
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
