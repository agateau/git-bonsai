// SPDX-FileCopyrightText: 2026 Aurélien Gâteau <mail@agateau.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use git::{AheadBehindStatus, Branch, CheckoutState, GitResult};

use crate::repositorymodel::{Column, RepositoryModel, SortBy};
use crate::task::Task;
use crate::ui::action::Action;

use ratatui::widgets::TableState;
use std::path::Path;
use std::time::Duration;
use std::{cmp, thread};

use crossterm::event::KeyCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Checkout,
    AskDeleteBranch,
    Filter,
    Quit,
    Sync,
    BackToNormal,
    Sort,
    DoDeleteBranch,
}

fn create_confirm_action(name: String, command: Command) -> Action<Command> {
    Action::new(name, KeyCode::Enter, command)
}

/// Global state of the application
pub enum AppState {
    /// Default state, showing branches
    Normal,
    /// Filter UI is visible
    EditFilter,
    /// Showing an error message
    Error(String),
    /// Showing a confirmation message
    Confirm {
        message: String,
        on_cancel: Action<Command>,
        on_confirm: Action<Command>,
    },
    Exiting,
    RunningTask {
        task: Box<dyn Task>,
        on_success: Command,
    },
}

/// The UI "model". Contains all the state used by the UI.
pub struct Model {
    pub actions: Vec<Action<Command>>,
    checkout_action_idx: usize,
    delete_action_idx: usize,
    filter_action_idx: usize,
    pub close_action: Action<Command>,
    pub cancel_action: Action<Command>,
    repo_model: RepositoryModel,
    pub table_state: TableState,
    pub focused_column: Column,
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
            Command::AskDeleteBranch,
        ));

        let filter_action_idx = actions.len();
        actions.push(Action::new(
            "Filter".into(),
            KeyCode::Char('/'),
            Command::Filter,
        ));

        actions.push(Action::new(
            "Sort".into(),
            KeyCode::Char('o'),
            Command::Sort,
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
        let cancel_action = Action::new("Cancel".into(), KeyCode::Esc, Command::BackToNormal);
        let close_action = Action::new("Close".into(), KeyCode::Esc, Command::BackToNormal);
        Self {
            actions,
            checkout_action_idx,
            delete_action_idx,
            filter_action_idx,
            cancel_action,
            close_action,
            repo_model: RepositoryModel::new(path),
            table_state: TableState::default(),
            focused_column: Column::Name,
            app_state: AppState::Normal,
            page_size: 10,
        }
    }

    pub fn filter(&self) -> &str {
        self.repo_model.filter()
    }

    pub fn set_filter(&mut self, value: &str) {
        self.repo_model.set_filter(value);
        let visible_branch_count = self.branches().len();
        if visible_branch_count == 0 {
            self.table_state.select(None);
        } else {
            match self.table_state.selected() {
                None => {
                    if visible_branch_count > 0 {
                        self.table_state.select(Some(0));
                    }
                }
                Some(x) => {
                    if x >= visible_branch_count {
                        self.table_state.select(Some(visible_branch_count - 1));
                    }
                }
            }
        }
    }

    pub fn branches(&self) -> &Vec<Branch> {
        self.repo_model.branches()
    }

    pub fn branches_containing(&self, branch: &str) -> Option<&Vec<String>> {
        self.repo_model.branches_containing(branch)
    }

    pub fn update(&mut self) {
        self.repo_model.update();
        if let AppState::RunningTask { task, on_success } = &mut self.app_state {
            task.update();
            if task.success() == Some(true) {
                log::info!("Running task successful");
                let cmd = *on_success;
                self.process_command(cmd);
            }
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
                if x > 0 {
                    self.table_state.select(Some(x - 1));
                }
            }
            None => self.table_state.select(Some(self.branches().len() - 1)),
        };
    }

    pub fn move_down(&mut self) {
        match self.table_state.selected() {
            Some(x) => {
                if x < self.branches().len() - 1 {
                    self.table_state.select(Some(x + 1));
                }
            }
            None => self.table_state.select(Some(0)),
        };
    }

    pub fn focus_previous_column(&mut self) {
        self.focused_column = self.focused_column.prev();
    }

    pub fn focus_next_column(&mut self) {
        self.focused_column = self.focused_column.next();
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

    pub fn move_start(&mut self) {
        self.table_state.select(Some(0));
    }

    pub fn move_end(&mut self) {
        self.table_state.select(Some(self.branches().len() - 1));
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
            .name
            .clone();
        if let Err(error) = self.repo_model.delete_branch(name) {
            self.app_state = AppState::Error(format!("{}", error));
            return;
        }
        // Select the previous branch if we were on the last one
        let nb_branches = self.branches().len();
        if self.table_state.selected() == Some(nb_branches) {
            self.table_state.select(Some(nb_branches - 1));
        }
        self.app_state = AppState::Normal;
    }

    pub fn sync(&mut self) {
        let mut task: Box<dyn Task> = Box::new(self.repo_model.start_syncing());
        task.start();
        self.app_state = AppState::RunningTask {
            task,
            on_success: Command::BackToNormal,
        };
    }

    pub fn stop_task(&mut self) {
        self.app_state = AppState::Normal;
        if let Err(error) = self.repo_model.update_branches() {
            self.app_state = AppState::Error(format!("{}", error));
        }
    }

    pub fn sort_by(&self) -> SortBy {
        self.repo_model.sort_by()
    }

    pub fn set_sort_by(&mut self, sort_by: SortBy) {
        self.repo_model.set_sort_by(sort_by)
    }

    /// Return a reason to confirm deletion if it's not safe to delete the branch
    pub fn get_reason_to_confirm_delete(&self) -> Option<String> {
        let branch = self.current_branch().unwrap();

        // If another branch contains `branch`, no need to confirm delete
        let mut is_contained: Option<bool> = None;
        while is_contained.is_none() {
            // Maybe show a waiting popup?
            thread::sleep(Duration::from_millis(100));
            is_contained = self
                .repo_model
                .branches_containing(&branch.name)
                .map(|x| !x.is_empty());
        }
        if is_contained == Some(true) {
            return None;
        }

        let Some(ref upstream) = branch.upstream else {
            return Some("It has never been pushed.".into());
        };
        let Some(ref ahead_behind) = upstream.ahead_behind else {
            // TODO: maybe return None for gone branches?
            return Some("Its upstream branch has been deleted.".into());
        };
        match ahead_behind.status() {
            AheadBehindStatus::UpToDate | AheadBehindStatus::Behind => None,
            AheadBehindStatus::Diverged => Some("It has diverged from its upstream branch.".into()),
            AheadBehindStatus::Ahead => Some("It is ahead of its upstream branch.".into()),
        }
    }

    pub fn confirm_delete(&mut self, reason: String) {
        self.app_state = AppState::Confirm {
            message: format!("Are you sure you want to delete this branch?\n\n{reason}"),
            on_cancel: self.cancel_action.clone(),
            on_confirm: create_confirm_action("Delete branch".into(), Command::DoDeleteBranch),
        };
    }

    pub fn process_command(&mut self, command: Command) {
        log::info!("Processing command {:?}", command);
        match command {
            Command::Checkout => self.checkout(),
            Command::Quit => self.quit(),
            Command::Filter => self.app_state = AppState::EditFilter,
            Command::AskDeleteBranch => {
                if let Some(reason) = self.get_reason_to_confirm_delete() {
                    self.confirm_delete(reason)
                } else {
                    self.delete()
                }
            }
            Command::Sync => self.sync(),
            Command::Sort => {
                let mut sort_by = self.sort_by();
                if self.focused_column == sort_by.column {
                    sort_by.ascending = !sort_by.ascending;
                } else {
                    sort_by.column = self.focused_column;
                }
                self.set_sort_by(sort_by);
            }
            Command::BackToNormal => {
                self.app_state = AppState::Normal;
            }
            Command::DoDeleteBranch => {
                self.delete();
            }
        }
    }
}

