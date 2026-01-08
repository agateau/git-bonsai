// SPDX-FileCopyrightText: 2026 Aurélien Gâteau <mail@agateau.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::Path;

use crate::git::{Branch, GitResult, Repository};

/// Knows the branch of a git repository, and can fetch info about them
pub struct RepositoryModel {
    repo: Repository,
    branches: Vec<Branch>,
}

impl RepositoryModel {
    pub fn new(path: &Path) -> Self {
        Self {
            repo: Repository::new(path),
            branches: vec![],
        }
    }

    pub fn update(&mut self) -> GitResult<()> {
        self.branches = self.repo.list_branches()?;
        Ok(())
    }

    pub fn branches(&self) -> &Vec<Branch> {
        &self.branches
    }

    pub fn checkout(&self, branch: &str) -> GitResult<()> {
        self.repo.checkout(branch)
    }

    pub fn delete_branch(&self, branch: &str) -> GitResult<()> {
        self.repo.delete_branch(branch)
    }
}
