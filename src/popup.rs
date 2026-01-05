use derive_setters::Setters;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget, Wrap},
};

use crate::action::{Action, DIM_STYLE};

#[derive(Debug, Setters)]
pub struct Popup<'a, T> {
    #[setters(into)]
    title: Line<'a>,
    #[setters(into)]
    content: Text<'a>,
    title_style: Style,
    style: Style,
    close_action: &'a Action<T>,
}

impl<'a, T> Popup<'a, T> {
    pub fn new(close_action: &'a Action<T>) -> Self {
        Self {
            title: Line::default(),
            content: Text::default(),
            title_style: Style::default(),
            style: Style::default(),
            close_action,
        }
    }
}

impl<T> Widget for Popup<'_, T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let close_spans = self.close_action.create_spans();
        let block = Block::new()
            .title(Line::from(format!(" {} ", self.title)).centered())
            .title_bottom(Line::from(close_spans).right_aligned())
            .title_style(self.title_style)
            .borders(Borders::ALL)
            .border_style(DIM_STYLE)
            .border_type(BorderType::Rounded);
        Paragraph::new(self.content)
            .wrap(Wrap { trim: true })
            .style(self.style)
            .block(block)
            .render(area, buf);
    }
}
