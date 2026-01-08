// SPDX-FileCopyrightText: 2026 Aurélien Gâteau <mail@agateau.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc;
use std::thread;

use crate::git::{Branch, GitResult, Repository};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Request {
    Stop,
    BranchesContainedIn(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Response {
    BranchesContainedIn {
        name: String,
        contained_in: Vec<String>,
    },
}

/// Knows the branch of a git repository, and can fetch info about them
pub struct RepositoryModel {
    repo: Repository,
    branches: Vec<Branch>,
    branches_contained_in: HashMap<String, Vec<String>>,
    request_tx: mpsc::Sender<Request>,
    response_rx: mpsc::Receiver<Response>,
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
                    Request::BranchesContainedIn(name) => {
                        let contained_in = worker_repo
                            .list_branches_containing(&name)
                            .unwrap() // FIXME
                            .into_iter()
                            // Do not list ourselves
                            .filter(|x| *x != name)
                            .collect();
                        response_tx
                            .send(Response::BranchesContainedIn { name, contained_in })
                            .unwrap();
                    }
                }
            }
        });

        Self {
            repo,
            branches: vec![],
            branches_contained_in: HashMap::new(),
            request_tx,
            response_rx,
        }
    }

    pub fn update_branches(&mut self) -> GitResult<()> {
        self.branches = self.repo.list_branches()?;

        self.branches_contained_in.clear();
        for branch in &self.branches {
            let msg = Request::BranchesContainedIn(branch.name.clone());
            self.request_tx.send(msg).unwrap();
        }
        Ok(())
    }

    pub fn update(&mut self) {
        while let Ok(response) = self.response_rx.try_recv() {
            match response {
                Response::BranchesContainedIn { name, contained_in } => {
                    self.branches_contained_in.insert(name, contained_in);
                }
            }
        }
    }

    pub fn branches(&self) -> &Vec<Branch> {
        &self.branches
    }

    pub fn branches_contained_in(&self, branch: &str) -> Option<&Vec<String>> {
        self.branches_contained_in.get(branch)
    }

    pub fn checkout(&self, branch: &str) -> GitResult<()> {
        self.repo.checkout(branch)
    }

    pub fn delete_branch(&self, branch: &str) -> GitResult<()> {
        self.repo.delete_branch(branch)
    }
}

impl Drop for RepositoryModel {
    fn drop(&mut self) {
        self.request_tx.send(Request::Stop).unwrap();
    }
}
