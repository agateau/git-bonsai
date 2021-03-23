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
use crate::appui::{AppUi, BranchToDeleteInfo};
use crate::tui;

pub struct BatchAppUi;

impl AppUi for BatchAppUi {
    fn log_info(&self, msg: &str) {
        tui::log_info(msg);
    }

    fn log_warning(&self, msg: &str) {
        tui::log_warning(msg);
    }

    fn log_error(&self, msg: &str) {
        tui::log_error(msg);
    }

    fn select_branches_to_delete(
        &self,
        branch_infos: &[BranchToDeleteInfo],
    ) -> Vec<BranchToDeleteInfo> {
        branch_infos.to_vec()
    }

    fn select_identical_branches_to_delete(&self, branches: &[String]) -> Vec<String> {
        branches.to_vec()
    }

    fn select_identical_branches_to_delete_keep_one(&self, branches: &[String]) -> Vec<String> {
        let mut to_delete = branches.to_vec();
        to_delete.sort();
        to_delete.remove(0);
        to_delete
    }
}
