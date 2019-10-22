use std::process::Command;

fn git(subcommand: &str, args: Vec<String>) -> Result<String, i32> {
    let mut cmd = Command::new("git");
    cmd.env("LANG", "C");
    cmd.arg(subcommand);
    for arg in args {
        cmd.arg(arg);
    }
    let output = match cmd.output() {
        Ok(x) => x,
        Err(_x) => {
            println!("Failed to execute process");
            return Err(-1);
        },
    };
    if !output.status.success() {
        println!("{}", String::from_utf8(output.stderr).expect("Failed to decode command stderr"));
        return match output.status.code() {
            Some(code) => Err(code),
            None => Err(-1)
        };
    }
    let out = String::from_utf8(output.stdout).expect("Failed to decode command stdout");
    Ok(out)
}

pub fn fetch() -> Result<(), i32> {
    match git("fetch", vec!["--prune".to_string()]) {
        Ok(_x) => Ok(()),
        Err(x) => Err(x),
    }
}

pub fn list_branches() -> Result<Vec<String>, i32> {
    list_branches_internal([].to_vec())
}

fn list_branches_internal(args: Vec<String>) -> Result<Vec<String>, i32> {
    let mut branches : Vec<String> = Vec::new();

    let stdout = match git("branch", args) {
        Ok(x) => x,
        Err(x) => return Err(x),
    };
    for line in stdout.lines() {
        let branch = line.get(2..).expect("Invalid branch name");
        branches.push(branch.to_string());
    }

    Ok(branches)
}

pub fn list_merged_branches(branch: &str) -> Result<Vec<String>, i32> {
    list_branches_internal(vec!["--merged".to_string(), branch.to_string()])
}

pub fn list_tracking_branches() -> Result<Vec<String>, i32> {
    let mut branches : Vec<String> = Vec::new();
    let lines = match list_branches_internal(vec!["-vv".to_string()]) {
        Ok(x) => x,
        Err(x) => return Err(x),
    };
    for line in lines {
        if line.contains("[origin/") && !line.contains(": gone]") {
            let branch = line.split(" ").next();
            branches.push(branch.unwrap().to_string());
        }
    }
    Ok(branches)
}

pub fn checkout(branch: &str) -> Result<(), i32> {
    match git("checkout", vec![branch.to_string()]) {
        Ok(_x) => Ok(()),
        Err(x) => Err(x),
    }
}

pub fn delete_branch(branch: &str) -> Result<(), i32> {
    match git("branch", vec!["-d".to_string(), branch.to_string()]) {
        Ok(_x) => Ok(()),
        Err(x) => Err(x),
    }
}

pub fn get_current_branch() -> Option<String> {
    let stdout = git("branch", [].to_vec());
    if stdout.is_err() {
        return None;
    }
    for line in stdout.unwrap().lines() {
        if line.chars().nth(0) == Some('*') {
            return Some(line[2..].to_string());
        }
    }
    None
}

pub fn pull() -> Result<(), i32> {
    match git("pull", [].to_vec()) {
        Ok(_x) => Ok(()),
        Err(x) => Err(x),
    }
}
