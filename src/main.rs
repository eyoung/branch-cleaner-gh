use core::fmt;
use std::{
    env,
    error::Error,
    fmt::write,
    path::{self, Path},
};

use git2::{Branches, Repository};

fn main() -> Result<(), Box<dyn Error>> {
    let dir = env::current_dir()?;
    let repo = GitRepository::new(&dir)?;
    println!("{:?}", repo.list_branch_names());
    Ok(())
}

#[cfg(test)]
mod test {
    use std::{
        default, env,
        error::Error,
        io::{Read, Write},
        str::from_utf8,
    };

    use git2::Repository;

    use crate::{get_local_branches, BranchRepository, GitRepository};

    #[test]
    fn can_list_local_branches_in_repository() -> Result<(), Box<dyn Error>> {
        let expected = vec!["main".to_owned()];
        let path = env::current_dir()?;
        let repo = GitRepository::new(&path)?;

        let branch_names = get_local_branches(&repo);

        assert_eq!(expected, branch_names);
        Ok(())
    }

    #[test]
    fn can_list_multiple_local_branches_in_repository() {
        let expected = make_mock_branch_names();

        let branches = MockBranches::new(make_mock_branch_names());

        assert_eq!(expected, get_local_branches(&branches))
    }

    #[test]
    fn error_is_printed_if_path_doesnt_contian_repo() -> Result<(), Box<dyn Error>> {
        let path = env::current_dir()?;
        let repo = FailToOpenRepo::new(&path);
        let mut out = TestErr::default();

        if let Err(e) = repo {
            write!(out, "{}", e);
        }

        assert_eq!(
            out.written.as_deref(),
            Some("Directory is not a git repository")
        );
        Ok(())
    }

    #[derive(Default)]
    struct TestErr {
        written: Option<String>,
    }

    impl Write for TestErr {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            if let Some(ref mut output) = &mut self.written {
                let next = from_utf8(buf).unwrap();
                output.push_str(next);
                Ok(next.len())
            } else {
                self.written = Some(from_utf8(buf).unwrap().to_owned());
                Ok(buf.len())
            }
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    struct MockBranches {
        branches: Vec<String>,
    }

    impl MockBranches {
        fn new(branches: Vec<String>) -> Self {
            Self { branches }
        }
    }

    impl BranchRepository for MockBranches {
        fn list_branch_names(&self) -> Vec<String> {
            return self.branches.clone();
        }

        fn new(_path: &std::path::Path) -> Result<impl BranchRepository, Box<dyn Error>> {
            Ok(Self::new(make_mock_branch_names()))
        }
    }

    fn make_mock_branch_names() -> Vec<String> {
        vec!["main", "feature/multiple"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    struct FailToOpenRepo {}

    impl BranchRepository for FailToOpenRepo {
        fn list_branch_names(&self) -> Vec<String> {
            todo!()
        }

        fn new(path: &std::path::Path) -> Result<Self, Box<dyn Error>> {
            Err(Box::new(
                crate::BranchCleanerError::RepositoryDoesNotExistError,
            ))
        }
    }
}

fn get_local_branches<T: BranchRepository>(branches: &T) -> Vec<String> {
    branches.list_branch_names()
}

trait BranchRepository {
    fn new(path: &Path) -> Result<impl BranchRepository, Box<dyn Error>>;
    fn list_branch_names(&self) -> Vec<String>;
}

struct GitRepository {
    repo: Repository,
}

impl BranchRepository for GitRepository {
    fn list_branch_names(&self) -> Vec<String> {
        self.repo
            .branches(Some(git2::BranchType::Local))
            .unwrap()
            .into_iter()
            .map(|branch| branch.unwrap().0.name().unwrap().unwrap().to_owned())
            .collect()
    }

    fn new(path: &Path) -> Result<impl BranchRepository, Box<dyn Error>> {
        let r = Repository::open(&path)?;
        Ok(GitRepository { repo: r })
    }
}

#[derive(Debug)]
enum BranchCleanerError {
    RepositoryDoesNotExistError,
}

impl std::error::Error for BranchCleanerError {}

impl fmt::Display for BranchCleanerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BranchCleanerError::RepositoryDoesNotExistError => {
                write!(f, "Directory is not a git repository")
            }
        }
    }
}
