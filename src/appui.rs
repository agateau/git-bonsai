/*
 * Copyright 2021 Aurélien Gâteau <mail@agateau.com>
 *
 * This file is part of git-bonsai.
 *
 * Git-bonsai is free software: you can redistribute it and/or modify it under
 * the terms of the GNU General Public License as published by the Free
 * Software Foundation, either version 3 of the License, or (at your option)
 * any later version.
 *
 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
 * FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License for
 * more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * this program.  If not, see <http://www.gnu.org/licenses/>.
 */
/**
 * This module provides a "high-level" interface for the UI
 */
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct BranchToDeleteInfo {
    pub name: String,
    pub contained_in: HashSet<String>,
}

pub trait AppUi {
    fn log_info(&self, msg: &str);
    fn log_warning(&self, msg: &str);
    fn log_error(&self, msg: &str);

    fn select_branches_to_delete(
        &self,
        branch_infos: &[BranchToDeleteInfo],
    ) -> Vec<BranchToDeleteInfo>;
}
