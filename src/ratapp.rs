// SPDX-FileCopyrightText: 2026 Aurélien Gâteau <mail@agateau.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::io;
use std::path::Path;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Row, Table};
use ratatui::Frame;

use git::{AheadBehind, AheadBehindStatus, CheckoutState, Upstream};

use crate::cliargs::CliArgs;
use crate::model::{AppState, Command, Model};
use crate::repositorymodel::Column;
use crate::ui::action::Action;
use crate::ui::{popup::Popup, uiutils};

// Wait that long for an input event before redrawing the screen
const EVENT_POLL_DURATION: Duration = Duration::from_millis(32);

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
        while !matches!(self.model.app_state, AppState::Exiting) {
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
                    CheckoutState::WorkTree(_) => "+",
                };
                let (upstream_str, status_str): (String, &str) = match &branch.upstream {
                    None => (String::new(), EMPTY_STR),
                    Some(Upstream { name, ahead_behind }) => {
                        (name.clone(), get_ahead_behind_str(ahead_behind))
                    }
                };
                let contained_in_str: String = match self.model.branches_containing(&branch.name) {
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
                    uiutils::format_datetime(&branch.last_commit_date),
                    status_str.into(),
                    upstream_str,
                    contained_in_str,
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
            // Upstream
            Constraint::Fill(1),
            // Contained in
            Constraint::Fill(1),
        ];

        let sort_by = self.model.sort_by();
        let format_column = |text: &str, column: Column| {
            let is_current_column = sort_by.column == column;
            let indicator = if is_current_column {
                if sort_by.ascending {
                    " ▲"
                } else {
                    " ▼"
                }
            } else {
                ""
            };
            let txt = format!("{text}{indicator}");
            if matches!(self.model.app_state, AppState::EditSort) && is_current_column {
                Span::raw(txt).yellow()
            } else {
                Span::raw(txt)
            }
        };

        let table = Table::new(rows, widths)
            .column_spacing(2)
            .header(
                Row::new(vec![
                    Span::raw(" "),
                    format_column("Name", Column::Name),
                    format_column("Last commit", Column::LastCommit),
                    format_column("Status", Column::Status),
                    Span::raw("Upstream"),
                    Span::raw("Contained in"),
                ])
                .style(Style::new().bold().blue()),
            )
            .row_highlight_style(Style::new().white().on_dark_gray());

        frame.render_stateful_widget(table, area, &mut self.model.table_state);
    }

    fn render_toolbar(&mut self, frame: &mut Frame, area: Rect) {
        uiutils::render_toolbar(frame, area, &self.model.actions);
    }

    fn render_progress_popup(&mut self, frame: &mut Frame) {
        let AppState::RunningTask { ref task } = self.model.app_state else {
            return;
        };
        let action = if task.success().is_some() {
            &self.model.close_action
        } else {
            &self.model.cancel_action
        };
        let popup = Popup::default()
            .actions(vec![action.clone()])
            .title(task.title())
            .content(Text::raw(task.output()));

        let frame_area = frame.area();
        let margin = 2;
        let area = Rect {
            x: frame_area.x + margin,
            y: frame_area.y + margin,
            width: frame_area.width - 2 * margin,
            height: frame_area.height - 2 * margin,
        };

        frame.render_widget(popup, area);
    }

    fn render_error_popup(&mut self, frame: &mut Frame) {
        let AppState::Error(ref error) = self.model.app_state else {
            return;
        };
        let popup = Popup::default()
            .actions(vec![self.model.close_action.clone()])
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

    fn render_confirm_popup(&mut self, frame: &mut Frame) {
        let AppState::Confirm {
            ref message,
            ref on_confirm,
            ref on_cancel,
        } = self.model.app_state
        else {
            return;
        };
        let popup = Popup::default()
            .actions(vec![on_cancel.clone(), on_confirm.clone()])
            .title("Confirm")
            .content(Text::raw(message));

        let frame_area = frame.area();
        let area = Rect {
            x: frame_area.width / 3,
            y: frame_area.height / 4,
            width: frame_area.width / 3,
            height: frame_area.height / 4,
        };

        frame.render_widget(popup, area);
    }

    fn render_filter_bar(&mut self, frame: &mut Frame, area: Rect, filter: String) {
        frame.render_widget(Line::from(format!("Filter: {}▎", &filter)), area);
    }

    fn render_sort_bar(&mut self, frame: &mut Frame, area: Rect) {
        let actions: Vec<Action<()>> = vec![
            Action::new("Previous".into(), KeyCode::Left, ()),
            Action::new("Next".into(), KeyCode::Right, ()),
            Action::new("Ascending".into(), KeyCode::Up, ()),
            Action::new("Descending".into(), KeyCode::Down, ()),
            Action::new("Done".into(), KeyCode::Esc, ()),
        ];
        uiutils::render_toolbar(frame, area, &actions);
    }

    fn draw(&mut self, frame: &mut Frame) {
        let [content, footer] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());

        self.render_branch_table(frame, content);
        self.render_progress_popup(frame);
        self.render_error_popup(frame);
        self.render_confirm_popup(frame);

        // Toolbar
        match &self.model.app_state {
            AppState::EditFilter => {
                self.render_filter_bar(frame, footer, self.model.filter().into())
            }
            AppState::EditSort => self.render_sort_bar(frame, footer),
            AppState::Normal => self.render_toolbar(frame, footer),
            _ => {}
        };
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if event::poll(EVENT_POLL_DURATION)? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    self.handle_key_event(key_event)
                }
                _ => {}
            };
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match &self.model.app_state {
            AppState::Error(_) => {
                self.handle_error_key_event(key_event);
            }
            AppState::Normal => {
                self.handle_action_key_event(key_event, &self.model.actions.clone());
                self.handle_branch_table_navigation_key_event(key_event);
            }
            AppState::EditFilter => {
                self.handle_edit_filter_key_event(key_event);
            }
            AppState::EditSort => {
                self.handle_edit_sort_key_event(key_event);
            }
            AppState::Confirm {
                on_cancel,
                on_confirm,
                ..
            } => {
                self.handle_action_key_event(key_event, &[on_cancel.clone(), on_confirm.clone()]);
            }
            AppState::Exiting => {}
            AppState::RunningTask { task: _ } => {
                self.handle_running_task_key_event(key_event);
            }
        }
    }

    fn handle_action_key_event(&mut self, key_event: KeyEvent, actions: &[Action<Command>]) {
        for action in actions {
            if action.keycode == key_event.code {
                if action.enabled {
                    self.model.process_command(action.command);
                }
                return;
            }
        }
    }

    fn handle_branch_table_navigation_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Up => self.model.move_up(),
            KeyCode::Down => self.model.move_down(),
            KeyCode::PageUp => self.model.page_up(),
            KeyCode::PageDown => self.model.page_down(),
            KeyCode::Home => self.model.move_start(),
            KeyCode::End => self.model.move_end(),
            _ => {}
        }
    }

    fn handle_error_key_event(&mut self, key_event: KeyEvent) {
        if self.model.close_action.keycode == key_event.code {
            self.model.app_state = AppState::Normal;
        }
    }

    fn handle_edit_filter_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char(ch) => {
                let mut filter = self.model.filter().to_string();
                filter.push(ch);
                self.model.set_filter(&filter);
            }
            KeyCode::Backspace => {
                if !self.model.filter().is_empty() {
                    let filter = self.model.filter()[..self.model.filter().len() - 1].to_string();
                    self.model.set_filter(&filter);
                }
            }
            KeyCode::Enter => {
                self.model.app_state = AppState::Normal;
            }
            KeyCode::Esc => {
                self.model.set_filter("");
                self.model.app_state = AppState::Normal;
            }
            _ => {}
        }
    }

    fn handle_edit_sort_key_event(&mut self, key_event: KeyEvent) {
        let mut sort_by = self.model.sort_by();
        match key_event.code {
            KeyCode::Left => {
                sort_by.column = sort_by.column.prev();
                self.model.set_sort_by(sort_by);
            }
            KeyCode::Right => {
                sort_by.column = sort_by.column.next();
                self.model.set_sort_by(sort_by);
            }
            KeyCode::Up => {
                sort_by.ascending = true;
                self.model.set_sort_by(sort_by);
            }
            KeyCode::Down => {
                sort_by.ascending = false;
                self.model.set_sort_by(sort_by);
            }
            KeyCode::Enter | KeyCode::Esc => {
                self.model.app_state = AppState::Normal;
            }
            _ => {}
        }
    }

    fn handle_running_task_key_event(&mut self, key_event: KeyEvent) {
        if key_event.code == self.model.cancel_action.keycode {
            self.model.stop_task();
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
