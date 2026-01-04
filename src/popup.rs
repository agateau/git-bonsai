use crossterm::event::KeyCode;
use derive_setters::Setters;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget, Wrap},
};

use crate::uiutils::{self, DIM_STYLE};

#[derive(Debug, Default, Setters)]
pub struct Popup<'a> {
    #[setters(into)]
    title: Line<'a>,
    #[setters(into)]
    content: Text<'a>,
    title_style: Style,
    style: Style,
}

impl Widget for Popup<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let close_spans = uiutils::create_action_spans("Close", KeyCode::Esc, true);
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
