fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod test {
    use std::env;

    use git2::{Repository, Branches};


    #[test]
    fn can_list_local_branches_in_repository() {
        let path = env::current_dir().unwrap();
        let repo = Repository::open(path).unwrap();
        
        let branches: Branches<'_> = repo.branches(Some(git2::BranchType::Local)).unwrap();

        let expected = vec!["main".to_owned()];
        let branch_names: Vec<String> = branches.map(|b| {
            let n = b.unwrap().0.name().unwrap().unwrap().to_owned();
            n
        }).collect();

        println!("{:?}", branch_names);

        assert_eq!(expected, branch_names)
    }
}
