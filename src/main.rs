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

fn remove_merged_branches(config: &Config) -> Result<(), i32> {
    let protected_branches = get_protected_branches(&config);

    let branches = match git::list_branches() {
        Ok(x) => x,
        Err(x) => {
            log_error("Failed to list branches");
            return Err(x);
        }
    };
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
            if branch == merged_branch {
                continue;
            }
            let entry = to_delete.entry(merged_branch).or_insert(HashSet::new());
            (*entry).insert(branch.clone());
        }
    }

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
        println!("{}, contained in:", branch);
        for br in contained_in {
            println!("  {}", br);
        }
        println!();
    }

    if !tui::confirm("Delete them?") {
        return Ok(());
    }
    for (branch, contained_in) in &to_delete {
        log_info(&format!("Deleting {}", branch));
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

fn fetch_branches() -> Result<(), i32> {
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

    for branch in branches {
        log_info(&format!("Updating {}", branch));
        if let Err(x) = git::checkout(&branch) {
            log_error("Failed to checkout branch");
            return Err(x);
        }
        if let Err(x) = git::pull() {
            log_error("Failed to pull branch");
            return Err(x);
        }
    }
    Ok(())
}

fn runapp() -> i32 {
    let config = Config::from_args();

    let current_branch = match git::get_current_branch() {
        Some(x) => x,
        None => {
            log_error("No current branch");
            return 1;
        }
    };

    if let Err(x) = fetch_branches() {
        return x;
    }

    if let Err(x) = update_tracking_branches() {
        return x;
    }

    if let Err(x) = remove_merged_branches(&config) {
        return x;
    }

    match git::checkout(&current_branch) {
        Ok(()) => 0,
        Err(x) => x,
    }
}

fn main() {
    ::std::process::exit(runapp());
}
