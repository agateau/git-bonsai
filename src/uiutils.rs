use crossterm::event::KeyCode;
use ratatui::style::Style;
use ratatui::text::Span;

pub const DIM_STYLE: Style = Style::new().dark_gray();

/// Create a styled vec of spans representing an action and its shortcut
pub fn create_action_spans<'a>(name: &'a str, keycode: KeyCode, enabled: bool) -> Vec<Span<'a>> {
    let text_style = Style::new();
    let shortcut_style = Style::new().yellow();

    vec![
        Span::styled("┤", DIM_STYLE),
        Span::styled(name, if enabled { text_style } else { DIM_STYLE }),
        Span::styled(" (", DIM_STYLE),
        Span::styled(
            format!("{}", keycode),
            if enabled { shortcut_style } else { DIM_STYLE },
        ),
        Span::styled(")├", DIM_STYLE),
    ]
}
