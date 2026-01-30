// SPDX-FileCopyrightText: 2026 Aurélien Gâteau <mail@agateau.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::{DateTime, FixedOffset, Utc};

use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::Frame;

use crate::ui::action::Action;

pub const DIM_STYLE: Style = Style::new().dark_gray();
// TODO: handle light backgrounds
pub const ACTION_SHORTCUT_STYLE: Style = Style::new().yellow().bold();
pub const ACTION_TEXT_STYLE: Style = Style::new().white();

pub fn format_datetime(datetime: &DateTime<FixedOffset>) -> String {
    let delta = Utc::now().signed_duration_since(datetime);
    if delta.num_days() == 0 {
        return datetime.format("Today, %H:%M").to_string();
    }
    if delta.num_days() < 7 {
        // Less than a week ago
        return datetime.format("%A, %H:%M").to_string();
    }
    if delta.num_days() < 366 {
        return datetime.format("%b %d, %H:%M").to_string();
    }
    datetime.format("%Y %b %d, %H:%M").to_string()
}

/// Create a styled vec of spans representing an action and its shortcut
pub fn create_spans_for_action<T>(action: &Action<T>) -> Vec<Span<'_>>
where
    T: Clone,
{
    let name = &action.name;
    let keycode = action.keycode;
    let enabled = action.enabled;

    let keycode_str: String = match keycode {
        KeyCode::Left => "🠤".into(),
        KeyCode::Right => "🠦".into(),
        KeyCode::Up => "🠥".into(),
        KeyCode::Down => "🠧".into(),
        KeyCode::Enter => "↵".into(),
        _ => format!("{}", keycode),
    };

    vec![
        Span::raw(" "),
        Span::styled(
            format!("[{}]", keycode_str),
            if enabled {
                ACTION_SHORTCUT_STYLE
            } else {
                DIM_STYLE
            },
        ),
        Span::raw(" "),
        Span::styled(
            name,
            if enabled {
                ACTION_TEXT_STYLE
            } else {
                DIM_STYLE
            },
        ),
        Span::raw(" "),
    ]
}

pub fn create_spans_for_actions<T>(actions: &[Action<T>]) -> Vec<Span<'_>>
where
    T: Clone,
{
    actions
        .iter()
        .map(create_spans_for_action)
        .collect::<Vec<Vec<Span<'_>>>>()
        .join(&Span::styled("╱", DIM_STYLE))
}

pub fn render_toolbar<T>(frame: &mut Frame, area: Rect, actions: &[Action<T>])
where
    T: Clone,
{
    let spans = create_spans_for_actions(actions);

    let toolbar = Line::from(spans);
    frame.render_widget(toolbar, area);
}
