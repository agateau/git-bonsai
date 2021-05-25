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
use std::fmt;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;

// Define this environment variable to print all executed git commands to stderr
const GIT_BONSAI_DEBUG: &str = "GB_DEBUG";

// If a branch is checked out in a separate worktree, then `git branch` prefixes it with this
// string
const WORKTREE_BRANCH_PREFIX: &str = "+ ";

#[derive(Debug, PartialEq)]
pub enum GitError {
    FailedToRunGit,
    CommandFailed { exit_code: i32 },
    TerminatedBySignal,
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitError::FailedToRunGit => {
                write!(f, "Failed to run git")
            }
            GitError::CommandFailed { exit_code: e } => {
                write!(f, "Command exited with code {}", e)
            }
            GitError::TerminatedBySignal => {
                write!(f, "Terminated by signal")
            }
        }
    }
}

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
    pub path: PathBuf,
}

impl Repository {
    pub fn new(path: &Path) -> Repository {
        Repository {
            path: path.to_path_buf(),
        }
    }

    #[allow(dead_code)]
    pub fn clone(path: &Path, url: &str) -> Result<Repository, GitError> {
        let repo = Repository::new(path);
        repo.git("clone", &[url, path.to_str().unwrap()])?;
        Ok(repo)
    }

    pub fn git(&self, subcommand: &str, args: &[&str]) -> Result<String, GitError> {
        let mut cmd = Command::new("git");
        cmd.current_dir(&self.path);
        cmd.env("LANG", "C");
        cmd.arg(subcommand);
        for arg in args {
            cmd.arg(arg);
        }
        if env::var(GIT_BONSAI_DEBUG).is_ok() {
            eprintln!(
                "DEBUG: pwd={}: git {} {}",
                self.path.to_str().unwrap(),
                subcommand,
                args.join(" ")
            );
        }
        let output = match cmd.output() {
            Ok(x) => x,
            Err(_x) => {
                println!("Failed to execute process");
                return Err(GitError::FailedToRunGit);
            }
        };
        if !output.status.success() {
            // TODO: store error message in GitError
            println!(
                "{}",
                String::from_utf8(output.stderr).expect("Failed to decode command stderr")
            );
            return match output.status.code() {
                Some(code) => Err(GitError::CommandFailed { exit_code: code }),
                None => Err(GitError::TerminatedBySignal),
            };
        }
        let out = String::from_utf8(output.stdout).expect("Failed to decode command stdout");
        Ok(out)
    }

    pub fn fetch(&self) -> Result<(), GitError> {
        self.git("fetch", &["--prune"])?;
        Ok(())
    }

    pub fn list_branches(&self) -> Result<Vec<String>, GitError> {
        self.list_branches_internal(&[])
    }

    pub fn list_branches_with_sha1s(&self) -> Result<Vec<(String, String)>, GitError> {
        let mut list: Vec<(String, String)> = Vec::new();

        let lines = self.list_branches_internal(&["-v"])?;

        for line in lines {
            let mut it = line.split_whitespace();
            let branch = it.next().unwrap().to_string();
            let sha1 = it.next().unwrap().to_string();
            list.push((branch, sha1));
        }
        Ok(list)
    }

    fn list_branches_internal(&self, args: &[&str]) -> Result<Vec<String>, GitError> {
        let mut branches: Vec<String> = Vec::new();

        let stdout = self.git("branch", args)?;

        for line in stdout.lines() {
            if line.starts_with(WORKTREE_BRANCH_PREFIX) {
                continue;
            }
            let branch = line.get(2..).expect("Invalid branch name");
            branches.push(branch.to_string());
        }
        Ok(branches)
    }

    pub fn list_branches_containing(&self, commit: &str) -> Result<Vec<String>, GitError> {
        self.list_branches_internal(&["--contains", commit])
    }

    pub fn list_tracking_branches(&self) -> Result<Vec<String>, GitError> {
        let mut branches: Vec<String> = Vec::new();

        let lines = self.list_branches_internal(&["-vv"])?;

        for line in lines {
            if line.contains("[origin/") && !line.contains(": gone]") {
                let branch = line.split(' ').next();
                branches.push(branch.unwrap().to_string());
            }
        }
        Ok(branches)
    }

    pub fn checkout(&self, branch: &str) -> Result<(), GitError> {
        self.git("checkout", &[branch])?;
        Ok(())
    }

