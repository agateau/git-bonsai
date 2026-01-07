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
use std::borrow::Cow;
use std::env;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

// Define this environment variable to print all executed git commands to stderr
const GIT_BONSAI_DEBUG: &str = "GB_DEBUG";

// If a branch is checked out in a separate worktree, then `git branch` prefixes it with this
// string
const WORKTREE_BRANCH_PREFIX: &str = "+ ";

// Used by test code when creating repositories.
// Use neither `main` nor `master` to ensure we do not depend on the local setting for the initial
// branch.
pub const INITIAL_BRANCH: &str = "initial-branch";

const GIT_BRANCH_CMD_FIELDS: [&str; 5] = [
    "refname:short",
    "upstream:short",
    "upstream:track,nobracket",
    "worktreepath",
    "committerdate",
];

pub type GitResult<T> = Result<T, GitError>;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum GitError {
    #[error("failed to run git: {0}")]
    FailedToRunGit(String),
    #[error("command `{command}` exited with code {exit_code}:\n{stderr}")]
    CommandFailed {
        command: String,
        exit_code: i32,
        stderr: String,
    },
    #[error("command `{command}` terminated by signal")]
    TerminatedBySignal { command: String },
    #[error("unexpected output: {0}")]
    UnexpectedOutput(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Branch {
    pub name: String,
    pub checkout_state: CheckoutState,
    pub last_commit_date: String, // FIXME: use a proper date format
    pub upstream: Option<Upstream>,
    pub contained_in: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckoutState {
    NotCheckedOut,
    Current,
    WorkTree,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Upstream {
    pub name: String,
    pub ahead_behind: Option<AheadBehind>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AheadBehindStatus {
    UpToDate,
    Diverged,
    Ahead,
    Behind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AheadBehind {
    ahead: u32,
    behind: u32,
}

impl AheadBehind {
    fn new(ahead: u32, behind: u32) -> Self {
        Self { ahead, behind }
    }

    pub fn status(&self) -> AheadBehindStatus {
        if self.ahead == 0 && self.behind == 0 {
            AheadBehindStatus::UpToDate
        } else if self.ahead == 0 {
            AheadBehindStatus::Behind
        } else if self.behind == 0 {
            AheadBehindStatus::Ahead
        } else {
            AheadBehindStatus::Diverged
        }
    }
}

pub struct Repository {
    pub path: PathBuf,
}

impl Repository {
    pub fn new(path: &Path) -> Repository {
        Repository {
            path: path.to_path_buf().canonicalize().unwrap(),
        }
    }

    #[allow(dead_code)]
    pub fn clone(path: &Path, url: &str) -> GitResult<Repository> {
        let repo = Repository::new(path);
        repo.git("clone", &[url, path.to_str().unwrap()])?;
        Ok(repo)
    }

    pub fn git(&self, subcommand: &str, args: &[&str]) -> GitResult<String> {
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
            Err(x) => {
                return Err(GitError::FailedToRunGit(x.to_string()));
            }
        };
        if !output.status.success() {
            let command_str = get_command_str(&cmd);
            return match output.status.code() {
                Some(code) => Err(GitError::CommandFailed {
                    command: command_str,
                    exit_code: code,
                    stderr: String::from_utf8_lossy(&output.stderr).into(),
                }),
                None => Err(GitError::TerminatedBySignal {
                    command: command_str,
                }),
            };
        }
        let out = String::from_utf8(output.stdout).expect("Failed to decode command stdout");
        Ok(out)
    }

    pub fn fetch(&self) -> GitResult<()> {
        self.git("fetch", &["--prune"])?;
        Ok(())
    }

    /// Reads config keys defined with `git config --add <key> <value>`
    pub fn get_config_keys(&self, key: &str) -> GitResult<Vec<String>> {
        let stdout = match self.git("config", &["--get-all", key]) {
            Ok(x) => x,
            Err(x) => match x {
                GitError::CommandFailed { exit_code: 1, .. } => {
                    // Happens when reading a non-existing key
                    return Ok([].to_vec());
                }
                x => {
                    return Err(x);
                }
            },
        };

        let values: Vec<String> = stdout.lines().map(|x| x.into()).collect();
        Ok(values)
    }

    pub fn set_config_key(&self, key: &str, value: &str) -> GitResult<()> {
        self.git("config", &[key, value])?;
        Ok(())
    }

    pub fn find_default_branch(&self) -> GitResult<String> {
        let stdout = self.git("ls-remote", &["--symref", "origin", "HEAD"])?;
        /* Output looks like this:
         *
         * ref: refs/heads/main\tHEAD
         * 960389f1c69e8b9c3fe06d29866d0d193375a6cb\tHEAD
         *
         * We want the extra "main" from the first line
         */
        let line = stdout.lines().next().ok_or_else(|| {
            GitError::UnexpectedOutput("ls-remote returned an empty string".to_string())
        })?;

        let line = line
            .strip_prefix("ref: refs/heads/")
            .ok_or_else(|| GitError::UnexpectedOutput("missing prefix".to_string()))?;

        let line = line
            .strip_suffix("\tHEAD")
            .ok_or_else(|| GitError::UnexpectedOutput("missing suffix".to_string()))?;

        Ok(line.to_string())
    }

    pub fn list_branch_names(&self) -> GitResult<Vec<String>> {
        self.list_branches_internal(&[])
    }

    pub fn list_branches_with_sha1s(&self) -> GitResult<Vec<(String, String)>> {
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

    fn list_branches_internal(&self, args: &[&str]) -> GitResult<Vec<String>> {
        let mut branches: Vec<String> = Vec::new();

        let stdout = self.git("branch", args)?;

        for line in stdout.lines() {
            if line.starts_with(WORKTREE_BRANCH_PREFIX) {
                continue;
            }
            let Some(branch) = line.get(2..) else {
                let msg = format!("invalid line in `git branch` output: {line}");
                return Err(GitError::UnexpectedOutput(msg));
            };
            branches.push(branch.to_string());
        }
        Ok(branches)
    }

    pub fn list_branches_containing(&self, commit: &str) -> GitResult<Vec<String>> {
        self.list_branches_internal(&["--contains", commit])
    }

    pub fn list_tracking_branches(&self) -> GitResult<Vec<String>> {
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

    pub fn list_branches(&self) -> GitResult<Vec<Branch>> {
        // Create a 0-separated format arg
        let format_arg = GIT_BRANCH_CMD_FIELDS
            .iter()
            .map(|x| format!("%({x})"))
            .collect::<Vec<_>>()
            .join("%00");
        let stdout = self.git("branch", &["--format", &format_arg])?;

        let mut branches: Vec<Branch> = vec![];
        for line in stdout.lines() {
            let mut branch = parse_git_branch_line(line, &self.path)?;
            branch.contained_in = self
                .list_branches_containing(&branch.name)?
                .into_iter()
                // Do not list ourselves
                .filter(|x| *x != branch.name)
                .collect();
            branches.push(branch);
        }
        Ok(branches)
    }

    pub fn checkout(&self, branch: &str) -> GitResult<()> {
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

    /// Update the current branch to its upstream if it can be fast-forwarded
    pub fn fast_forward_branch(&self) -> GitResult<()> {
        self.git("merge", &["--ff-only"])?;
        Ok(())
    }

    pub fn has_changes(&self) -> GitResult<bool> {
        let out = self.git("status", &["--short"])?;
        Ok(!out.is_empty())
    }

    #[allow(dead_code)]
    pub fn get_current_sha1(&self) -> GitResult<String> {
        let out = self.git("show", &["--no-patch", "--oneline"])?;
        let sha1 = out.split(' ').next().unwrap().to_string();
        Ok(sha1)
    }
}

/// Parse the value returned by the upstream:track,nobracket field
fn parse_upstream_track_field(field: &str) -> GitResult<Option<AheadBehind>> {
    if field == "gone" {
        return Ok(None);
    }
    if field.is_empty() {
        return Ok(Some(AheadBehind::new(0, 0)));
    }
    let mut ahead_behind = AheadBehind::new(0, 0);
    let tokens: Vec<_> = field.split(", ").collect();

    let create_error = || {
        let msg = format!("failed to parse upstream:track field: `{field}`");
        GitError::UnexpectedOutput(msg)
    };

    for token in tokens {
        let sub_tokens: Vec<_> = token.split(" ").collect();
        match sub_tokens[..] {
            ["ahead", value_str] => {
                ahead_behind.ahead = value_str.parse().map_err(|_| create_error())?;
            }
            ["behind", value_str] => {
                ahead_behind.behind = value_str.parse().map_err(|_| create_error())?;
            }
            _ => {
                return Err(create_error());
            }
        };
    }
    Ok(Some(ahead_behind))
}

/// Parse a line returned by `git branch --format $FORMAT`, where $FORMAT is defined
/// by `GIT_BRANCH_CMD_FIELDS`
fn parse_git_branch_line(line: &str, repo_path: &Path) -> GitResult<Branch> {
    let tokens: Vec<_> = line.split("\0").collect();

    let [refname, upstream_str, track_str, worktree, commit_date_str] = tokens[..] else {
        let msg = format!(
            "Unexpected number of tokens in `{line}`. Expected {} got {}",
            GIT_BRANCH_CMD_FIELDS.len(),
            tokens.len()
        );
        return Err(GitError::UnexpectedOutput(msg));
    };

    let upstream: Option<Upstream> = if upstream_str.is_empty() {
        None
    } else {
        Some(Upstream {
            name: upstream_str.into(),
            ahead_behind: parse_upstream_track_field(track_str)?,
        })
    };

    let checkout_state = {
        let worktree_path = Path::new(worktree);
        if worktree.is_empty() {
            CheckoutState::NotCheckedOut
        } else if repo_path.ancestors().any(|x| x == worktree_path) {
            CheckoutState::Current
        } else {
            CheckoutState::WorkTree
        }
    };

    Ok(Branch {
        name: refname.into(),
        checkout_state,
        upstream,
        last_commit_date: commit_date_str.into(),
        contained_in: vec![],
    })
}

// Used by test code
#[allow(dead_code)]
pub fn create_test_repository(path: &Path) -> Repository {
    let repo = Repository::new(path);

    repo.git("init", &["--initial-branch", INITIAL_BRANCH])
        .expect("init failed");
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

/// Returns a string version of command executed by [cmd]
fn get_command_str(cmd: &Command) -> String {
    let mut parts: Vec<Cow<'_, str>> = vec![cmd.get_program().to_string_lossy()];
    for arg in cmd.get_args() {
        parts.push(arg.to_string_lossy());
    }
    // TODO implement quoting
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    extern crate assert_fs;

    use super::*;

    use yare::parameterized;

    use std::fs;

    #[test]
    fn get_current_branch() {
        let dir = assert_fs::TempDir::new().unwrap();
        let repo = create_test_repository(dir.path());
        assert_eq!(repo.get_current_branch().unwrap(), INITIAL_BRANCH);

        repo.git("checkout", &["-b", "test"])
            .expect("create branch failed");
        assert_eq!(repo.get_current_branch().unwrap(), "test");
    }

    #[test]
    fn delete_branch() {
        // GIVEN a repository with a test branch containing unique content
        let dir = assert_fs::TempDir::new().unwrap();
        let repo = create_test_repository(dir.path());
        assert_eq!(repo.get_current_branch().unwrap(), INITIAL_BRANCH);

        repo.git("checkout", &["-b", "test"]).unwrap();
        File::create(dir.path().join("test")).unwrap();
        repo.git("add", &["test"]).unwrap();
        repo.git("commit", &["-m", &format!("Create file")])
            .unwrap();

        repo.checkout(INITIAL_BRANCH).unwrap();

        // WHEN I call delete_branch
        let result = repo.delete_branch("test");

        // THEN the branch is deleted
        assert_eq!(result, Ok(()));

        // AND only the main branch remains
        assert_eq!(repo.list_branch_names().unwrap(), &[INITIAL_BRANCH]);
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
    fn list_branch_names_skip_worktree_branches() {
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
        let branches = clone_repo.list_branch_names().unwrap();

        // THEN it does not list worktree branches
        assert_eq!(branches.len(), 1);
        assert_eq!(branches, &[INITIAL_BRANCH]);
    }

    #[test]
    fn find_default_branch_happy_path() {
        // GIVEN a source repository
        let tmp_dir = assert_fs::TempDir::new().unwrap();
        let source_path = tmp_dir.path().join("source");
        fs::create_dir_all(&source_path).unwrap();
        create_test_repository(&source_path);

        // AND a clone of this repository
        let clone_path = tmp_dir.path().join("clone");
        fs::create_dir_all(&clone_path).unwrap();
        let clone_repo = Repository::clone(&clone_path, &source_path.to_str().unwrap()).unwrap();

        // WHEN I call find_default_branch() on the clone
        let branch = clone_repo.find_default_branch();

        // THEN it finds the default branch name
        assert_eq!(branch, Ok(INITIAL_BRANCH.to_string()));
    }

    #[test]
    fn find_default_branch_no_remote() {
        // GIVEN a repository without a remote
        let tmp_dir = assert_fs::TempDir::new().unwrap();
        let repo = create_test_repository(&tmp_dir.path());

        // WHEN I call find_default_branch()
        let err = repo.find_default_branch().unwrap_err();

        // THEN it fails
        match err {
            GitError::CommandFailed { exit_code: 128, .. } => (),
            e => panic!("unexpected error: {}", e),
        };
    }

    #[test]
    fn parse_simple_git_branch_line() {
        let tokens = vec![
            "master",
            "origin/master",
            "ahead 2, behind 4",
            "",
            "Tue Dec 30 23:23:09 2025 +0100",
        ];
        assert_eq!(tokens.len(), GIT_BRANCH_CMD_FIELDS.len());
        let line = tokens.join("\x00");
        let branch = parse_git_branch_line(&line, Path::new(".")).unwrap();
        assert_eq!(
            branch,
            Branch {
                name: "master".into(),
                checkout_state: CheckoutState::NotCheckedOut,
                last_commit_date: "Tue Dec 30 23:23:09 2025 +0100".into(),
                upstream: Some(Upstream {
                    name: "origin/master".into(),
                    ahead_behind: Some(AheadBehind::new(2, 4)),
                }),
                contained_in: vec![],
            }
        );
    }

    #[parameterized(
        gone = { "gone", None },
        ahead = { "ahead 23", Some(AheadBehind::new(23, 0)) },
        behind = { "behind 34", Some(AheadBehind::new(0, 34)) },
        diverged = { "ahead 45, behind 56", Some(AheadBehind::new(45, 56)) },
        up_to_date  = { "", Some(AheadBehind::new(0, 0)) },
    )]
    fn test_parse_upstream_track_field(field: &str, expected: Option<AheadBehind>) {
        let result = parse_upstream_track_field(field).unwrap();
        assert_eq!(result, expected);
    }
}
