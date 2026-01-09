use octocrab::{params, Octocrab};

use crate::error::{BranchCleanerError, Result};
use crate::{BCBranch, PrStatus};

/// GitHubClient wraps octocrab with higher-level operations
#[derive(Clone)]
pub struct GitHubClient {
    octocrab: Octocrab,
    owner: String,
    repo: String,
}

impl GitHubClient {
    /// Creates a new client from environment token
    pub fn from_env(owner: String, repo: String) -> Result<Self> {
        let token =
            std::env::var("GITHUB_TOKEN").map_err(|_| BranchCleanerError::TokenNotFound)?;

        let octocrab = Octocrab::builder().personal_token(token).build()?;

        Ok(Self {
            octocrab,
            owner,
            repo,
        })
    }

    /// Creates a client that works offline (marks all as NONE)
    pub fn offline(owner: String, repo: String) -> Self {
        // Use default octocrab (no auth) - will fail gracefully
        let octocrab = Octocrab::default();
        Self {
            octocrab,
            owner,
            repo,
        }
    }

    /// Fetches PR info for a branch name, returns (status, number, title)
    pub async fn get_pr_for_branch(
        &self,
        branch_name: &str,
    ) -> Result<Option<(PrStatus, u32, String)>> {
        // Try to list PRs with this branch as head
        // Use format "owner:branch" for forks, or just "branch" for same repo
        let head_ref = format!("{}:{}", self.owner, branch_name);

        let result = self
            .octocrab
            .pulls(&self.owner, &self.repo)
            .list()
            .head(&head_ref)
            .state(params::State::All) // Get both open and closed PRs
            .per_page(1)
            .send()
            .await;

        match result {
            Ok(page) => {
                if let Some(pr) = page.items.first() {
                    let status = if pr.merged_at.is_some() {
                        PrStatus::MERGED
                    } else {
                        // Check if PR is open using matches! macro
                        match &pr.state {
                            Some(state) if matches!(state, octocrab::models::IssueState::Open) => {
                                PrStatus::OPEN
                            }
                            _ => PrStatus::NONE,
                        }
                    };

                    let title = pr.title.clone().unwrap_or_default();
                    let number = pr.number as u32;

                    Ok(Some((status, number, title)))
                } else {
                    // Try without owner prefix (for same-repo PRs)
                    let result_without_owner = self
                        .octocrab
                        .pulls(&self.owner, &self.repo)
                        .list()
                        .head(branch_name)
                        .state(params::State::All)
                        .per_page(1)
                        .send()
                        .await;

                    match result_without_owner {
                        Ok(page) if page.items.first().is_some() => {
                            let pr = page.items.first().unwrap();
                            let status = if pr.merged_at.is_some() {
                                PrStatus::MERGED
                            } else {
                                match &pr.state {
                                    Some(state) if matches!(state, octocrab::models::IssueState::Open) => {
                                        PrStatus::OPEN
                                    }
                                    _ => PrStatus::NONE,
                                }
                            };

                            let title = pr.title.clone().unwrap_or_default();
                            let number = pr.number as u32;

                            Ok(Some((status, number, title)))
                        }
                        _ => Ok(None),
                    }
                }
            }
            Err(_) => {
                // API error - return None to mark as no PR
                Ok(None)
            }
        }
    }

    /// Enriches branch names with PR information
    pub async fn enrich_branches(&self, branch_names: Vec<String>) -> Vec<BCBranch> {
        let mut branches = Vec::new();

        for name in branch_names {
            let branch = match self.get_pr_for_branch(&name).await {
                Ok(Some((status, number, title))) => BCBranch::with_pr(&name, status, number, &title),
                Ok(None) | Err(_) => {
                    // No PR found or API error - mark as NONE
                    BCBranch::new(&name, PrStatus::NONE)
                }
            };
            branches.push(branch);
        }

        branches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires GITHUB_TOKEN and network
    async fn can_create_client_from_env() {
        let result = GitHubClient::from_env("octocat".to_string(), "Hello-World".to_string());
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn can_create_offline_client() {
        let client = GitHubClient::offline("owner".to_string(), "repo".to_string());
        assert_eq!(client.owner, "owner");
        assert_eq!(client.repo, "repo");
    }

    #[tokio::test]
    #[ignore] // Requires GITHUB_TOKEN and network
    async fn can_fetch_pr_for_branch() {
        // This test requires a real GitHub token and will query the GitHub API
        let client = GitHubClient::from_env("octocat".to_string(), "Hello-World".to_string())
            .expect("GITHUB_TOKEN must be set");

        // The octocat/Hello-World repo may not have PRs, so we just test that it doesn't panic
        let result = client.get_pr_for_branch("main").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires GITHUB_TOKEN and network
    async fn can_enrich_branches() {
        let client = GitHubClient::from_env("octocat".to_string(), "Hello-World".to_string())
            .expect("GITHUB_TOKEN must be set");

        let branches = client
            .enrich_branches(vec!["main".to_string()])
            .await;

        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].name, "main");
    }
}
