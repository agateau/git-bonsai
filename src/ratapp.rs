// SPDX-FileCopyrightText: 2026 Aurélien Gâteau <mail@agateau.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::io;
use std::path::Path;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Row, Table};
use ratatui::Frame;

use crate::action::DIM_STYLE;
use crate::cliargs::CliArgs;
use crate::git::{AheadBehind, AheadBehindStatus, CheckoutState, Upstream};
use crate::model::{AppState, Command, Model};
use crate::popup::Popup;

// If the branch is contained in more than this number of branches, show "and N others"
const MAX_CONTAINED_IN_BRANCHES: usize = 2;

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

struct App {
    model: Model,
}

impl App {
    fn new(_cli_args: CliArgs, path: &Path) -> Self {
        Self {
            model: Model::new(path),
        }
    }

    fn run(&mut self) -> io::Result<()> {
        self.model
            .update_branches()
            .unwrap_or_else(|x| panic!("Listing branches failed: {}", x));
        if !self.model.branches().is_empty() {
            self.model.table_state.select(Some(0));
        }
        let mut terminal = ratatui::init();
        while self.model.app_state != AppState::Exiting {
            self.model.update();
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render_branch_table(&mut self, frame: &mut Frame, area: Rect) {
        self.model.page_size = (area.height - 1) as usize;

        let rows: Vec<_> = self
            .model
            .branches()
            .iter()
            .map(|branch| {
                let checkout_symbol = match branch.checkout_state {
                    CheckoutState::NotCheckedOut => " ",
                    CheckoutState::Current => "*",
                    CheckoutState::WorkTree => "+",
                };
                let (upstream_str, status_str): (String, &str) = match &branch.upstream {
                    None => (String::new(), EMPTY_STR),
                    Some(Upstream { name, ahead_behind }) => {
                        (name.clone(), get_ahead_behind_str(ahead_behind))
                    }
                };
                let contained_in_str: String = match self.model.branches_contained_in(&branch.name)
                {
                    // We don't know yet
                    None => "...".into(),
                    // We have the info
                    Some(contained_in) => match contained_in.len() {
                        0 => "".into(),
                        1..=MAX_CONTAINED_IN_BRANCHES => contained_in.join(", "),
                        _ => {
                            format!(
                                "{} and {} other(s)",
                                contained_in
                                    .get(..MAX_CONTAINED_IN_BRANCHES)
                                    .unwrap()
                                    .join(", "),
                                contained_in.len() - MAX_CONTAINED_IN_BRANCHES
                            )
                        }
                    },
                };
                let cells: Vec<String> = vec![
                    checkout_symbol.into(),
                    branch.name.clone(),
                    branch.last_commit_date.clone(),
                    status_str.into(),
                    contained_in_str,
                    upstream_str,
                ];
                Row::new(cells)
            })
            .collect();

        let widths = [
            // Checkout state
            Constraint::Length(1),
            // Name
            Constraint::Fill(2),
            // Last commit
            Constraint::Length(30),
            // Status
            Constraint::Length(10),
            // Contained in
            Constraint::Fill(1),
            // Upstream
            Constraint::Fill(1),
        ];

        let table = Table::new(rows, widths)
            .column_spacing(2)
            .header(
                Row::new(vec![
                    " ",
                    "Name",
                    "Last commit",
                    "Status",
                    "Contained in",
                    "Upstream",
                ])
                .style(Style::new().bold().blue()),
            )
            .row_highlight_style(Style::new().white().on_dark_gray());

        frame.render_stateful_widget(table, area, &mut self.model.table_state);
    }

    fn render_toolbar(&mut self, frame: &mut Frame, area: Rect) {
        let spans: Vec<Span> = self
            .model
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
        let AppState::Error(ref error) = self.model.app_state else {
            return;
        };
        let popup = Popup::new(&self.model.close_popup_action)
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
        if matches!(self.model.app_state, AppState::Error(_)) {
            self.handle_error_key_event(key_event);
            return;
        }
        for action in &self.model.actions {
            if action.keycode == key_event.code {
                if action.enabled {
                    match action.command {
                        Command::Checkout => self.model.checkout(),
                        Command::Quit => self.model.quit(),
                        Command::Delete => self.model.delete(),
                        _ => panic!("Unexpected command: {:?}", action.command),
                    }
                }
                return;
            }
        }
        match key_event.code {
            KeyCode::Up => self.model.move_up(),
            KeyCode::Down => self.model.move_down(),
            KeyCode::PageUp => self.model.page_up(),
            KeyCode::PageDown => self.model.page_down(),
            _ => {}
        }
    }

    fn handle_error_key_event(&mut self, key_event: KeyEvent) {
        if self.model.close_popup_action.keycode == key_event.code {
            self.model.app_state = AppState::Normal;
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
