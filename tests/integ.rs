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
mod tests {
    extern crate assert_cmd;
    extern crate assert_fs;
    extern crate git_bonsai;

    use std::fs::File;

    use git_bonsai::git::create_test_repository;
    use git_bonsai::git::Repository;
    use assert_cmd::Command;

    fn create_repository() -> (assert_fs::TempDir, Repository) {
        let dir = assert_fs::TempDir::new().unwrap();
        let path_str = dir.path().to_str().unwrap();
        let repo = create_test_repository(&path_str);
        (dir, repo)
    }

    fn create_branch(repo: &Repository, name: &str) {
        repo.git("checkout", &["-b", name]).unwrap();
        File::create(repo.get_dir().to_owned() + "/" + name).unwrap();
        repo.git("add", &[name]).unwrap();
        repo.git("commit", &["-m", "Create branch"]).unwrap();
    }

    fn merge_branch(repo: &Repository, name: &str) {
        repo.git("merge", &["--no-ff", name, "-m", "Merging branch"]).unwrap();
    }

    #[test]
    fn no_op() {
        let (dir, _repo) = create_repository();
        let path_str = dir.path().to_str().unwrap();

        let mut cmd = Command::cargo_bin("git-bonsai").unwrap();
        cmd.current_dir(&path_str);
        cmd.assert().success();
    }

    #[test]
    fn delete_merged_branch() {
        let (dir, repo) = create_repository();
        let path_str = dir.path().to_str().unwrap();
        create_branch(&repo, "topic1");
        repo.checkout("master").unwrap();
        merge_branch(&repo, "topic1");

        {
            let branches = repo.list_branches().unwrap();
            assert_eq!(branches, ["master", "topic1"].to_vec());
        }

        let mut cmd = Command::cargo_bin("git-bonsai").unwrap();
        cmd.arg("-y");
        cmd.current_dir(&path_str);
        cmd.assert().success();

        {
            let branches = repo.list_branches().unwrap();
            assert_eq!(branches, ["master"].to_vec());
        }
    }
}
