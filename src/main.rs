use std::collections::{HashMap,HashSet};
use structopt::StructOpt;

mod git;
mod tui;

use tui::{log_warning,log_error,log_info};

/// Keep a git repository clean and tidy.
#[derive(StructOpt)]
struct Config {
    /// Branches to protect from suppression (in addition to master)
    #[structopt(short="x", long)]
    excluded: Vec<String>,
}

fn get_protected_branches(config: &Config) -> HashSet<String> {
    let mut protected_branches : HashSet<String> = HashSet::new();
    protected_branches.insert("master".to_string());
    for branch in &config.excluded {
        protected_branches.insert(branch.to_string());
    }
    protected_branches
}

/// Returns a map of branch_to_delete => (branches containing it)
fn get_deletable_branches(config: &Config, branches: &Vec<String>) -> Result<HashMap<String, HashSet<String>>, i32> {
    let protected_branches = get_protected_branches(&config);

    let mut to_delete : HashMap<String, HashSet<String>> = HashMap::new();
    for branch in branches {
        let merged_branches = match git::list_merged_branches(&branch) {
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

fn vec_for_map_keys<K: Clone, V>(map: &HashMap<K, V>) -> Vec<K> {
    map.keys().map(|x| x.clone()).collect()
}

fn select_branches_to_delete(branches: &Vec<String>) -> Vec<String> {
    tui::select("Select branches to delete", &branches)
}

fn remove_merged_branches(config: &Config) -> Result<(), i32> {
    let branches = match git::list_branches() {
        Ok(x) => x,
        Err(x) => {
            log_error("Failed to list branches");
            return Err(x);
        }
    };
    let mut to_delete = match get_deletable_branches(&config, &branches) {
        Ok(x) => x,
        Err(x) => {
            return Err(x);
        }
    };

    // Remove deletable branches from to_delete values
    let mut deletable_branches : HashSet<String> = HashSet::new();
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

    println!("Deletable branches:\n");
    for (branch, contained_in) in &to_delete {
        println!("- {}, contained in:", branch);
        for br in contained_in {
            println!("    {}", br);
        }
        println!();
    }

    let selected_branches = select_branches_to_delete(&vec_for_map_keys(&to_delete));
    if selected_branches.is_empty() {
        return Ok(());
    }

    let _restorer = git::BranchRestorer::new();
    for branch in &selected_branches {
        log_info(&format!("Deleting {}", branch));
        let contained_in = &to_delete[branch];
        let container = contained_in.iter().next().unwrap();
        if git::checkout(container).is_err() {
            log_warning("Failed to checkout branch");
            continue;
        }
        if git::delete_branch(branch).is_err() {
            log_warning("Failed to delete branch");
        }
    }
    Ok(())
}

fn fetch_changes() -> Result<(), i32> {
    log_info("Fetching changes");
    git::fetch()
}

fn update_tracking_branches() -> Result<(), i32> {
    let branches = match git::list_tracking_branches() {
        Ok(x) => x,
        Err(x) => {
            log_error("Failed to list tracking branches");
            return Err(x);
        }
    };

    let _restorer = git::BranchRestorer::new();
    for branch in branches {
        log_info(&format!("Updating {}", branch));
        if let Err(x) = git::checkout(&branch) {
            log_error("Failed to checkout branch");
            return Err(x);
        }
        if let Err(_x) = git::update_branch() {
            log_warning("Failed to update branch");
            // This is not wrong, it can happen if the branches have diverged
            // let's continue
        }
    }
    Ok(())
}

fn is_working_tree_clean() -> bool {
    if git::get_current_branch() == None {
        log_error("No current branch");
        return false;
    }
    match git::has_changes() {
        Ok(has_changes) => {
            if has_changes {
                log_error("Can't work in a tree with uncommitted changes");
                return false;
            }
            true
        },
        Err(()) => {
            log_error("Failed to get working tree status");
            false
        }
    }
}

fn runapp() -> i32 {
    let config = Config::from_args();

    if !is_working_tree_clean() {
        return 1;
    }

    if let Err(x) = fetch_changes() {
        return x;
    }

    if let Err(x) = update_tracking_branches() {
        return x;
    }

    if let Err(x) = remove_merged_branches(&config) {
        return x;
    }
    0
}

fn main() {
    ::std::process::exit(runapp());
}
