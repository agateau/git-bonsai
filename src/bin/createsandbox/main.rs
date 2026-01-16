// SPDX-FileCopyrightText: 2026 Aurélien Gâteau <mail@agateau.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use git_bonsai::logger::setup_stderr_logger;
use regex::Regex;
use structopt::StructOpt;

use git::Repository;

static WORD_LIST: LazyLock<HashSet<String>> = LazyLock::new(read_word_list);

#[derive(StructOpt)]
struct CliArgs {
    #[structopt(short = "d", long = "debug")]
    pub debug: bool,
    #[structopt(subcommand)]
    pub cmd: Command,
}

#[derive(StructOpt)]
enum Command {
    /// Create a test repository with lots of branches
    ManyBranches {
        /// Directory that is going the test repository
        repository_dir: PathBuf,
    },
    /// Create a local repository and a remote one, with branches in different states
    BranchStates {
        /// Directory that is going to contain the local and the remote repositories
        sandbox_dir: PathBuf,
    },
}

fn create_sandbox_dir(sandbox_dir: &Path) {
    if sandbox_dir.exists() {
        fs::remove_dir_all(sandbox_dir).expect("Removing sandbox dir failed");
    }
    eprintln!("Creating {}", sandbox_dir.display());
    fs::create_dir(sandbox_dir).expect("Creating testrepo dir failed");
}

/// Create a list of words to use as branch names
fn read_word_list() -> HashSet<String> {
    let word_regex = Regex::new("[a-zA-Z0-9]+").unwrap();
    let file_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("LICENSE");
    let text = fs::read_to_string(&file_path)
        .unwrap_or_else(|e| panic!("Failed to read {:?}: {}", file_path, e));
    word_regex
        .find_iter(&text)
        .map(|m| m.as_str().into())
        .collect()
}

fn many_branches_cmd(repo_path: PathBuf) {
    create_sandbox_dir(&repo_path);
    let repo = Repository::new(&repo_path);
    repo.init().expect("Failed to init repository");
    create_empty_commit(&repo);

    eprintln!("Creating branches");
    for name in WORD_LIST.iter().take(200) {
        repo.create_branch(name).unwrap_or_else(|err| {
            panic!("Failed to create branch {}: {}", name, err);
        });
    }
}

fn create_empty_commit(repo: &Repository) {
    repo.git("commit", &["--allow-empty", "-m", "Empty"])
        .unwrap();
}

fn commit_all(repo: &Repository) {
    repo.git("add", &["."]).unwrap();
    repo.git("commit", &["-m", "All changes"]).unwrap();
}

fn branch_states_cmd(sandbox_dir: PathBuf) {
    create_sandbox_dir(&sandbox_dir);

    let remote_path = sandbox_dir.join("remote");
    let local_path = sandbox_dir.join("local");

    eprintln!("Creating remote repo");
    fs::create_dir(&remote_path).unwrap();
    let remote_repo = Repository::new(&remote_path);
    remote_repo.init_bare().unwrap();

    eprintln!("Creating local repo");
    fs::create_dir(&local_path).unwrap();
    let remote_url = format!("file://{}", remote_path.display());
    let local_repo = Repository::clone_repository(&local_path, &remote_url).unwrap();

    eprintln!("Creating commits in main branch");
    create_empty_commit(&local_repo);
    create_empty_commit(&local_repo);
    local_repo.push().unwrap();

    eprintln!("Creating a branch in advance");
    local_repo.create_branch("in-advance").unwrap();
    create_empty_commit(&local_repo);
    local_repo.push().unwrap();
    create_empty_commit(&local_repo);

    eprintln!("Creating a branch that can be fast-forwarded");
    local_repo.checkout("main").unwrap();
    local_repo.create_branch("can-be-ff").unwrap();
    create_empty_commit(&local_repo);
    create_empty_commit(&local_repo);
    create_empty_commit(&local_repo);
    local_repo.push().unwrap();
    local_repo.git("reset", &["--hard", "HEAD~2"]).unwrap();

    eprintln!("Creating a branch that has diverged");
    local_repo.checkout("main").unwrap();
    local_repo.create_branch("diverged").unwrap();
    fs::write(local_path.join("x"), "x").unwrap();
    fs::write(local_path.join("y"), "y").unwrap();
    commit_all(&local_repo);
    fs::write(local_path.join("z"), "z").unwrap();
    commit_all(&local_repo);
    local_repo.push().unwrap();
    local_repo.git("reset", &["--hard", "HEAD~1"]).unwrap();
    fs::write(local_path.join("a"), "a").unwrap();
    commit_all(&local_repo);
}

fn main() {
    let args = CliArgs::from_args();
    if args.debug {
        setup_stderr_logger();
    }
    match args.cmd {
        Command::ManyBranches { repository_dir } => many_branches_cmd(repository_dir),
        Command::BranchStates { sandbox_dir } => branch_states_cmd(sandbox_dir),
    };
}
