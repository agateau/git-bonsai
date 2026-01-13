// SPDX-FileCopyrightText: 2026 Aurélien Gâteau <mail@agateau.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use git::{Branch, CheckoutState, GitResult};

use crate::action::Action;
use crate::repositorymodel::RepositoryModel;
use crate::task::Task;

use ratatui::widgets::TableState;
use std::cmp;
use std::path::Path;

use crossterm::event::KeyCode;

#[derive(Debug)]
pub enum Command {
    Checkout,
    Delete,
    Filter,
    Quit,
    ClosePopup,
    Sync,
    CancelTask,
}

/// Global state of the application
pub enum AppState {
    /// Default state, showing branches
    Normal,
    /// Filter UI is visible
    EditFilter,
    /// Showing an error message
    Error(String),
    Exiting,
    RunningTask {
        task: Box<dyn Task>,
    },
}

/// The UI "model". Contains all the state used by the UI.
pub struct Model {
    pub actions: Vec<Action<Command>>,
    checkout_action_idx: usize,
    delete_action_idx: usize,
    filter_action_idx: usize,
    pub close_popup_action: Action<Command>,
    pub cancel_task_action: Action<Command>,
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

        let filter_action_idx = actions.len();
        actions.push(Action::new(
            "Filter".into(),
            KeyCode::Char('/'),
            Command::Filter,
        ));

        actions.push(Action::new(
            "Sync".into(),
            KeyCode::Char('S'),
            Command::Sync,
        ));

        actions.push(Action::new(
            "Quit".into(),
            KeyCode::Char('q'),
            Command::Quit,
        ));
        let cancel_task_action = Action::new("Cancel".into(), KeyCode::Esc, Command::CancelTask);
        let close_popup_action = Action::new("Close".into(), KeyCode::Esc, Command::ClosePopup);
        Self {
            actions,
            checkout_action_idx,
            delete_action_idx,
            filter_action_idx,
            cancel_task_action,
            close_popup_action,
            repo_model: RepositoryModel::new(path),
            table_state: TableState::default(),
            app_state: AppState::Normal,
            page_size: 10,
        }
    }

    pub fn filter(&self) -> &str {
        self.repo_model.filter()
    }

    pub fn set_filter(&mut self, value: &str) {
        self.repo_model.set_filter(value);
    }

    pub fn branches(&self) -> &Vec<Branch> {
        self.repo_model.branches()
    }

    pub fn branches_contained_in(&self, branch: &str) -> Option<&Vec<String>> {
        self.repo_model.branches_contained_in(branch)
    }

    pub fn update(&mut self) {
        self.repo_model.update();
        if let AppState::RunningTask { task } = &mut self.app_state {
            task.update();
        }

        let branch = self.current_branch();
        let is_not_checked_out =
            branch.is_some_and(|x| x.checkout_state == CheckoutState::NotCheckedOut);
        self.actions[self.checkout_action_idx].enabled = is_not_checked_out;
        self.actions[self.delete_action_idx].enabled = is_not_checked_out;

        let filter_suffix = if self.filter().is_empty() { "" } else { "*" };
        self.actions[self.filter_action_idx].name = format!("Filter{filter_suffix}");
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
        // Select the previous branch if we were on the last one
        let nb_branches = self.branches().len();
        if self.table_state.selected() == Some(nb_branches - 1) {
            self.table_state.select(Some(nb_branches - 2));
        }
        self.update_branches()
            .expect("update_branches() should not fail after a successful delete");
    }

    pub fn sync(&mut self) {
        let mut task: Box<dyn Task> = Box::new(self.repo_model.start_syncing());
        task.start();
        self.app_state = AppState::RunningTask { task };
    }

    pub fn stop_task(&mut self) {
        self.app_state = AppState::Normal;
        if let Err(error) = self.repo_model.update_branches() {
            self.app_state = AppState::Error(format!("{}", error));
        }
    }
}

#[cfg(test)]
mod test {
    use git::{Repository, INITIAL_BRANCH};

    use crate::model::Model;

    #[test]
    fn delete_last_branch() {
        // GIVEN a source repository with two branches
        let tmp_dir = assert_fs::TempDir::new().unwrap();

        let repo = Repository::new(&tmp_dir);
        repo.init().unwrap();
        repo.git("commit", &["-m", "empty", "--allow-empty"])
            .unwrap();
        repo.create_branch("z").unwrap();
        repo.checkout(INITIAL_BRANCH).unwrap();

        // AND a model on this repo
        let mut model = Model::new(&tmp_dir);
        model.update_branches().unwrap();
        assert_eq!(model.branches().len(), 2);

        // AND the second branch is selected
        model.table_state.select(Some(1));
        let branch = model.current_branch().unwrap();
        assert_eq!(branch.name, "z");

        // WHEN I delete the branch
        model.delete();

        // THEN the branch is deleted
        assert_eq!(repo.list_branch_names().unwrap(), &[INITIAL_BRANCH]);

        // AND the first branch is selected
        assert_eq!(model.table_state.selected(), Some(0));
    }
}
