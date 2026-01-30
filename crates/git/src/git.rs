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
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{DateTime, FixedOffset};
use thiserror::Error;

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
    "committerdate:iso-strict",
];

pub type GitResult<T> = Result<T, GitError>;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
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
    #[error("branch `{0}` cannot be fast-forwarded")]
    CannotBeFastForwarded(String),
    #[error("cannot parse commit date: {0}")]
    InvalidCommitDate(String),
}

impl From<chrono::ParseError> for GitError {
    fn from(value: chrono::ParseError) -> Self {
        GitError::InvalidCommitDate(value.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Branch {
    pub name: String,
    pub checkout_state: CheckoutState,
    pub last_commit_date: DateTime<FixedOffset>,
    pub upstream: Option<Upstream>,
}

impl Branch {
    pub fn can_be_fast_forwarded(&self) -> bool {
        let upstream = match &self.upstream {
            None => {
                return false;
            }
            Some(x) => x,
        };
        let ahead_behind = match &upstream.ahead_behind {
            None => {
                return false;
            }
            Some(x) => x,
        };
        ahead_behind.status() == AheadBehindStatus::Behind
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckoutState {
    NotCheckedOut,
    Current,
    WorkTree(PathBuf),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Upstream {
    pub name: String,
    /// Can be None if the remote branch is gone
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

#[derive(Clone)]
pub struct Repository {
    path: PathBuf,
}

impl Repository {
    pub fn new(path: &Path) -> Repository {
        Repository {
            path: path.to_path_buf().canonicalize().unwrap(),
        }
    }

    pub fn init(&self) -> GitResult<()> {
        self.git("init", &["--initial-branch", INITIAL_BRANCH])?;
        Ok(())
    }

    pub fn init_bare(&self) -> GitResult<()> {
        self.git("init", &["--initial-branch", INITIAL_BRANCH, "--bare"])?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn clone_repository(path: &Path, url: &str) -> GitResult<Repository> {
        let repo = Repository::new(path);
        repo.git("clone", &[url, path.to_str().unwrap()])?;
        Ok(repo)
    }

    /// Low-level function to execute any git command and return its stdout
    pub fn git(&self, subcommand: &str, args: &[&str]) -> GitResult<String> {
        let mut cmd = Command::new("git");
        cmd.current_dir(&self.path);
        cmd.env("LANG", "C");
        cmd.arg(subcommand);
        for arg in args {
            cmd.arg(arg);
        }
        let command_str = get_command_str(&cmd);
        log::info!(
            "running git command. pwd={} command=`{command_str}`",
            self.path.display()
        );
        let output = match cmd.output() {
            Ok(x) => x,
            Err(x) => {
                return Err(GitError::FailedToRunGit(x.to_string()));
            }
        };
        if !output.status.success() {
            log::error!(
                "git command failed. pwd={} command=`{command_str}`\nstdout=\n{}\nstderr=\n{}",
                self.path.display(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
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
        log::debug!(
            "git command succeeded. pwd={} command=`{command_str}` stdout={out}",
            self.path.display()
        );
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

    pub fn list_branch_names(&self) -> GitResult<Vec<String>> {
        self.list_branches_internal(&[])
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
            let branch = parse_git_branch_line(line, &self.path)?;
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

    /// Update `branch` to its upstream if it can be fast-forwarded
    pub fn fast_forward_branch(&self, branch: &Branch) -> GitResult<()> {
        let fail = || GitError::CannotBeFastForwarded(branch.name.clone());
        let upstream = branch.upstream.clone().ok_or(fail())?;
        let ahead_behind = upstream.ahead_behind.ok_or(fail())?;
        if ahead_behind.status() != AheadBehindStatus::Behind {
            return Err(fail());
        }
        match &branch.checkout_state {
            CheckoutState::Current => {
                self.git("merge", &["--ff-only"])?;
            }
            CheckoutState::WorkTree(path) => {
                let worktree_repo = Repository::new(path);
                worktree_repo.git("merge", &["--ff-only"])?;
            }
            CheckoutState::NotCheckedOut => {
                let full_branch_name = format!("refs/heads/{}", branch.name);
                self.git("update-ref", &[&full_branch_name, &upstream.name])?;
            }
        };
        Ok(())
    }

    pub fn get_current_sha1(&self) -> GitResult<String> {
        let out = self.git("show", &["--no-patch", "--oneline"])?;
        let sha1 = out.split(' ').next().unwrap().to_string();
        Ok(sha1)
    }

    pub fn create_branch(&self, name: &str) -> GitResult<()> {
        self.git("checkout", &["-b", name])?;
        Ok(())
    }

    pub fn push(&self) -> GitResult<()> {
        self.git("push", &[])?;
        Ok(())
    }

    pub fn add(&self, names: &[&str]) -> GitResult<()> {
        self.git("add", names)?;
        Ok(())
    }

    pub fn commit(&self, message: &str) -> GitResult<()> {
        self.git("commit", &["-m", message])?;
        Ok(())
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
            CheckoutState::WorkTree(worktree_path.into())
        }
    };

    let last_commit_date = DateTime::parse_from_rfc3339(commit_date_str)?;

    Ok(Branch {
        name: refname.into(),
        checkout_state,
        upstream,
        last_commit_date,
    })
}

// Used by test code
pub fn create_test_repository(path: &Path) -> Repository {
    let repo = Repository::new(path);

    repo.init().expect("init failed");
    repo.set_config_key("usern.name", "test")
        .expect("setting username failed");
    repo.set_config_key("user.email", "test@example.com")
        .expect("setting email failed");

    // Create a file so that we have more than the start commit
    File::create(path.join("f")).unwrap();
    repo.add(&["."]).expect("add failed");
    repo.commit("init").expect("commit failed");

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
    use super::*;

    use chrono::TimeZone;
    use yare::parameterized;

    use std::fs;

    fn create_file_and_commit(repo: &Repository, content: &str) {
        let filename = format!("{content}.txt");
        let path = repo.path.join(&filename);
        fs::write(path, content).unwrap();
        repo.add(&[&filename]).unwrap();
        repo.commit(content).unwrap();
    }

    #[test]
    fn get_current_branch() {
        let dir = assert_fs::TempDir::new().unwrap();
        let repo = create_test_repository(dir.path());
        assert_eq!(repo.get_current_branch().unwrap(), INITIAL_BRANCH);

        repo.create_branch("test").unwrap();
        assert_eq!(repo.get_current_branch().unwrap(), "test");
    }

    #[test]
    fn delete_branch() {
        // GIVEN a repository with a test branch containing unique content
        let dir = assert_fs::TempDir::new().unwrap();
        let repo = create_test_repository(dir.path());
        assert_eq!(repo.get_current_branch().unwrap(), INITIAL_BRANCH);

        repo.create_branch("test").unwrap();
        File::create(dir.path().join("test")).unwrap();
        repo.add(&["test"]).unwrap();
        repo.commit(&format!("Create file")).unwrap();

        repo.checkout(INITIAL_BRANCH).unwrap();

        // WHEN I call delete_branch
        let result = repo.delete_branch("test");

        // THEN the branch is deleted
        assert_eq!(result, Ok(()));

        // AND only the main branch remains
        assert_eq!(repo.list_branch_names().unwrap(), &[INITIAL_BRANCH]);
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
        let clone_repo =
            Repository::clone_repository(&clone_path, &source_path.to_str().unwrap()).unwrap();

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
    fn parse_simple_git_branch_line() {
        let tokens = vec![
            "master",
            "origin/master",
            "ahead 2, behind 4",
            "",
            "2025-12-30T12:34:56+01:00",
        ];
        assert_eq!(tokens.len(), GIT_BRANCH_CMD_FIELDS.len());
        let line = tokens.join("\x00");
        let branch = parse_git_branch_line(&line, Path::new(".")).unwrap();
        let expected_commit_date = FixedOffset::east_opt(3600)
            .unwrap()
            .with_ymd_and_hms(2025, 12, 30, 12, 34, 56)
            .unwrap();
        assert_eq!(
            branch,
            Branch {
                name: "master".into(),
                checkout_state: CheckoutState::NotCheckedOut,
                last_commit_date: expected_commit_date,
                upstream: Some(Upstream {
                    name: "origin/master".into(),
                    ahead_behind: Some(AheadBehind::new(2, 4)),
                }),
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

    #[test]
    fn fast_forward_current_branch() {
        let tmp_dir = assert_fs::TempDir::new().unwrap();

        let remote_path = tmp_dir.join("remote");
        let local_path = tmp_dir.join("local");

        // GIVEN a remote repository
        fs::create_dir(&remote_path).unwrap();
        let remote_repo = Repository::new(&remote_path);
        remote_repo.init_bare().unwrap();

        // AND a local clone
        fs::create_dir(&local_path).unwrap();
        let remote_url = format!("file://{}", remote_path.display());
        let local_repo = Repository::clone_repository(&local_path, &remote_url).unwrap();

        // AND the current branch can be fast-forwarded
        create_file_and_commit(&local_repo, "Hello");
        create_file_and_commit(&local_repo, "World");
        let target_sha1 = local_repo.get_current_sha1().unwrap();
        local_repo.push().unwrap();
        local_repo.git("reset", &["--hard", "HEAD^"]).unwrap();

        // WHEN fast_forward_branch() is called
        let branches = local_repo.list_branches().unwrap();
        local_repo.fast_forward_branch(&branches[0]).unwrap();

        // THEN the branch is fast-forwarded
        assert_eq!(local_repo.get_current_sha1().unwrap(), target_sha1);
    }

    #[test]
    fn fast_forward_other_branch() {
        let tmp_dir = assert_fs::TempDir::new().unwrap();

        let remote_path = tmp_dir.join("remote");
        let local_path = tmp_dir.join("local");

        // GIVEN a remote repository
        fs::create_dir(&remote_path).unwrap();
        let remote_repo = Repository::new(&remote_path);
        remote_repo.init_bare().unwrap();

        // AND a local clone
        fs::create_dir(&local_path).unwrap();
        let remote_url = format!("file://{}", remote_path.display());
        let local_repo = Repository::clone_repository(&local_path, &remote_url).unwrap();

        // AND the initial branch can be fast-forwarded
        create_file_and_commit(&local_repo, "Hello");
        create_file_and_commit(&local_repo, "World");
        let target_sha1 = local_repo.get_current_sha1().unwrap();
        local_repo.push().unwrap();
        local_repo.git("reset", &["--hard", "HEAD^"]).unwrap();

        // AND another branch is checked-out
        local_repo.create_branch("work").unwrap();

        // WHEN fast_forward_branch() is called
        let branches = local_repo.list_branches().unwrap();
        local_repo.fast_forward_branch(&branches[0]).unwrap();

        // THEN the initial branch is fast-forwarded
        local_repo.checkout(INITIAL_BRANCH).unwrap();
        assert_eq!(local_repo.get_current_sha1().unwrap(), target_sha1);
    }

    #[test]
    fn fast_forward_worktree() {
        let tmp_dir = assert_fs::TempDir::new().unwrap();

        let remote_path = tmp_dir.join("remote");
        let local_path = tmp_dir.join("local");
        let worktree_path = tmp_dir.join("worktree");

        // GIVEN a remote repository
        fs::create_dir(&remote_path).unwrap();
        let remote_repo = Repository::new(&remote_path);
        remote_repo.init_bare().unwrap();

        // AND a local clone
        fs::create_dir(&local_path).unwrap();
        let remote_url = format!("file://{}", remote_path.display());
        let local_repo = Repository::clone_repository(&local_path, &remote_url).unwrap();

        // AND the initial branch can be fast-forwarded
        create_file_and_commit(&local_repo, "Hello");
        create_file_and_commit(&local_repo, "World");
        let target_sha1 = local_repo.get_current_sha1().unwrap();
        local_repo.push().unwrap();
        local_repo.git("reset", &["--hard", "HEAD^"]).unwrap();

        // AND another branch is checked-out in a separate worktree
        local_repo
            .git(
                "worktree",
                &["add", "-b", "work", worktree_path.to_str().unwrap()],
            )
            .unwrap();
        let worktree_repo = Repository::new(&worktree_path);

        // WHEN fast_forward_branch() is called from the worktree
        // (meaning the initial branch is seen as in CheckoutState::WorkTree)
        let branches = worktree_repo.list_branches().unwrap();
        let branch = &branches[0];
        assert_eq!(branch.name, INITIAL_BRANCH);
        assert!(matches!(branch.checkout_state, CheckoutState::WorkTree(_)));
        worktree_repo.fast_forward_branch(&branch).unwrap();

        // THEN the initial branch is fast-forwarded
        assert_eq!(local_repo.get_current_sha1().unwrap(), target_sha1);
    }
}
