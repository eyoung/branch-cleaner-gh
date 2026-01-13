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
use store::InMemoryBranchStore;

#[cfg(feature = "github-api")]
use store::GitHubBranchStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "github-api")]
    {
        // Use real GitHub API
        let store = GitHubBranchStore::new(".")?;
        store.load().await?;
        tui::run_branch_tui(store)?;
    }
    
    #[cfg(feature = "in-memory")]
    {
        // Use in-memory store for testing
        let store = InMemoryBranchStore::default();
        tui::run_branch_tui(store)?;
    }
    
    Ok(())
}

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
