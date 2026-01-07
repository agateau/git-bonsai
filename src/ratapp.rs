// SPDX-FileCopyrightText: 2026 Aurélien Gâteau <mail@agateau.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::io;
use std::path::{Path, PathBuf};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Row, Table, TableState};
use ratatui::Frame;

use crate::action::{Action, DIM_STYLE};
use crate::cliargs::CliArgs;
use crate::git::{
    AheadBehind, AheadBehindStatus, Branch, CheckoutState, GitResult, Repository, Upstream,
};
use crate::popup::Popup;

const EMPTY_STR: &str = "";

const AB_GONE: &str = "Gone";
const AB_UP_TO_DATE: &str = "Up-to-date";
const AB_DIVERGED: &str = "Diverged";
const AB_BEHIND: &str = "Can be FF";
const AB_AHEAD: &str = "In advance";

fn get_ahead_behind_str(ahead_behind: &Option<AheadBehind>) -> &'static str {
    let Some(ahead_behind) = ahead_behind else {
        return AB_GONE;
    };
    match ahead_behind.status() {
        AheadBehindStatus::UpToDate => AB_UP_TO_DATE,
        AheadBehindStatus::Behind => AB_BEHIND,
        AheadBehindStatus::Ahead => AB_AHEAD,
        AheadBehindStatus::Diverged => AB_DIVERGED,
    }
}

#[derive(Debug)]
pub enum Command {
    Checkout,
    Quit,
    ClosePopup,
}

/// Global state of the application
#[derive(Debug, Clone, PartialEq, Eq)]
enum ModelState {
    /// Default state, showing branches
    Normal,
    /// Showing an error message
    Error(String),
    Exiting,
}

struct Model {
    path: PathBuf,
    table_state: TableState,
    branches: Vec<Branch>,
    model_state: ModelState,
}

impl Model {
    fn update_branches(&mut self) -> GitResult<()> {
        let repo = Repository::new(&self.path);
        self.branches = repo.list_branches()?;
        Ok(())
    }

    fn current_branch(&self) -> Option<&Branch> {
        self.table_state.selected().map(|x| &self.branches[x])
    }

    fn move_up(&mut self) {
        match self.table_state.selected() {
            Some(x) => {
                let x = if x == 0 {
                    self.branches.len() - 1
                } else {
                    x - 1
                };
                self.table_state.select(Some(x));
            }
            None => self.table_state.select(Some(self.branches.len() - 1)),
        };
    }

    fn move_down(&mut self) {
        match self.table_state.selected() {
            Some(x) => {
                let x = if x < self.branches.len() - 1 {
                    x + 1
                } else {
                    0
                };
                self.table_state.select(Some(x));
            }
            None => self.table_state.select(Some(0)),
        };
    }

    fn checkout(&mut self) {
        let repo = Repository::new(&self.path);
        let name = &self
            .current_branch()
            .expect("checkout should not be callable without an active branch")
            .name;
        if let Err(error) = repo.checkout(name) {
            self.model_state = ModelState::Error(format!("{}", error));
            return;
        }
        self.update_branches()
            .expect("update_branches() should not fail after a successful checkout");
    }

    fn quit(&mut self) {
        self.model_state = ModelState::Exiting;
    }
}

struct App {
    actions: Vec<Action<Command>>,
    checkout_action_idx: usize,
    close_popup_action: Action<Command>,
    model: Model,
}

impl App {
    fn new(_cli_args: CliArgs, path: &Path) -> Self {
        let mut actions: Vec<Action<Command>> = vec![];
        actions.push(Action::new(
            "Checkout".into(),
            KeyCode::Char('c'),
            Command::Checkout,
        ));
        let checkout_action_idx = actions.len();
        actions.push(Action::new(
            "Quit".into(),
            KeyCode::Char('q'),
            Command::Quit,
        ));
        let close_popup_action = Action::new("Close".into(), KeyCode::Esc, Command::ClosePopup);
        Self {
            actions,
            checkout_action_idx,
            close_popup_action,
            model: Model {
                path: path.into(),
                table_state: TableState::default(),
                branches: vec![],
                model_state: ModelState::Normal,
            },
        }
    }

