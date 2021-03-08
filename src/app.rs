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

use crate::appui::{AppUi, BranchToDeleteInfo};
use crate::cliargs::CliArgs;
use crate::git::{BranchRestorer, Repository};
use crate::tui::{self, log_error, log_info, log_warning};

struct InteractiveAppUi;

fn format_branch_info(branch_info: &BranchToDeleteInfo) -> String {
    let container_str = branch_info
        .contained_in
        .iter()
        .map(|x| format!("      - {}", x))
        .collect::<Vec<String>>()
        .join("\n");

    format!("{}, contained in:\n{} \n", branch_info.name, container_str)
}

impl AppUi for InteractiveAppUi {
    fn log_info(&self, msg: &str) {
        tui::log_info(msg);
    }

    fn log_warning(&self, msg: &str) {
        tui::log_warning(msg);
    }

    fn log_error(&self, msg: &str) {
        tui::log_error(msg);
    }

    fn select_branches_to_delete(
        &self,
        branch_infos: &[BranchToDeleteInfo],
    ) -> Vec<BranchToDeleteInfo> {
        let select_items: Vec<String> = branch_infos
            .iter()
            .map(|x| format_branch_info(&x))
            .collect::<Vec<String>>();

        let selections = tui::select("Select branches to delete", &select_items);

        selections
            .iter()
            .map(|&x| branch_infos[x].clone())
            .collect::<Vec<BranchToDeleteInfo>>()
    }
}

pub struct App {
    repo: Repository,
    protected_branches: HashSet<String>,
    ask_confirmation: bool,
    ui: Box<dyn AppUi>,
}

impl App {
    pub fn new(args: &CliArgs, repo_dir: &str) -> App {
        let mut branches: HashSet<String> = HashSet::new();
        branches.insert("master".to_string());
        for branch in &args.excluded {
            branches.insert(branch.to_string());
        }
        App {
            repo: Repository::new(repo_dir),
            protected_branches: branches,
            ask_confirmation: !args.yes,
            ui: Box::new(InteractiveAppUi {}),
        }
    }

    pub fn is_working_tree_clean(&self) -> bool {
        if self.repo.get_current_branch() == None {
            log_error("No current branch");
            return false;
        }
        match self.repo.has_changes() {
            Ok(has_changes) => {
                if has_changes {
                    log_error("Can't work in a tree with uncommitted changes");
                    return false;
                }
                true
            }
            Err(()) => {
                log_error("Failed to get working tree status");
                false
            }
        }
    }

    pub fn fetch_changes(&self) -> Result<(), i32> {
        log_info("Fetching changes");
        self.repo.fetch()
    }

    pub fn update_tracking_branches(&self) -> Result<(), i32> {
        let branches = match self.repo.list_tracking_branches() {
            Ok(x) => x,
            Err(x) => {
                log_error("Failed to list tracking branches");
                return Err(x);
            }
        };

        let _restorer = BranchRestorer::new(&self.repo);
        for branch in branches {
            log_info(&format!("Updating {}", branch));
            if let Err(x) = self.repo.checkout(&branch) {
                log_error("Failed to checkout branch");
                return Err(x);
            }
            if let Err(_x) = self.repo.update_branch() {
                log_warning("Failed to update branch");
                // This is not wrong, it can happen if the branches have diverged
                // let's continue
            }
        }
        Ok(())
    }
    pub fn remove_merged_branches(&self) -> Result<(), i32> {
        let to_delete = match self.get_deletable_branches() {
            Ok(x) => x,
            Err(x) => {
                return Err(x);
            }
        };

        if to_delete.is_empty() {
            println!("No deletable branches");
            return Ok(());
        }

        let selected_branches = match self.ask_confirmation {
            true => self.ui.select_branches_to_delete(&to_delete),
            false => to_delete,
        };
        if selected_branches.is_empty() {
            return Ok(());
        }

        let _restorer = BranchRestorer::new(&self.repo);
        for branch_info in &selected_branches {
            log_info(&format!("Deleting {}", branch_info.name));
            let contained_in = &branch_info.contained_in;
            let container = contained_in.iter().next().unwrap();
            if self.repo.checkout(container).is_err() {
                log_warning("Failed to checkout branch");
                continue;
            }
            if self.repo.delete_branch(&branch_info.name).is_err() {
                log_warning("Failed to delete branch");
            }
        }
        Ok(())
    }

    fn get_deletable_branches(&self) -> Result<Vec<BranchToDeleteInfo>, i32> {
        let branches = match self.repo.list_branches() {
            Ok(x) => x,
            Err(x) => {
                log_error("Failed to list branches");
                return Err(x);
            }
        };

        let mut to_delete: HashMap<String, HashSet<String>> = HashMap::new();
        for branch in &branches {
            let merged_branches = match self.repo.list_merged_branches(&branch) {
                Ok(x) => x,
                Err(x) => {
                    log_error("Failed to list merged branches");
                    return Err(x);
                }
            };
            for merged_branch in &merged_branches {
                if self.protected_branches.contains(merged_branch) {
                    continue;
                }
                if branch == merged_branch {
                    continue;
                }
                let entry = to_delete
                    .entry(merged_branch.to_string())
                    .or_insert_with(HashSet::new);
                (*entry).insert(branch.clone());
            }
        }

        // Remove deletable branches from to_delete values
        let branch_names = to_delete
            .keys()
            .map(|x| x.clone())
            .collect::<HashSet<String>>();
        for (_, contained_in) in to_delete.iter_mut() {
            let diff = (*contained_in)
                .difference(&branch_names)
                .map(|x| x.clone())
                .collect::<HashSet<String>>();
            *contained_in = diff;
        }

        // Create our final list
        let deletable_branches = to_delete
            .iter()
            .map(|(name, contained_in)| BranchToDeleteInfo {
                name: name.to_string(),
                contained_in: contained_in.clone(),
            })
            .collect::<Vec<BranchToDeleteInfo>>();

        Ok(deletable_branches)
    }
}
