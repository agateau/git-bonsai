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
use std::process::Command;
use std::fs::File;

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
    dir: String,
}

impl Repository {
    pub fn new(dir: &str) -> Repository {
        Repository {
            dir: dir.to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn get_dir(&self) -> &str {
        &self.dir
    }

    pub fn git(&self, subcommand: &str, args: &[&str]) -> Result<String, i32> {
        let mut cmd = Command::new("git");
        cmd.current_dir(&self.dir);
        cmd.env("LANG", "C");
        cmd.arg(subcommand);
        for arg in args {
            cmd.arg(arg);
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

    fn list_branches_internal(&self, args: &[&str]) -> Result<Vec<String>, i32> {
        let mut branches: Vec<String> = Vec::new();

        let stdout = match self.git("branch", args) {
            Ok(x) => x,
            Err(x) => return Err(x),
        };
        for line in stdout.lines() {
            let branch = line.get(2..).expect("Invalid branch name");
            branches.push(branch.to_string());
        }

        Ok(branches)
    }

    pub fn list_merged_branches(&self, branch: &str) -> Result<Vec<String>, i32> {
        self.list_branches_internal(&["--merged", branch])
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

    pub fn delete_branch(&self, branch: &str) -> Result<(), i32> {
        match self.git("branch", &["-d", branch]) {
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
}

// Used by test code
#[allow(dead_code)]
pub fn create_test_repository(repo_dir: &str) -> Repository {
    let repo = Repository::new(repo_dir);

    repo.git("init", &[]).expect("init failed");
    repo.git("config", &["user.name", "test"]).expect("setting username failed");
    repo.git("config", &["user.email", "test@example.com"]).expect("setting email failed");

    // Create a file so that we have more than the start commit
    File::create(repo_dir.to_owned() + "/f").unwrap();
    repo.git("add", &["."]).expect("add failed");
    repo.git("commit", &["-m", "init"]).expect("commit failed");

    repo
}

#[cfg(test)]
mod tests {
    extern crate assert_cmd;
    extern crate assert_fs;

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
}