#[cfg(test)]
mod test {
    use git::{Repository, INITIAL_BRANCH};

    use crate::model::Model;

    fn create_empty_commit(repo: &Repository) {
        repo.git("commit", &["-m", "empty", "--allow-empty"])
            .unwrap();
    }

    #[test]
    fn delete_last_branch() {
        // GIVEN a source repository with two branches
        let tmp_dir = assert_fs::TempDir::new().unwrap();

        let repo = Repository::new(&tmp_dir);
        repo.init().unwrap();
        create_empty_commit(&repo);
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

    #[test]
    fn filter_out_current_branch() {
        // GIVEN a source repository with 3 branches
        let tmp_dir = assert_fs::TempDir::new().unwrap();

        let repo = Repository::new(&tmp_dir);
        repo.init().unwrap();
        create_empty_commit(&repo);
        repo.git("branch", &["-m", "x1"]).unwrap();
        repo.create_branch("x2").unwrap();
        repo.create_branch("y").unwrap();

        // AND a model on this repo
        let mut model = Model::new(&tmp_dir);
        model.update_branches().unwrap();
        assert_eq!(model.branches().len(), 3);

        // AND the 3rd one is selected
        model.table_state.select(Some(2));
        let branch = model.current_branch().unwrap();
        assert_eq!(branch.name, "y");

        // WHEN the filter hides the 3rd branch
        model.set_filter("x");

        // THEN the 2nd branch is selected
        let branch = model.current_branch().unwrap();
        assert_eq!(branch.name, "x2");
    }

    #[test]
    fn filter_out_all_branches() {
        // GIVEN a source repository
        let tmp_dir = assert_fs::TempDir::new().unwrap();

        let repo = Repository::new(&tmp_dir);
        repo.init().unwrap();
        repo.git("branch", &["-m", "x"]).unwrap();
        create_empty_commit(&repo);

        // AND a model on this repo
        let mut model = Model::new(&tmp_dir);
        model.update_branches().unwrap();
        assert_eq!(model.branches().len(), 1);

        // AND the branch is selected
        model.table_state.select(Some(0));
        let branch = model.current_branch().unwrap();
        assert_eq!(branch.name, "x");

        // WHEN the filter hides the branch
        model.set_filter("y");

        // THEN no branches are selected
        assert_eq!(model.current_branch(), None);
    }
}
