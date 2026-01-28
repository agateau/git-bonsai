use derive_setters::Setters;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget, Wrap},
};

use crate::{action::Action, uiutils};

#[derive(Debug, Setters)]
pub struct Popup<'a, T>
where
    T: Clone,
{
    #[setters(into)]
    title: Line<'a>,
    #[setters(into)]
    content: Text<'a>,
    #[setters()]
    actions: Vec<Action<T>>,
    title_style: Style,
    style: Style,
}

impl<'a, T> Default for Popup<'a, T>
where
    T: Clone,
{
    fn default() -> Self {
        Self {
            title: Line::default(),
            content: Text::default(),
            title_style: Style::default(),
            style: Style::default(),
            actions: vec![],
        }
    }
}

impl<T> Widget for Popup<'_, T>
where
    T: Clone,
{
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let spans = uiutils::create_spans_for_actions(&self.actions);

        let block = Block::new()
            .title(Line::from(format!(" {} ", self.title)).centered())
            .title_bottom(Line::from(spans).right_aligned())
            .title_style(self.title_style)
            .borders(Borders::ALL)
            .border_style(uiutils::DIM_STYLE)
            .border_type(BorderType::Rounded);
        Paragraph::new(self.content)
            .wrap(Wrap { trim: true })
            .style(self.style)
            .block(block)
            .render(area, buf);
    }
}