    fn run(&mut self) -> io::Result<()> {
        self.model
            .update_branches()
            .unwrap_or_else(|x| panic!("Listing branches failed: {}", x));
        if !self.model.branches.is_empty() {
            self.model.table_state.select(Some(0));
        }
        let mut terminal = ratatui::init();
        while self.model.model_state != ModelState::Exiting {
            self.update();
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn update(&mut self) {
        let branch = self.model.current_branch();
        let can_checkout = branch.is_some_and(|x| x.checkout_state == CheckoutState::NotCheckedOut);
        self.actions[self.checkout_action_idx].enabled = can_checkout;
    }

    fn render_branch_table(&mut self, frame: &mut Frame, area: Rect) {
        let rows: Vec<_> = self
            .model
            .branches
            .iter()
            .map(|branch| {
                let checkout_symbol = match branch.checkout_state {
                    CheckoutState::NotCheckedOut => " ",
                    CheckoutState::Current => "*",
                    CheckoutState::WorkTree => "+",
                };
                let (upstream_str, status_str) = match &branch.upstream {
                    None => (EMPTY_STR, EMPTY_STR),
                    Some(Upstream { name, ahead_behind }) => {
                        (name.as_ref(), get_ahead_behind_str(ahead_behind))
                    }
                };
                Row::new(vec![
                    checkout_symbol,
                    &branch.name,
                    &branch.last_commit_date,
                    status_str,
                    upstream_str,
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(1),
            Constraint::Length(25),
            Constraint::Length(35),
            Constraint::Length(20),
            Constraint::Length(25),
        ];

        let table = Table::new(rows, widths)
            .column_spacing(2)
            .header(
                Row::new(vec![" ", "Name", "Last commit", "Status", "Upstream"])
                    .style(Style::new().bold().green()),
            )
            .row_highlight_style(Style::new().reversed());

        frame.render_stateful_widget(table, area, &mut self.model.table_state);
    }

    fn render_toolbar(&mut self, frame: &mut Frame, area: Rect) {
        let spans: Vec<Span> = self
            .actions
            .iter()
            .flat_map(|x| {
                let mut action_spans = x.create_spans();
                action_spans.push(Span::styled("─", DIM_STYLE));
                action_spans
            })
            .collect();

        let toolbar = Line::from(spans);
        let toolbar_end = toolbar.width() as u16;
        frame.render_widget(toolbar, area);

        let padding = area.width - toolbar_end;
        frame.render_widget(
            Line::styled("─".repeat(padding as usize), DIM_STYLE),
            Rect {
                x: toolbar_end,
                y: area.y,
                width: padding,
                height: 1,
            },
        );
    }

    fn render_error_message(&mut self, frame: &mut Frame) {
        let ModelState::Error(ref error) = self.model.model_state else {
            return;
        };
        let popup = Popup::new(&self.close_popup_action)
            .title("Error")
            .content(Text::raw(error));

        let frame_area = frame.area();
        let area = Rect {
            x: frame_area.width / 3,
            y: frame_area.height / 4,
            width: frame_area.width / 3,
            height: frame_area.height / 4,
        };

        frame.render_widget(popup, area);
    }

    fn draw(&mut self, frame: &mut Frame) {
        let [content, footer] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());

        self.render_branch_table(frame, content);
        self.render_error_message(frame);
        self.render_toolbar(frame, footer);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if matches!(self.model.model_state, ModelState::Error(_)) {
            self.handle_error_key_event(key_event);
            return;
        }
        for action in &self.actions {
            if action.keycode == key_event.code {
                if action.enabled {
                    match action.command {
                        Command::Checkout => self.model.checkout(),
                        Command::Quit => self.model.quit(),
                        _ => panic!("Unexpected command: {:?}", action.command),
                    }
                }
                return;
            }
        }
        match key_event.code {
            KeyCode::Up => self.model.move_up(),
            KeyCode::Down => self.model.move_down(),
            _ => {}
        }
    }

    fn handle_error_key_event(&mut self, key_event: KeyEvent) {
        if self.close_popup_action.keycode == key_event.code {
            self.model.model_state = ModelState::Normal;
        }
    }
}

pub fn run(args: CliArgs, dir: &str) -> i32 {
    let mut app = App::new(args, Path::new(dir));
    let result = app.run();
    ratatui::restore();
    match result {
        Ok(()) => 0,
        Err(x) => {
            eprintln!("Error: {}", x);
            1
        }
    }
}
