use std::env;

use git2::{Branches, Repository};

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod test {
    use std::env;

    use git2::Repository;

    use crate::{get_local_branches, LocalBranches};

    #[test]
    fn can_list_local_branches_in_repository() {
        let expected = vec!["main".to_owned()];
        let path = env::current_dir().unwrap();
        let repo = Repository::open(path).unwrap();

        let branch_names = get_local_branches(&repo);

        assert_eq!(expected, branch_names)
    }

    #[test]
    fn can_list_multiple_local_branches_in_repository() {
        let expected = make_mock_branch_names();

        let branches = MockBranches::new(make_mock_branch_names());

        assert_eq!(expected, get_local_branches(&branches))
    }

    struct MockBranches {
        branches: Vec<String>,
    }

    impl MockBranches {
        fn new(branches: Vec<String>) -> Self {
            Self { branches }
        }
    }

    impl LocalBranches for MockBranches {
        fn list_branch_names(&self) -> Vec<String> {
            return self.branches.clone();
        }
    }

    fn make_mock_branch_names() -> Vec<String> {
        vec!["main", "feature/multiple"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    }
}

fn get_local_branches<T: LocalBranches>(branches: &T) -> Vec<String> {
    branches.list_branch_names()
}

trait LocalBranches {
    fn list_branch_names(&self) -> Vec<String>;
}

impl LocalBranches for Repository {
    fn list_branch_names(&self) -> Vec<String> {
        self.branches(Some(git2::BranchType::Local))
            .unwrap()
            .into_iter()
            .map(|branch| branch.unwrap().0.name().unwrap().unwrap().to_owned())
            .collect()
    }
}
