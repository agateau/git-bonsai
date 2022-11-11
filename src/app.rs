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
use std::collections::{HashMap, HashSet};
use std::convert::From;
use std::fmt;
use std::path::PathBuf;

use crate::appui::{AppUi, BranchToDeleteInfo};
use crate::batchappui::BatchAppUi;
use crate::cliargs::CliArgs;
use crate::git::{BranchRestorer, GitError, Repository};
use crate::interactiveappui::InteractiveAppUi;

pub static DEFAULT_BRANCH_CONFIG_KEY: &str = "git-bonsai.default-branch";

#[derive(Debug, PartialEq, Eq)]
pub enum AppError {
    Git(GitError),
    UnsafeDelete,
    InterruptedByUser,
}

impl From<GitError> for AppError {
    fn from(error: GitError) -> Self {
        AppError::Git(error)
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Git(error) => error.fmt(f),
            AppError::UnsafeDelete => {
                write!(f, "This branch cannot be deleted safely")
            }
            AppError::InterruptedByUser => {
                write!(f, "Interrupted")
            }
        }
    }
}

pub struct App {
    repo: Repository,
    protected_branches: HashSet<String>,
    ui: Box<dyn AppUi>,
    fetch: bool,
}

impl App {
    pub fn new(args: &CliArgs, ui: Box<dyn AppUi>, repo_dir: &str) -> App {
        let repo = Repository::new(&PathBuf::from(repo_dir));

        let mut branches: HashSet<String> = HashSet::new();
        for branch in repo
            .get_config_keys("git-bonsai.protected-branches")
            .unwrap()
        {
            branches.insert(branch.to_string());
        }
        for branch in &args.excluded {
            branches.insert(branch.to_string());
        }
        App {
            repo,
            protected_branches: branches,
            ui,
            fetch: !args.no_fetch,
        }
    }

    // Used by test code
    #[allow(dead_code)]
    pub fn get_protected_branches(&self) -> HashSet<String> {
        self.protected_branches.clone()
    }

    pub fn is_working_tree_clean(&self) -> bool {
        if self.repo.get_current_branch() == None {
            self.ui.log_error("No current branch");
            return false;
        }
        match self.repo.has_changes() {
            Ok(has_changes) => {
                if has_changes {
                    self.ui
                        .log_error("Can't work in a tree with uncommitted changes");
                    return false;
                }
                true
            }
            Err(_) => {
                self.ui.log_error("Failed to get working tree status");
                false
            }
        }
    }

    /// Ask git the name of the default branch, and store the result in git config. If we can't
    /// find it using git, fallback to asking the user.
    pub fn find_default_branch_from_git(&self) -> Result<String, AppError> {
        self.ui.log_info("Determining repository default branch");
        let branch = match self.repo.find_default_branch() {
            Ok(x) => x,
            Err(err) => {
                self.ui.log_error(&format!(
                    "Can't determine default branch: {}",
                    &err.to_string()
                ));
                return self.find_default_branch_from_user();
            }
        };
        self.repo
            .set_config_key(DEFAULT_BRANCH_CONFIG_KEY, &branch)?;
        self.ui.log_info(&format!("Default branch is {}", branch));
        Ok(branch)
    }

    /// Ask the user the name of the default branch, and store the result in git config
    pub fn find_default_branch_from_user(&self) -> Result<String, AppError> {
        let branch = match self.ui.select_default_branch(&self.repo.list_branches()?) {
            Some(x) => x,
            None => {
                return Err(AppError::InterruptedByUser);
            }
        };
        self.repo
            .set_config_key(DEFAULT_BRANCH_CONFIG_KEY, &branch)?;
        Ok(branch)
    }

    /// Return the default branch stored in git config, if any
    pub fn get_default_branch(&self) -> Result<Option<String>, AppError> {
        match self.repo.get_config_keys(DEFAULT_BRANCH_CONFIG_KEY) {
            Ok(values) => Ok(if values.len() != 1 {
                None
            } else {
                Some(values[0].clone())
            }),
            Err(x) => Err(AppError::Git(x)),
        }
    }

