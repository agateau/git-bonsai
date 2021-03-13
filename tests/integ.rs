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

    use git_bonsai::cliargs::CliArgs;
    use git_bonsai::git::create_test_repository;
    use git_bonsai::git::Repository;
    use git_bonsai::libmain::libmain;

    fn create_repository() -> (assert_fs::TempDir, Repository) {
        let dir = assert_fs::TempDir::new().unwrap();
        let path_str = dir.path().to_str().unwrap();
        let repo = create_test_repository(&path_str);
        (dir, repo)
    }

    fn create_branch(repo: &Repository, name: &str) {
        repo.git("checkout", &["-b", name]).unwrap();
        File::create(repo.dir.to_owned() + "/" + name).unwrap();
        repo.git("add", &[name]).unwrap();
        repo.git("commit", &["-m", "Create branch"]).unwrap();
    }

    fn merge_branch(repo: &Repository, name: &str) {
        repo.git("merge", &["--no-ff", name, "-m", "Merging branch"])
            .unwrap();
    }

    fn run_git_bonsai(cwd: &str, argv: &[&str]) -> i32 {
        let mut full_argv = vec!["git-bonsai"];
        full_argv.extend(argv);
        let args = CliArgs::from_iter(full_argv);
        libmain(args, &cwd)
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
}
