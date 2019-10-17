use std::collections::{HashMap,HashSet};
use std::io::{stdin,stdout,Write};
use structopt::StructOpt;

mod git;

/// Keep a git repository clean and tidy.
#[derive(StructOpt)]
struct Config {
    /// Branches to protect from suppression (in addition to master)
    #[structopt(short="x", long)]
    excluded: Vec<String>,
}

fn read_line() -> String {
    let mut input = String::new();
    stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn confirm(msg: &str) -> bool {
    print!("{} ", msg);
    stdout().flush().expect("Failed to flush");

    let input = read_line();

    input == "y" || input == "Y"
}

fn log_warning(msg: &str) {
    println!("Warning: {}", msg);
}

fn log_error(msg: &str) {
    println!("Error: {}", msg);
}

fn get_protected_branches(config: &Config) -> HashSet<String> {
    let mut protected_branches : HashSet<String> = HashSet::new();
    protected_branches.insert("master".to_string());
    for branch in &config.excluded {
        protected_branches.insert(branch.to_string());
    }
    protected_branches
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

    let protected_branches = get_protected_branches(&config);

    let branches = match git::list_branches() {
        Ok(x) => x,
        Err(x) => {
            log_error("Failed to list branches");
            return x;
        }
    };
    let mut to_delete : HashMap<String, HashSet<String>> = HashMap::new();
    for branch in branches {
        let merged_branches = match git::list_merged_branches(&branch) {
            Ok(x) => x,
            Err(x) => {
                log_error("Failed to list merged branches");
                return x;
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
        return 0;
    }

    println!("Deletable branches:\n");
    for (branch, contained_in) in &to_delete {
        println!("{}, contained in:", branch);
        for br in contained_in {
            println!("  {}", br);
        }
        println!();
    }

    if !confirm("Delete them?") {
        return 0;
    }
    for (branch, contained_in) in &to_delete {
        println!("Deleting {}", branch);
        let container = contained_in.iter().next().unwrap();
        if git::checkout(container).is_err() {
            log_warning("Failed to checkout branch");
            continue;
        }
        if git::delete_branch(branch).is_err() {
            log_warning("Failed to delete branch");
        }
    }

    match git::checkout(&current_branch) {
        Ok(()) => 0,
        Err(x) => x,
    }
}

fn main() {
    ::std::process::exit(runapp());
}
