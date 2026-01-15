#[cfg(feature = "github-api")]
use std::path::Path;

#[cfg(feature = "github-api")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "github-api")]
use tokio::sync::mpsc::{self, UnboundedReceiver};

#[cfg(feature = "github-api")]
use crate::error::Result;
#[cfg(feature = "github-api")]
use crate::git::GitRepository;
#[cfg(feature = "github-api")]
use crate::github::GitHubClient;
use crate::{BCBranch, PrStatus};

/// BranchStore trait for managing BCBranch objects
/// This is a higher-level abstraction that works with
/// rich domain objects (BCBranch) instead of just branch names
pub trait BranchStore: std::fmt::Debug + Clone + Send + Sync + 'static {
    /// Returns all branches from the store
    fn list_branches(&self) -> Vec<BCBranch>;

    /// Deletes branches by name from the store
    fn delete_branches(&mut self, names: &[String]);
}

/// In-memory implementation of BranchStore for testing and demo purposes
#[derive(Debug, Clone)]
pub struct InMemoryBranchStore {
    branches: Vec<BCBranch>,
}

impl InMemoryBranchStore {
    /// Creates a new InMemoryBranchStore with the given branches
    pub fn new(branches: Vec<BCBranch>) -> Self {
        Self { branches }
    }
}

impl Default for InMemoryBranchStore {
    fn default() -> Self {
        Self {
            branches: vec![
                BCBranch::new("main", PrStatus::NONE),
                BCBranch::with_pr(
                    "feature/add-tui",
                    PrStatus::OPEN,
                    42,
                    "Add TUI interface with Ratatui",
                ),
                BCBranch::with_pr(
                    "old-feature-branch",
                    PrStatus::MERGED,
                    23,
                    "Old feature implementation",
                ),
                BCBranch::new("experimental/refactor", PrStatus::NONE),
                BCBranch::with_pr(
                    "bugfix/handle-errors",
                    PrStatus::MERGED,
                    15,
                    "Fix error handling in repository",
                ),
                BCBranch::with_pr(
                    "feature/github-integration",
                    PrStatus::OPEN,
                    50,
                    "Integrate GitHub API for PR fetching",
                ),
                BCBranch::with_pr(
                    "cleanup/remove-old-code",
                    PrStatus::MERGED,
                    31,
                    "Remove deprecated functions and cleanup",
                ),
            ],
        }
    }
}

impl BranchStore for InMemoryBranchStore {
    fn list_branches(&self) -> Vec<BCBranch> {
        self.branches.clone()
    }

    fn delete_branches(&mut self, names: &[String]) {
        self.branches.retain(|b| !names.contains(&b.name));
    }
}

/// GitHubBranchStore integrates Git and GitHub API
#[cfg(feature = "github-api")]
#[derive(Clone)]
pub struct GitHubBranchStore {
    git: GitRepository,
    github: GitHubClient,
    // Cache to avoid repeated API calls
    cache: Arc<Mutex<Option<Vec<BCBranch>>>>,
}

#[cfg(feature = "github-api")]
impl GitHubBranchStore {
    /// Creates a new GitHubBranchStore from a repository path
    /// Note: Call `load().await` immediately after creation to fetch GitHub data
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let git = GitRepository::open(path)?;

        // Parse GitHub repo info from remote
        let remote_url = git.get_origin_url()?;
        let (owner, repo) = crate::git::parse_github_remote(&remote_url)?;

        // Try to create authenticated client, fall back to offline
        let github = match GitHubClient::from_env(owner.clone(), repo.clone()) {
            Ok(client) => client,
            Err(_) => {
                eprintln!("Warning: GITHUB_TOKEN not found. PR status will show as 'No PR'.");
                GitHubClient::offline(owner, repo)
            }
        };

        Ok(Self {
            git,
            github,
            cache: Arc::new(Mutex::new(None)),
        })
    }

    /// Loads branches from git and starts async PR enrichment
    /// Returns immediately with branches in LOADING state + a receiver for streaming updates
    pub fn load(&self) -> Result<(Vec<BCBranch>, UnboundedReceiver<BCBranch>)> {
        // Get local branches from git (fast, no API calls)
        let branch_names = self.git.list_local_branches()?;

        // Create initial branches with LOADING status
        let initial_branches: Vec<BCBranch> = branch_names
            .iter()
            .map(|name| BCBranch::new(name, PrStatus::LOADING))
            .collect();

        // Update cache with loading state
        *self.cache.lock().unwrap() = Some(initial_branches.clone());

        // Create channel for streaming updates (one branch at a time)
        let (tx, rx) = mpsc::unbounded_channel();

        // Clone what we need for the spawned task
        let github = self.github.clone();
        let cache = Arc::clone(&self.cache);

        // Spawn async task to fetch PR data - streams each branch as it's enriched
        tokio::spawn(async move {
            let enriched = github.enrich_branches_streaming(branch_names, tx).await;

            // Update cache with final state
            *cache.lock().unwrap() = Some(enriched);
        });

        Ok((initial_branches, rx))
    }
}

#[cfg(feature = "github-api")]
impl BranchStore for GitHubBranchStore {
    fn list_branches(&self) -> Vec<BCBranch> {
        // Return from cache (must call load() first!)
        self.cache
            .lock()
            .unwrap()
            .as_ref()
            .cloned()
            .unwrap_or_default()
    }

    fn delete_branches(&mut self, names: &[String]) {
        // Delete from git
        if let Err(e) = self.git.delete_branches(names) {
            eprintln!("Error deleting branches: {}", e);
            return;
        }

        // Update cache by removing deleted branches
        if let Some(ref mut branches) = *self.cache.lock().unwrap() {
            branches.retain(|b| !names.contains(&b.name));
        }
    }
}

#[cfg(feature = "github-api")]
impl std::fmt::Debug for GitHubBranchStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GitHubBranchStore")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_use_in_memory_store() {
        let store = InMemoryBranchStore::default();
        let branches = store.list_branches();
        assert!(!branches.is_empty());
    }

    #[test]
    fn in_memory_store_can_delete_branches() {
        let mut store = InMemoryBranchStore::default();
        let initial_count = store.list_branches().len();

        store.delete_branches(&["main".to_string()]);

        let remaining = store.list_branches();
        assert_eq!(remaining.len(), initial_count - 1);
        assert!(!remaining.iter().any(|b| b.name == "main"));
    }
}