    pub fn delete_branch(&self, branch: &str) -> Result<(), GitError> {
        self.git("branch", &["-D", branch])?;
        Ok(())
    }

    pub fn get_current_branch(&self) -> Option<String> {
        let stdout = self.git("branch", &[]);
        if stdout.is_err() {
            return None;
        }
        for line in stdout.unwrap().lines() {
            if line.starts_with('*') {
                return Some(line[2..].to_string());
            }
        }
        None
    }

    pub fn update_branch(&self) -> Result<(), GitError> {
        let out = self.git("merge", &["--ff-only"])?;
        println!("{}", out);
        Ok(())
    }

    pub fn has_changes(&self) -> Result<bool, GitError> {
        let out = self.git("status", &["--short"])?;
        Ok(!out.is_empty())
    }

    #[allow(dead_code)]
    pub fn get_current_sha1(&self) -> Result<String, GitError> {
        let out = self.git("show", &["--no-patch", "--oneline"])?;
        let sha1 = out.split(' ').next().unwrap().to_string();
        Ok(sha1)
    }
}

// Used by test code
#[allow(dead_code)]
pub fn create_test_repository(path: &Path) -> Repository {
    let repo = Repository::new(path);

    repo.git("init", &[]).expect("init failed");
    repo.git("config", &["user.name", "test"])
        .expect("setting username failed");
    repo.git("config", &["user.email", "test@example.com"])
        .expect("setting email failed");

    // Create a file so that we have more than the start commit
    File::create(path.join("f")).unwrap();
    repo.git("add", &["."]).expect("add failed");
    repo.git("commit", &["-m", "init"]).expect("commit failed");

    repo
}

#[cfg(test)]
mod tests {
    extern crate assert_fs;

    use super::*;
    use std::fs;

    #[test]
    fn get_current_branch() {
        let dir = assert_fs::TempDir::new().unwrap();
        let repo = create_test_repository(dir.path());
        assert_eq!(repo.get_current_branch().unwrap(), "master");

        repo.git("checkout", &["-b", "test"])
            .expect("create branch failed");
        assert_eq!(repo.get_current_branch().unwrap(), "test");
    }

    #[test]
    fn delete_branch() {
        // GIVEN a repository with a test branch containing unique content
        let dir = assert_fs::TempDir::new().unwrap();
        let repo = create_test_repository(dir.path());
        assert_eq!(repo.get_current_branch().unwrap(), "master");

        repo.git("checkout", &["-b", "test"]).unwrap();
        File::create(dir.path().join("test")).unwrap();
        repo.git("add", &["test"]).unwrap();
        repo.git("commit", &["-m", &format!("Create file")])
            .unwrap();

        repo.checkout("master").unwrap();

        // WHEN I call delete_branch
        let result = repo.delete_branch("test");

        // THEN the branch is deleted
        assert_eq!(result, Ok(()));

        // AND only the master branch remains
        assert_eq!(repo.list_branches().unwrap(), &["master"]);
    }

    #[test]
    fn list_branches_with_sha1s() {
        // GIVEN a repository with two branches
        let dir = assert_fs::TempDir::new().unwrap();
        let repo = create_test_repository(dir.path());

        repo.git("checkout", &["-b", "test"]).unwrap();
        File::create(dir.path().join("test")).unwrap();
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

        let source_path = tmp_dir.path().join("source");
        fs::create_dir_all(&source_path).unwrap();
        let source_repo = create_test_repository(&source_path);
        source_repo.git("branch", &["topic1"]).unwrap();

        // AND a clone of this repository
        let clone_path = tmp_dir.path().join("clone");
        fs::create_dir_all(&clone_path).unwrap();
        let clone_repo = Repository::clone(&clone_path, &source_path.to_str().unwrap()).unwrap();

        // with the topic1 branch checked-out in a separate worktree
        let worktree_dir = assert_fs::TempDir::new().unwrap();
        let worktree_path_str = worktree_dir.path().to_str().unwrap();
        clone_repo
            .git("worktree", &["add", worktree_path_str, "topic1"])
            .unwrap();

        // WHEN I list branches
        let branches = clone_repo.list_branches().unwrap();

        // THEN it does not list worktree branches
        assert_eq!(branches.len(), 1);
        assert_eq!(branches, &["master"]);
    }
}
