// SPDX-FileCopyrightText: 2026 Aurélien Gâteau <mail@agateau.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc;
use std::thread;

use git::{AheadBehindStatus, Branch, GitResult, Repository};

use crate::gitsynctask::GitSyncTask;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Request {
    Stop,
    BranchesContaining(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Response {
    BranchesContaining {
        name: String,
        contained_in: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Name,
    LastCommit,
    Status,
}

const COLUMNS: &[Column; 3] = &[Column::Name, Column::LastCommit, Column::Status];

impl Column {
    pub fn next(&self) -> Self {
        for (mut idx, value) in COLUMNS.iter().enumerate() {
            if value == self {
                idx = (idx + 1) % COLUMNS.len();
                return COLUMNS[idx];
            }
        }
        panic!("This should not happen");
    }
    pub fn prev(&self) -> Self {
        for (mut idx, value) in COLUMNS.iter().enumerate() {
            if value == self {
                if idx == 0 {
                    idx = COLUMNS.len() - 1;
                } else {
                    idx -= 1;
                }
                return COLUMNS[idx];
            }
        }
        panic!("This should not happen");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortBy {
    pub column: Column,
    pub ascending: bool,
}

fn get_status_key(branch: &Branch) -> usize {
    let Some(ref upstream) = branch.upstream else {
        return 0;
    };
    let Some(ref ahead_behind) = upstream.ahead_behind else {
        return 1;
    };
    match ahead_behind.status() {
        AheadBehindStatus::Diverged => 2,
        AheadBehindStatus::Ahead => 3,
        AheadBehindStatus::UpToDate => 4,
        AheadBehindStatus::Behind => 5,
    }
}

/// Knows the branch of a git repository, and can fetch info about them
pub struct RepositoryModel {
    repo: Repository,
    filter: String,
    /// The branches, before applying the filter
    all_branches: Vec<Branch>,
    /// The filtered branches
    branches: Vec<Branch>,
    branches_containing: HashMap<String, Vec<String>>,
    request_tx: mpsc::Sender<Request>,
    response_rx: mpsc::Receiver<Response>,
    sort_by: SortBy,
}

impl RepositoryModel {
    pub fn new(path: &Path) -> Self {
        let (request_tx, request_rx) = mpsc::channel();
        let (response_tx, response_rx) = mpsc::channel();

        let repo = Repository::new(path);
        let worker_repo = repo.clone();
        thread::spawn(move || {
            while let Ok(request) = request_rx.recv() {
                match request {
                    Request::Stop => {
                        return;
                    }
                    Request::BranchesContaining(name) => {
                        let Ok(contained_in) = worker_repo.list_branches_containing(&name) else {
                            // Failure can happen if the branch has just been removed. Ignore it.
                            continue;
                        };
                        // Remove ourselves
                        let contained_in =
                            contained_in.into_iter().filter(|x| *x != name).collect();
                        response_tx
                            .send(Response::BranchesContaining { name, contained_in })
                            .unwrap();
                    }
                }
            }
        });

        Self {
            repo,
            filter: "".into(),
            all_branches: vec![],
            branches: vec![],
            branches_containing: HashMap::new(),
            request_tx,
            response_rx,
            sort_by: SortBy {
                column: Column::Name,
                ascending: true,
            },
        }
    }

    pub fn filter(&self) -> &str {
        &self.filter
    }

    pub fn set_filter(&mut self, filter: &str) {
        if self.filter == filter {
            return;
        }
        self.filter = filter.into();
        self.apply_filter();
    }

    pub fn sort_by(&self) -> SortBy {
        self.sort_by
    }

    pub fn set_sort_by(&mut self, sort_by: SortBy) {
        if self.sort_by == sort_by {
            return;
        }
        self.sort_by = sort_by;
        self.apply_filter();
    }

    /// Update self.branches from self.all_branches
    fn apply_filter(&mut self) {
        self.branches = self
            .all_branches
            .iter()
            .filter(|x| x.name.contains(&self.filter))
            .cloned()
            .collect();
        match self.sort_by.column {
            Column::Name => self.branches.sort_by_key(|x| x.name.clone()),
            Column::LastCommit => self.branches.sort_by_key(|x| x.last_commit_date),
            Column::Status => self.branches.sort_by_key(get_status_key),
        }
        if !self.sort_by.ascending {
            self.branches.reverse();
        }
    }

    pub fn update_branches(&mut self) -> GitResult<()> {
        self.all_branches = self.repo.list_branches()?;
        self.apply_filter();

        self.branches_containing.clear();
        for branch in &self.branches {
            let msg = Request::BranchesContaining(branch.name.clone());
            self.request_tx.send(msg).unwrap();
        }
        Ok(())
    }

    pub fn update(&mut self) {
        while let Ok(response) = self.response_rx.try_recv() {
            match response {
                Response::BranchesContaining { name, contained_in } => {
                    self.branches_containing.insert(name, contained_in);
                }
            }
        }
    }

    pub fn branches(&self) -> &Vec<Branch> {
        &self.branches
    }

    /// Returns the list of branches that contain `branch`. Can be None if we don't have the list yet.
    pub fn branches_containing(&self, branch: &str) -> Option<&Vec<String>> {
        self.branches_containing.get(branch)
    }

    pub fn checkout(&self, branch: &str) -> GitResult<()> {
        self.repo.checkout(branch)
    }

    pub fn delete_branch(&mut self, branch_name: &str) -> GitResult<()> {
        self.repo.delete_branch(branch_name)?;

        // Remove branch_name from self.all_branches
        // A given branch can only appear once so we can stop as soon as we find it
        let idx = self.all_branches.iter().position(|x| x.name == branch_name);
        if let Some(idx) = idx {
            self.all_branches.remove(idx);
        }

        // Update self.branches
        self.apply_filter();

        // Remove branch_name from self.branches_containing, as a contained
        // and as a container
        // Similarly, a given branch can only appear once in the container list
        // so we can stop as soon as we find it
        self.branches_containing.remove(branch_name);
        for (_, containers) in self.branches_containing.iter_mut() {
            let idx = containers.iter().position(|x| x == branch_name);
            if let Some(idx) = idx {
                containers.remove(idx);
            }
        }
        Ok(())
    }

    pub fn start_syncing(&self) -> GitSyncTask {
        GitSyncTask::new(self.repo.clone())
    }
}

impl Drop for RepositoryModel {
    fn drop(&mut self) {
        self.request_tx.send(Request::Stop).unwrap();
    }
}

#[cfg(test)]
mod test {
    use std::{
        thread,
        time::{Duration, Instant},
    };

    use git::{Repository, INITIAL_BRANCH};

    use crate::repositorymodel::RepositoryModel;

    fn create_empty_commit(repo: &Repository) {
        repo.git("commit", &["-m", "empty", "--allow-empty"])
            .unwrap();
    }

    fn ensure_model_ready(model: &mut RepositoryModel) {
        model.update_branches().unwrap();
        let start = Instant::now();
        while model.branches_containing.is_empty() {
            model.update();
            thread::sleep(Duration::from_millis(100));
            assert!(start.elapsed().as_secs() < 2);
        }
    }

    #[test]
    fn delete_branch() {
        // GIVEN a source repository
        let tmp_dir = assert_fs::TempDir::new().unwrap();

        let repo = Repository::new(&tmp_dir);
        repo.init().unwrap();
        create_empty_commit(&repo);

        // AND a 2nd branch that contains the initial branch
        repo.create_branch("x").unwrap();
        create_empty_commit(&repo);

        // AND a 3rd branch that contains the two others
        repo.create_branch("y").unwrap();
        create_empty_commit(&repo);

        // AND a model on this repo
        let mut model = RepositoryModel::new(&tmp_dir);
        ensure_model_ready(&mut model);
        assert_eq!(model.branches().len(), 3);

        assert_eq!(
            model.branches_containing(INITIAL_BRANCH),
            Some(&vec!["x".to_string(), "y".to_string()])
        );

        // WHEN I delete x
        model.delete_branch("x").unwrap();

        // THEN the branch is deleted
        assert_eq!(repo.list_branch_names().unwrap(), &[INITIAL_BRANCH, "y"]);

        // AND x is not in the list of branches that contain INITIAL_BRANCH
        assert_eq!(
            model.branches_containing(INITIAL_BRANCH),
            Some(&vec!["y".to_string()])
        );
    }
}
