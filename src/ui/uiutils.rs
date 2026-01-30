// SPDX-FileCopyrightText: 2026 Aurélien Gâteau <mail@agateau.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::{DateTime, FixedOffset, Utc};

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::Frame;

use crate::ui::action::Action;

pub const DIM_STYLE: Style = Style::new().dark_gray();
// TODO: handle light backgrounds
pub const ACTION_SHORTCUT_STYLE: Style = Style::new().yellow();
pub const ACTION_TEXT_STYLE: Style = Style::new().white();

pub fn format_datetime(datetime: &DateTime<FixedOffset>) -> String {
    let delta = datetime.signed_duration_since(Utc::now());
    if delta.num_days() == 0 {
        return datetime.format("Today, %H:%M").to_string();
    }
    if delta.num_days() > -7 {
        // Less than a week ago
        return datetime.format("%A, %H:%M").to_string();
    }
    if delta.num_days() > -365 {
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

    vec![
        Span::styled("┤", DIM_STYLE),
        Span::styled(
            name,
            if enabled {
                ACTION_TEXT_STYLE
            } else {
                DIM_STYLE
            },
        ),
        Span::styled(" (", DIM_STYLE),
        Span::styled(
            format!("{}", keycode),
            if enabled {
                ACTION_SHORTCUT_STYLE
            } else {
                DIM_STYLE
            },
        ),
        Span::styled(")├", DIM_STYLE),
    ]
}

pub fn create_spans_for_actions<T>(actions: &[Action<T>]) -> Vec<Span<'_>>
where
    T: Clone,
{
    actions
        .iter()
        .flat_map(|x| {
            let mut action_spans = create_spans_for_action(x);
            action_spans.push(Span::styled("─", DIM_STYLE));
            action_spans
        })
        .collect()
}

pub fn render_toolbar<T>(frame: &mut Frame, area: Rect, actions: &[Action<T>])
where
    T: Clone,
{
    let spans = create_spans_for_actions(actions);

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