    pub fn fetch_changes(&self) -> Result<(), AppError> {
        self.ui.log_info("Fetching changes");
        self.repo.fetch()?;
        Ok(())
    }

    pub fn update_tracking_branches(&self) -> Result<(), AppError> {
        let branches = match self.repo.list_tracking_branches() {
            Ok(x) => x,
            Err(x) => {
                self.ui.log_error("Failed to list tracking branches");
                return Err(AppError::Git(x));
            }
        };

        let _restorer = BranchRestorer::new(&self.repo);
        for branch in branches {
            self.ui.log_info(&format!("Updating {}", branch));
            if let Err(x) = self.repo.checkout(&branch) {
                self.ui.log_error("Failed to checkout branch");
                return Err(AppError::Git(x));
            }
            if let Err(_x) = self.repo.update_branch() {
                self.ui.log_warning("Failed to update branch");
                // This is not wrong, it can happen if the branches have diverged
                // let's continue
            }
        }
        Ok(())
    }
    pub fn remove_merged_branches(&self) -> Result<(), AppError> {
        let to_delete = self.get_deletable_branches()?;

        if to_delete.is_empty() {
            self.ui.log_info("No deletable branches");
            return Ok(());
        }

        let selected_branches = self.ui.select_branches_to_delete(&to_delete);
        if selected_branches.is_empty() {
            return Ok(());
        }

        let branch_names: Vec<String> = selected_branches
            .iter()
            .map(|x| x.name.to_string())
            .collect();
        self.delete_branches(&branch_names[..])?;
        Ok(())
    }

    /// Delete the specified branches, takes care of checking out another branch if we are deleting
    /// the current one
    fn delete_branches(&self, branches: &[String]) -> Result<(), AppError> {
        let current_branch = self.repo.get_current_branch().unwrap();

        let mut current_branch_deleted = false;
        let default_branch = self.get_default_branch().unwrap().unwrap();

        match self.repo.checkout(&default_branch) {
            Ok(()) => (),
            Err(x) => {
                let msg = format!("Failed to switch to default branch '{}'", default_branch);
                self.ui.log_error(&msg);
                return Err(AppError::Git(x));
            }
        }

        for branch in branches {
            self.ui.log_info(&format!("Deleting {}", branch));

            if self.safe_delete_branch(branch).is_err() {
                self.ui.log_warning("Failed to delete branch");
            } else if *branch == current_branch {
                current_branch_deleted = true;
            }
        }

        if !current_branch_deleted {
            self.repo.checkout(&current_branch)?;
        }
        Ok(())
    }

    fn get_deletable_branches(&self) -> Result<Vec<BranchToDeleteInfo>, AppError> {
        let deletable_branches: Vec<BranchToDeleteInfo> = match self.repo.list_branches() {
            Ok(x) => x,
            Err(x) => {
                self.ui.log_error("Failed to list branches");
                return Err(AppError::Git(x));
            }
        }
        .iter()
        .filter(|&x| !self.protected_branches.contains(x))
        .map(|branch| {
            let contained_in: HashSet<String> = match self.repo.list_branches_containing(branch) {
                Ok(x) => x,
                Err(_x) => {
                    self.ui
                        .log_error(&format!("Failed to list branches containing {}", branch));
                    [].to_vec()
                }
            }
            .iter()
            .filter(|&x| x != branch)
            .cloned()
            .collect();

            BranchToDeleteInfo {
                name: branch.to_string(),
                contained_in,
            }
        })
        .filter(|x| !x.contained_in.is_empty())
        .collect();

        Ok(deletable_branches)
    }

