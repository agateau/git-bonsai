/*
 * Copyright 2021 Aurélien Gâteau <mail@agateau.com>
 *
 * This file is part of git-bonsai.
 *
 * Git-bonsai is free software: you can redistribute it and/or modify it under
 * the terms of the GNU General Public License as published by the Free
 * Software Foundation, either version 3 of the License, or (at your option)
 * any later version.
 *
 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
 * FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License for
 * more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * this program.  If not, see <http://www.gnu.org/licenses/>.
 */
#[cfg(test)]
mod integ {
    extern crate assert_fs;
    extern crate git_bonsai;

    use std::fs::File;
    use structopt::StructOpt;

    use assert_fs::prelude::*;
    use predicates::prelude::*;

    use git_bonsai::app;
    use git_bonsai::cliargs::CliArgs;
    use git_bonsai::git::create_test_repository;
    use git_bonsai::git::Repository;

    fn create_repository() -> (assert_fs::TempDir, Repository) {
        let dir = assert_fs::TempDir::new().unwrap();
        let path_str = dir.path().to_str().unwrap();
        let repo = create_test_repository(&path_str);
        (dir, repo)
    }

    fn clone_repository(url: &str) -> (assert_fs::TempDir, Repository) {
        let dir = assert_fs::TempDir::new().unwrap();
        let path_str = dir.path().to_str().unwrap();
        let repo = Repository::clone(&path_str, &url).unwrap();
        (dir, repo)
    }

    fn create_branch(repo: &Repository, name: &str) {
        repo.git("checkout", &["-b", name]).unwrap();
        create_and_commit_file(&repo, name);
    }

    fn create_and_commit_file(repo: &Repository, name: &str) {
        File::create(repo.dir.to_owned() + "/" + name).unwrap();
        repo.git("add", &[name]).unwrap();
        repo.git("commit", &["-m", &format!("Create file {}", name)]).unwrap();
    }

    fn merge_branch(repo: &Repository, name: &str) {
        repo.git("merge", &["--no-ff", name, "-m", "Merging branch"])
            .unwrap();
    }

    fn run_git_bonsai(cwd: &str, argv: &[&str]) -> i32 {
        let mut full_argv = vec!["git-bonsai"];
        full_argv.extend(argv);
        let args = CliArgs::from_iter(full_argv);
        app::run(args, &cwd)
    }

    #[test]
    fn no_op() {
        // GIVEN a repository with a single branch
        let (dir, _repo) = create_repository();
        let path_str = dir.path().to_str().unwrap();

        // WHEN git-bonsai runs
        let result = run_git_bonsai(&path_str, &["-y"]);

        // THEN it succeeds
        assert_eq!(result, 0);
    }

    #[test]
    fn delete_merged_branch() {
        // GIVEN a repository with two topic branches, topic1 and topic2
        let (dir, repo) = create_repository();
        let path_str = dir.path().to_str().unwrap();
        create_branch(&repo, "topic1");
        create_branch(&repo, "topic2");
        // AND topic1 has been merged in master
        repo.checkout("master").unwrap();
        merge_branch(&repo, "topic1");

        {
            let branches = repo.list_branches().unwrap();
            assert_eq!(branches, ["master", "topic1", "topic2"].to_vec());
        }

        // WHEN git-bonsai runs
        let result = run_git_bonsai(&path_str, &["-y"]);
        assert_eq!(result, 0);

        // THEN only the topic1 branch has been removed
        {
            let branches = repo.list_branches().unwrap();
            assert_eq!(branches, ["master", "topic2"].to_vec());
        }
    }

    #[test]
    fn skip_protected_branch() {
        // GIVEN a repository with a protected, merged branch: "protected"
        let (dir, repo) = create_repository();
        let path_str = dir.path().to_str().unwrap();
        create_branch(&repo, "protected");
        repo.checkout("master").unwrap();
        merge_branch(&repo, "protected");

        {
            let branches = repo.list_branches().unwrap();
            assert_eq!(branches, ["master", "protected"].to_vec());
        }

        // WHEN git-bonsai runs with "-x protected"
        let result = run_git_bonsai(&path_str, &["-y", "-x", "protected"]);
        assert_eq!(result, 0);

        // THEN the protected branch is still there
        {
            let branches = repo.list_branches().unwrap();
            assert_eq!(branches, ["master", "protected"].to_vec());
        }

        // WHEN git-bonsai runs without "-x protected"
        let result = run_git_bonsai(&path_str, &["-y"]);
        assert_eq!(result, 0);

        // THEN the protected branch is gone
        {
            let branches = repo.list_branches().unwrap();
            assert_eq!(branches, ["master"].to_vec());
        }
    }

    #[test]
    fn update_branch() {
        // GIVEN a source repository
        let (source_dir, source_repo) = create_repository();

        // AND a clone of it
        let (clone_dir, _clone_repo) = clone_repository(source_dir.path().to_str().unwrap());
        let clone_dir_str = clone_dir.path().to_str().unwrap();

        // AND a new commit in the source repository
        create_and_commit_file(&source_repo, "new");

        // WHEN git-bonsai runs in the clone
        let result = run_git_bonsai(&clone_dir_str, &["-y"]);
        assert_eq!(result, 0);

        // THEN the clone repository now contains the new commit
        clone_dir.child("new").assert(predicate::path::exists());
    }
}
