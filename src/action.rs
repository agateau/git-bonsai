use crossterm::event::KeyCode;
use ratatui::style::Style;
use ratatui::text::Span;

pub const DIM_STYLE: Style = Style::new().dark_gray();

#[derive(Debug, Clone)]
pub struct Action<T> {
    pub name: String,
    pub keycode: KeyCode,
    pub enabled: bool,
    pub command: T,
}

impl<T> Action<T> {
    pub fn new(name: String, keycode: KeyCode, command: T) -> Self {
        Self {
            name,
            keycode,
            enabled: true,
            command,
        }
    }

    /// Create a styled vec of spans representing an action and its shortcut
    pub fn create_spans(&self) -> Vec<Span<'_>> {
        let name = &self.name;
        let keycode = self.keycode;
        let enabled = self.enabled;
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
}
