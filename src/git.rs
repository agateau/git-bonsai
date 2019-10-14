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

pub fn list_branches() -> Vec<String> {
    list_branches_internal([].to_vec())
}

fn list_branches_internal(args: Vec<String>) -> Vec<String> {
    let mut branches : Vec<String> = Vec::new();

    let stdout = git("branch", args);
    for line in stdout.lines() {
        let branch = line.get(2..).expect("Invalid branch name");
        branches.push(branch.to_string());
    }

    branches
}

pub fn list_merged_branches(branch: &str) -> Vec<String> {
    return list_branches_internal(vec!["--merged".to_string(), branch.to_string()]);
}

pub fn checkout(branch: &str) {
    git("checkout", vec![branch.to_string()]);
}

pub fn delete_branch(branch: &str) {
    git("branch", vec!["-d".to_string(), branch.to_string()]);
}