    fn is_sha1_contained_in_another_branch(
        &self,
        sha1: &str,
        branches: &HashSet<String>,
    ) -> Result<bool, GitError> {
        for branch in self.repo.list_branches_containing(sha1).unwrap() {
            if !branches.contains(&branch) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn do_delete_identical_branches(
        &self,
        sha1: &str,
        branch_set: &HashSet<String>,
    ) -> Result<(), AppError> {
        let unprotected_branch_set: HashSet<_> =
            branch_set.difference(&self.protected_branches).collect();
        if !self
            .is_sha1_contained_in_another_branch(sha1, branch_set)
            .unwrap()
        {
            let contains_protected_branches = unprotected_branch_set.len() < branch_set.len();
            let branches: Vec<String> = unprotected_branch_set
                .iter()
                .map(|x| x.to_string())
                .collect();
            if !contains_protected_branches {
                let selected_branches: Vec<String> = self
                    .ui
                    .select_identical_branches_to_delete_keep_one(&branches)
                    .iter()
                    .map(|x| x.to_string())
                    .collect();
                self.delete_branches(&selected_branches)?;
                return Ok(());
            }
        }
        if unprotected_branch_set.is_empty() {
            // Aliases are only protected branches, do nothing
            return Ok(());
        }
        let branches: Vec<String> = unprotected_branch_set
            .iter()
            .map(|x| x.to_string())
            .collect();
        let selected_branches: Vec<_> = self
            .ui
            .select_identical_branches_to_delete(&branches)
            .iter()
            .map(|x| x.to_string())
            .collect();
        self.delete_branches(&selected_branches)?;
        Ok(())
    }

    pub fn delete_identical_branches(&self) -> Result<(), AppError> {
        // Create a hashmap sha1 => set(branches)
        let mut branches_for_sha1: HashMap<String, HashSet<String>> = HashMap::new();

        match self.repo.list_branches_with_sha1s() {
            Ok(x) => x,
            Err(x) => {
                self.ui.log_error("Failed to list branches");
                return Err(AppError::Git(x));
            }
        }
        .iter()
        .for_each(|(branch, sha1)| {
            let branch_set = branches_for_sha1
                .entry(sha1.to_string())
                .or_insert_with(HashSet::<String>::new);
            branch_set.insert(branch.to_string());
        });

        // Delete identical branches if there are more than one for the same sha1
        for (sha1, branch_set) in branches_for_sha1 {
            if branch_set.len() == 1 {
                continue;
            }
            if let Err(x) = self.do_delete_identical_branches(&sha1, &branch_set) {
                self.ui.log_error("Failed to list branches");
                return Err(x);
            }
        }

        Ok(())
    }

    pub fn safe_delete_branch(&self, branch: &str) -> Result<(), AppError> {
        // A branch is only safe to delete if at least another branch contains it
        let contained_in = self.repo.list_branches_containing(branch).unwrap();
        if contained_in.len() < 2 {
            self.ui.log_error(&format!(
                "Not deleting {}, no other branches contain it",
                branch
            ));
            return Err(AppError::UnsafeDelete);
        }
        self.repo.delete_branch(branch)?;
        Ok(())
    }

    pub fn add_default_branch_to_protected_branches(&mut self) -> Result<(), AppError> {
        let default_branch = match self.get_default_branch()? {
            Some(x) => x,
            None => {
                if self.fetch {
                    self.find_default_branch_from_git()?
                } else {
                    self.find_default_branch_from_user()?
                }
            }
        };
        self.protected_branches.insert(default_branch);
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), AppError> {
        self.add_default_branch_to_protected_branches()?;
        if self.fetch {
            self.fetch_changes()?;
        }

        self.update_tracking_branches()?;
        self.delete_identical_branches()?;
        self.remove_merged_branches()?;
        Ok(())
    }
}

pub fn run(args: CliArgs, dir: &str) -> i32 {
    let ui: Box<dyn AppUi> = match args.yes {
        false => Box::new(InteractiveAppUi {}),
        true => Box::new(BatchAppUi {}),
    };
    let mut app = App::new(&args, ui, dir);

    if !app.is_working_tree_clean() {
        return 1;
    }

    match app.run() {
        Ok(()) => 0,
        Err(_) => 1,
    }
}
