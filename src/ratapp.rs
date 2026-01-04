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

use crate::cliargs::CliArgs;
use crate::git::{
    AheadBehind, AheadBehindStatus, Branch, CheckoutState, GitResult, Repository, Upstream,
};
use crate::popup::Popup;

trait Action {
    fn name(&self) -> &str;
    fn keycode(&self) -> KeyCode;
    fn is_enabled(&self, model: &Model) -> bool;
    fn trigger(&self, model: &mut Model);
}

#[derive(Default)]
struct QuitAction;

impl Action for QuitAction {
    fn name(&self) -> &str {
        "Quit"
    }

    fn keycode(&self) -> KeyCode {
        KeyCode::Char('q')
    }

    fn is_enabled(&self, _model: &Model) -> bool {
        true
    }

    fn trigger(&self, model: &mut Model) {
        model.exit = true;
    }
}

#[derive(Default)]
struct CheckoutAction;

impl Action for CheckoutAction {
    fn name(&self) -> &str {
        "Checkout"
    }

    fn keycode(&self) -> KeyCode {
        KeyCode::Char('c')
    }

    fn is_enabled(&self, model: &Model) -> bool {
        let branch = model.current_branch();
        branch.is_some_and(|x| x.checkout_state == CheckoutState::NotCheckedOut)
    }

    fn trigger(&self, model: &mut Model) {
        if let Err(error) = model.checkout() {
            model.model_state = ModelState::Error(format!("{}", error));
        }
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
enum ModelState {
    Normal,
    Error(String),
}

struct Model {
    path: PathBuf,
    table_state: TableState,
    branches: Vec<Branch>,
    exit: bool,
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

    fn checkout(&mut self) -> GitResult<()> {
        let repo = Repository::new(&self.path);
        let name = &self
            .current_branch()
            .expect("checkout should not be callable without an active branch")
            .name;
        repo.checkout(name)?;
        self.update_branches()?;
        Ok(())
    }
}

struct App {
    actions: Vec<Box<dyn Action>>,
    model: Model,
}

impl App {
    fn new(_cli_args: CliArgs, path: &Path) -> Self {
        Self {
            actions: vec![Box::new(CheckoutAction), Box::new(QuitAction)],
            model: Model {
                path: path.into(),
                table_state: TableState::default(),
                branches: vec![],
                exit: false,
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
        while !self.model.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
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
        let dim_style = Style::new().dark_gray();
        let text_style = Style::new();
        let shortcut_style = Style::new().yellow();

        let spans: Vec<Span> = self
            .actions
            .iter()
            .flat_map(|x| {
                let enabled = x.is_enabled(&self.model);

                let action_spans: Vec<Span> = vec![
                    Span::styled("[", dim_style),
                    Span::styled(x.name(), if enabled { text_style } else { dim_style }),
                    Span::styled(" (", dim_style),
                    Span::styled(
                        format!("{}", x.keycode()),
                        if enabled { shortcut_style } else { dim_style },
                    ),
                    Span::styled(")] ", dim_style),
                ];
                action_spans
            })
            .collect();

        let toolbar = Line::from(spans);
        frame.render_widget(toolbar, area);
    }

    fn render_error_message(&mut self, frame: &mut Frame) {
        let ModelState::Error(ref error) = self.model.model_state else {
            return;
        };
        let popup = Popup::default().title("Error").content(Text::raw(error));

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
            if action.keycode() == key_event.code {
                if action.is_enabled(&self.model) {
                    action.trigger(&mut self.model);
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
        if matches!(key_event.code, KeyCode::Esc | KeyCode::Char('q')) {
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
