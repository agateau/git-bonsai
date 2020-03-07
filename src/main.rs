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
use std::collections::{HashMap, HashSet};
use structopt::StructOpt;

mod git;
mod tui;

use git::Repository;
use tui::{log_error, log_info, log_warning};

/// Keep a git repository clean and tidy.
#[derive(StructOpt)]
struct Config {
    /// Branches to protect from suppression (in addition to master)
    #[structopt(short = "x", long)]
    excluded: Vec<String>,
}

fn get_protected_branches(config: &Config) -> HashSet<String> {
    let mut protected_branches: HashSet<String> = HashSet::new();
    protected_branches.insert("master".to_string());
    for branch in &config.excluded {
        protected_branches.insert(branch.to_string());
    }
    protected_branches
}

/// Returns a map of branch_to_delete => (branches containing it)
fn get_deletable_branches(
    config: &Config,
    repo: &Repository,
    branches: &Vec<String>,
) -> Result<HashMap<String, HashSet<String>>, i32> {
    let protected_branches = get_protected_branches(&config);

    let mut to_delete: HashMap<String, HashSet<String>> = HashMap::new();
    for branch in branches {
        let merged_branches = match repo.list_merged_branches(&branch) {
            Ok(x) => x,
            Err(x) => {
                log_error("Failed to list merged branches");
                return Err(x);
            }
        };
        for merged_branch in merged_branches {
            if protected_branches.contains(&merged_branch) {
                continue;
            }
            if branch == &merged_branch {
                continue;
            }
            let entry = to_delete.entry(merged_branch).or_insert(HashSet::new());
            (*entry).insert(branch.clone());
        }
    }
    Ok(to_delete)
}

fn format_select_item(branch: &str, containers: &HashSet<String>) -> String {
    let container_str = containers
        .iter()
        .map(|x| format!("      - {}", x))
        .collect::<Vec<String>>()
        .join("\n");

    format!("{}, contained in:\n{} \n", branch, container_str)
}

fn select_branches_to_delete(to_delete: &HashMap<String, HashSet<String>>) -> Vec<String> {
    let (branches, select_items): (Vec<String>, Vec<String>) = to_delete
        .iter()
        .map(|(key, value)| (key.to_owned(), format_select_item(key, value)))
        .unzip();

    let selections = tui::select("Select branches to delete", &select_items);

    selections
        .iter()
        .map(|&x| branches[x].clone())
        .collect::<Vec<String>>()
}

fn remove_merged_branches(config: &Config, repo: &Repository) -> Result<(), i32> {
    let branches = match repo.list_branches() {
        Ok(x) => x,
        Err(x) => {
            log_error("Failed to list branches");
            return Err(x);
        }
    };
    let mut to_delete = match get_deletable_branches(&config, &repo, &branches) {
        Ok(x) => x,
        Err(x) => {
            return Err(x);
        }
    };

    // Remove deletable branches from to_delete values
    let mut deletable_branches: HashSet<String> = HashSet::new();
    for branch in to_delete.keys() {
        deletable_branches.insert(branch.clone());
    }
    for (_, contained_in) in to_delete.iter_mut() {
        for deletable_branch in &deletable_branches {
            contained_in.remove(deletable_branch);
        }
    }

    if to_delete.is_empty() {
        println!("No deletable branches");
        return Ok(());
    }

    let selected_branches = select_branches_to_delete(&to_delete);
    if selected_branches.is_empty() {
        return Ok(());
    }

    let _restorer = git::BranchRestorer::new(&repo);
    for branch in &selected_branches {
        log_info(&format!("Deleting {}", branch));
        let contained_in = &to_delete[branch];
        let container = contained_in.iter().next().unwrap();
        if repo.checkout(container).is_err() {
            log_warning("Failed to checkout branch");
            continue;
        }
        if repo.delete_branch(branch).is_err() {
            log_warning("Failed to delete branch");
        }
    }
    Ok(())
}

fn fetch_changes(repo: &Repository) -> Result<(), i32> {
    log_info("Fetching changes");
    repo.fetch()
}

fn update_tracking_branches(repo: &Repository) -> Result<(), i32> {
    let branches = match repo.list_tracking_branches() {
        Ok(x) => x,
        Err(x) => {
            log_error("Failed to list tracking branches");
            return Err(x);
        }
    };

    let _restorer = git::BranchRestorer::new(&repo);
    for branch in branches {
        log_info(&format!("Updating {}", branch));
        if let Err(x) = repo.checkout(&branch) {
            log_error("Failed to checkout branch");
            return Err(x);
        }
        if let Err(_x) = repo.update_branch() {
            log_warning("Failed to update branch");
            // This is not wrong, it can happen if the branches have diverged
            // let's continue
        }
    }
    Ok(())
}

fn is_working_tree_clean(repo: &Repository) -> bool {
    if repo.get_current_branch() == None {
        log_error("No current branch");
        return false;
    }
    match repo.has_changes() {
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

fn runapp() -> i32 {
    let config = Config::from_args();

    let repo = Repository::new(".");

    if !is_working_tree_clean(&repo) {
        return 1;
    }

    if let Err(x) = fetch_changes(&repo) {
        return x;
    }

    if let Err(x) = update_tracking_branches(&repo) {
        return x;
    }

    if let Err(x) = remove_merged_branches(&config, &repo) {
        return x;
    }
    0
}

fn main() {
    ::std::process::exit(runapp());
}
