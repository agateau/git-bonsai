// SPDX-FileCopyrightText: 2026 Aurélien Gâteau <mail@agateau.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::path::Path;

use crate::git::{Branch, GitResult, Repository};

/// Knows the branch of a git repository, and can fetch info about them
pub struct RepositoryModel {
    repo: Repository,
    branches: Vec<Branch>,
    branches_contained_in: HashMap<String, Vec<String>>,
}

impl RepositoryModel {
    pub fn new(path: &Path) -> Self {
        Self {
            repo: Repository::new(path),
            branches: vec![],
            branches_contained_in: HashMap::new(),
        }
    }

    pub fn update(&mut self) -> GitResult<()> {
        self.branches = self.repo.list_branches()?;

        self.branches_contained_in.clear();
        for branch in &self.branches {
            let contained_in = self
                .repo
                .list_branches_containing(&branch.name)?
                .into_iter()
                // Do not list ourselves
                .filter(|x| *x != branch.name)
                .collect();
            self.branches_contained_in
                .insert(branch.name.clone(), contained_in);
        }
        Ok(())
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
