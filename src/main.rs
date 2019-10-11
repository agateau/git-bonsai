use std::collections::{HashMap,HashSet};
use std::io::{stdin,stdout,Write};
use std::process::Command;

fn git(subcommand: &str, args: Vec<String>) -> String {
    let mut cmd = Command::new("git");
    cmd.arg(subcommand);
    for arg in args {
        cmd.arg(arg);
    }
    let out = cmd.output().expect("Failed to run command");
    String::from_utf8(out.stdout).expect("Failed to decode command output")
}

fn list_branches(args: Vec<String>) -> Vec<String> {
    let mut branches : Vec<String> = Vec::new();

    let stdout = git("branch", args);
    for line in stdout.lines() {
        let branch = line.get(2..).expect("Invalid branch name");
        branches.push(branch.to_string());
    }

    branches
}

fn list_merged_branches(branch: &str) -> Vec<String> {
    return list_branches(vec!["--merged".to_string(), branch.to_string()]);
}

fn read_line() -> String {
    let mut input = String::new();
    stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn confirm(msg: &str) -> bool {
    print!("{} ", msg);
    stdout().flush();

    let input = read_line();

    input == "y" || input == "Y"
}

fn main() {
    let mut protected_branches : HashSet<String> = HashSet::new();
    protected_branches.insert("master".to_string());

    let branches = list_branches([].to_vec());
    let mut to_delete : HashMap<String, HashSet<String>> = HashMap::new();
    for branch in branches {
        let merged_branches = list_merged_branches(&branch);
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
        return;
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
        return;
    }
    for (branch, contained_in) in &to_delete {
        println!("Deleting {}", branch);
        let container = contained_in.iter().next().unwrap();
        git("checkout", vec![container.to_string()]);
        git("branch", vec!["-d".to_string(), branch.to_string()]);
    }
}
