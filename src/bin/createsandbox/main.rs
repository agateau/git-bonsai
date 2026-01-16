// SPDX-FileCopyrightText: 2026 Aurélien Gâteau <mail@agateau.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use git_bonsai::logger::setup_stderr_logger;
use regex::Regex;
use structopt::StructOpt;

use git::Repository;

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

/// Load a list of words from a file
fn read_word_list() -> Vec<String> {
    let word_regex = Regex::new("[a-zA-Z0-9]+").unwrap();
    let file_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("LICENSE");
    let text = fs::read_to_string(&file_path)
        .unwrap_or_else(|e| panic!("Failed to read {:?}: {}", file_path, e));
    let word_set: HashSet<_> = word_regex
        .find_iter(&text)
        .map(|m| m.as_str().into())
        .collect();
    word_set.into_iter().collect()
}

struct RandomWordProvider {
    words: Vec<String>,
    idx: usize,
}

impl RandomWordProvider {
    fn new() -> Self {
        Self {
            words: read_word_list(),
            idx: 0,
        }
    }

    fn next(&mut self) -> String {
        let word = self.words[self.idx].clone();
        self.idx = (self.idx + 1) % self.words.len();
        word
    }
}

struct RandomCommitCreator {
    repo: Repository,
    word_provider: RandomWordProvider,
}

impl RandomCommitCreator {
    fn new(repo: &Repository) -> Self {
        Self {
            repo: repo.clone(),
            word_provider: RandomWordProvider::new(),
        }
    }

    fn create(&mut self) {
        let word = self.word_provider.next();
        let filename = format!("{word}.txt");
        let path = self.repo.path().join(&filename);
        fs::write(path, &word).unwrap();
        self.repo.git("add", &[&filename]).unwrap();
        self.repo
            .git("commit", &["-m", &format!("Add {word}")])
            .unwrap();
    }
}

fn many_branches_cmd(repo_path: PathBuf) {
    create_sandbox_dir(&repo_path);
    let mut word_provider = RandomWordProvider::new();
    let repo = Repository::new(&repo_path);
    repo.init().expect("Failed to init repository");
    RandomCommitCreator::new(&repo).create();

    eprintln!("Creating branches");
    for _ in 0..200 {
        let name = word_provider.next();
        repo.create_branch(&name).unwrap_or_else(|err| {
            panic!("Failed to create branch {}: {}", name, err);
        });
    }
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
    let mut commit_creator = RandomCommitCreator::new(&local_repo);
    commit_creator.create();
    commit_creator.create();
    local_repo.push().unwrap();

    eprintln!("Creating a branch in advance");
    local_repo.create_branch("in-advance").unwrap();
    commit_creator.create();
    local_repo.push().unwrap();
    commit_creator.create();

    eprintln!("Creating a branch that can be fast-forwarded");
    local_repo.checkout("main").unwrap();
    local_repo.create_branch("can-be-ff").unwrap();
    commit_creator.create();
    commit_creator.create();
    commit_creator.create();
    local_repo.push().unwrap();
    local_repo.git("reset", &["--hard", "HEAD~2"]).unwrap();

    eprintln!("Creating a branch that has diverged");
    local_repo.checkout("main").unwrap();
    local_repo.create_branch("diverged").unwrap();
    commit_creator.create();
    commit_creator.create();
    local_repo.push().unwrap();
    local_repo.git("reset", &["--hard", "HEAD~1"]).unwrap();
    commit_creator.create();
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
