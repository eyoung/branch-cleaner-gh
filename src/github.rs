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
    /// Only finds PRs where this branch is the SOURCE (head), not the target (base)
    pub async fn get_pr_for_branch(
        &self,
        branch_name: &str,
    ) -> Result<Option<(PrStatus, u32, String)>> {
        // Try both formats: plain branch name (same-repo PRs) and owner:branch (fork PRs)
        let head_formats = [
            branch_name.to_string(),                           // "feature-branch"
            format!("{}:{}", self.owner, branch_name),          // "owner:feature-branch"
        ];

        for head_format in &head_formats {
            let result = self
                .octocrab
                .pulls(&self.owner, &self.repo)
                .list()
                .head(head_format)
                .state(params::State::All) // Get both open and closed PRs
                .per_page(1)
                .send()
                .await;

            if let Ok(page) = result {
                if let Some(pr) = page.items.first() {
                    // Verify this PR actually has our branch as the head (source)
                    if pr.head.ref_field.as_str() == branch_name {
                        let status = if pr.merged_at.is_some() {
                            PrStatus::MERGED
                        } else {
                            match &pr.state {
                                Some(state) if matches!(state, octocrab::models::IssueState::Open) => {
                                    PrStatus::OPEN
                                }
                                Some(state) if matches!(state, octocrab::models::IssueState::Closed) => {
                                    PrStatus::CLOSED
                                }
                                _ => PrStatus::NONE,
                            }
                        };

                        let title = pr.title.clone().unwrap_or_default();
                        let number = pr.number as u32;
                        return Ok(Some((status, number, title)));
                    }
                }
            }
        }

        Ok(None) // No PR found with this branch as source
    }

    /// Enriches branch names with PR information, streaming each result as it's ready
    pub async fn enrich_branches_streaming(
        &self,
        branch_names: Vec<String>,
        tx: tokio::sync::mpsc::UnboundedSender<BCBranch>,
    ) -> Vec<BCBranch> {
        let mut branches = Vec::new();

        for name in branch_names {
            let branch = match self.get_pr_for_branch(&name).await {
                Ok(Some((status, number, title))) => BCBranch::with_pr(&name, status, number, &title),
                Ok(None) | Err(_) => {
                    // No PR found or API error - mark as NONE
                    BCBranch::new(&name, PrStatus::NONE)
                }
            };

            // Send immediately to TUI (ignore error if receiver dropped)
            let _ = tx.send(branch.clone());

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
    async fn can_enrich_branches_streaming() {
        let client = GitHubClient::from_env("octocat".to_string(), "Hello-World".to_string())
            .expect("GITHUB_TOKEN must be set");

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let branches = client
            .enrich_branches_streaming(vec!["main".to_string()], tx)
            .await;

        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].name, "main");
    }
}
