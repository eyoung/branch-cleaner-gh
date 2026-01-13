use git2::{BranchType, Repository};
use std::path::{Path, PathBuf};

use crate::error::{BranchCleanerError, Result};

/// GitRepository wraps git2::Repository with convenience methods
/// Stores the repo path to enable cloning by reopening
pub struct GitRepository {
    repo: Repository,
    path: PathBuf,
}

impl Clone for GitRepository {
    fn clone(&self) -> Self {
        // Clone by reopening the repository at the same path
        // This is necessary because git2::Repository doesn't implement Clone
        Self::open(&self.path).expect("Failed to reopen repository")
    }
}

// SAFETY: git2::Repository is thread-safe (libgit2 is thread-safe)
// The raw pointer in Repository is just an implementation detail
// and libgit2 handles thread safety internally
unsafe impl Sync for GitRepository {}
unsafe impl Send for GitRepository {}

impl GitRepository {
    /// Opens repository at the given path (or discovers from current dir)
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let repo = Repository::discover(path.as_ref())?;
        let path = repo.path().parent().unwrap_or(repo.path()).to_path_buf();
        Ok(Self { repo, path })
    }

    /// Lists all local branch names
    pub fn list_local_branches(&self) -> Result<Vec<String>> {
        let branches = self
            .repo
            .branches(Some(BranchType::Local))?
            .filter_map(|b| {
                b.ok().and_then(|(branch, _)| {
                    branch.name().ok()?.map(|s| s.to_owned())
                })
            })
            .collect();
        Ok(branches)
    }

    /// Deletes local branches by name
    pub fn delete_branches(&self, names: &[String]) -> Result<()> {
        for name in names {
            if let Ok(mut branch) = self.repo.find_branch(name, BranchType::Local) {
                branch.delete()?;
            }
        }
        Ok(())
    }

    /// Gets the origin remote URL
    pub fn get_origin_url(&self) -> Result<String> {
        let remote = self
            .repo
            .find_remote("origin")
            .map_err(|_| BranchCleanerError::NoOriginRemote)?;

        let url = remote.url().ok_or_else(|| {
            BranchCleanerError::RemoteParseError("Invalid UTF-8 in remote URL".into())
        })?;

        Ok(url.to_owned())
    }
}

/// Parses GitHub owner and repo from a git remote URL
/// Supports both SSH (git@github.com:owner/repo.git) and HTTPS formats
pub fn parse_github_remote(url: &str) -> Result<(String, String)> {
    use git_url_parse::GitUrl;

    let parsed = GitUrl::parse(url)
        .map_err(|e| BranchCleanerError::RemoteParseError(e.to_string()))?;

    let owner = parsed
        .owner
        .ok_or_else(|| BranchCleanerError::RemoteParseError("No owner in URL".into()))?;

    let name = parsed.name;

    Ok((owner, name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_parse_ssh_github_url() {
        let (owner, repo) = parse_github_remote("git@github.com:owner/repo.git").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }

    #[test]
    fn can_parse_https_github_url() {
        let (owner, repo) = parse_github_remote("https://github.com/owner/repo").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }

    #[test]
    fn can_parse_https_github_url_with_git_extension() {
        let (owner, repo) = parse_github_remote("https://github.com/owner/repo.git").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }

    #[test]
    fn error_on_invalid_url() {
        // Test with a clearly invalid git URL
        let result = parse_github_remote("not-a-valid-url");
        assert!(result.is_err());
    }

    // Integration test with real repository - only run manually
    #[test]
    #[ignore]
    fn can_list_branches_in_real_repo() {
        let git = GitRepository::open(".").unwrap();
        let branches = git.list_local_branches().unwrap();
        assert!(!branches.is_empty());
    }

    #[test]
    #[ignore]
    fn can_get_origin_url_in_real_repo() {
        let git = GitRepository::open(".").unwrap();
        let url = git.get_origin_url().unwrap();
        println!("Origin URL: {}", url);
        assert!(!url.is_empty());
    }
}
