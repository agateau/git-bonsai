// SPDX-FileCopyrightText: 2026 Aurélien Gâteau <mail@agateau.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::action::Action;
use crate::git::{Branch, CheckoutState, GitResult};
use crate::repositorymodel::RepositoryModel;
use ratatui::widgets::TableState;
use std::cmp;
use std::path::Path;

use crossterm::event::KeyCode;

#[derive(Debug)]
pub enum Command {
    Checkout,
    Delete,
    Quit,
    ClosePopup,
}

/// Global state of the application
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    /// Default state, showing branches
    Normal,
    /// Showing an error message
    Error(String),
    Exiting,
}

/// The UI "model". Contains all the state used by the UI.
pub struct Model {
    pub actions: Vec<Action<Command>>,
    checkout_action_idx: usize,
    delete_action_idx: usize,
    pub close_popup_action: Action<Command>,
    repo_model: RepositoryModel,
    pub table_state: TableState,
    pub app_state: AppState,
    pub page_size: usize,
}

impl Model {
    pub fn new(path: &Path) -> Self {
        let mut actions: Vec<Action<Command>> = vec![];

        let checkout_action_idx = actions.len();
        actions.push(Action::new(
            "Checkout".into(),
            KeyCode::Char('c'),
            Command::Checkout,
        ));

        let delete_action_idx = actions.len();
        actions.push(Action::new(
            "Delete".into(),
            KeyCode::Char('d'),
            Command::Delete,
        ));

        actions.push(Action::new(
            "Quit".into(),
            KeyCode::Char('q'),
            Command::Quit,
        ));
        let close_popup_action = Action::new("Close".into(), KeyCode::Esc, Command::ClosePopup);
        Self {
            actions,
            checkout_action_idx,
            delete_action_idx,
            close_popup_action,
            repo_model: RepositoryModel::new(path),
            table_state: TableState::default(),
            app_state: AppState::Normal,
            page_size: 10,
        }
    }

    pub fn branches(&self) -> &Vec<Branch> {
        self.repo_model.branches()
    }

    pub fn branches_contained_in(&self, branch: &str) -> Option<&Vec<String>> {
        self.repo_model.branches_contained_in(branch)
    }

    pub fn update(&mut self) {
        self.repo_model.update();
        let branch = self.current_branch();
        let is_not_checked_out =
            branch.is_some_and(|x| x.checkout_state == CheckoutState::NotCheckedOut);
        self.actions[self.checkout_action_idx].enabled = is_not_checked_out;
        self.actions[self.delete_action_idx].enabled = is_not_checked_out;
    }

    pub fn update_branches(&mut self) -> GitResult<()> {
        self.repo_model.update_branches()?;
        Ok(())
    }

    fn current_branch(&self) -> Option<&Branch> {
        self.table_state.selected().map(|x| &self.branches()[x])
    }

    pub fn move_up(&mut self) {
        match self.table_state.selected() {
            Some(x) => {
                let x = if x == 0 {
                    self.branches().len() - 1
                } else {
                    x - 1
                };
                self.table_state.select(Some(x));
            }
            None => self.table_state.select(Some(self.branches().len() - 1)),
        };
    }

    pub fn move_down(&mut self) {
        match self.table_state.selected() {
            Some(x) => {
                let x = if x < self.branches().len() - 1 {
                    x + 1
                } else {
                    0
                };
                self.table_state.select(Some(x));
            }
            None => self.table_state.select(Some(0)),
        };
    }

    pub fn page_up(&mut self) {
        match self.table_state.selected() {
            Some(x) => {
                self.table_state
                    .select(Some(x.saturating_sub(self.page_size)));
            }
            None => self.table_state.select(Some(0)),
        };
    }

    pub fn page_down(&mut self) {
        match self.table_state.selected() {
            Some(x) => {
                let x = cmp::min(x + self.page_size, self.branches().len() - 1);
                self.table_state.select(Some(x));
            }
            None => self.table_state.select(Some(0)),
        };
    }

    pub fn checkout(&mut self) {
        let name = &self
            .current_branch()
            .expect("checkout() should not be callable without an active branch")
            .name;
        if let Err(error) = self.repo_model.checkout(name) {
            self.app_state = AppState::Error(format!("{}", error));
            return;
        }
        self.update_branches()
            .expect("update_branches() should not fail after a successful checkout");
    }

    pub fn quit(&mut self) {
        self.app_state = AppState::Exiting;
    }

    pub fn delete(&mut self) {
        let name = &self
            .current_branch()
            .expect("delete() should not be callable without an active branch")
            .name;
        // TODO show confirmation popup if deleting the branch is not safe
        if let Err(error) = self.repo_model.delete_branch(name) {
            self.app_state = AppState::Error(format!("{}", error));
            return;
        }
        self.update_branches()
            .expect("update_branches() should not fail after a successful delete");
    }
}
