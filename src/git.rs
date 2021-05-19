/*
 * Copyright 2020 Aurélien Gâteau <mail@agateau.com>
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
use std::env;
use std::fs::File;
use std::process::Command;

// Define this environment variable to print all executed git commands to stderr
const GIT_BONSAI_DEBUG: &str = "GB_DEBUG";

// If a branch is checked out in a separate worktree, then `git branch` prefixes it with this
// string
const WORKTREE_BRANCH_PREFIX: &str = "+ ";

/**
 * Restores the current git branch when dropped
 * Assumes we are on a real branch
 */
pub struct BranchRestorer<'a> {
    repository: &'a Repository,
    branch: String,
}

impl BranchRestorer<'_> {
    pub fn new(repo: &Repository) -> BranchRestorer {
        let current_branch = repo.get_current_branch().expect("Can't get current branch");
        BranchRestorer {
            repository: &repo,
            branch: current_branch,
        }
    }
}

impl Drop for BranchRestorer<'_> {
    fn drop(&mut self) {
        if let Err(_x) = self.repository.checkout(&self.branch) {
            println!("Failed to restore original branch {}", self.branch);
        }
    }
}

pub struct Repository {
    pub dir: String,
}

impl Repository {
    pub fn new(dir: &str) -> Repository {
        Repository {
            dir: dir.to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn clone(dir: &str, url: &str) -> Result<Repository, i32> {
        let repo = Repository::new(dir);
        match repo.git("clone", &[url, dir]) {
            Ok(_x) => Ok(repo),
            Err(x) => Err(x),
        }
    }

    pub fn git(&self, subcommand: &str, args: &[&str]) -> Result<String, i32> {
        let mut cmd = Command::new("git");
        cmd.current_dir(&self.dir);
        cmd.env("LANG", "C");
        cmd.arg(subcommand);
        for arg in args {
            cmd.arg(arg);
        }
        if env::var(GIT_BONSAI_DEBUG).is_ok() {
            eprintln!("DEBUG: pwd={}: git {} {}", self.dir, subcommand, args.join(" "));
        }
        let output = match cmd.output() {
            Ok(x) => x,
            Err(_x) => {
                println!("Failed to execute process");
                return Err(-1);
            }
        };
        if !output.status.success() {
            println!(
                "{}",
                String::from_utf8(output.stderr).expect("Failed to decode command stderr")
            );
            return match output.status.code() {
                Some(code) => Err(code),
                None => Err(-1),
            };
        }
        let out = String::from_utf8(output.stdout).expect("Failed to decode command stdout");
        Ok(out)
    }

    pub fn fetch(&self) -> Result<(), i32> {
        match self.git("fetch", &["--prune"]) {
            Ok(_x) => Ok(()),
            Err(x) => Err(x),
        }
    }

    pub fn list_branches(&self) -> Result<Vec<String>, i32> {
        self.list_branches_internal(&[])
    }

    pub fn list_branches_with_sha1s(&self) -> Result<Vec<(String, String)>, i32> {
        let mut list: Vec<(String, String)> = Vec::new();
        let lines = match self.list_branches_internal(&["-v"]) {
            Ok(x) => x,
            Err(x) => return Err(x),
        };
        for line in lines {
            let mut it = line.split_whitespace();
            let branch = it.next().unwrap().to_string();
            let sha1 = it.next().unwrap().to_string();
            list.push((branch, sha1));
        }
        Ok(list)
    }

    fn list_branches_internal(&self, args: &[&str]) -> Result<Vec<String>, i32> {
        let mut branches: Vec<String> = Vec::new();

        let stdout = match self.git("branch", args) {
            Ok(x) => x,
            Err(x) => return Err(x),
        };
        for line in stdout.lines() {
            if line.starts_with(WORKTREE_BRANCH_PREFIX) {
                continue;
            }
            let branch = line.get(2..).expect("Invalid branch name");
            branches.push(branch.to_string());
        }

        Ok(branches)
    }

    pub fn list_branches_containing(&self, commit: &str) -> Result<Vec<String>, i32> {
        self.list_branches_internal(&["--contains", commit])
    }

    pub fn list_tracking_branches(&self) -> Result<Vec<String>, i32> {
        let mut branches: Vec<String> = Vec::new();
        let lines = match self.list_branches_internal(&["-vv"]) {
            Ok(x) => x,
            Err(x) => return Err(x),
        };
        for line in lines {
            if line.contains("[origin/") && !line.contains(": gone]") {
                let branch = line.split(' ').next();
                branches.push(branch.unwrap().to_string());
            }
        }
        Ok(branches)
    }

    pub fn checkout(&self, branch: &str) -> Result<(), i32> {
        match self.git("checkout", &[branch]) {
            Ok(_x) => Ok(()),
            Err(x) => Err(x),
        }
    }

    pub fn safe_delete_branch(&self, branch: &str) -> Result<(), i32> {
        // A branch is only safe to delete if at least another branch contains it
        let contained_in = self.list_branches_containing(branch).unwrap();
        if contained_in.len() < 2 {
            println!("Not deleting {}, no other branches contain it", branch);
            // TODO: switch to real errors
            return Err(1);
        }
        match self.git("branch", &["-D", branch]) {
            Ok(_x) => Ok(()),
            Err(x) => Err(x),
        }
    }

    pub fn get_current_branch(&self) -> Option<String> {
        let stdout = self.git("branch", &[]);
        if stdout.is_err() {
            return None;
        }
        for line in stdout.unwrap().lines() {
            if line.chars().nth(0) == Some('*') {
                return Some(line[2..].to_string());
            }
        }
        None
    }

    pub fn update_branch(&self) -> Result<(), i32> {
        match self.git("merge", &["--ff-only"]) {
            Ok(out) => {
                println!("{}", out);
                Ok(())
            }
            Err(x) => Err(x),
        }
    }

    pub fn has_changes(&self) -> Result<bool, ()> {
        let stdout = self.git("status", &["--short"]);
        if stdout.is_err() {
            return Err(());
        }
        let has_changes = !stdout.unwrap().is_empty();
        Ok(has_changes)
    }

    #[allow(dead_code)]
    pub fn get_current_sha1(&self) -> Result<String, i32> {
        match self.git("show", &["--no-patch", "--oneline"]) {
            Ok(out) => {
                let sha1 = out.split(' ').next().unwrap();
                Ok(sha1.to_string())
            }
            Err(x) => Err(x),
        }
    }
}

// Used by test code
#[allow(dead_code)]
pub fn create_test_repository(repo_dir: &str) -> Repository {
    let repo = Repository::new(repo_dir);

    repo.git("init", &[]).expect("init failed");
    repo.git("config", &["user.name", "test"])
        .expect("setting username failed");
    repo.git("config", &["user.email", "test@example.com"])
        .expect("setting email failed");

    // Create a file so that we have more than the start commit
    File::create(repo_dir.to_owned() + "/f").unwrap();
    repo.git("add", &["."]).expect("add failed");
    repo.git("commit", &["-m", "init"]).expect("commit failed");

    repo
}

#[cfg(test)]
mod tests {
    extern crate assert_fs;

    use std::fs;
    use super::*;

    #[test]
    fn get_current_branch() {
        let dir = assert_fs::TempDir::new().unwrap();
        let path_str = dir.path().to_str().unwrap();
        let repo = create_test_repository(path_str);
        assert_eq!(repo.get_current_branch().unwrap(), "master");

        repo.git("checkout", &["-b", "test"])
            .expect("create branch failed");
        assert_eq!(repo.get_current_branch().unwrap(), "test");
    }

    #[test]
    fn safe_delete_branch() {
        // GIVEN a repository with a test branch equals to master
        let dir = assert_fs::TempDir::new().unwrap();
        let path_str = dir.path().to_str().unwrap();
        let repo = create_test_repository(path_str);
        assert_eq!(repo.get_current_branch().unwrap(), "master");

        repo.git("branch", &["test"]).unwrap();

        // WHEN I call safe_delete_branch
        let result = repo.safe_delete_branch("test");

        // THEN it succeeds
        assert_eq!(result, Ok(()));

        // AND only the master branch remains
        assert_eq!(repo.list_branches().unwrap(), &["master"]);
    }

    #[test]
    fn cant_delete_unique_branch() {
        // GIVEN a repository with a test branch containing unique content
        let dir = assert_fs::TempDir::new().unwrap();
        let path_str = dir.path().to_str().unwrap();
        let repo = create_test_repository(path_str);
        assert_eq!(repo.get_current_branch().unwrap(), "master");

        repo.git("checkout", &["-b", "test"]).unwrap();
        File::create(path_str.to_owned() + "/test").unwrap();
        repo.git("add", &["test"]).unwrap();
        repo.git("commit", &["-m", &format!("Create file")])
            .unwrap();

        repo.checkout("master").unwrap();

        // WHEN I call safe_delete_branch
        let result = repo.safe_delete_branch("test");

        // THEN it fails
        assert_eq!(result, Err(1));

        // AND the test branch still exists
        assert_eq!(repo.list_branches().unwrap(), &["master", "test"]);
    }

    #[test]
    fn list_branches_with_sha1s() {
        // GIVEN a repository with two branches
        let dir = assert_fs::TempDir::new().unwrap();
        let path_str = dir.path().to_str().unwrap();
        let repo = create_test_repository(path_str);

        repo.git("checkout", &["-b", "test"]).unwrap();
        File::create(path_str.to_owned() + "/test").unwrap();
        repo.git("add", &["test"]).unwrap();
        repo.git("commit", &["-m", &format!("Create file")])
            .unwrap();

        // WHEN I list branches with sha1
        let branches_with_sha1 = repo.list_branches_with_sha1s().unwrap();

        // THEN the list contains two entries
        assert_eq!(branches_with_sha1.len(), 2);

        // AND when switching to each branch, the current sha1 is the expected one
        for (branch, sha1) in branches_with_sha1 {
            repo.git("checkout", &[&branch]).unwrap();
            assert_eq!(repo.get_current_sha1().unwrap(), sha1);
        }
    }

    #[test]
    fn list_branches_skip_worktree_branches() {
        // GIVEN a source repository with two branches
        let tmp_dir = assert_fs::TempDir::new().unwrap();
        let tmp_path_str = tmp_dir.path().to_str().unwrap();

        let source_path_str = tmp_path_str.to_owned() + "/source";
        fs::create_dir_all(&source_path_str).unwrap();
        let source_repo = create_test_repository(&source_path_str);
        source_repo.git("branch", &["topic1"]).unwrap();

        // AND a clone of this repository
        let clone_path_str = tmp_path_str.to_owned() + "/clone";
        fs::create_dir_all(&clone_path_str).unwrap();
        let clone_repo = Repository::clone(&clone_path_str, &source_path_str).unwrap();

        // with the topic1 branch checked-out in a separate worktree
        let worktree_dir = assert_fs::TempDir::new().unwrap();
        let worktree_path_str = worktree_dir.path().to_str().unwrap();
        clone_repo.git("worktree", &["add", worktree_path_str, "topic1"]).unwrap();

        // WHEN I list branches
        let branches = clone_repo.list_branches().unwrap();

        // THEN it does not list worktree branches
        assert_eq!(branches.len(), 1);
        assert_eq!(branches, &["master"]);
    }
}
