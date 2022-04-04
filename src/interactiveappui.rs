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

pub struct InteractiveAppUi;

fn format_branch_info(branch_info: &BranchToDeleteInfo) -> String {
    let container_str = branch_info
        .contained_in
        .iter()
        .map(|x| format!("      - {}", x))
        .collect::<Vec<String>>()
        .join("\n");

    format!("{}, contained in:\n{} \n", branch_info.name, container_str)
}

impl AppUi for InteractiveAppUi {
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
        let select_items: Vec<String> = branch_infos
            .iter()
            .map(format_branch_info)
            .collect::<Vec<String>>();

        let selections = tui::select("Select branches to delete", &select_items);

        selections
            .iter()
            .map(|&x| branch_infos[x].clone())
            .collect::<Vec<BranchToDeleteInfo>>()
    }

    fn select_identical_branches_to_delete(&self, branches: &[String]) -> Vec<String> {
        let mut items = branches.to_vec();
        items.sort();

        let selections = tui::select(
            "These branches point to the same commit, which is contained in another branch,\
            so it is safe to delete them all.\n\
            Select branches to delete",
            &items,
        );

        selections
            .iter()
            .map(|&x| items[x].clone())
            .collect::<Vec<String>>()
    }

    fn select_identical_branches_to_delete_keep_one(&self, branches: &[String]) -> Vec<String> {
        let mut items = branches.to_vec();
        items.sort();

        let mut selections: Vec<usize>;
        println!(
            "These branches point to the same commit, but no other branch contains this commit, \
                so you can delete all of them but one.\n"
        );
        loop {
            selections = tui::select("Select branches to delete", &items);
            if selections.len() == items.len() {
                self.log_error("You must leave at least one branch unchecked.");
            } else {
                break;
            }
        }
        selections
            .iter()
            .map(|&x| items[x].clone())
            .collect::<Vec<String>>()
    }
}
